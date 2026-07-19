#[cfg(target_os = "android")]
mod platform {
    use std::sync::mpsc;

    use jni::objects::{JClass, JString, JValue};
    use tauri::{AppHandle, Manager};

    use crate::asr_manager::asr_apply_corrections;
    use crate::asr_manager::audio::{
        canonicalize_existing_path, decode_pcm_bytes, decode_wav_file, downmix_to_mono,
        resample_to_16khz, ASR_SAMPLE_RATE,
    };
    use crate::asr_manager::engine::{
        AsrModelRef, AsrRuntimeLoadRequest, AsrTranscribeFileRequest, AsrTranscribePcmRequest,
    };
    use crate::asr_manager::runtime::{AsrWhisperSegment, AsrWhisperTranscriptionResponse};

    fn normalized_language(language: Option<&str>) -> String {
        match language.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) if value.eq_ignore_ascii_case("auto") => "auto".to_string(),
            Some(value) if value.to_ascii_lowercase().starts_with("zh") => "zh".to_string(),
            Some(value) if value.to_ascii_lowercase().starts_with("yue") => "yue".to_string(),
            Some(value) if value.to_ascii_lowercase().starts_with("en") => "en".to_string(),
            Some(value) if value.to_ascii_lowercase().starts_with("ja") => "ja".to_string(),
            Some(value) if value.to_ascii_lowercase().starts_with("ko") => "ko".to_string(),
            Some(_) | None => "auto".to_string(),
        }
    }

    fn resolved_model(model: &AsrModelRef) -> Result<(String, String), String> {
        let model_path = canonicalize_existing_path(&model.path)?;
        let tokens = model
            .tokens_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| {
                crate::utils::err_msg(module_path!(), line!(), "SenseVoice requires a tokens file")
            })?;
        let tokens_path = canonicalize_existing_path(tokens)?;
        Ok((
            model_path.to_string_lossy().to_string(),
            tokens_path.to_string_lossy().to_string(),
        ))
    }

    fn transcribe_bridge(
        app: &AppHandle,
        model_path: String,
        tokens_path: String,
        language: String,
        threads: i32,
        samples: Vec<f32>,
    ) -> Result<String, String> {
        call_bridge(app, move |env, class| {
            let model = env.new_string(model_path)?;
            let tokens = env.new_string(tokens_path)?;
            let language = env.new_string(language)?;
            let audio = env.new_float_array(samples.len() as i32)?;
            env.set_float_array_region(&audio, 0, &samples)?;
            let value = env.call_static_method(
                class,
                "transcribe",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I[FI)Ljava/lang/String;",
                &[
                    JValue::Object(&model),
                    JValue::Object(&tokens),
                    JValue::Object(&language),
                    JValue::Int(threads),
                    JValue::Object(&audio),
                    JValue::Int(ASR_SAMPLE_RATE as i32),
                ],
            )?;
            let output: String = env.get_string(&JString::from(value.l()?))?.into();
            Ok(output)
        })
    }

    fn preload_bridge(
        app: &AppHandle,
        model_path: String,
        tokens_path: String,
        language: String,
        threads: i32,
    ) -> Result<(), String> {
        call_bridge(app, move |env, class| {
            let model = env.new_string(model_path)?;
            let tokens = env.new_string(tokens_path)?;
            let language = env.new_string(language)?;
            env.call_static_method(
                class,
                "preload",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)V",
                &[
                    JValue::Object(&model),
                    JValue::Object(&tokens),
                    JValue::Object(&language),
                    JValue::Int(threads),
                ],
            )?;
            Ok(())
        })
    }

    fn clear_bridge(app: &AppHandle) -> Result<usize, String> {
        call_bridge(app, |env, class| {
            Ok(env
                .call_static_method(class, "clearCache", "()I", &[])?
                .i()? as usize)
        })
    }

    fn call_bridge<T, F>(app: &AppHandle, callback: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut jni::JNIEnv<'_>, JClass<'_>) -> Result<T, jni::errors::Error>
            + Send
            + 'static,
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "SenseVoice requires the main Android WebView".to_string())?;
        let bridge_name = option_env!("SENSE_VOICE_ANDROID_BRIDGE_CLASS")
            .unwrap_or("com.lettuceai.app.SenseVoiceBridge")
            .to_string();
        let (tx, rx) = mpsc::channel();
        window
            .with_webview(move |webview| {
                webview.jni_handle().exec(move |env, activity, _webview| {
                    let result = (|| -> Result<T, jni::errors::Error> {
                        let class_loader = env
                            .call_method(
                                activity,
                                "getClassLoader",
                                "()Ljava/lang/ClassLoader;",
                                &[],
                            )?
                            .l()?;
                        let class_name = env.new_string(bridge_name)?;
                        let class_obj = env
                            .call_method(
                                &class_loader,
                                "loadClass",
                                "(Ljava/lang/String;)Ljava/lang/Class;",
                                &[JValue::Object(&class_name)],
                            )?
                            .l()?;
                        callback(env, JClass::from(class_obj))
                    })();
                    if env.exception_check().unwrap_or(false) {
                        let _ = env.exception_describe();
                        let _ = env.exception_clear();
                    }
                    let _ = tx.send(result.map_err(|error| error.to_string()));
                });
            })
            .map_err(|error| error.to_string())?;
        rx.recv().map_err(|error| error.to_string())?
    }

    fn run(
        app: &AppHandle,
        request: &AsrTranscribePcmRequest,
        audio_path: String,
        samples: Vec<f32>,
        source_sample_rate: u32,
        source_channels: usize,
    ) -> Result<AsrWhisperTranscriptionResponse, String> {
        let mono = downmix_to_mono(&samples, source_channels.max(1));
        let pcm = resample_to_16khz(&mono, source_sample_rate);
        let (model_path, tokens_path) = resolved_model(&request.model)?;
        let language = normalized_language(request.language.as_deref());
        let threads = request.threads.unwrap_or(4).clamp(1, i32::MAX as usize) as i32;
        let audio_len = pcm.len();

        tracing::info!(
            target: "asr",
            engine = "senseVoice",
            model = %model_path,
            language = %language,
            threads,
            audio_ms = (audio_len as u64 * 1000 / ASR_SAMPLE_RATE as u64),
            "transcription started"
        );
        let started = std::time::Instant::now();
        let raw_text =
            transcribe_bridge(app, model_path, tokens_path, language.clone(), threads, pcm)?
                .trim()
                .to_string();
        tracing::info!(
            target: "asr",
            engine = "senseVoice",
            elapsed_ms = started.elapsed().as_millis() as u64,
            text_len = raw_text.chars().count(),
            "transcription complete"
        );

        let corrected = asr_apply_corrections(
            app.clone(),
            raw_text.clone(),
            request.language.clone(),
            request.scopes.clone(),
        )?;
        let end_ms = audio_len as i64 * 1000 / ASR_SAMPLE_RATE as i64;
        let segments = if raw_text.is_empty() {
            Vec::new()
        } else {
            vec![AsrWhisperSegment {
                index: 0,
                start_ms: 0,
                end_ms,
                text: raw_text.clone(),
                no_speech_probability: 0.0,
                speaker_turn_next: false,
            }]
        };

        if !request.keep_model_loaded.unwrap_or(true) {
            let _ = clear_bridge(app)?;
        }

        Ok(AsrWhisperTranscriptionResponse {
            audio_path,
            model_path: request.model.path.clone(),
            sample_rate_hz: ASR_SAMPLE_RATE,
            prompt: String::new(),
            raw_text,
            corrected_text: corrected.corrected_text,
            detected_language: Some(language),
            segments,
            applied_corrections: corrected.applied,
        })
    }

    pub(crate) fn transcribe_pcm_sync(
        app: &AppHandle,
        request: AsrTranscribePcmRequest,
    ) -> Result<AsrWhisperTranscriptionResponse, String> {
        if request.sample_rate_hz == 0 {
            return Err("Sample rate must be greater than zero".to_string());
        }
        let pcm = decode_pcm_bytes(&request.pcm_bytes)?;
        if pcm.is_empty() {
            return Err("PCM payload is empty".to_string());
        }
        let sample_rate = request.sample_rate_hz;
        let channels = request.channels.unwrap_or(1).max(1) as usize;
        run(app, &request, String::new(), pcm, sample_rate, channels)
    }

    pub(crate) fn transcribe_file_sync(
        app: &AppHandle,
        request: AsrTranscribeFileRequest,
    ) -> Result<AsrWhisperTranscriptionResponse, String> {
        let audio_path = canonicalize_existing_path(&request.audio_path)?;
        let (pcm, sample_rate) = decode_wav_file(&audio_path)?;
        let pcm_request = AsrTranscribePcmRequest {
            model: request.model,
            pcm_bytes: Vec::new(),
            sample_rate_hz: sample_rate,
            channels: Some(1),
            language: request.language,
            scopes: request.scopes,
            initial_prompt: request.initial_prompt,
            translate: request.translate,
            detect_language: request.detect_language,
            no_context: request.no_context,
            single_segment: request.single_segment,
            token_timestamps: request.token_timestamps,
            split_on_word: request.split_on_word,
            max_len: request.max_len,
            max_tokens: request.max_tokens,
            offset_ms: request.offset_ms,
            duration_ms: request.duration_ms,
            threads: request.threads,
            best_of: request.best_of,
            temperature: request.temperature,
            temperature_inc: request.temperature_inc,
            use_gpu: request.use_gpu,
            force_cpu: request.force_cpu,
            keep_model_loaded: request.keep_model_loaded,
            flash_attn: request.flash_attn,
            gpu_device: request.gpu_device,
        };
        run(
            app,
            &pcm_request,
            audio_path.to_string_lossy().to_string(),
            pcm,
            sample_rate,
            1,
        )
    }

    pub(crate) fn preload_model_sync(
        app: &AppHandle,
        request: AsrRuntimeLoadRequest,
    ) -> Result<(), String> {
        let (model_path, tokens_path) = resolved_model(&request.model)?;
        preload_bridge(
            app,
            model_path,
            tokens_path,
            "auto".to_string(),
            request.threads.unwrap_or(4).clamp(1, i32::MAX as usize) as i32,
        )
    }

    pub(crate) fn clear_cache(app: &AppHandle) -> Result<usize, String> {
        clear_bridge(app)
    }
}

#[cfg(not(target_os = "android"))]
mod platform {
    use crate::asr_manager::engine::{
        AsrRuntimeLoadRequest, AsrTranscribeFileRequest, AsrTranscribePcmRequest,
    };
    use crate::asr_manager::runtime::AsrWhisperTranscriptionResponse;

    fn unsupported<T>() -> Result<T, String> {
        Err("SenseVoice is currently available on Android only".to_string())
    }

    pub(crate) fn transcribe_pcm_sync(
        _app: &tauri::AppHandle,
        _request: AsrTranscribePcmRequest,
    ) -> Result<AsrWhisperTranscriptionResponse, String> {
        unsupported()
    }

    pub(crate) fn transcribe_file_sync(
        _app: &tauri::AppHandle,
        _request: AsrTranscribeFileRequest,
    ) -> Result<AsrWhisperTranscriptionResponse, String> {
        unsupported()
    }

    pub(crate) fn preload_model_sync(
        _app: &tauri::AppHandle,
        _request: AsrRuntimeLoadRequest,
    ) -> Result<(), String> {
        unsupported()
    }

    pub(crate) fn clear_cache(_app: &tauri::AppHandle) -> Result<usize, String> {
        Ok(0)
    }
}

pub(crate) use platform::{
    clear_cache, preload_model_sync, transcribe_file_sync, transcribe_pcm_sync,
};

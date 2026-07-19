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

    struct ResolvedModel {
        model: String,
        tokens: String,
        punctuation: String,
    }

    fn resolved_model(model: &AsrModelRef) -> Result<ResolvedModel, String> {
        let required_path = |path: Option<&str>, label: &str| -> Result<String, String> {
            let path = path
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    crate::utils::err_msg(
                        module_path!(),
                        line!(),
                        format!("Zipformer CTC requires a {label} file"),
                    )
                })?;
            Ok(canonicalize_existing_path(path)?
                .to_string_lossy()
                .to_string())
        };
        Ok(ResolvedModel {
            model: canonicalize_existing_path(&model.path)?
                .to_string_lossy()
                .to_string(),
            tokens: required_path(model.tokens_path.as_deref(), "tokens")?,
            punctuation: required_path(model.punctuation_path.as_deref(), "punctuation model")?,
        })
    }

    fn transcribe_bridge(
        app: &AppHandle,
        model: &ResolvedModel,
        threads: i32,
        samples: Vec<f32>,
    ) -> Result<String, String> {
        let model_path = model.model.clone();
        let tokens_path = model.tokens.clone();
        let punctuation_path = model.punctuation.clone();
        call_bridge(app, move |env, class| {
            let model = env.new_string(model_path)?;
            let tokens = env.new_string(tokens_path)?;
            let punctuation = env.new_string(punctuation_path)?;
            let audio = env.new_float_array(samples.len() as i32)?;
            env.set_float_array_region(&audio, 0, &samples)?;
            let value = env.call_static_method(
                class,
                "transcribe",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I[FI)Ljava/lang/String;",
                &[
                    JValue::Object(&model),
                    JValue::Object(&tokens),
                    JValue::Object(&punctuation),
                    JValue::Int(threads),
                    JValue::Object(&audio),
                    JValue::Int(ASR_SAMPLE_RATE as i32),
                ],
            )?;
            Ok(env.get_string(&JString::from(value.l()?))?.into())
        })
    }

    fn punctuation_bridge(
        app: &AppHandle,
        model: &ResolvedModel,
        threads: i32,
        text: String,
    ) -> Result<String, String> {
        let model_path = model.model.clone();
        let tokens_path = model.tokens.clone();
        let punctuation_path = model.punctuation.clone();
        call_bridge(app, move |env, class| {
            let model = env.new_string(model_path)?;
            let tokens = env.new_string(tokens_path)?;
            let punctuation = env.new_string(punctuation_path)?;
            let input = env.new_string(text)?;
            let value = env.call_static_method(
                class,
                "addPunctuation",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;ILjava/lang/String;)Ljava/lang/String;",
                &[
                    JValue::Object(&model),
                    JValue::Object(&tokens),
                    JValue::Object(&punctuation),
                    JValue::Int(threads),
                    JValue::Object(&input),
                ],
            )?;
            Ok(env.get_string(&JString::from(value.l()?))?.into())
        })
    }

    fn preload_bridge(app: &AppHandle, model: ResolvedModel, threads: i32) -> Result<(), String> {
        call_bridge(app, move |env, class| {
            let model_path = env.new_string(model.model)?;
            let tokens = env.new_string(model.tokens)?;
            let punctuation = env.new_string(model.punctuation)?;
            env.call_static_method(
                class,
                "preload",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)V",
                &[
                    JValue::Object(&model_path),
                    JValue::Object(&tokens),
                    JValue::Object(&punctuation),
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
            .ok_or_else(|| "Zipformer CTC requires the main Android WebView".to_string())?;
        let bridge_name = option_env!("ZIPFORMER_CTC_ANDROID_BRIDGE_CLASS")
            .unwrap_or("com.lettuceai.app.ZipformerCtcBridge")
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
        let model = resolved_model(&request.model)?;
        let threads = request.threads.unwrap_or(4).clamp(1, i32::MAX as usize) as i32;
        let audio_len = pcm.len();

        tracing::info!(
            target: "asr",
            engine = "zipformerCtc",
            model = %model.model,
            punctuation_model = %model.punctuation,
            threads,
            audio_ms = (audio_len as u64 * 1000 / ASR_SAMPLE_RATE as u64),
            "transcription started"
        );
        let started = std::time::Instant::now();
        let raw_text = transcribe_bridge(app, &model, threads, pcm)?
            .trim()
            .to_string();
        let corrected = asr_apply_corrections(
            app.clone(),
            raw_text.clone(),
            request.language.clone(),
            request.scopes.clone(),
        )?;
        let corrected_text = if corrected.corrected_text.trim().is_empty() {
            String::new()
        } else {
            punctuation_bridge(app, &model, threads, corrected.corrected_text.clone())?
                .trim()
                .to_string()
        };
        tracing::info!(
            target: "asr",
            engine = "zipformerCtc",
            elapsed_ms = started.elapsed().as_millis() as u64,
            raw_text_len = raw_text.chars().count(),
            final_text_len = corrected_text.chars().count(),
            "transcription and punctuation complete"
        );

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
            let cleared = clear_bridge(app)?;
            tracing::debug!(target: "asr", engine = "zipformerCtc", cleared, "runtime cache cleared");
        }

        Ok(AsrWhisperTranscriptionResponse {
            audio_path,
            model_path: request.model.path.clone(),
            sample_rate_hz: ASR_SAMPLE_RATE,
            prompt: String::new(),
            raw_text,
            corrected_text,
            detected_language: Some("zh".to_string()),
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
        let model = resolved_model(&request.model)?;
        let threads = request.threads.unwrap_or(4).clamp(1, i32::MAX as usize) as i32;
        tracing::info!(target: "asr", engine = "zipformerCtc", threads, "preloading runtime");
        preload_bridge(app, model, threads)
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
        Err("Zipformer CTC is currently available on Android only".to_string())
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

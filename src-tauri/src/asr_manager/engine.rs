use serde::{Deserialize, Serialize};

use super::runtime::{
    AsrWhisperRuntimeLoadRequest, AsrWhisperTranscribePcmRequest, AsrWhisperTranscribeRequest,
    AsrWhisperTranscriptionResponse,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AsrEngine {
    Whisper,
    SenseVoice,
    ZipformerCtc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrModelRef {
    pub engine: AsrEngine,
    pub path: String,
    pub tokens_path: Option<String>,
    pub punctuation_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrTranscribePcmRequest {
    pub model: AsrModelRef,
    pub pcm_bytes: Vec<u8>,
    pub sample_rate_hz: u32,
    pub channels: Option<u16>,
    pub language: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub initial_prompt: Option<String>,
    pub translate: Option<bool>,
    pub detect_language: Option<bool>,
    pub no_context: Option<bool>,
    pub single_segment: Option<bool>,
    pub token_timestamps: Option<bool>,
    pub split_on_word: Option<bool>,
    pub max_len: Option<i32>,
    pub max_tokens: Option<i32>,
    pub offset_ms: Option<i32>,
    pub duration_ms: Option<i32>,
    pub threads: Option<usize>,
    pub best_of: Option<i32>,
    pub temperature: Option<f32>,
    pub temperature_inc: Option<f32>,
    pub use_gpu: Option<bool>,
    pub force_cpu: Option<bool>,
    pub keep_model_loaded: Option<bool>,
    pub flash_attn: Option<bool>,
    pub gpu_device: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrTranscribeFileRequest {
    pub model: AsrModelRef,
    pub audio_path: String,
    pub language: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub initial_prompt: Option<String>,
    pub translate: Option<bool>,
    pub detect_language: Option<bool>,
    pub no_context: Option<bool>,
    pub single_segment: Option<bool>,
    pub token_timestamps: Option<bool>,
    pub split_on_word: Option<bool>,
    pub max_len: Option<i32>,
    pub max_tokens: Option<i32>,
    pub offset_ms: Option<i32>,
    pub duration_ms: Option<i32>,
    pub threads: Option<usize>,
    pub best_of: Option<i32>,
    pub temperature: Option<f32>,
    pub temperature_inc: Option<f32>,
    pub use_gpu: Option<bool>,
    pub force_cpu: Option<bool>,
    pub keep_model_loaded: Option<bool>,
    pub flash_attn: Option<bool>,
    pub gpu_device: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrRuntimeLoadRequest {
    pub model: AsrModelRef,
    pub threads: Option<usize>,
    pub use_gpu: Option<bool>,
    pub force_cpu: Option<bool>,
    pub flash_attn: Option<bool>,
    pub gpu_device: Option<i32>,
}

impl AsrTranscribePcmRequest {
    fn into_whisper(self) -> AsrWhisperTranscribePcmRequest {
        AsrWhisperTranscribePcmRequest {
            model_path: self.model.path,
            pcm_bytes: self.pcm_bytes,
            sample_rate_hz: self.sample_rate_hz,
            channels: self.channels,
            language: self.language,
            scopes: self.scopes,
            initial_prompt: self.initial_prompt,
            translate: self.translate,
            detect_language: self.detect_language,
            no_context: self.no_context,
            single_segment: self.single_segment,
            token_timestamps: self.token_timestamps,
            split_on_word: self.split_on_word,
            max_len: self.max_len,
            max_tokens: self.max_tokens,
            offset_ms: self.offset_ms,
            duration_ms: self.duration_ms,
            threads: self.threads,
            best_of: self.best_of,
            temperature: self.temperature,
            temperature_inc: self.temperature_inc,
            use_gpu: self.use_gpu,
            force_cpu: self.force_cpu,
            keep_model_loaded: self.keep_model_loaded,
            flash_attn: self.flash_attn,
            gpu_device: self.gpu_device,
        }
    }
}

impl AsrTranscribeFileRequest {
    fn into_whisper(self) -> AsrWhisperTranscribeRequest {
        AsrWhisperTranscribeRequest {
            model_path: self.model.path,
            audio_path: self.audio_path,
            language: self.language,
            scopes: self.scopes,
            initial_prompt: self.initial_prompt,
            translate: self.translate,
            detect_language: self.detect_language,
            no_context: self.no_context,
            single_segment: self.single_segment,
            token_timestamps: self.token_timestamps,
            split_on_word: self.split_on_word,
            max_len: self.max_len,
            max_tokens: self.max_tokens,
            offset_ms: self.offset_ms,
            duration_ms: self.duration_ms,
            threads: self.threads,
            best_of: self.best_of,
            temperature: self.temperature,
            temperature_inc: self.temperature_inc,
            use_gpu: self.use_gpu,
            force_cpu: self.force_cpu,
            keep_model_loaded: self.keep_model_loaded,
            flash_attn: self.flash_attn,
            gpu_device: self.gpu_device,
        }
    }
}

#[tauri::command]
pub async fn asr_transcribe_pcm(
    app: tauri::AppHandle,
    request: AsrTranscribePcmRequest,
) -> Result<AsrWhisperTranscriptionResponse, String> {
    tauri::async_runtime::spawn_blocking(move || match request.model.engine {
        AsrEngine::Whisper => super::runtime::transcribe_pcm_sync(&app, request.into_whisper()),
        AsrEngine::SenseVoice => super::sense_voice::transcribe_pcm_sync(&app, request),
        AsrEngine::ZipformerCtc => super::zipformer_ctc::transcribe_pcm_sync(&app, request),
    })
    .await
    .map_err(|e| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("ASR task join error: {}", e),
        )
    })?
}

#[tauri::command]
pub async fn asr_transcribe_file(
    app: tauri::AppHandle,
    request: AsrTranscribeFileRequest,
) -> Result<AsrWhisperTranscriptionResponse, String> {
    tauri::async_runtime::spawn_blocking(move || match request.model.engine {
        AsrEngine::Whisper => super::runtime::transcribe_file_sync(&app, request.into_whisper()),
        AsrEngine::SenseVoice => super::sense_voice::transcribe_file_sync(&app, request),
        AsrEngine::ZipformerCtc => super::zipformer_ctc::transcribe_file_sync(&app, request),
    })
    .await
    .map_err(|e| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("ASR task join error: {}", e),
        )
    })?
}

#[tauri::command]
pub async fn asr_runtime_preload_model(
    app: tauri::AppHandle,
    request: AsrRuntimeLoadRequest,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || match request.model.engine {
        AsrEngine::Whisper => super::runtime::preload_model_sync(
            &app,
            AsrWhisperRuntimeLoadRequest {
                model_path: request.model.path,
                use_gpu: request.use_gpu,
                force_cpu: request.force_cpu,
                flash_attn: request.flash_attn,
                gpu_device: request.gpu_device,
            },
        ),
        AsrEngine::SenseVoice => super::sense_voice::preload_model_sync(&app, request),
        AsrEngine::ZipformerCtc => super::zipformer_ctc::preload_model_sync(&app, request),
    })
    .await
    .map_err(|e| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("ASR preload task join error: {}", e),
        )
    })?
}

#[tauri::command]
pub fn asr_runtime_clear_cache(app: tauri::AppHandle) -> Result<usize, String> {
    let whisper = super::runtime::clear_cache(&app)?;
    let sense_voice = super::sense_voice::clear_cache(&app)?;
    let zipformer_ctc = super::zipformer_ctc::clear_cache(&app)?;
    Ok(whisper + sense_voice + zipformer_ctc)
}

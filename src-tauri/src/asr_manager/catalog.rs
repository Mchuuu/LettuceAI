use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::AppHandle;

use super::engine::{AsrEngine, AsrModelRef};

const SENSE_VOICE_ID: &str = "sensevoice-small-int8";
const SENSE_VOICE_REPO: &str = "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17";
const SENSE_VOICE_RESOLVE_BASE: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main";
const SENSE_VOICE_MODEL_FILE: &str = "model.int8.onnx";
const SENSE_VOICE_TOKENS_FILE: &str = "tokens.txt";
const SENSE_VOICE_MODEL_SIZE: u64 = 239_000_000;
const SENSE_VOICE_TOKENS_SIZE: u64 = 316_000;

const ZIPFORMER_CTC_ID: &str = "zipformer-ctc-zh-int8";
const ZIPFORMER_CTC_REPO: &str = "csukuangfj/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03";
const ZIPFORMER_CTC_RESOLVE_BASE: &str =
    "https://huggingface.co/csukuangfj/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/resolve/main";
const ZIPFORMER_CTC_MODEL_FILE: &str = "model.int8.onnx";
const ZIPFORMER_CTC_TOKENS_FILE: &str = "tokens.txt";
const ZIPFORMER_CTC_MODEL_SIZE: u64 = 367_074_356;
const ZIPFORMER_CTC_TOKENS_SIZE: u64 = 13_366;
const ZIPFORMER_PUNCT_REPO: &str =
    "lorneluo/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8";
const ZIPFORMER_PUNCT_RESOLVE_BASE: &str = "https://huggingface.co/lorneluo/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8/resolve/main";
const ZIPFORMER_PUNCT_SOURCE_FILE: &str = "model.int8.onnx";
const ZIPFORMER_PUNCT_FILE: &str = "punctuation.int8.onnx";
const ZIPFORMER_PUNCT_SIZE: u64 = 75_519_198;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrModelPreset {
    pub id: String,
    pub engine: AsrEngine,
    pub filename: String,
    pub tokens_filename: Option<String>,
    pub punctuation_filename: Option<String>,
    pub label: String,
    pub repo: String,
    pub download_url: String,
    pub size_bytes: u64,
    pub english_only: bool,
    pub quantized: bool,
    pub recommended: bool,
    pub recommended_for_mobile: bool,
    pub recommended_for_desktop: bool,
    pub supports_gpu: bool,
    pub supports_initial_prompt: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrInstalledModel {
    pub id: String,
    pub engine: AsrEngine,
    pub filename: String,
    pub label: String,
    pub path: String,
    pub tokens_path: Option<String>,
    pub punctuation_path: Option<String>,
    pub size_bytes: u64,
    pub english_only: bool,
    pub quantized: bool,
    pub supports_gpu: bool,
    pub supports_initial_prompt: bool,
}

fn models_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = crate::infra::utils::ensure_lettuce_dir(app)?.join("models");
    std::fs::create_dir_all(&root)
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(root)
}

fn sense_voice_models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = models_root(app)?.join("sense-voice");
    std::fs::create_dir_all(&dir)
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(dir)
}

fn zipformer_ctc_models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = models_root(app)?.join("zipformer-ctc");
    std::fs::create_dir_all(&dir)
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(dir)
}

fn sense_voice_preset() -> AsrModelPreset {
    AsrModelPreset {
        id: SENSE_VOICE_ID.to_string(),
        engine: AsrEngine::SenseVoice,
        filename: SENSE_VOICE_MODEL_FILE.to_string(),
        tokens_filename: Some(SENSE_VOICE_TOKENS_FILE.to_string()),
        punctuation_filename: None,
        label: "SenseVoiceSmall INT8".to_string(),
        repo: SENSE_VOICE_REPO.to_string(),
        download_url: format!("{SENSE_VOICE_RESOLVE_BASE}/{SENSE_VOICE_MODEL_FILE}"),
        size_bytes: SENSE_VOICE_MODEL_SIZE + SENSE_VOICE_TOKENS_SIZE,
        english_only: false,
        quantized: true,
        recommended: true,
        recommended_for_mobile: true,
        recommended_for_desktop: false,
        supports_gpu: false,
        supports_initial_prompt: false,
    }
}

fn installed_sense_voice_models(app: &AppHandle) -> Result<Vec<AsrInstalledModel>, String> {
    if !cfg!(target_os = "android") {
        return Ok(Vec::new());
    }
    let install_dir = sense_voice_models_dir(app)?.join(SENSE_VOICE_ID);
    let model_path = install_dir.join(SENSE_VOICE_MODEL_FILE);
    let tokens_path = install_dir.join(SENSE_VOICE_TOKENS_FILE);
    if !model_path.is_file() || !tokens_path.is_file() {
        return Ok(Vec::new());
    }
    let size_bytes = model_path.metadata().map(|value| value.len()).unwrap_or(0)
        + tokens_path.metadata().map(|value| value.len()).unwrap_or(0);
    Ok(vec![AsrInstalledModel {
        id: SENSE_VOICE_ID.to_string(),
        engine: AsrEngine::SenseVoice,
        filename: SENSE_VOICE_MODEL_FILE.to_string(),
        label: "SenseVoiceSmall INT8".to_string(),
        path: model_path.to_string_lossy().to_string(),
        tokens_path: Some(tokens_path.to_string_lossy().to_string()),
        punctuation_path: None,
        size_bytes,
        english_only: false,
        quantized: true,
        supports_gpu: false,
        supports_initial_prompt: false,
    }])
}

fn zipformer_ctc_preset() -> AsrModelPreset {
    AsrModelPreset {
        id: ZIPFORMER_CTC_ID.to_string(),
        engine: AsrEngine::ZipformerCtc,
        filename: ZIPFORMER_CTC_MODEL_FILE.to_string(),
        tokens_filename: Some(ZIPFORMER_CTC_TOKENS_FILE.to_string()),
        punctuation_filename: Some(ZIPFORMER_PUNCT_FILE.to_string()),
        label: "Zipformer CTC Chinese INT8 + Punctuation".to_string(),
        repo: ZIPFORMER_CTC_REPO.to_string(),
        download_url: format!("{ZIPFORMER_CTC_RESOLVE_BASE}/{ZIPFORMER_CTC_MODEL_FILE}"),
        size_bytes: ZIPFORMER_CTC_MODEL_SIZE + ZIPFORMER_CTC_TOKENS_SIZE + ZIPFORMER_PUNCT_SIZE,
        english_only: false,
        quantized: true,
        recommended: true,
        recommended_for_mobile: true,
        recommended_for_desktop: false,
        supports_gpu: false,
        supports_initial_prompt: false,
    }
}

fn installed_zipformer_ctc_models(app: &AppHandle) -> Result<Vec<AsrInstalledModel>, String> {
    if !cfg!(target_os = "android") {
        return Ok(Vec::new());
    }
    let install_dir = zipformer_ctc_models_dir(app)?.join(ZIPFORMER_CTC_ID);
    let model_path = install_dir.join(ZIPFORMER_CTC_MODEL_FILE);
    let tokens_path = install_dir.join(ZIPFORMER_CTC_TOKENS_FILE);
    let punctuation_path = install_dir.join(ZIPFORMER_PUNCT_FILE);
    if !model_path.is_file() || !tokens_path.is_file() || !punctuation_path.is_file() {
        return Ok(Vec::new());
    }
    let size_bytes = [&model_path, &tokens_path, &punctuation_path]
        .into_iter()
        .map(|path| path.metadata().map(|value| value.len()).unwrap_or(0))
        .sum();
    Ok(vec![AsrInstalledModel {
        id: ZIPFORMER_CTC_ID.to_string(),
        engine: AsrEngine::ZipformerCtc,
        filename: ZIPFORMER_CTC_MODEL_FILE.to_string(),
        label: "Zipformer CTC Chinese INT8 + Punctuation".to_string(),
        path: model_path.to_string_lossy().to_string(),
        tokens_path: Some(tokens_path.to_string_lossy().to_string()),
        punctuation_path: Some(punctuation_path.to_string_lossy().to_string()),
        size_bytes,
        english_only: false,
        quantized: true,
        supports_gpu: false,
        supports_initial_prompt: false,
    }])
}

fn whisper_preset(model: super::models::AsrWhisperModelPreset) -> AsrModelPreset {
    AsrModelPreset {
        id: model.id,
        engine: AsrEngine::Whisper,
        filename: model.filename,
        tokens_filename: None,
        punctuation_filename: None,
        label: model.label,
        repo: model.repo,
        download_url: model.download_url,
        size_bytes: model.size_bytes,
        english_only: model.english_only,
        quantized: model.quantized,
        recommended: model.recommended,
        recommended_for_mobile: model.recommended_for_mobile,
        recommended_for_desktop: model.recommended_for_desktop,
        supports_gpu: true,
        supports_initial_prompt: true,
    }
}

fn whisper_installed(model: super::models::AsrInstalledWhisperModel) -> AsrInstalledModel {
    AsrInstalledModel {
        id: model.id,
        engine: AsrEngine::Whisper,
        filename: model.filename,
        label: model.label,
        path: model.path,
        tokens_path: None,
        punctuation_path: None,
        size_bytes: model.size_bytes,
        english_only: model.english_only,
        quantized: model.quantized,
        supports_gpu: true,
        supports_initial_prompt: true,
    }
}

#[tauri::command]
pub async fn asr_list_available_models() -> Result<Vec<AsrModelPreset>, String> {
    let mut models = Vec::new();
    if cfg!(target_os = "android") {
        models.push(sense_voice_preset());
        models.push(zipformer_ctc_preset());
    }

    match super::models::fetch_remote_whisper_models().await {
        Ok(whisper) => models.extend(whisper.into_iter().map(whisper_preset)),
        Err(error) if !models.is_empty() => {
            tracing::warn!(target: "asr", %error, "Whisper catalog unavailable; using built-in catalog");
        }
        Err(error) => return Err(error),
    }

    Ok(models)
}

#[tauri::command]
pub fn asr_get_models_dir(app: AppHandle) -> Result<String, String> {
    Ok(models_root(&app)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn asr_list_installed_models(app: AppHandle) -> Result<Vec<AsrInstalledModel>, String> {
    let mut models = installed_sense_voice_models(&app)?;
    models.extend(installed_zipformer_ctc_models(&app)?);
    models.extend(
        super::models::list_installed_whisper_models(&app)?
            .into_iter()
            .map(whisper_installed),
    );
    Ok(models)
}

fn download_metadata(
    install_dir: &Path,
    model_id: &str,
    label: &str,
    download_url: String,
    destination_filename: &str,
    role: &str,
) -> crate::hf_browser::QueueDownloadMetadata {
    crate::hf_browser::QueueDownloadMetadata {
        create_model_when_finished: false,
        mmproj_file: crate::hf_browser::MmprojFileLink::Disabled(false),
        mtp_file: crate::hf_browser::MmprojFileLink::Disabled(false),
        mtp_bundled: false,
        install_id: Some(format!("asr-{model_id}")),
        display_name: Some(label.to_string()),
        context_length: None,
        kv_type: None,
        llama_offload_kqv: None,
        llama_gpu_layers: None,
        llama_model_offload_mode: None,
        download_role: Some(role.to_string()),
        queue_kind: Some("asr".to_string()),
        asset_root: Some(install_dir.to_string_lossy().to_string()),
        install_kind: Some("asr-model".to_string()),
        variant: Some(model_id.to_string()),
        voice_id: None,
        download_url: Some(download_url),
        destination_path: Some(
            install_dir
                .join(destination_filename)
                .to_string_lossy()
                .to_string(),
        ),
        force_redownload: false,
    }
}

async fn queue_sense_voice_download(app: AppHandle) -> Result<String, String> {
    if !cfg!(target_os = "android") {
        return Err("SenseVoice is currently available on Android only".to_string());
    }
    let install_dir = sense_voice_models_dir(&app)?.join(SENSE_VOICE_ID);
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;

    let model_queue_id = crate::hf_browser::hf_queue_download(
        app.clone(),
        SENSE_VOICE_REPO.to_string(),
        SENSE_VOICE_MODEL_FILE.to_string(),
        Some(download_metadata(
            &install_dir,
            SENSE_VOICE_ID,
            "SenseVoiceSmall INT8",
            format!("{SENSE_VOICE_RESOLVE_BASE}/{SENSE_VOICE_MODEL_FILE}"),
            SENSE_VOICE_MODEL_FILE,
            "model",
        )),
    )
    .await?;
    crate::hf_browser::hf_queue_download(
        app,
        SENSE_VOICE_REPO.to_string(),
        SENSE_VOICE_TOKENS_FILE.to_string(),
        Some(download_metadata(
            &install_dir,
            SENSE_VOICE_ID,
            "SenseVoiceSmall INT8",
            format!("{SENSE_VOICE_RESOLVE_BASE}/{SENSE_VOICE_TOKENS_FILE}"),
            SENSE_VOICE_TOKENS_FILE,
            "tokens",
        )),
    )
    .await?;
    Ok(model_queue_id)
}

async fn queue_zipformer_ctc_download(app: AppHandle) -> Result<String, String> {
    if !cfg!(target_os = "android") {
        return Err("Zipformer CTC is currently available on Android only".to_string());
    }
    let install_dir = zipformer_ctc_models_dir(&app)?.join(ZIPFORMER_CTC_ID);
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    let label = "Zipformer CTC Chinese INT8 + Punctuation";

    let model_queue_id = crate::hf_browser::hf_queue_download(
        app.clone(),
        ZIPFORMER_CTC_REPO.to_string(),
        ZIPFORMER_CTC_MODEL_FILE.to_string(),
        Some(download_metadata(
            &install_dir,
            ZIPFORMER_CTC_ID,
            label,
            format!("{ZIPFORMER_CTC_RESOLVE_BASE}/{ZIPFORMER_CTC_MODEL_FILE}"),
            ZIPFORMER_CTC_MODEL_FILE,
            "model",
        )),
    )
    .await?;
    crate::hf_browser::hf_queue_download(
        app.clone(),
        ZIPFORMER_CTC_REPO.to_string(),
        ZIPFORMER_CTC_TOKENS_FILE.to_string(),
        Some(download_metadata(
            &install_dir,
            ZIPFORMER_CTC_ID,
            label,
            format!("{ZIPFORMER_CTC_RESOLVE_BASE}/{ZIPFORMER_CTC_TOKENS_FILE}"),
            ZIPFORMER_CTC_TOKENS_FILE,
            "tokens",
        )),
    )
    .await?;
    crate::hf_browser::hf_queue_download(
        app,
        ZIPFORMER_PUNCT_REPO.to_string(),
        ZIPFORMER_PUNCT_SOURCE_FILE.to_string(),
        Some(download_metadata(
            &install_dir,
            ZIPFORMER_CTC_ID,
            label,
            format!("{ZIPFORMER_PUNCT_RESOLVE_BASE}/{ZIPFORMER_PUNCT_SOURCE_FILE}"),
            ZIPFORMER_PUNCT_FILE,
            "punctuation",
        )),
    )
    .await?;
    Ok(model_queue_id)
}

#[tauri::command]
pub async fn asr_queue_model_download(
    app: AppHandle,
    engine: AsrEngine,
    model_id: String,
) -> Result<String, String> {
    match engine {
        AsrEngine::Whisper => super::models::asr_whisper_queue_model_download(app, model_id).await,
        AsrEngine::SenseVoice if model_id == SENSE_VOICE_ID => {
            queue_sense_voice_download(app).await
        }
        AsrEngine::SenseVoice => Err(format!("Unknown SenseVoice model: {model_id}")),
        AsrEngine::ZipformerCtc if model_id == ZIPFORMER_CTC_ID => {
            queue_zipformer_ctc_download(app).await
        }
        AsrEngine::ZipformerCtc => Err(format!("Unknown Zipformer CTC model: {model_id}")),
    }
}

#[tauri::command]
pub async fn asr_delete_installed_model(app: AppHandle, model: AsrModelRef) -> Result<(), String> {
    match model.engine {
        AsrEngine::Whisper => {
            super::models::asr_whisper_delete_installed_model(app, model.path).await
        }
        AsrEngine::SenseVoice => {
            let model_path = PathBuf::from(&model.path);
            let install_root = sense_voice_models_dir(&app)?;
            let canonical_root = std::fs::canonicalize(&install_root)
                .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
            let install_dir = model_path.parent().ok_or_else(|| {
                crate::utils::err_msg(module_path!(), line!(), "Invalid SenseVoice model path")
            })?;
            let canonical_install = std::fs::canonicalize(install_dir)
                .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
            if !canonical_install.starts_with(&canonical_root) {
                return Err(
                    "Cannot delete files outside the SenseVoice models directory".to_string(),
                );
            }
            super::sense_voice::clear_cache(&app)?;
            tokio::fs::remove_dir_all(canonical_install)
                .await
                .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
        }
        AsrEngine::ZipformerCtc => {
            let model_path = PathBuf::from(&model.path);
            let install_root = zipformer_ctc_models_dir(&app)?;
            let canonical_root = std::fs::canonicalize(&install_root)
                .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
            let install_dir = model_path.parent().ok_or_else(|| {
                crate::utils::err_msg(module_path!(), line!(), "Invalid Zipformer CTC model path")
            })?;
            let canonical_install = std::fs::canonicalize(install_dir)
                .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
            if !canonical_install.starts_with(&canonical_root) {
                return Err(
                    "Cannot delete files outside the Zipformer CTC models directory".to_string(),
                );
            }
            super::zipformer_ctc::clear_cache(&app)?;
            tokio::fs::remove_dir_all(canonical_install)
                .await
                .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
        }
    }
}

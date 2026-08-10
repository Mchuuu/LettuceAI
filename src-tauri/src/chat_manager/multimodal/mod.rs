mod volcengine_files;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use crate::chat_manager::attachments::ModelImageLoadMode;
use crate::chat_manager::service::{require_api_key, ChatContext};
use crate::chat_manager::types::{ImageAttachment, ProviderCredential};
use crate::utils::{log_info, log_warn};

use self::volcengine_files::{
    compress_inline_messages_for_fallback, FileResolution, VolcengineFilesTransport,
};

pub const DEFAULT_VOLCENGINE_ARK_FILES_ENDPOINT: &str =
    "https://ark.cn-beijing.volces.com/api/v3/files";

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ImageUploadMode {
    #[default]
    Base64,
    VolcengineArkFiles,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ApiKeySource {
    #[default]
    Provider,
    Custom,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageUploadConfig {
    #[serde(default)]
    mode: ImageUploadMode,
    #[serde(default = "default_volcengine_files_endpoint")]
    endpoint: String,
    #[serde(default)]
    api_key_source: ApiKeySource,
    #[serde(default)]
    api_key: Option<String>,
}

impl Default for ImageUploadConfig {
    fn default() -> Self {
        Self {
            mode: ImageUploadMode::Base64,
            endpoint: default_volcengine_files_endpoint(),
            api_key_source: ApiKeySource::Provider,
            api_key: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedImageUpload {
    file_id: String,
    cache_hit: bool,
}

fn default_volcengine_files_endpoint() -> String {
    DEFAULT_VOLCENGINE_ARK_FILES_ENDPOINT.to_string()
}

fn image_upload_config(credential: &ProviderCredential) -> ImageUploadConfig {
    credential
        .config
        .as_ref()
        .and_then(|config| config.get("imageUpload"))
        .cloned()
        .and_then(|config| serde_json::from_value(config).ok())
        .unwrap_or_default()
}

pub fn model_image_load_mode(credential: &ProviderCredential) -> ModelImageLoadMode {
    match image_upload_config(credential).mode {
        ImageUploadMode::Base64 => ModelImageLoadMode::CompressedBase64,
        ImageUploadMode::VolcengineArkFiles => ModelImageLoadMode::OriginalForRemoteUpload,
    }
}

fn resolve_files_api_key<'a>(
    config: &'a ImageUploadConfig,
    provider_api_key: &'a str,
) -> Option<&'a str> {
    let value = match config.api_key_source {
        ApiKeySource::Provider => provider_api_key,
        ApiKeySource::Custom => config.api_key.as_deref().unwrap_or_default(),
    };
    (!value.trim().is_empty()).then_some(value.trim())
}

fn build_transport(
    app: &AppHandle,
    credential: &ProviderCredential,
    provider_api_key: &str,
) -> Result<Option<VolcengineFilesTransport>, String> {
    let config = image_upload_config(credential);
    if config.mode == ImageUploadMode::Base64 {
        return Ok(None);
    }

    let api_key = resolve_files_api_key(&config, provider_api_key)
        .ok_or_else(|| {
            crate::utils::err_msg(
                module_path!(),
                line!(),
                "Volcengine Files API key is missing",
            )
        })?
        .to_string();

    VolcengineFilesTransport::new(
        app.clone(),
        credential.id.clone(),
        credential.provider_id.clone(),
        config.endpoint,
        api_key,
    )
    .map(Some)
}

/// Rewrites only inline image parts. Base64 mode returns `None` without cloning messages.
/// Files API failures are compressed and retained as Base64 on a per-image basis.
pub async fn prepare_messages_for_provider(
    app: &AppHandle,
    credential: &ProviderCredential,
    provider_api_key: &str,
    messages: &[Value],
) -> Option<Vec<Value>> {
    let transport = match build_transport(app, credential, provider_api_key) {
        Ok(Some(transport)) => transport,
        Ok(None) => return None,
        Err(error) => {
            log_warn(
                app,
                "image_upload",
                format!(
                    "Files API unavailable for credential={}; retaining compressed Base64: {}",
                    credential.id, error
                ),
            );
            return compress_inline_messages_for_fallback(app, messages);
        }
    };

    let report = transport.rewrite_messages(messages).await;
    if report.candidates > 0 {
        log_info(
            app,
            "image_upload",
            format!(
                "Prepared chat images: credential={} candidates={} file_ids={} cache_hits={} base64_fallbacks={}",
                credential.id,
                report.candidates,
                report.file_ids,
                report.cache_hits,
                report.base64_fallbacks
            ),
        );
    }
    report.messages
}

/// Pre-warms the provider upload cache when an image is selected in the composer.
#[tauri::command]
pub async fn prepare_provider_image_upload(
    app: AppHandle,
    credential_id: String,
    attachment: ImageAttachment,
) -> Result<PreparedImageUpload, String> {
    if attachment.data.is_empty() || !attachment.mime_type.starts_with("image/") {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            "A readable image attachment is required",
        ));
    }

    let context = ChatContext::initialize(app.clone())?;
    let credential = context
        .settings
        .provider_credentials
        .iter()
        .find(|credential| credential.id == credential_id)
        .cloned()
        .ok_or_else(|| {
            crate::utils::err_msg(module_path!(), line!(), "Provider credential not found")
        })?;
    let provider_api_key = require_api_key(&app, &credential, "image_upload")?;
    let transport = build_transport(&app, &credential, &provider_api_key)?.ok_or_else(|| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            "This provider is configured to use Base64 image uploads",
        )
    })?;

    let FileResolution { file_id, cache_hit } = transport
        .resolve_data_url(
            &attachment.data,
            &attachment.mime_type,
            attachment.filename.as_deref(),
        )
        .await?;

    Ok(PreparedImageUpload { file_id, cache_hit })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{image_upload_config, model_image_load_mode};
    use crate::chat_manager::attachments::ModelImageLoadMode;
    use crate::chat_manager::types::ProviderCredential;

    fn credential(config: Option<serde_json::Value>) -> ProviderCredential {
        ProviderCredential {
            id: "credential".to_string(),
            provider_id: "custom".to_string(),
            label: "Ark".to_string(),
            api_key: Some("key".to_string()),
            base_url: None,
            default_model: None,
            headers: None,
            config,
        }
    }

    #[test]
    fn old_provider_config_remains_compressed_base64() {
        let credential = credential(None);

        assert_eq!(
            model_image_load_mode(&credential),
            ModelImageLoadMode::CompressedBase64
        );
    }

    #[test]
    fn files_mode_loads_original_image_and_applies_defaults() {
        let credential = credential(Some(json!({
            "imageUpload": { "mode": "volcengine-ark-files" }
        })));
        let parsed = image_upload_config(&credential);

        assert_eq!(
            model_image_load_mode(&credential),
            ModelImageLoadMode::OriginalForRemoteUpload
        );
        assert_eq!(
            parsed.endpoint,
            super::DEFAULT_VOLCENGINE_ARK_FILES_ENDPOINT
        );
    }
}

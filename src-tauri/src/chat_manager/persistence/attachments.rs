use std::fs;

use tauri::AppHandle;

use crate::chat_manager::types::{ImageAttachment, StoredMessage};
use crate::storage_manager::legacy::storage_root;
use crate::storage_manager::media::{
    storage_load_session_attachment, storage_save_session_attachment,
    storage_save_session_attachment_with_policy,
};
use crate::utils::{log_error, log_info};

use super::model_image::{load_model_image_payload, prepare_inline_model_image_payload};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelImageLoadMode {
    CompressedBase64,
    OriginalForRemoteUpload,
}

pub fn persist_attachments(
    app: &AppHandle,
    character_id: &str,
    session_id: &str,
    message_id: &str,
    role: &str,
    attachments: Vec<ImageAttachment>,
) -> Result<Vec<ImageAttachment>, String> {
    persist_attachments_with_mode(
        app,
        character_id,
        session_id,
        message_id,
        role,
        attachments,
        ModelImageLoadMode::CompressedBase64,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn persist_attachments_with_mode(
    app: &AppHandle,
    character_id: &str,
    session_id: &str,
    message_id: &str,
    role: &str,
    attachments: Vec<ImageAttachment>,
    image_mode: ModelImageLoadMode,
) -> Result<Vec<ImageAttachment>, String> {
    let mut persisted = Vec::new();

    for attachment in attachments {
        if attachment.storage_path.is_some() && attachment.data.is_empty() {
            persisted.push(attachment);
            continue;
        }

        if attachment.data.is_empty() {
            continue;
        }

        let preserve_image_source = image_mode == ModelImageLoadMode::OriginalForRemoteUpload
            && !attachment.mime_type.starts_with("audio/");
        let storage_path = if preserve_image_source {
            storage_save_session_attachment_with_policy(
                app.clone(),
                character_id.to_string(),
                session_id.to_string(),
                message_id.to_string(),
                attachment.id.clone(),
                role.to_string(),
                attachment.data.clone(),
                true,
            )?
        } else {
            storage_save_session_attachment(
                app.clone(),
                character_id.to_string(),
                session_id.to_string(),
                message_id.to_string(),
                attachment.id.clone(),
                role.to_string(),
                attachment.data.clone(),
            )?
        };

        persisted.push(ImageAttachment {
            id: attachment.id,
            data: String::new(),
            mime_type: attachment.mime_type,
            filename: attachment.filename,
            width: attachment.width,
            height: attachment.height,
            storage_path: Some(storage_path),
        });
    }

    Ok(persisted)
}

pub fn load_attachment_data(app: &AppHandle, message: &StoredMessage) -> StoredMessage {
    load_attachment_data_with_mode(app, message, ModelImageLoadMode::CompressedBase64)
}

pub fn load_attachment_data_with_mode(
    app: &AppHandle,
    message: &StoredMessage,
    image_mode: ModelImageLoadMode,
) -> StoredMessage {
    let mut loaded_message = message.clone();

    loaded_message.attachments = message
        .attachments
        .iter()
        .map(|attachment| {
            if !attachment.data.is_empty() {
                if attachment.mime_type.starts_with("audio/") {
                    return attachment.clone();
                }

                if image_mode == ModelImageLoadMode::OriginalForRemoteUpload {
                    return attachment.clone();
                }

                return match prepare_inline_model_image_payload(app, &attachment.data) {
                    Ok(payload) => ImageAttachment {
                        id: attachment.id.clone(),
                        data: payload.data_url,
                        mime_type: payload.mime_type,
                        filename: attachment.filename.clone(),
                        width: attachment.width,
                        height: attachment.height,
                        storage_path: attachment.storage_path.clone(),
                    },
                    Err(error) => {
                        crate::utils::log_warn(
                            app,
                            "model_image",
                            format!(
                                "Failed to prepare inline image attachment {}: {}",
                                attachment.id, error
                            ),
                        );
                        attachment.clone()
                    }
                };
            }

            let storage_path = match &attachment.storage_path {
                Some(path) => path,
                None => return attachment.clone(),
            };

            let loaded = if attachment.mime_type.starts_with("audio/") {
                storage_load_session_attachment(app.clone(), storage_path.clone())
                    .map(|data| (data, attachment.mime_type.clone()))
            } else if image_mode == ModelImageLoadMode::OriginalForRemoteUpload {
                storage_load_session_attachment(app.clone(), storage_path.clone())
                    .map(|data| (data, attachment.mime_type.clone()))
            } else {
                load_model_image_payload(app, storage_path)
                    .map(|payload| (payload.data_url, payload.mime_type))
                    .or_else(|error| {
                        crate::utils::log_warn(
                            app,
                            "model_image",
                            format!(
                                "Failed to prepare stored image attachment {}: {}; using original",
                                storage_path, error
                            ),
                        );
                        storage_load_session_attachment(app.clone(), storage_path.clone())
                            .map(|data| (data, attachment.mime_type.clone()))
                    })
            };

            match loaded {
                Ok((data, mime_type)) => ImageAttachment {
                    id: attachment.id.clone(),
                    data,
                    mime_type,
                    filename: attachment.filename.clone(),
                    width: attachment.width,
                    height: attachment.height,
                    storage_path: attachment.storage_path.clone(),
                },
                Err(_) => attachment.clone(),
            }
        })
        .collect();

    loaded_message
}

pub fn compress_inline_image_for_model(
    app: &AppHandle,
    data_url: &str,
) -> Result<(String, String), String> {
    prepare_inline_model_image_payload(app, data_url)
        .map(|payload| (payload.data_url, payload.mime_type))
}

pub fn cleanup_attachments(app: &AppHandle, attachments: &[ImageAttachment], scope: &str) {
    for attachment in attachments {
        let Some(storage_path) = &attachment.storage_path else {
            continue;
        };

        let full_path = match storage_root(app) {
            Ok(root) => root.join(storage_path),
            Err(err) => {
                log_error(
                    app,
                    scope,
                    format!(
                        "failed to resolve storage root while cleaning attachment {}: {}",
                        storage_path, err
                    ),
                );
                return;
            }
        };

        if !full_path.exists() {
            continue;
        }

        if let Err(err) = fs::remove_file(&full_path) {
            log_error(
                app,
                scope,
                format!("failed to remove attachment {}: {}", storage_path, err),
            );
            continue;
        }

        log_info(app, scope, format!("removed attachment {}", storage_path));
    }
}

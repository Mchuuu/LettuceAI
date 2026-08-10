use serde_json::{json, Map, Value as JsonValue};
use tauri::AppHandle;
use uuid::Uuid;

use crate::chat_manager::types::ImageAttachment;
use crate::utils::log_warn;

use super::media::{storage_load_session_attachment, storage_save_session_attachment};

#[derive(Debug, Default)]
pub(super) struct ExportedInlineImages {
    pub media: Vec<JsonValue>,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InlineImageSource {
    data_url: String,
    filename: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

fn image_mime_from_data_url(value: &str) -> Option<String> {
    let header = value.strip_prefix("data:")?.split_once(',')?.0;
    let mut parts = header.split(';');
    let mime_type = parts.next()?.trim().to_ascii_lowercase();
    let is_base64 = parts.any(|part| part.trim().eq_ignore_ascii_case("base64"));
    if !is_base64 || !mime_type.starts_with("image/") {
        return None;
    }

    Some(match mime_type.as_str() {
        "image/jpg" | "image/pjpeg" => "image/jpeg".to_string(),
        "image/x-png" => "image/png".to_string(),
        _ => mime_type,
    })
}

fn stored_image_mime(storage_path: &str, fallback: &str) -> String {
    let lower = storage_path.to_ascii_lowercase();
    if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else {
        fallback.to_string()
    }
}

fn export_attachment_data_url(
    app: &AppHandle,
    attachment: &ImageAttachment,
) -> Result<String, String> {
    if image_mime_from_data_url(&attachment.data).is_some() {
        return Ok(attachment.data.clone());
    }

    let storage_path = attachment.storage_path.as_ref().ok_or_else(|| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            "Image attachment has neither inline data nor a storage path",
        )
    })?;
    let data_url = storage_load_session_attachment(app.clone(), storage_path.clone())?;
    if image_mime_from_data_url(&data_url).is_none() {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            "Stored attachment is not a Base64 image",
        ));
    }

    Ok(data_url)
}

fn media_value(attachment: &ImageAttachment, data_url: String) -> JsonValue {
    let mut media = Map::new();
    media.insert("url".to_string(), JsonValue::String(data_url));
    media.insert("type".to_string(), JsonValue::String("image".to_string()));
    media.insert(
        "source".to_string(),
        JsonValue::String("upload".to_string()),
    );
    if let Some(filename) = attachment
        .filename
        .as_ref()
        .filter(|filename| !filename.trim().is_empty())
    {
        media.insert("title".to_string(), JsonValue::String(filename.clone()));
    }
    if let Some(width) = attachment.width {
        media.insert("width".to_string(), json!(width));
    }
    if let Some(height) = attachment.height {
        media.insert("height".to_string(), json!(height));
    }
    JsonValue::Object(media)
}

/// Resolves message image attachments to portable data URLs without changing stored messages.
pub(super) fn export_message_images(app: &AppHandle, message: &JsonValue) -> ExportedInlineImages {
    let Some(attachments) = message.get("attachments").and_then(JsonValue::as_array) else {
        return ExportedInlineImages::default();
    };

    let mut result = ExportedInlineImages::default();
    for raw_attachment in attachments {
        let attachment = match serde_json::from_value::<ImageAttachment>(raw_attachment.clone()) {
            Ok(attachment) => attachment,
            Err(error) => {
                result.skipped += 1;
                log_warn(
                    app,
                    "jsonl_export",
                    format!("Skipping malformed message attachment: {error}"),
                );
                continue;
            }
        };
        if !attachment
            .mime_type
            .to_ascii_lowercase()
            .starts_with("image/")
        {
            continue;
        }

        match export_attachment_data_url(app, &attachment) {
            Ok(data_url) => result.media.push(media_value(&attachment, data_url)),
            Err(error) => {
                result.skipped += 1;
                log_warn(
                    app,
                    "jsonl_export",
                    format!(
                        "Skipping unreadable image attachment id={}: {error}",
                        attachment.id
                    ),
                );
            }
        }
    }

    result
}

/// Uses SillyTavern's current media array shape while keeping the image bytes inline.
pub(super) fn attach_images_to_exported_message(message: &mut JsonValue, media: Vec<JsonValue>) {
    if media.is_empty() {
        return;
    }

    let Some(message) = message.as_object_mut() else {
        return;
    };
    let extra = message
        .entry("extra")
        .or_insert_with(|| JsonValue::Object(Map::new()));
    if !extra.is_object() {
        *extra = JsonValue::Object(Map::new());
    }
    let extra = extra
        .as_object_mut()
        .expect("extra was normalized to an object");
    extra.insert("inline_image".to_string(), JsonValue::Bool(true));
    extra.insert(
        "media_display".to_string(),
        JsonValue::String("list".to_string()),
    );
    extra.insert("media".to_string(), JsonValue::Array(media));
}

fn optional_u32(value: Option<&JsonValue>) -> Option<u32> {
    value
        .and_then(JsonValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn push_inline_source(
    sources: &mut Vec<InlineImageSource>,
    data_url: &str,
    filename: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
) {
    if image_mime_from_data_url(data_url).is_none()
        || sources
            .iter()
            .any(|source| source.data_url.as_str() == data_url)
    {
        return;
    }

    sources.push(InlineImageSource {
        data_url: data_url.to_string(),
        filename,
        width,
        height,
    });
}

fn inline_image_sources(entry: &JsonValue) -> Vec<InlineImageSource> {
    let Some(extra) = entry.get("extra").and_then(JsonValue::as_object) else {
        return Vec::new();
    };

    let mut sources = Vec::new();
    if let Some(media) = extra.get("media").and_then(JsonValue::as_array) {
        for item in media {
            if item
                .get("type")
                .and_then(JsonValue::as_str)
                .is_some_and(|media_type| media_type != "image")
            {
                continue;
            }
            let Some(data_url) = item.get("url").and_then(JsonValue::as_str) else {
                continue;
            };
            push_inline_source(
                &mut sources,
                data_url,
                item.get("title")
                    .and_then(JsonValue::as_str)
                    .map(str::to_owned),
                optional_u32(item.get("width")),
                optional_u32(item.get("height")),
            );
        }
    }

    // Read the pre-1.14 singular image field as a compatibility fallback.
    if let Some(data_url) = extra.get("image").and_then(JsonValue::as_str) {
        push_inline_source(
            &mut sources,
            data_url,
            extra
                .get("title")
                .and_then(JsonValue::as_str)
                .map(str::to_owned),
            None,
            None,
        );
    }

    sources
}

/// Persists portable JSONL data URLs back to the regular session attachment store.
pub(super) fn import_message_images(
    app: &AppHandle,
    entry: &JsonValue,
    character_id: &str,
    session_id: &str,
    message_id: &str,
    role: &str,
) -> Result<Vec<JsonValue>, String> {
    let sources = inline_image_sources(entry);
    let mut attachments = Vec::with_capacity(sources.len());

    for source in sources {
        let attachment_id = Uuid::new_v4().to_string();
        let mime_type = image_mime_from_data_url(&source.data_url).ok_or_else(|| {
            crate::utils::err_msg(module_path!(), line!(), "Invalid inline image data URL")
        })?;
        let storage_path = storage_save_session_attachment(
            app.clone(),
            character_id.to_string(),
            session_id.to_string(),
            message_id.to_string(),
            attachment_id.clone(),
            role.to_string(),
            source.data_url,
        )?;
        let attachment = ImageAttachment {
            id: attachment_id,
            data: String::new(),
            mime_type: stored_image_mime(&storage_path, &mime_type),
            filename: source.filename,
            width: source.width,
            height: source.height,
            storage_path: Some(storage_path),
        };
        attachments.push(
            serde_json::to_value(attachment)
                .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?,
        );
    }

    Ok(attachments)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{attach_images_to_exported_message, inline_image_sources, stored_image_mime};

    #[test]
    fn reads_current_and_legacy_inline_images_without_duplicates() {
        let first = "data:image/png;base64,AA==";
        let second = "data:image/jpeg;base64,AQ==";
        let entry = json!({
            "extra": {
                "media": [
                    {"url": first, "type": "image", "title": "first.png", "width": 12},
                    {"url": "https://example.com/remote.png", "type": "image"},
                    {"url": "data:audio/mpeg;base64,AA==", "type": "audio"}
                ],
                "image": second
            }
        });

        let sources = inline_image_sources(&entry);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].data_url, first);
        assert_eq!(sources[0].filename.as_deref(), Some("first.png"));
        assert_eq!(sources[0].width, Some(12));
        assert_eq!(sources[1].data_url, second);
    }

    #[test]
    fn writes_portable_media_array_to_message_extra() {
        let data_url = "data:image/webp;base64,AA==";
        let mut message = json!({"mes": "hello", "extra": {}});
        attach_images_to_exported_message(
            &mut message,
            vec![json!({"url": data_url, "type": "image"})],
        );

        assert_eq!(message["extra"]["inline_image"], true);
        assert_eq!(message["extra"]["media_display"], "list");
        assert_eq!(message["extra"]["media"][0]["url"], data_url);
    }

    #[test]
    fn stored_mime_tracks_lossless_webp_conversion() {
        assert_eq!(
            stored_image_mime("sessions/a/image.webp", "image/png"),
            "image/webp"
        );
    }
}

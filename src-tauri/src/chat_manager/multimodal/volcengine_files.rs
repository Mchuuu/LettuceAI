use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose, Engine as _};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio::time::sleep;
use url::Url;
use uuid::Uuid;

use crate::chat_manager::attachments::compress_inline_image_for_model;
use crate::utils::{log_debug, log_info, log_warn};

const CACHE_PROFILE: &[u8] = b"volcengine-ark-files-v1";
const CACHE_FALLBACK_TTL_MS: u64 = 6 * 60 * 60 * 1000;
const CACHE_EXPIRY_SAFETY_MS: u64 = 10 * 60 * 1000;
const CACHE_MAX_ENTRIES: usize = 4096;
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const POLL_TIMEOUT: Duration = Duration::from_secs(300);
const REQUEST_TIMEOUT_MS: u64 = 5 * 60 * 1000;

#[derive(Debug)]
pub struct FileResolution {
    pub file_id: String,
    pub cache_hit: bool,
}

pub struct RewriteReport {
    pub messages: Option<Vec<Value>>,
    pub candidates: usize,
    pub file_ids: usize,
    pub cache_hits: usize,
    pub base64_fallbacks: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct ArkFile {
    #[serde(default)]
    id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    expire_at: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedArkFile {
    file_id: String,
    cached_at_ms: u64,
    expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
struct ImagePartLocation {
    message_index: usize,
    part_index: usize,
}

struct ParsedDataUrl {
    mime_type: String,
    bytes: Vec<u8>,
}

pub struct VolcengineFilesTransport {
    app: AppHandle,
    credential_id: String,
    endpoint: Url,
    endpoint_for_log: String,
    api_key: String,
    api_key_fingerprint: String,
    client: reqwest::Client,
    cache_dir: Option<PathBuf>,
}

pub(super) fn compress_inline_messages_for_fallback(
    app: &AppHandle,
    messages: &[Value],
) -> Option<Vec<Value>> {
    let locations = inline_image_locations(messages);
    if locations.is_empty() {
        return None;
    }

    let mut rewritten = messages.to_vec();
    let mut changed = false;
    for location in locations {
        let Some(data_url) = image_url_at(&rewritten, location).map(str::to_string) else {
            continue;
        };
        match compress_inline_image_for_model(app, &data_url) {
            Ok((compressed, _)) => {
                changed |= replace_with_data_url(&mut rewritten, location, compressed);
            }
            Err(error) => log_warn(
                app,
                "image_upload",
                format!("Failed to prepare compressed Base64 fallback: {error}"),
            ),
        }
    }
    changed.then_some(rewritten)
}

impl VolcengineFilesTransport {
    pub fn new(
        app: AppHandle,
        credential_id: String,
        provider_id: String,
        endpoint: String,
        api_key: String,
    ) -> Result<Self, String> {
        let endpoint = normalize_endpoint(&endpoint)?;
        let endpoint_for_log = sanitized_url(&endpoint);
        let client = crate::transport::build_client(
            &app,
            Some(REQUEST_TIMEOUT_MS),
            false,
            Some(&provider_id),
            Some(endpoint.as_str()),
        )
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let cache_dir = match ark_file_cache_dir(&app) {
            Ok(path) => Some(path),
            Err(error) => {
                log_warn(
                    &app,
                    "image_upload",
                    format!("Files API cache unavailable; continuing without cache: {error}"),
                );
                None
            }
        };
        let api_key_fingerprint = blake3::hash(api_key.as_bytes()).to_hex().to_string();

        Ok(Self {
            app,
            credential_id,
            endpoint,
            endpoint_for_log,
            api_key,
            api_key_fingerprint,
            client,
            cache_dir,
        })
    }

    pub async fn resolve_data_url(
        &self,
        data_url: &str,
        fallback_mime_type: &str,
        filename: Option<&str>,
    ) -> Result<FileResolution, String> {
        let parsed = parse_image_data_url(data_url, fallback_mime_type)?;
        self.resolve_bytes(&parsed.bytes, &parsed.mime_type, filename)
            .await
    }

    pub async fn rewrite_messages(&self, messages: &[Value]) -> RewriteReport {
        let locations = inline_image_locations(messages);
        if locations.is_empty() {
            return RewriteReport {
                messages: None,
                candidates: 0,
                file_ids: 0,
                cache_hits: 0,
                base64_fallbacks: 0,
            };
        }

        let mut rewritten = messages.to_vec();
        let mut request_cache = HashMap::<String, String>::new();
        let mut file_ids = 0;
        let mut cache_hits = 0;
        let mut base64_fallbacks = 0;
        let mut changed = false;

        for location in &locations {
            let Some(data_url) = image_url_at(&rewritten, *location).map(str::to_string) else {
                continue;
            };
            let request_key = blake3::hash(data_url.as_bytes()).to_hex().to_string();

            let resolution = if let Some(file_id) = request_cache.get(&request_key) {
                Ok(FileResolution {
                    file_id: file_id.clone(),
                    cache_hit: true,
                })
            } else {
                self.resolve_data_url(&data_url, "image/jpeg", None).await
            };

            match resolution {
                Ok(resolution) => {
                    request_cache.insert(request_key, resolution.file_id.clone());
                    if replace_with_file_id(&mut rewritten, *location, &resolution.file_id) {
                        changed = true;
                        file_ids += 1;
                        if resolution.cache_hit {
                            cache_hits += 1;
                        }
                    }
                }
                Err(error) => {
                    log_warn(
                        &self.app,
                        "image_upload",
                        format!(
                            "Files API image preparation failed; using compressed Base64: credential={} error={}",
                            self.credential_id, error
                        ),
                    );
                    if let Ok((compressed, _)) =
                        compress_inline_image_for_model(&self.app, &data_url)
                    {
                        if replace_with_data_url(&mut rewritten, *location, compressed) {
                            changed = true;
                        }
                    }
                    base64_fallbacks += 1;
                }
            }
        }

        RewriteReport {
            messages: changed.then_some(rewritten),
            candidates: locations.len(),
            file_ids,
            cache_hits,
            base64_fallbacks,
        }
    }

    async fn resolve_bytes(
        &self,
        bytes: &[u8],
        mime_type: &str,
        filename: Option<&str>,
    ) -> Result<FileResolution, String> {
        let cache_key = self.cache_key(bytes, mime_type);
        if let Some(cached) = self.read_cache(&cache_key).await {
            log_debug(
                &self.app,
                "image_upload",
                format!(
                    "Files API cache hit: credential={} cache_key={} file_id={}",
                    self.credential_id,
                    short_value(&cache_key),
                    short_value(&cached.file_id)
                ),
            );
            return Ok(FileResolution {
                file_id: cached.file_id,
                cache_hit: true,
            });
        }

        log_info(
            &self.app,
            "image_upload",
            format!(
                "Files API upload start: credential={} endpoint={} bytes={} mime_type={} cache_key={}",
                self.credential_id,
                self.endpoint_for_log,
                bytes.len(),
                mime_type,
                short_value(&cache_key)
            ),
        );

        let uploaded = self.upload(bytes, mime_type, filename).await?;
        let active = self.wait_until_active(uploaded).await?;
        let expires_at_ms = active.expire_at.as_ref().and_then(parse_expiry_ms);
        let cached = CachedArkFile {
            file_id: active.id.clone(),
            cached_at_ms: now_ms(),
            expires_at_ms,
        };
        self.write_cache(&cache_key, &cached).await;

        log_info(
            &self.app,
            "image_upload",
            format!(
                "Files API image ready: credential={} file_id={} expires_at_ms={:?}",
                self.credential_id,
                short_value(&active.id),
                expires_at_ms
            ),
        );

        Ok(FileResolution {
            file_id: active.id,
            cache_hit: false,
        })
    }

    async fn upload(
        &self,
        bytes: &[u8],
        mime_type: &str,
        filename: Option<&str>,
    ) -> Result<ArkFile, String> {
        let filename = normalized_filename(filename, mime_type);
        let part = Part::bytes(bytes.to_vec())
            .file_name(filename)
            .mime_str(mime_type)
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let form = Form::new().text("purpose", "user_data").part("file", part);
        let response = self
            .client
            .post(self.endpoint.clone())
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

        parse_file_response(response, "upload").await
    }

    async fn retrieve(&self, file_id: &str) -> Result<ArkFile, String> {
        let url = file_url(&self.endpoint, file_id)?;
        let request = self.client.get(url).bearer_auth(&self.api_key);
        let response =
            crate::transport::send_with_retries(&self.app, "image_upload", request, 2, None)
                .await
                .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        parse_file_response(response, "status query").await
    }

    async fn wait_until_active(&self, mut file: ArkFile) -> Result<ArkFile, String> {
        if file.id.trim().is_empty() {
            return Err(crate::utils::err_msg(
                module_path!(),
                line!(),
                "Files API upload response did not include a file id",
            ));
        }

        let deadline = Instant::now() + POLL_TIMEOUT;
        let mut last_status = String::new();
        loop {
            let status = file.status.trim().to_ascii_lowercase();
            if status != last_status {
                log_debug(
                    &self.app,
                    "image_upload",
                    format!(
                        "Files API status: credential={} file_id={} status={}",
                        self.credential_id,
                        short_value(&file.id),
                        if status.is_empty() {
                            "unknown"
                        } else {
                            &status
                        }
                    ),
                );
                last_status = status.clone();
            }

            if status == "active" {
                return Ok(file);
            }
            if status == "failed" {
                return Err(crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!(
                        "Files API failed to process the image: {}",
                        file.error.unwrap_or_else(|| json!("unknown error"))
                    ),
                ));
            }
            if Instant::now() >= deadline {
                return Err(crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!(
                        "Files API image {} did not become active within {} seconds",
                        short_value(&file.id),
                        POLL_TIMEOUT.as_secs()
                    ),
                ));
            }

            sleep(POLL_INTERVAL).await;
            file = self.retrieve(&file.id).await?;
        }
    }

    fn cache_key(&self, bytes: &[u8], mime_type: &str) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(CACHE_PROFILE);
        hasher.update(self.endpoint.as_str().as_bytes());
        hasher.update(self.credential_id.as_bytes());
        hasher.update(self.api_key_fingerprint.as_bytes());
        hasher.update(mime_type.as_bytes());
        hasher.update(bytes);
        hasher.finalize().to_hex().to_string()
    }

    async fn read_cache(&self, cache_key: &str) -> Option<CachedArkFile> {
        let path = self.cache_dir.as_ref()?.join(format!("{cache_key}.json"));
        let bytes = tokio::fs::read(&path).await.ok()?;
        let cached = serde_json::from_slice::<CachedArkFile>(&bytes).ok()?;
        if cache_entry_is_valid(&cached) {
            return Some(cached);
        }
        let _ = tokio::fs::remove_file(path).await;
        None
    }

    async fn write_cache(&self, cache_key: &str, cached: &CachedArkFile) {
        let Some(cache_dir) = &self.cache_dir else {
            return;
        };
        let path = cache_dir.join(format!("{cache_key}.json"));
        let temporary = cache_dir.join(format!(".{}.tmp", Uuid::new_v4()));
        let result = serde_json::to_vec(cached)
            .map_err(|error| error.to_string())
            .and_then(|bytes| std::fs::write(&temporary, bytes).map_err(|error| error.to_string()))
            .and_then(|_| std::fs::rename(&temporary, &path).map_err(|error| error.to_string()));
        if let Err(error) = result {
            let _ = std::fs::remove_file(&temporary);
            if !path.exists() {
                log_warn(
                    &self.app,
                    "image_upload",
                    format!("Failed to cache Files API result: {error}"),
                );
            }
            return;
        }
        prune_cache(cache_dir);
    }
}

async fn parse_file_response(
    response: reqwest::Response,
    operation: &str,
) -> Result<ArkFile, String> {
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .or_else(|| response.headers().get("x-tt-logid"))
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let text = response
        .text()
        .await
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    if !status.is_success() {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            format!(
                "Files API {operation} failed: HTTP {} request_id={:?} body={}",
                status.as_u16(),
                request_id,
                truncate(&text, 2000)
            ),
        ));
    }

    serde_json::from_str(&text).map_err(|error| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!(
                "Files API {operation} returned invalid JSON: {error}; body={}",
                truncate(&text, 1000)
            ),
        )
    })
}

fn normalize_endpoint(raw: &str) -> Result<Url, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let endpoint = Url::parse(trimmed)
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    if !matches!(endpoint.scheme(), "http" | "https") {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            "Files API URL must use HTTP or HTTPS",
        ));
    }
    Ok(endpoint)
}

fn sanitized_url(url: &Url) -> String {
    let mut value = url.clone();
    value.set_query(None);
    value.set_fragment(None);
    value.to_string()
}

fn file_url(endpoint: &Url, file_id: &str) -> Result<Url, String> {
    let base = format!("{}/", endpoint.as_str().trim_end_matches('/'));
    Url::parse(&base)
        .and_then(|base| base.join(&urlencoding::encode(file_id)))
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

fn parse_image_data_url(data_url: &str, fallback_mime_type: &str) -> Result<ParsedDataUrl, String> {
    let (metadata, encoded) = data_url.split_once(',').ok_or_else(|| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            "Image attachment is not a data URL",
        )
    })?;
    if !metadata.starts_with("data:") || !metadata.contains(";base64") {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            "Image attachment must be Base64 encoded",
        ));
    }
    let mime_type = metadata
        .trim_start_matches("data:")
        .split(';')
        .next()
        .filter(|value| value.starts_with("image/"))
        .unwrap_or(fallback_mime_type);
    let bytes = general_purpose::STANDARD.decode(encoded).map_err(|error| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to decode image attachment: {error}"),
        )
    })?;
    Ok(ParsedDataUrl {
        mime_type: normalize_image_mime_type(mime_type).to_string(),
        bytes,
    })
}

fn normalize_image_mime_type(mime_type: &str) -> &str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpg" | "image/pjpeg" => "image/jpeg",
        "image/x-png" => "image/png",
        _ => mime_type,
    }
}

fn inline_image_locations(messages: &[Value]) -> Vec<ImagePartLocation> {
    let mut locations = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        let Some(parts) = message.get("content").and_then(Value::as_array) else {
            continue;
        };
        for (part_index, part) in parts.iter().enumerate() {
            let is_image = part.get("type").and_then(Value::as_str) == Some("image_url");
            let is_inline = part
                .get("image_url")
                .and_then(|image| image.get("url"))
                .and_then(Value::as_str)
                .map(|url| url.starts_with("data:image/"))
                .unwrap_or(false);
            if is_image && is_inline {
                locations.push(ImagePartLocation {
                    message_index,
                    part_index,
                });
            }
        }
    }
    locations
}

fn image_url_at(messages: &[Value], location: ImagePartLocation) -> Option<&str> {
    messages
        .get(location.message_index)?
        .get("content")?
        .as_array()?
        .get(location.part_index)?
        .get("image_url")?
        .get("url")?
        .as_str()
}

fn image_part_mut(
    messages: &mut [Value],
    location: ImagePartLocation,
) -> Option<&mut serde_json::Map<String, Value>> {
    messages
        .get_mut(location.message_index)?
        .get_mut("content")?
        .as_array_mut()?
        .get_mut(location.part_index)?
        .as_object_mut()
}

fn replace_with_file_id(
    messages: &mut [Value],
    location: ImagePartLocation,
    file_id: &str,
) -> bool {
    let Some(part) = image_part_mut(messages, location) else {
        return false;
    };
    part.insert("image_url".to_string(), json!({ "file_id": file_id }));
    true
}

fn replace_with_data_url(
    messages: &mut [Value],
    location: ImagePartLocation,
    data_url: String,
) -> bool {
    let Some(part) = image_part_mut(messages, location) else {
        return false;
    };
    part.insert(
        "image_url".to_string(),
        json!({ "url": data_url, "detail": "auto" }),
    );
    true
}

fn normalized_filename(filename: Option<&str>, mime_type: &str) -> String {
    let filename = filename.map(str::trim).filter(|value| !value.is_empty());
    if let Some(filename) = filename {
        return Path::new(filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("image")
            .to_string();
    }
    let extension = match mime_type {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    };
    format!("image.{extension}")
}

fn ark_file_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_cache_dir()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
        .join("llm-image-uploads")
        .join("volcengine-ark-files")
        .join("v1");
    std::fs::create_dir_all(&path)
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    Ok(path)
}

fn cache_entry_is_valid(cached: &CachedArkFile) -> bool {
    let now = now_ms();
    match cached.expires_at_ms {
        Some(expires_at) => expires_at > now.saturating_add(CACHE_EXPIRY_SAFETY_MS),
        None => cached.cached_at_ms.saturating_add(CACHE_FALLBACK_TTL_MS) > now,
    }
}

fn parse_expiry_ms(value: &Value) -> Option<u64> {
    let numeric = value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
        .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()));
    if let Some(value) = numeric {
        return Some(if value < 10_000_000_000 {
            value.saturating_mul(1000)
        } else {
            value
        });
    }
    value
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .and_then(|value| u64::try_from(value.timestamp_millis()).ok())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn prune_cache(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    let mut files = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                return None;
            }
            let modified = entry
                .metadata()
                .ok()?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((path, modified))
        })
        .collect::<Vec<_>>();
    if files.len() <= CACHE_MAX_ENTRIES {
        return;
    }
    files.sort_by_key(|(_, modified)| *modified);
    let remove_count = files.len().saturating_sub(CACHE_MAX_ENTRIES);
    for (path, _) in files.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(path);
    }
}

fn short_value(value: &str) -> &str {
    let end = value.len().min(12);
    &value[..end]
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{inline_image_locations, parse_expiry_ms, replace_with_file_id, ImagePartLocation};

    #[test]
    fn finds_and_rewrites_only_inline_images() {
        let mut messages = vec![json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "hello" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AA==", "detail": "auto" } },
                { "type": "image_url", "image_url": { "url": "https://example.com/image.png" } }
            ]
        })];

        let locations = inline_image_locations(&messages);
        assert_eq!(locations.len(), 1);
        assert!(replace_with_file_id(
            &mut messages,
            ImagePartLocation {
                message_index: 0,
                part_index: 1,
            },
            "file-123"
        ));
        assert_eq!(
            messages[0]["content"][1]["image_url"],
            json!({ "file_id": "file-123" })
        );
    }

    #[test]
    fn normalizes_second_and_millisecond_expiries() {
        assert_eq!(
            parse_expiry_ms(&json!(1_800_000_000_u64)),
            Some(1_800_000_000_000)
        );
        assert_eq!(
            parse_expiry_ms(&json!(1_800_000_000_123_u64)),
            Some(1_800_000_000_123)
        );
    }
}

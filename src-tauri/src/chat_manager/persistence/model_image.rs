use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageDecoder, ImageFormat, RgbImage, RgbaImage};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::storage_manager::legacy::storage_root;
use crate::storage_manager::media::SESSION_IMAGE_MAX_PIXELS;
use crate::utils::{log_debug, log_warn};

const MODEL_IMAGE_TARGET_BYTES: usize = 4 * 1024 * 1024;
const MODEL_IMAGE_CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;
const MODEL_IMAGE_CACHE_PROFILE: &[u8] = b"jpeg-v1-target-4m-q82-70-58";
const MODEL_IMAGE_JPEG_QUALITIES: [u8; 3] = [82, 70, 58];

pub struct ModelImagePayload {
    pub data_url: String,
    pub mime_type: String,
}

struct EncodedJpeg {
    bytes: Vec<u8>,
    quality: u8,
}

pub fn load_model_image_payload(
    app: &AppHandle,
    storage_path: &str,
) -> Result<ModelImagePayload, String> {
    let full_path = storage_root(app)?.join(storage_path);
    let bytes = fs::read(&full_path)
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    prepare_model_image_bytes(app, &bytes, storage_path)
}

pub fn prepare_inline_model_image_payload(
    app: &AppHandle,
    data_url: &str,
) -> Result<ModelImagePayload, String> {
    let encoded = data_url
        .split_once(',')
        .map(|(_, encoded)| encoded)
        .unwrap_or(data_url);
    let bytes = general_purpose::STANDARD.decode(encoded).map_err(|error| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to decode inline image attachment: {error}"),
        )
    })?;
    prepare_model_image_bytes(app, &bytes, "inline")
}

fn prepare_model_image_bytes(
    app: &AppHandle,
    source_bytes: &[u8],
    source_label: &str,
) -> Result<ModelImagePayload, String> {
    let format = image::guess_format(source_bytes).map_err(|error| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to detect model image format: {error}"),
        )
    })?;

    if source_bytes.len() <= MODEL_IMAGE_TARGET_BYTES {
        return Ok(data_url_payload(source_bytes, image_mime_type(format)));
    }

    let cache_dir = model_image_cache_dir(app)?;
    let cache_key = model_image_cache_key(source_bytes);
    let cache_path = cache_dir.join(format!("{cache_key}.jpg"));
    if let Ok(cached_bytes) = fs::read(&cache_path) {
        if !cached_bytes.is_empty() {
            log_debug(
                app,
                "model_image",
                format!(
                    "Model image cache hit: source={} source_bytes={} model_bytes={}",
                    source_label,
                    source_bytes.len(),
                    cached_bytes.len()
                ),
            );
            return Ok(data_url_payload(&cached_bytes, "image/jpeg"));
        }
    }

    let encoded = encode_model_jpeg(source_bytes, format)?;
    if encoded.bytes.len() >= source_bytes.len() {
        log_debug(
            app,
            "model_image",
            format!(
                "Kept original model image: source={} source_bytes={} jpeg_bytes={}",
                source_label,
                source_bytes.len(),
                encoded.bytes.len()
            ),
        );
        return Ok(data_url_payload(source_bytes, image_mime_type(format)));
    }

    if let Err(error) = write_cache_file(&cache_path, &encoded.bytes) {
        log_warn(
            app,
            "model_image",
            format!("Failed to cache compressed model image: {error}"),
        );
    } else {
        prune_model_image_cache(app, &cache_dir, &cache_path);
    }

    log_debug(
        app,
        "model_image",
        format!(
            "Prepared model image: source={} source_bytes={} model_bytes={} jpeg_quality={}",
            source_label,
            source_bytes.len(),
            encoded.bytes.len(),
            encoded.quality
        ),
    );

    Ok(data_url_payload(&encoded.bytes, "image/jpeg"))
}

fn encode_model_jpeg(source_bytes: &[u8], format: ImageFormat) -> Result<EncodedJpeg, String> {
    let (width, height) = image::ImageReader::with_format(Cursor::new(source_bytes), format)
        .into_dimensions()
        .map_err(|error| {
            crate::utils::err_msg(
                module_path!(),
                line!(),
                format!("Failed to inspect model image: {error}"),
            )
        })?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0 || height == 0 || pixels > SESSION_IMAGE_MAX_PIXELS {
        return Err(crate::utils::err_msg(
            module_path!(),
            line!(),
            format!(
                "Model image is {width}x{height} ({pixels} pixels); the limit is {} pixels",
                SESSION_IMAGE_MAX_PIXELS
            ),
        ));
    }

    let mut decoder = image::ImageReader::with_format(Cursor::new(source_bytes), format)
        .into_decoder()
        .map_err(|error| {
            crate::utils::err_msg(
                module_path!(),
                line!(),
                format!("Failed to initialize model image decoder: {error}"),
            )
        })?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(|error| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to decode model image: {error}"),
        )
    })?;
    image.apply_orientation(orientation);
    let rgb = into_jpeg_rgb(image);
    let mut smallest: Option<EncodedJpeg> = None;

    for quality in MODEL_IMAGE_JPEG_QUALITIES {
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
            .encode_image(&rgb)
            .map_err(|error| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to encode model image as JPEG: {error}"),
                )
            })?;

        if bytes.len() <= MODEL_IMAGE_TARGET_BYTES {
            return Ok(EncodedJpeg { bytes, quality });
        }

        if smallest
            .as_ref()
            .map(|current| bytes.len() < current.bytes.len())
            .unwrap_or(true)
        {
            smallest = Some(EncodedJpeg { bytes, quality });
        }
    }

    smallest.ok_or_else(|| {
        crate::utils::err_msg(module_path!(), line!(), "Failed to encode model image")
    })
}

fn into_jpeg_rgb(image: DynamicImage) -> RgbImage {
    match image {
        DynamicImage::ImageRgb8(rgb) => rgb,
        DynamicImage::ImageRgba8(rgba) => flatten_rgba_onto_white(rgba),
        image if image.color().has_alpha() => flatten_rgba_onto_white(image.to_rgba8()),
        image => image.to_rgb8(),
    }
}

fn flatten_rgba_onto_white(image: RgbaImage) -> RgbImage {
    let (width, height) = image.dimensions();
    let mut bytes = image.into_raw();
    let pixel_count = width as usize * height as usize;

    for index in 0..pixel_count {
        let source = index * 4;
        let target = index * 3;
        let alpha = u16::from(bytes[source + 3]);
        let inverse_alpha = 255 - alpha;
        for channel in 0..3 {
            let value = u16::from(bytes[source + channel]);
            bytes[target + channel] = ((value * alpha + 255 * inverse_alpha + 127) / 255) as u8;
        }
    }

    bytes.truncate(pixel_count * 3);
    RgbImage::from_raw(width, height, bytes).expect("RGBA compaction preserves image dimensions")
}

fn model_image_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
        .join("llm-image-attachments")
        .join("v1");
    fs::create_dir_all(&cache_dir)
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    Ok(cache_dir)
}

fn model_image_cache_key(bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(MODEL_IMAGE_CACHE_PROFILE);
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

fn write_cache_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            "Model image cache path has no parent",
        )
    })?;
    let temporary_path = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    fs::write(&temporary_path, bytes)
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    match fs::rename(&temporary_path, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            let _ = fs::remove_file(&temporary_path);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary_path);
            Err(crate::utils::err_to_string(module_path!(), line!(), error))
        }
    }
}

fn prune_model_image_cache(app: &AppHandle, cache_dir: &Path, keep_path: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jpg") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        total_bytes = total_bytes.saturating_add(metadata.len());
        files.push((
            path,
            metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            metadata.len(),
        ));
    }

    if total_bytes <= MODEL_IMAGE_CACHE_MAX_BYTES {
        return;
    }

    files.sort_by_key(|(_, modified, _)| *modified);
    for (path, _, size) in files {
        if total_bytes <= MODEL_IMAGE_CACHE_MAX_BYTES {
            break;
        }
        if path == keep_path {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            total_bytes = total_bytes.saturating_sub(size);
        }
    }

    log_debug(
        app,
        "model_image",
        format!("Pruned model image cache to {total_bytes} bytes"),
    );
}

fn image_mime_type(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::Gif => "image/gif",
        ImageFormat::WebP => "image/webp",
        _ => "application/octet-stream",
    }
}

fn data_url_payload(bytes: &[u8], mime_type: &str) -> ModelImagePayload {
    ModelImagePayload {
        data_url: format!(
            "data:{mime_type};base64,{}",
            general_purpose::STANDARD.encode(bytes)
        ),
        mime_type: mime_type.to_string(),
    }
}

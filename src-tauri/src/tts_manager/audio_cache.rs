use base64::{engine::general_purpose::STANDARD, Engine};
use std::fs;
use std::path::PathBuf;

use crate::storage_manager::legacy::storage_root;

const TTS_AUDIO_DIR: &str = "tts_audio";

pub fn generate_cache_key(
    provider_id: &str,
    model_id: &str,
    voice_id: &str,
    text: &str,
    prompt: Option<&str>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    // Bump the cache namespace when audio encoding defaults change.
    hasher.update(b"tts-quality-v5-stream-pcm-clone-24000|");
    hasher.update(provider_id.as_bytes());
    hasher.update(b"|");
    hasher.update(model_id.as_bytes());
    hasher.update(b"|");
    hasher.update(voice_id.as_bytes());
    hasher.update(b"|");
    hasher.update(text.as_bytes());
    hasher.update(b"|");
    if let Some(p) = prompt {
        hasher.update(p.as_bytes());
    }
    let result = hasher.finalize();
    result.to_hex().to_string()
}

pub fn pcm16_mono_to_wav(audio_data: &[u8], sample_rate: u32) -> Result<Vec<u8>, String> {
    if sample_rate == 0 {
        return Err("PCM sample rate must be greater than zero".to_string());
    }
    if audio_data.len() % 2 != 0 {
        return Err("PCM16 audio data must contain complete samples".to_string());
    }

    let data_size = u32::try_from(audio_data.len())
        .map_err(|_| "PCM audio data is too large for a WAV container".to_string())?;
    let riff_size = 36u32
        .checked_add(data_size)
        .ok_or_else(|| "PCM audio data is too large for a WAV container".to_string())?;
    let byte_rate = sample_rate
        .checked_mul(2)
        .ok_or_else(|| "PCM sample rate is too large for a WAV container".to_string())?;

    let mut wav = Vec::with_capacity(44 + audio_data.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(audio_data);
    Ok(wav)
}

/// Get the directory for TTS audio cache
fn tts_audio_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let root = storage_root(app)?;
    let dir = root.join(TTS_AUDIO_DIR);
    fs::create_dir_all(&dir).map_err(|e| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to create TTS audio directory: {}", e),
        )
    })?;
    Ok(dir)
}

fn format_to_extension(format: &str) -> &str {
    match format {
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" | "audio/wave" => "wav",
        "audio/ogg" => "ogg",
        "audio/webm" => "webm",
        "audio/pcm" => "pcm",
        _ => "audio",
    }
}

fn extension_to_format(ext: &str) -> String {
    match ext {
        "mp3" => "audio/mpeg".to_string(),
        "wav" => "audio/wav".to_string(),
        "ogg" => "audio/ogg".to_string(),
        "webm" => "audio/webm".to_string(),
        "pcm" => "audio/pcm".to_string(),
        _ => "audio/octet-stream".to_string(),
    }
}

pub fn save_audio_to_cache(
    app: &tauri::AppHandle,
    cache_key: &str,
    audio_data: &[u8],
    format: &str,
) -> Result<(), String> {
    let dir = tts_audio_dir(app)?;
    let ext = format_to_extension(format);
    let file_path = dir.join(format!("{}.{}", cache_key, ext));
    fs::write(&file_path, audio_data).map_err(|e| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to save TTS audio to cache: {}", e),
        )
    })?;
    Ok(())
}

pub fn load_audio_from_cache(
    app: &tauri::AppHandle,
    cache_key: &str,
) -> Result<Option<(Vec<u8>, String)>, String> {
    let dir = tts_audio_dir(app)?;

    for ext in &["mp3", "wav", "ogg", "webm", "pcm", "audio"] {
        let file_path = dir.join(format!("{}.{}", cache_key, ext));
        if file_path.exists() {
            let audio_data = fs::read(&file_path).map_err(|e| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to read TTS audio from cache: {}", e),
                )
            })?;
            let format = extension_to_format(ext);
            return Ok(Some((audio_data, format)));
        }
    }

    Ok(None)
}

pub fn audio_exists_in_cache(app: &tauri::AppHandle, cache_key: &str) -> Result<bool, String> {
    let dir = tts_audio_dir(app)?;

    for ext in &["mp3", "wav", "ogg", "webm", "pcm", "audio"] {
        let file_path = dir.join(format!("{}.{}", cache_key, ext));
        if file_path.exists() {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn delete_audio_from_cache(app: &tauri::AppHandle, cache_key: &str) -> Result<(), String> {
    let dir = tts_audio_dir(app)?;

    for ext in &["mp3", "wav", "ogg", "webm", "pcm", "audio"] {
        let file_path = dir.join(format!("{}.{}", cache_key, ext));
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to delete TTS audio from cache: {}", e),
                )
            })?;
        }
    }

    Ok(())
}

pub fn clear_audio_cache(app: &tauri::AppHandle) -> Result<u64, String> {
    let dir = tts_audio_dir(app)?;
    let mut count = 0u64;

    if dir.exists() {
        for entry in fs::read_dir(&dir)
            .map_err(|e| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to read TTS cache directory: {}", e),
                )
            })?
            .flatten()
        {
            let path = entry.path();
            if path.is_file() && fs::remove_file(&path).is_ok() {
                count += 1;
            }
        }
    }

    Ok(count)
}

pub fn get_cache_size(app: &tauri::AppHandle) -> Result<u64, String> {
    let dir = tts_audio_dir(app)?;
    let mut total_size = 0u64;

    if dir.exists() {
        for entry in fs::read_dir(&dir)
            .map_err(|e| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to read TTS cache directory: {}", e),
                )
            })?
            .flatten()
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }
    }

    Ok(total_size)
}

pub fn get_cache_count(app: &tauri::AppHandle) -> Result<u64, String> {
    let dir = tts_audio_dir(app)?;
    let mut count = 0u64;

    if dir.exists() {
        for entry in fs::read_dir(&dir)
            .map_err(|e| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to read TTS cache directory: {}", e),
                )
            })?
            .flatten()
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

#[tauri::command]
pub fn tts_cache_key(
    provider_id: String,
    model_id: String,
    voice_id: String,
    text: String,
    prompt: Option<String>,
) -> String {
    generate_cache_key(&provider_id, &model_id, &voice_id, &text, prompt.as_deref())
}

#[tauri::command]
pub fn tts_cache_exists(app: tauri::AppHandle, cache_key: String) -> Result<bool, String> {
    audio_exists_in_cache(&app, &cache_key)
}

#[tauri::command]
pub fn tts_cache_get(
    app: tauri::AppHandle,
    cache_key: String,
) -> Result<Option<super::types::TtsPreviewResponse>, String> {
    match load_audio_from_cache(&app, &cache_key)? {
        Some((audio_data, format)) => {
            let audio_base64 = STANDARD.encode(&audio_data);
            Ok(Some(super::types::TtsPreviewResponse {
                audio_base64,
                format,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn tts_cache_save(
    app: tauri::AppHandle,
    cache_key: String,
    audio_base64: String,
    format: String,
) -> Result<(), String> {
    let audio_data = STANDARD.decode(&audio_base64).map_err(|e| {
        crate::utils::err_msg(
            module_path!(),
            line!(),
            format!("Failed to decode audio base64: {}", e),
        )
    })?;
    save_audio_to_cache(&app, &cache_key, &audio_data, &format)
}

#[tauri::command]
pub fn tts_cache_delete(app: tauri::AppHandle, cache_key: String) -> Result<(), String> {
    delete_audio_from_cache(&app, &cache_key)
}

#[tauri::command]
pub fn tts_cache_clear(app: tauri::AppHandle) -> Result<u64, String> {
    clear_audio_cache(&app)
}

#[tauri::command]
pub fn tts_cache_stats(app: tauri::AppHandle) -> Result<TtsCacheStats, String> {
    let size_bytes = get_cache_size(&app)?;
    let count = get_cache_count(&app)?;
    Ok(TtsCacheStats { size_bytes, count })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsCacheStats {
    pub size_bytes: u64,
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::pcm16_mono_to_wav;

    #[test]
    fn wraps_pcm16_mono_with_sample_rate_metadata() {
        let pcm = [0x01, 0x02, 0x03, 0x04];
        let wav = pcm16_mono_to_wav(&pcm, 24_000).expect("valid PCM should be wrapped");

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 24_000);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4);
        assert_eq!(&wav[44..], &pcm);
    }

    #[test]
    fn rejects_incomplete_pcm16_sample() {
        assert!(pcm16_mono_to_wav(&[0x01], 24_000).is_err());
    }
}

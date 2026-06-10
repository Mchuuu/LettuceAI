use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::registry::diffusion_root;
use super::types::SdBinaryInfo;

const RELEASES_API: &str =
    "https://api.github.com/repos/leejet/stable-diffusion.cpp/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdEngineVariant {
    pub id: String,
    pub asset_name: String,
    pub size: u64,
    pub release_tag: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdQueuedInstall {
    pub install_id: String,
    pub queue_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingInstall {
    variant: String,
    release_tag: String,
}

pub fn bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = diffusion_root(app)?.join("bin");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn binary_info_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(diffusion_root(app)?.join("binary.json"))
}

pub fn read_binary_info(app: &AppHandle) -> Option<SdBinaryInfo> {
    let path = binary_info_path(app).ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let info: SdBinaryInfo = serde_json::from_str(&raw).ok()?;
    if PathBuf::from(&info.path).is_file() {
        Some(info)
    } else {
        None
    }
}

pub fn write_binary_info(app: &AppHandle, info: &SdBinaryInfo) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(info).map_err(|e| e.to_string())?;
    fs::write(binary_info_path(app)?, raw).map_err(|e| e.to_string())
}

pub fn detect_recommended_variant() -> String {
    if cfg!(target_os = "macos") {
        return "default".to_string();
    }
    if cfg!(target_os = "windows") && has_nvidia_gpu() {
        return "cuda".to_string();
    }
    "vulkan".to_string()
}

fn has_nvidia_gpu() -> bool {
    if cfg!(target_os = "macos") {
        return false;
    }
    std::process::Command::new("nvidia-smi")
        .arg("-L")
        .output()
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}

async fn fetch_latest_release() -> Result<GithubRelease, String> {
    let client = reqwest::Client::builder()
        .user_agent("LettuceAI/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(RELEASES_API)
        .send()
        .await
        .map_err(|e| format!("Failed to query stable-diffusion.cpp releases: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to query stable-diffusion.cpp releases: HTTP {}",
            response.status()
        ));
    }
    response
        .json::<GithubRelease>()
        .await
        .map_err(|e| format!("Invalid release response: {e}"))
}

fn matches_current_os(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if cfg!(target_os = "windows") {
        lower.contains("win")
    } else if cfg!(target_os = "macos") {
        lower.contains("darwin") || lower.contains("macos")
    } else {
        lower.contains("linux") || lower.contains("ubuntu")
    }
}

fn classify_backend(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    if !lower.starts_with("sd-") || !lower.ends_with(".zip") {
        return None;
    }
    if lower.contains("cudart") {
        return None;
    }
    if lower.contains("cuda") || lower.contains("cu12") {
        return Some("cuda".to_string());
    }
    if lower.contains("vulkan") {
        return Some("vulkan".to_string());
    }
    if lower.contains("rocm") {
        return Some("rocm".to_string());
    }
    if cfg!(target_os = "macos") {
        return Some("default".to_string());
    }
    Some("cpu".to_string())
}

fn cpu_tier_score(name: &str) -> i32 {
    let lower = name.to_ascii_lowercase();
    let supported = |feature: &str| -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            match feature {
                "avx512" => std::arch::is_x86_feature_detected!("avx512f"),
                "avx2" => std::arch::is_x86_feature_detected!("avx2"),
                "avx" => std::arch::is_x86_feature_detected!("avx"),
                _ => true,
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = feature;
            false
        }
    };
    if lower.contains("noavx") {
        1
    } else if lower.contains("avx512") {
        if supported("avx512") {
            4
        } else {
            -1
        }
    } else if lower.contains("avx2") {
        if supported("avx2") {
            3
        } else {
            -1
        }
    } else if lower.contains("avx") {
        if supported("avx") {
            2
        } else {
            -1
        }
    } else {
        3
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let chunks = |s: &str| -> Vec<(u64, String)> {
        let mut out = Vec::new();
        let mut digits = String::new();
        let mut text = String::new();
        for ch in s.chars() {
            if ch.is_ascii_digit() {
                if !text.is_empty() {
                    out.push((0, std::mem::take(&mut text)));
                }
                digits.push(ch);
            } else {
                if !digits.is_empty() {
                    out.push((digits.parse().unwrap_or(0), String::new()));
                    digits.clear();
                }
                text.push(ch);
            }
        }
        if !digits.is_empty() {
            out.push((digits.parse().unwrap_or(0), String::new()));
        }
        if !text.is_empty() {
            out.push((0, text));
        }
        out
    };
    chunks(a).cmp(&chunks(b))
}

fn pick_asset_for_variant(assets: &[GithubAsset], variant: &str) -> Option<GithubAsset> {
    let mut candidates: Vec<&GithubAsset> = assets
        .iter()
        .filter(|asset| matches_current_os(&asset.name))
        .filter(|asset| classify_backend(&asset.name).as_deref() == Some(variant))
        .collect();
    if variant == "cpu" {
        candidates.retain(|asset| cpu_tier_score(&asset.name) > 0);
        candidates.sort_by_key(|asset| cpu_tier_score(&asset.name));
    } else {
        candidates.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    }
    candidates.last().map(|asset| (*asset).clone())
}

fn find_cudart_companion(assets: &[GithubAsset]) -> Option<GithubAsset> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    assets
        .iter()
        .find(|asset| {
            asset.name.to_ascii_lowercase().contains("cudart")
                && matches_current_os(&asset.name)
        })
        .cloned()
}

pub async fn list_engine_variants() -> Result<Vec<SdEngineVariant>, String> {
    let release = fetch_latest_release().await?;
    let recommended = detect_recommended_variant();
    let mut variant_ids: Vec<String> = release
        .assets
        .iter()
        .filter(|asset| matches_current_os(&asset.name))
        .filter_map(|asset| classify_backend(&asset.name))
        .collect();
    variant_ids.sort();
    variant_ids.dedup();

    let mut variants = Vec::new();
    for id in variant_ids {
        if let Some(asset) = pick_asset_for_variant(&release.assets, &id) {
            variants.push(SdEngineVariant {
                recommended: id == recommended,
                id,
                asset_name: asset.name,
                size: asset.size,
                release_tag: release.tag_name.clone(),
            });
        }
    }
    if variants.is_empty() {
        let names: Vec<String> = release.assets.iter().map(|a| a.name.clone()).collect();
        return Err(format!(
            "No compatible engine build found in release {}. Assets: {}",
            release.tag_name,
            names.join(", ")
        ));
    }
    variants.sort_by_key(|variant| !variant.recommended);
    Ok(variants)
}

pub async fn queue_binary_install(
    app: &AppHandle,
    variant: Option<String>,
) -> Result<SdQueuedInstall, String> {
    let release = fetch_latest_release().await?;
    let variant = variant.unwrap_or_else(detect_recommended_variant);
    let asset = pick_asset_for_variant(&release.assets, &variant).ok_or_else(|| {
        let names: Vec<String> = release
            .assets
            .iter()
            .filter(|a| matches_current_os(&a.name))
            .map(|a| a.name.clone())
            .collect();
        format!(
            "No {} build for this platform in release {}. Available: {}",
            variant,
            release.tag_name,
            names.join(", ")
        )
    })?;

    let target_dir = bin_dir(app)?;
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let mut downloads = vec![asset];
    if variant == "cuda" {
        if let Some(cudart) = find_cudart_companion(&release.assets) {
            downloads.push(cudart);
        }
    }

    let pending = PendingInstall {
        variant: variant.clone(),
        release_tag: release.tag_name.clone(),
    };
    let pending_raw = serde_json::to_string(&pending).map_err(|e| e.to_string())?;
    fs::write(diffusion_root(app)?.join("pending_install.json"), pending_raw)
        .map_err(|e| e.to_string())?;

    let install_id = uuid::Uuid::new_v4().to_string();
    let mut queue_ids = Vec::new();
    for asset in downloads {
        let destination = target_dir.join(&asset.name);
        let metadata = crate::hf_browser::QueueDownloadMetadata {
            install_id: Some(install_id.clone()),
            display_name: Some(format!("Stable Diffusion engine ({variant})")),
            download_role: Some("engine".to_string()),
            queue_kind: Some("sdcpp".to_string()),
            install_kind: Some("engine".to_string()),
            variant: Some(variant.clone()),
            download_url: Some(asset.browser_download_url.clone()),
            destination_path: Some(destination.to_string_lossy().to_string()),
            force_redownload: true,
            ..Default::default()
        };
        let queue_id = crate::hf_browser::hf_queue_download(
            app.clone(),
            "leejet/stable-diffusion.cpp".to_string(),
            asset.name.clone(),
            Some(metadata),
        )
        .await?;
        queue_ids.push(queue_id);
    }

    Ok(SdQueuedInstall {
        install_id,
        queue_ids,
    })
}

pub fn finalize_binary_install(app: &AppHandle) -> Result<SdBinaryInfo, String> {
    let target_dir = bin_dir(app)?;
    let pending_path = diffusion_root(app)?.join("pending_install.json");
    let pending: PendingInstall = fs::read_to_string(&pending_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .ok_or_else(|| "No pending engine install found".to_string())?;

    let zips: Vec<PathBuf> = fs::read_dir(&target_dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map(|ext| ext.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        })
        .collect();
    if zips.is_empty() {
        return Err("No downloaded engine archive found".to_string());
    }

    for zip_path in &zips {
        extract_zip_flat(zip_path, &target_dir)?;
        fs::remove_file(zip_path).map_err(|e| e.to_string())?;
    }

    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &["sd-cli.exe", "sd.exe"]
    } else {
        &["sd-cli", "sd"]
    };
    let binary_path = candidates
        .iter()
        .map(|name| target_dir.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "Engine archive did not contain a CLI binary ({})",
                candidates.join(" or ")
            )
        })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }

    let info = SdBinaryInfo {
        path: binary_path.to_string_lossy().to_string(),
        variant: pending.variant,
        release_tag: pending.release_tag,
    };
    write_binary_info(app, &info)?;
    let _ = fs::remove_file(pending_path);
    Ok(info)
}

fn extract_zip_flat(zip_path: &PathBuf, target_dir: &PathBuf) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to open engine archive: {e}"))?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(filename) = PathBuf::from(entry.name())
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            continue;
        };
        let out_path = target_dir.join(filename);
        let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn remove_binary(app: &AppHandle) -> Result<(), String> {
    let target_dir = bin_dir(app)?;
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir).map_err(|e| e.to_string())?;
    }
    let info_path = binary_info_path(app)?;
    if info_path.exists() {
        fs::remove_file(info_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

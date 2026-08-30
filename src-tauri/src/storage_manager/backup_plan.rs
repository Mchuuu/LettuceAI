use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::Manager;
use walkdir::WalkDir;

use super::db::open_db;
use super::legacy::storage_root;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BackupSelectionMode {
    #[default]
    Full,
    Custom,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCharacterSelection {
    pub character_id: String,
    #[serde(default = "default_true")]
    pub include_audio: bool,
    #[serde(default = "default_true")]
    pub include_images: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSelection {
    #[serde(default)]
    pub mode: BackupSelectionMode,
    #[serde(default)]
    pub characters: Vec<BackupCharacterSelection>,
    #[serde(default = "default_true")]
    pub include_unassigned_audio: bool,
}

impl Default for BackupSelection {
    fn default() -> Self {
        Self {
            mode: BackupSelectionMode::Full,
            characters: Vec::new(),
            include_unassigned_audio: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCharacterEstimate {
    pub character_id: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub core_bytes: u64,
    pub audio_bytes: u64,
    pub image_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEstimate {
    pub characters: Vec<BackupCharacterEstimate>,
    pub data_bytes: u64,
    pub resource_bytes: u64,
    pub unassigned_audio_bytes: u64,
    pub total_bytes: u64,
    pub total_files: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct BackupArchiveFile {
    pub source_path: PathBuf,
    pub archive_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct BackupScope {
    pub mode: BackupSelectionMode,
    pub selected_character_ids: HashSet<String>,
    pub selected_session_ids: HashSet<String>,
    pub selected_group_session_ids: HashSet<String>,
    pub selected_group_character_ids: HashSet<String>,
}

impl BackupScope {
    pub(crate) fn is_full(&self) -> bool {
        self.mode == BackupSelectionMode::Full
    }

    pub(crate) fn includes_character(&self, character_id: &str) -> bool {
        self.is_full() || self.selected_character_ids.contains(character_id)
    }

    pub(crate) fn includes_session(&self, session_id: &str) -> bool {
        self.is_full() || self.selected_session_ids.contains(session_id)
    }

    pub(crate) fn includes_group_session(&self, session_id: &str) -> bool {
        self.is_full() || self.selected_group_session_ids.contains(session_id)
    }

    pub(crate) fn includes_group_character(&self, group_character_id: &str) -> bool {
        self.is_full()
            || self
                .selected_group_character_ids
                .contains(group_character_id)
    }
}

pub(crate) struct BackupPlan {
    pub selection: BackupSelection,
    pub scope: BackupScope,
    pub files: Vec<BackupArchiveFile>,
    pub estimate: BackupEstimate,
}

#[derive(Default)]
struct CharacterResources {
    images: HashMap<String, BackupArchiveFile>,
    audio: HashMap<String, BackupArchiveFile>,
}

impl CharacterResources {
    fn image_bytes(&self) -> u64 {
        self.images.values().map(|file| file.size_bytes).sum()
    }

    fn audio_bytes(&self) -> u64 {
        self.audio.values().map(|file| file.size_bytes).sum()
    }
}

fn archive_path(prefix: &str, relative: &Path) -> String {
    let relative = relative.to_string_lossy().replace('\\', "/");
    format!("{prefix}/{relative}")
}

fn archive_file(source_path: PathBuf, archive_name: String) -> Option<BackupArchiveFile> {
    let size_bytes = source_path.metadata().ok()?.len();
    Some(BackupArchiveFile {
        source_path,
        archive_name,
        size_bytes,
    })
}

fn add_directory_files(
    target: &mut HashMap<String, BackupArchiveFile>,
    dir: &Path,
    prefix: &str,
    predicate: impl Fn(&Path) -> bool,
) {
    if !dir.exists() {
        return;
    }

    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !predicate(path) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(dir) else {
            continue;
        };
        let name = archive_path(prefix, relative);
        if let Some(file) = archive_file(path.to_path_buf(), name.clone()) {
            target.entry(name).or_insert(file);
        }
    }
}

fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp"
    )
}

fn is_audio(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "wav" | "mp3" | "ogg" | "flac" | "aac" | "aiff" | "m4a" | "pcm"
    )
}

fn add_image_id(
    images_dir: &Path,
    image_id: &str,
    target: &mut HashMap<String, BackupArchiveFile>,
) {
    let image_id = image_id.trim();
    if image_id.is_empty() || image_id.starts_with("data:") || image_id.contains('/') {
        return;
    }
    for extension in ["jpg", "jpeg", "png", "gif", "webp"] {
        let source_path = images_dir.join(format!("{image_id}.{extension}"));
        if !source_path.is_file() {
            continue;
        }
        let name = format!("images/{image_id}.{extension}");
        if let Some(file) = archive_file(source_path, name.clone()) {
            target.entry(name).or_insert(file);
        }
        break;
    }
}

fn collect_attachment_storage_paths(value: &JsonValue, paths: &mut HashSet<String>) {
    match value {
        JsonValue::Object(map) => {
            for (key, value) in map {
                if matches!(key.as_str(), "storagePath" | "storage_path") {
                    if let Some(path) = value.as_str() {
                        paths.insert(path.replace('\\', "/"));
                    }
                }
                collect_attachment_storage_paths(value, paths);
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                collect_attachment_storage_paths(item, paths);
            }
        }
        _ => {}
    }
}

fn add_storage_path(
    storage: &Path,
    generated_images: &Path,
    relative: &str,
    images: &mut HashMap<String, BackupArchiveFile>,
    audio: &mut HashMap<String, BackupArchiveFile>,
) {
    let normalized = relative.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return;
    }

    let (source_path, archive_name) =
        if let Some(rest) = normalized.strip_prefix("generated_images/") {
            (generated_images.join(rest), normalized)
        } else {
            (storage.join(&normalized), normalized)
        };
    if !source_path.is_file() {
        return;
    }
    let Some(file) = archive_file(source_path, archive_name.clone()) else {
        return;
    };
    if is_audio(Path::new(&archive_name)) {
        audio.entry(archive_name).or_insert(file);
    } else if is_image(Path::new(&archive_name)) {
        images.entry(archive_name).or_insert(file);
    }
}

fn parse_character_ids(raw: &str) -> HashSet<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn resolve_scope(
    conn: &Connection,
    selection: &BackupSelection,
    all_character_ids: &HashSet<String>,
) -> Result<BackupScope, String> {
    let selected_character_ids = if selection.mode == BackupSelectionMode::Full {
        all_character_ids.clone()
    } else {
        selection
            .characters
            .iter()
            .filter(|item| all_character_ids.contains(&item.character_id))
            .map(|item| item.character_id.clone())
            .collect()
    };

    let mut selected_session_ids = HashSet::new();
    let mut stmt = conn
        .prepare("SELECT id, character_id FROM sessions")
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    for row in rows {
        let (id, character_id) =
            row.map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        if selected_character_ids.contains(&character_id) {
            selected_session_ids.insert(id);
        }
    }

    let mut selected_group_session_ids = HashSet::new();
    let mut selected_group_character_ids = HashSet::new();
    let mut stmt = conn
        .prepare("SELECT id, group_character_id, character_ids FROM group_sessions")
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    for row in rows {
        let (id, group_character_id, raw_ids) =
            row.map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let ids = parse_character_ids(&raw_ids);
        if !ids.is_empty() && ids.is_subset(&selected_character_ids) {
            selected_group_session_ids.insert(id);
            if let Some(group_character_id) = group_character_id {
                selected_group_character_ids.insert(group_character_id);
            }
        }
    }

    let mut stmt = conn
        .prepare("SELECT id, character_ids FROM group_characters")
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    for row in rows {
        let (id, raw_ids) =
            row.map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let ids = parse_character_ids(&raw_ids);
        if !ids.is_empty() && ids.is_subset(&selected_character_ids) {
            selected_group_character_ids.insert(id);
        }
    }

    Ok(BackupScope {
        mode: selection.mode,
        selected_character_ids,
        selected_session_ids,
        selected_group_session_ids,
        selected_group_character_ids,
    })
}

fn estimate_character_core_bytes(conn: &Connection, character_id: &str) -> u64 {
    let query = r#"
        SELECT COALESCE(SUM(
          LENGTH(COALESCE(m.content, '')) +
          LENGTH(COALESCE(m.attachments, '')) +
          LENGTH(COALESCE(m.reasoning, '')) + 256
        ), 0)
        FROM messages m
        JOIN sessions s ON s.id = m.session_id
        WHERE s.character_id = ?1
    "#;
    let messages = conn
        .query_row(query, params![character_id], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
        .max(0) as u64;

    let variants = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(LENGTH(COALESCE(v.content, '')) + 128), 0)
            FROM message_variants v
            JOIN messages m ON m.id = v.message_id
            JOIN sessions s ON s.id = m.session_id
            WHERE s.character_id = ?1
            "#,
            params![character_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;

    messages.saturating_add(variants).saturating_add(8 * 1024)
}

fn database_size(conn: &Connection) -> u64 {
    let page_count = conn
        .query_row("PRAGMA page_count", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
        .max(0) as u64;
    let page_size = conn
        .query_row("PRAGMA page_size", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0)
        .max(0) as u64;
    page_count.saturating_mul(page_size)
}

fn character_image_ids(conn: &Connection, character_id: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(Some((avatar, background))) = conn
        .query_row(
            "SELECT avatar_path, background_image_path FROM characters WHERE id = ?1",
            params![character_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .optional()
    {
        ids.extend(avatar);
        ids.extend(background);
    }

    for query in [
        "SELECT background_image_path FROM scenes WHERE character_id = ?1",
        "SELECT background_image_path FROM sessions WHERE character_id = ?1",
    ] {
        let Ok(mut stmt) = conn.prepare(query) else {
            continue;
        };
        let Ok(rows) = stmt.query_map(params![character_id], |row| row.get::<_, Option<String>>(0))
        else {
            continue;
        };
        for value in rows.flatten().flatten() {
            ids.insert(value);
        }
    }
    ids
}

fn collect_selected_attachment_paths(conn: &Connection, character_id: &str) -> HashSet<String> {
    let mut paths = HashSet::new();
    let Ok(mut stmt) = conn.prepare(
        r#"
        SELECT m.attachments
        FROM messages m
        JOIN sessions s ON s.id = m.session_id
        WHERE s.character_id = ?1 AND m.attachments IS NOT NULL AND m.attachments != ''
        "#,
    ) else {
        return paths;
    };
    let Ok(rows) = stmt.query_map(params![character_id], |row| row.get::<_, String>(0)) else {
        return paths;
    };
    for raw in rows.flatten() {
        if let Ok(value) = serde_json::from_str::<JsonValue>(&raw) {
            collect_attachment_storage_paths(&value, &mut paths);
        }
    }
    paths
}

fn collect_group_resources(
    conn: &Connection,
    scope: &BackupScope,
    selection_by_character: &HashMap<&str, &BackupCharacterSelection>,
    storage: &Path,
    generated_images: &Path,
) -> Result<HashMap<String, BackupArchiveFile>, String> {
    let mut files = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT id, character_ids, background_image_path FROM group_sessions")
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    for row in rows {
        let (session_id, raw_character_ids, background_image_id) =
            row.map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        if !scope.includes_group_session(&session_id) {
            continue;
        }
        let character_ids = parse_character_ids(&raw_character_ids);
        let include_images = character_ids.iter().any(|id| {
            selection_by_character
                .get(id.as_str())
                .is_some_and(|item| item.include_images)
        });
        let include_audio = character_ids.iter().any(|id| {
            selection_by_character
                .get(id.as_str())
                .is_some_and(|item| item.include_audio)
        });
        if include_images {
            if let Some(image_id) = background_image_id {
                add_image_id(&storage.join("images"), &image_id, &mut files);
            }
        }

        let mut message_stmt = conn
            .prepare(
                "SELECT attachments FROM group_messages WHERE session_id = ?1 AND attachments IS NOT NULL AND attachments != ''",
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let attachments = message_stmt
            .query_map(params![session_id], |row| row.get::<_, String>(0))
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        for raw in attachments.flatten() {
            let Ok(value) = serde_json::from_str::<JsonValue>(&raw) else {
                continue;
            };
            let mut paths = HashSet::new();
            collect_attachment_storage_paths(&value, &mut paths);
            for path in paths {
                let mut images = HashMap::new();
                let mut audio = HashMap::new();
                add_storage_path(storage, generated_images, &path, &mut images, &mut audio);
                if include_images {
                    files.extend(images);
                }
                if include_audio {
                    files.extend(audio);
                }
            }
        }
    }
    Ok(files)
}

fn tts_files_by_character(
    conn: &Connection,
    tts_dir: &Path,
    all_character_ids: &HashSet<String>,
) -> Result<
    (
        HashMap<String, HashMap<String, BackupArchiveFile>>,
        HashMap<String, BackupArchiveFile>,
    ),
    String,
> {
    let mut owners_by_key: HashMap<String, HashSet<String>> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT cache_key, character_id FROM tts_cache_references WHERE character_id IS NOT NULL",
        )
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    for row in rows {
        let (key, character_id) =
            row.map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        if all_character_ids.contains(&character_id) {
            owners_by_key.entry(key).or_default().insert(character_id);
        }
    }

    let mut by_character: HashMap<String, HashMap<String, BackupArchiveFile>> = HashMap::new();
    let mut unassigned = HashMap::new();
    if !tts_dir.exists() {
        return Ok((by_character, unassigned));
    }
    for entry in WalkDir::new(tts_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(filename) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(cache_key) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let name = format!("tts_audio/{filename}");
        let Some(file) = archive_file(path.to_path_buf(), name.clone()) else {
            continue;
        };
        match owners_by_key.get(cache_key) {
            Some(owners) if !owners.is_empty() => {
                for owner in owners {
                    by_character
                        .entry(owner.clone())
                        .or_default()
                        .entry(name.clone())
                        .or_insert_with(|| file.clone());
                }
            }
            _ => {
                unassigned.entry(name).or_insert(file);
            }
        }
    }
    Ok((by_character, unassigned))
}

fn full_resource_files(
    storage: &Path,
    generated_images: &Path,
) -> HashMap<String, BackupArchiveFile> {
    let mut files = HashMap::new();
    for prefix in ["images", "avatars", "attachments", "sessions", "tts_audio"] {
        add_directory_files(&mut files, &storage.join(prefix), prefix, |_| true);
    }
    add_directory_files(&mut files, generated_images, "generated_images", |_| true);
    files
}

pub(crate) fn build_backup_plan(
    app: &tauri::AppHandle,
    selection: Option<BackupSelection>,
) -> Result<BackupPlan, String> {
    let selection = selection.unwrap_or_default();
    let conn = open_db(app)?;
    let storage = storage_root(app)?;
    let generated_images = app
        .path()
        .app_data_dir()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
        .join("generated_images");

    let characters = {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, avatar_path FROM characters ORDER BY updated_at DESC, name ASC",
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
    };
    let all_character_ids: HashSet<String> =
        characters.iter().map(|(id, _, _)| id.clone()).collect();
    let scope = resolve_scope(&conn, &selection, &all_character_ids)?;
    let selection_by_character: HashMap<&str, &BackupCharacterSelection> = selection
        .characters
        .iter()
        .map(|item| (item.character_id.as_str(), item))
        .collect();

    let (tts_by_character, unassigned_audio) =
        tts_files_by_character(&conn, &storage.join("tts_audio"), &all_character_ids)?;
    let mut resources_by_character: HashMap<String, CharacterResources> = HashMap::new();

    for (character_id, _, _) in &characters {
        let mut resources = CharacterResources::default();
        add_directory_files(
            &mut resources.images,
            &storage.join("avatars").join(character_id),
            &format!("avatars/{character_id}"),
            is_image,
        );
        let session_dir = storage.join("sessions").join(character_id);
        add_directory_files(
            &mut resources.images,
            &session_dir,
            &format!("sessions/{character_id}"),
            is_image,
        );
        add_directory_files(
            &mut resources.audio,
            &session_dir,
            &format!("sessions/{character_id}"),
            is_audio,
        );
        for image_id in character_image_ids(&conn, character_id) {
            add_image_id(&storage.join("images"), &image_id, &mut resources.images);
        }
        for relative in collect_selected_attachment_paths(&conn, character_id) {
            add_storage_path(
                &storage,
                &generated_images,
                &relative,
                &mut resources.images,
                &mut resources.audio,
            );
        }
        if let Some(files) = tts_by_character.get(character_id) {
            resources.audio.extend(files.clone());
        }
        resources_by_character.insert(character_id.clone(), resources);
    }

    let data_bytes = if selection.mode == BackupSelectionMode::Full {
        database_size(&conn)
    } else {
        scope
            .selected_character_ids
            .iter()
            .map(|id| estimate_character_core_bytes(&conn, id))
            .sum::<u64>()
            .saturating_add(256 * 1024)
    };

    let mut files = if selection.mode == BackupSelectionMode::Full {
        full_resource_files(&storage, &generated_images)
    } else {
        let mut selected = HashMap::new();
        for character_id in &scope.selected_character_ids {
            let Some(resources) = resources_by_character.get(character_id) else {
                continue;
            };
            let Some(item) = selection_by_character.get(character_id.as_str()) else {
                continue;
            };
            if item.include_images {
                selected.extend(resources.images.clone());
            }
            if item.include_audio {
                selected.extend(resources.audio.clone());
            }
        }
        if selection.include_unassigned_audio {
            selected.extend(unassigned_audio.clone());
        }
        selected.extend(collect_group_resources(
            &conn,
            &scope,
            &selection_by_character,
            &storage,
            &generated_images,
        )?);
        selected
    };

    let mut estimates = Vec::with_capacity(characters.len());
    for (character_id, name, avatar_path) in characters {
        let resources = resources_by_character
            .remove(&character_id)
            .unwrap_or_default();
        let core_bytes = estimate_character_core_bytes(&conn, &character_id);
        let audio_bytes = resources.audio_bytes();
        let image_bytes = resources.image_bytes();
        estimates.push(BackupCharacterEstimate {
            character_id,
            name,
            avatar_path,
            core_bytes,
            audio_bytes,
            image_bytes,
            total_bytes: core_bytes
                .saturating_add(audio_bytes)
                .saturating_add(image_bytes),
        });
    }

    let resource_bytes = files.values().map(|file| file.size_bytes).sum();
    let unassigned_audio_bytes = unassigned_audio.values().map(|file| file.size_bytes).sum();
    let total_files = files.len() as u64;
    let mut files: Vec<_> = files.drain().map(|(_, file)| file).collect();
    files.sort_by(|a, b| a.archive_name.cmp(&b.archive_name));

    Ok(BackupPlan {
        selection,
        scope,
        files,
        estimate: BackupEstimate {
            characters: estimates,
            data_bytes,
            resource_bytes,
            unassigned_audio_bytes,
            total_bytes: data_bytes.saturating_add(resource_bytes),
            total_files,
        },
    })
}

#[tauri::command]
pub async fn backup_estimate(
    app: tauri::AppHandle,
    selection: Option<BackupSelection>,
) -> Result<BackupEstimate, String> {
    tauri::async_runtime::spawn_blocking(move || {
        build_backup_plan(&app, selection).map(|plan| plan.estimate)
    })
    .await
    .map_err(|error| format!("Backup estimate task failed: {error}"))?
}

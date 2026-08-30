use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

use crate::storage_manager::db::open_db;
use crate::utils::now_millis;

const BACKUP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsCacheReference {
    pub conversation_kind: String,
    pub conversation_id: String,
    pub message_id: String,
    pub variant_id: Option<String>,
    pub character_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsCacheContext {
    pub provider_id: String,
    pub model_id: String,
    pub voice_id: String,
    pub reference: Option<TtsCacheReference>,
}

impl TtsCacheContext {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        voice_id: impl Into<String>,
        reference: Option<TtsCacheReference>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            voice_id: voice_id.into(),
            reference,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TtsCacheEntryRecord {
    cache_key: String,
    provider_id: String,
    model_id: String,
    voice_id: String,
    format: String,
    size_bytes: i64,
    created_at: i64,
    last_accessed_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TtsCacheReferenceRecord {
    cache_key: String,
    conversation_kind: String,
    conversation_id: String,
    message_id: String,
    variant_id: String,
    character_id: Option<String>,
    created_at: i64,
    last_used_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TtsCacheMetadataBackup {
    schema_version: u32,
    entries: Vec<TtsCacheEntryRecord>,
    references: Vec<TtsCacheReferenceRecord>,
}

pub(crate) fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tts_cache_entries (
          cache_key TEXT PRIMARY KEY,
          provider_id TEXT NOT NULL,
          model_id TEXT NOT NULL,
          voice_id TEXT NOT NULL,
          format TEXT NOT NULL,
          size_bytes INTEGER NOT NULL DEFAULT 0,
          created_at INTEGER NOT NULL,
          last_accessed_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tts_cache_entries_last_accessed
          ON tts_cache_entries(last_accessed_at DESC);

        -- Cache files may be shared by many messages. Conversation rows are kept as
        -- lightweight references and intentionally have no FK to chat tables so a
        -- backup can restore metadata and conversations independently.
        CREATE TABLE IF NOT EXISTS tts_cache_references (
          cache_key TEXT NOT NULL,
          conversation_kind TEXT NOT NULL CHECK(conversation_kind IN ('session', 'group_session')),
          conversation_id TEXT NOT NULL,
          message_id TEXT NOT NULL,
          variant_id TEXT NOT NULL DEFAULT '',
          character_id TEXT,
          created_at INTEGER NOT NULL,
          last_used_at INTEGER NOT NULL,
          PRIMARY KEY(cache_key, conversation_kind, conversation_id, message_id, variant_id),
          FOREIGN KEY(cache_key) REFERENCES tts_cache_entries(cache_key) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_tts_cache_references_character
          ON tts_cache_references(character_id, last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_tts_cache_references_conversation
          ON tts_cache_references(conversation_kind, conversation_id, last_used_at DESC);
        "#,
    )
    .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

fn timestamp_millis() -> Result<i64, String> {
    let value = now_millis()?;
    i64::try_from(value).map_err(|_| "Current timestamp exceeds SQLite integer range".to_string())
}

fn validate_context(context: &TtsCacheContext) -> Result<(), String> {
    if context.provider_id.trim().is_empty()
        || context.model_id.trim().is_empty()
        || context.voice_id.trim().is_empty()
    {
        return Err("TTS cache context requires provider, model, and voice IDs".to_string());
    }

    let Some(reference) = context.reference.as_ref() else {
        return Ok(());
    };
    if !matches!(
        reference.conversation_kind.as_str(),
        "session" | "group_session"
    ) {
        return Err(format!(
            "Unsupported TTS cache conversation kind: {}",
            reference.conversation_kind
        ));
    }
    if reference.conversation_id.trim().is_empty() || reference.message_id.trim().is_empty() {
        return Err("TTS cache reference requires conversation and message IDs".to_string());
    }
    Ok(())
}

pub(crate) fn record_cache_use(
    app: &AppHandle,
    cache_key: &str,
    format: &str,
    size_bytes: u64,
    context: &TtsCacheContext,
) -> Result<(), String> {
    validate_context(context)?;
    let now = timestamp_millis()?;
    let size_bytes = i64::try_from(size_bytes).unwrap_or(i64::MAX);
    let mut conn = open_db(app)?;
    let tx = conn
        .transaction()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    tx.execute(
        r#"
        INSERT INTO tts_cache_entries (
          cache_key, provider_id, model_id, voice_id, format, size_bytes, created_at, last_accessed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
        ON CONFLICT(cache_key) DO UPDATE SET
          provider_id = excluded.provider_id,
          model_id = excluded.model_id,
          voice_id = excluded.voice_id,
          format = excluded.format,
          size_bytes = CASE
            WHEN excluded.size_bytes > 0 THEN excluded.size_bytes
            ELSE tts_cache_entries.size_bytes
          END,
          last_accessed_at = excluded.last_accessed_at
        "#,
        params![
            cache_key,
            context.provider_id,
            context.model_id,
            context.voice_id,
            format,
            size_bytes,
            now,
        ],
    )
    .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    if let Some(reference) = context.reference.as_ref() {
        let variant_id = reference.variant_id.as_deref().unwrap_or("");
        tx.execute(
            r#"
            INSERT INTO tts_cache_references (
              cache_key, conversation_kind, conversation_id, message_id, variant_id,
              character_id, created_at, last_used_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(cache_key, conversation_kind, conversation_id, message_id, variant_id)
            DO UPDATE SET
              character_id = COALESCE(excluded.character_id, tts_cache_references.character_id),
              last_used_at = excluded.last_used_at
            "#,
            params![
                cache_key,
                reference.conversation_kind,
                reference.conversation_id,
                reference.message_id,
                variant_id,
                reference.character_id,
                now,
            ],
        )
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    }

    tx.commit()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

pub(crate) fn remove_entry(app: &AppHandle, cache_key: &str) -> Result<(), String> {
    let conn = open_db(app)?;
    conn.execute(
        "DELETE FROM tts_cache_entries WHERE cache_key = ?1",
        params![cache_key],
    )
    .map(|_| ())
    .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

pub(crate) fn clear_all(app: &AppHandle) -> Result<(), String> {
    let mut conn = open_db(app)?;
    let tx = conn
        .transaction()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    tx.execute("DELETE FROM tts_cache_references", [])
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    tx.execute("DELETE FROM tts_cache_entries", [])
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    tx.commit()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

pub(crate) fn export_backup(app: &AppHandle) -> Result<JsonValue, String> {
    let conn = open_db(app)?;
    let entries = {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT cache_key, provider_id, model_id, voice_id, format, size_bytes,
                       created_at, last_accessed_at
                FROM tts_cache_entries
                ORDER BY cache_key
                "#,
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(TtsCacheEntryRecord {
                    cache_key: row.get(0)?,
                    provider_id: row.get(1)?,
                    model_id: row.get(2)?,
                    voice_id: row.get(3)?,
                    format: row.get(4)?,
                    size_bytes: row.get(5)?,
                    created_at: row.get(6)?,
                    last_accessed_at: row.get(7)?,
                })
            })
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
    };

    let references = {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT cache_key, conversation_kind, conversation_id, message_id, variant_id,
                       character_id, created_at, last_used_at
                FROM tts_cache_references
                ORDER BY cache_key, conversation_kind, conversation_id, message_id, variant_id
                "#,
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(TtsCacheReferenceRecord {
                    cache_key: row.get(0)?,
                    conversation_kind: row.get(1)?,
                    conversation_id: row.get(2)?,
                    message_id: row.get(3)?,
                    variant_id: row.get(4)?,
                    character_id: row.get(5)?,
                    created_at: row.get(6)?,
                    last_used_at: row.get(7)?,
                })
            })
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
    };

    serde_json::to_value(TtsCacheMetadataBackup {
        schema_version: BACKUP_SCHEMA_VERSION,
        entries,
        references,
    })
    .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

pub(crate) fn replace_from_backup(app: &AppHandle, data: Option<&JsonValue>) -> Result<(), String> {
    let backup = match data {
        Some(value) => {
            let backup = serde_json::from_value::<TtsCacheMetadataBackup>(value.clone())
                .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
            if backup.schema_version > BACKUP_SCHEMA_VERSION {
                return Err(format!(
                    "Unsupported TTS cache metadata schema version: {}",
                    backup.schema_version
                ));
            }
            Some(backup)
        }
        None => None,
    };
    let mut conn = open_db(app)?;
    let tx = conn
        .transaction()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    tx.execute("DELETE FROM tts_cache_references", [])
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    tx.execute("DELETE FROM tts_cache_entries", [])
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    if let Some(backup) = backup {
        for entry in backup.entries {
            tx.execute(
                r#"
                INSERT INTO tts_cache_entries (
                  cache_key, provider_id, model_id, voice_id, format, size_bytes,
                  created_at, last_accessed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    entry.cache_key,
                    entry.provider_id,
                    entry.model_id,
                    entry.voice_id,
                    entry.format,
                    entry.size_bytes,
                    entry.created_at,
                    entry.last_accessed_at,
                ],
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        }
        for reference in backup.references {
            tx.execute(
                r#"
                INSERT INTO tts_cache_references (
                  cache_key, conversation_kind, conversation_id, message_id, variant_id,
                  character_id, created_at, last_used_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    reference.cache_key,
                    reference.conversation_kind,
                    reference.conversation_id,
                    reference.message_id,
                    reference.variant_id,
                    reference.character_id,
                    reference.created_at,
                    reference.last_used_at,
                ],
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        }
    }

    tx.commit()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

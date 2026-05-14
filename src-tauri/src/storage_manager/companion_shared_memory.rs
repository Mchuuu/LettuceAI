use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

use super::db::open_db;
use crate::storage_manager::memory_embeddings::SessionKind;

#[derive(Clone, Debug)]
pub struct EffectiveMemoryOwner {
    pub owner_id: String,
    pub kind: SessionKind,
    pub shared: bool,
}

#[derive(Clone, Debug)]
pub struct SharedMemoryState {
    pub memories_json: String,
    pub memory_summary: Option<String>,
    pub memory_summary_token_count: i64,
    pub memory_tool_events_json: String,
    pub memory_status: Option<String>,
    pub memory_error: Option<String>,
    pub memory_progress_step: Option<i64>,
}

impl Default for SharedMemoryState {
    fn default() -> Self {
        Self {
            memories_json: "[]".to_string(),
            memory_summary: None,
            memory_summary_token_count: 0,
            memory_tool_events_json: "[]".to_string(),
            memory_status: None,
            memory_error: None,
            memory_progress_step: None,
        }
    }
}

fn companion_shared_memory_enabled_for_character(
    conn: &Connection,
    character_id: &str,
    mode: &str,
) -> Result<bool, String> {
    if !mode.eq_ignore_ascii_case("companion") {
        return Ok(false);
    }

    let companion_json = conn
        .query_row(
            "SELECT companion FROM characters WHERE id = ?1",
            params![character_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?
        .flatten();

    Ok(companion_json
        .as_deref()
        .map(
            crate::chat_manager::companion::shared_memory_across_sessions_enabled_from_companion_json,
        )
        .unwrap_or(false))
}

pub fn resolve_effective_memory_owner(
    conn: &Connection,
    session_id: &str,
    character_id: &str,
    mode: &str,
) -> Result<EffectiveMemoryOwner, String> {
    let shared = companion_shared_memory_enabled_for_character(conn, character_id, mode)?;
    Ok(if shared {
        EffectiveMemoryOwner {
            owner_id: character_id.to_string(),
            kind: SessionKind::CompanionShared,
            shared: true,
        }
    } else {
        EffectiveMemoryOwner {
            owner_id: session_id.to_string(),
            kind: SessionKind::Session,
            shared: false,
        }
    })
}

pub fn resolve_effective_memory_owner_for_session(
    conn: &Connection,
    session_id: &str,
) -> Result<EffectiveMemoryOwner, String> {
    let (character_id, mode): (String, String) = conn
        .query_row(
            "SELECT character_id, mode FROM sessions WHERE id = ?1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    resolve_effective_memory_owner(conn, session_id, &character_id, &mode)
}

pub fn resolve_effective_memory_owner_for_session_app(
    app: &AppHandle,
    session_id: &str,
) -> Result<EffectiveMemoryOwner, String> {
    let conn = open_db(app)?;
    resolve_effective_memory_owner_for_session(&conn, session_id)
}

pub fn load_state(conn: &Connection, character_id: &str) -> Result<SharedMemoryState, String> {
    conn.query_row(
        "SELECT memories, memory_summary, memory_summary_token_count, memory_tool_events, memory_status, memory_error, memory_progress_step
         FROM companion_shared_memory_state WHERE character_id = ?1",
        params![character_id],
        |row| {
            Ok(SharedMemoryState {
                memories_json: row.get::<_, String>(0)?,
                memory_summary: row.get::<_, Option<String>>(1)?,
                memory_summary_token_count: row.get::<_, i64>(2)?,
                memory_tool_events_json: row.get::<_, String>(3)?,
                memory_status: row.get::<_, Option<String>>(4)?,
                memory_error: row.get::<_, Option<String>>(5)?,
                memory_progress_step: row.get::<_, Option<i64>>(6)?,
            })
        },
    )
    .optional()
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?
    .map(Ok)
    .unwrap_or_else(|| Ok(SharedMemoryState::default()))
}

pub fn upsert_state(
    conn: &Connection,
    character_id: &str,
    state: &SharedMemoryState,
) -> Result<(), String> {
    let now = super::db::now_ms() as i64;
    conn.execute(
        r#"
        INSERT INTO companion_shared_memory_state (
            character_id, memories, memory_summary, memory_summary_token_count,
            memory_tool_events, memory_status, memory_error, memory_progress_step,
            created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
        ON CONFLICT(character_id) DO UPDATE SET
            memories = excluded.memories,
            memory_summary = excluded.memory_summary,
            memory_summary_token_count = excluded.memory_summary_token_count,
            memory_tool_events = excluded.memory_tool_events,
            memory_status = excluded.memory_status,
            memory_error = excluded.memory_error,
            memory_progress_step = excluded.memory_progress_step,
            updated_at = excluded.updated_at
        "#,
        params![
            character_id,
            &state.memories_json,
            state.memory_summary.as_deref(),
            state.memory_summary_token_count,
            &state.memory_tool_events_json,
            state.memory_status.as_deref(),
            state.memory_error.as_deref(),
            state.memory_progress_step,
            now,
        ],
    )
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(())
}

pub fn export_all(app: &AppHandle) -> Result<Vec<JsonValue>, String> {
    let conn = open_db(app)?;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT character_id, memories, memory_summary, memory_summary_token_count,
                   memory_tool_events, memory_status, memory_error, memory_progress_step,
                   created_at, updated_at
            FROM companion_shared_memory_state
            ORDER BY character_id ASC
            "#,
        )
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "character_id": row.get::<_, String>(0)?,
                "memories": row.get::<_, String>(1)?,
                "memory_summary": row.get::<_, Option<String>>(2)?,
                "memory_summary_token_count": row.get::<_, i64>(3)?,
                "memory_tool_events": row.get::<_, String>(4)?,
                "memory_status": row.get::<_, Option<String>>(5)?,
                "memory_error": row.get::<_, Option<String>>(6)?,
                "memory_progress_step": row.get::<_, Option<i64>>(7)?,
                "created_at": row.get::<_, i64>(8)?,
                "updated_at": row.get::<_, i64>(9)?,
            }))
        })
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
}

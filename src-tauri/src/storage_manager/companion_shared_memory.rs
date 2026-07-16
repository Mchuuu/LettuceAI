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
    let row: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT companion, mode FROM characters WHERE id = ?1",
            params![character_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;

    let (companion_json, character_mode) = match row {
        Some(value) => value,
        None => return Ok(false),
    };

    let is_companion = mode.eq_ignore_ascii_case("companion")
        || character_mode
            .as_deref()
            .map(|m| m.eq_ignore_ascii_case("companion"))
            .unwrap_or(false);
    if !is_companion {
        return Ok(false);
    }

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

/// Seed a newly enabled shared-memory owner from the character's richest recent
/// session. On an off-to-on transition, a larger local snapshot repairs a stale
/// or previously truncated shared owner without discarding a richer shared one.
pub fn initialize_from_latest_session(
    conn: &mut Connection,
    character_id: &str,
    compare_existing_on_transition: bool,
) -> Result<Option<String>, String> {
    let existing_shared_count = super::memory_embeddings::count_for_session(
        conn,
        character_id,
        SessionKind::CompanionShared,
    )?;
    let shared_state_exists = conn
        .query_row(
            "SELECT 1 FROM companion_shared_memory_state WHERE character_id = ?1 LIMIT 1",
            params![character_id],
            |_| Ok(()),
        )
        .optional()
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?
        .is_some();

    if shared_state_exists && !compare_existing_on_transition {
        return Ok(None);
    }

    let source = conn
        .query_row(
            r#"
            SELECT id, memories, memory_embeddings, memory_summary,
                   memory_summary_token_count, memory_tool_events,
                   memory_status, memory_error, memory_progress_step
            FROM sessions
            WHERE character_id = ?1
            ORDER BY
                (
                    SELECT COUNT(*) FROM memory_embeddings normalized
                    WHERE normalized.session_id = sessions.id
                      AND normalized.session_kind = 'session'
                ) DESC,
                CASE WHEN json_valid(memory_embeddings)
                     THEN json_array_length(memory_embeddings)
                     ELSE 0 END DESC,
                CASE WHEN memories <> '[]'
                       OR memory_embeddings <> '[]'
                       OR EXISTS (
                           SELECT 1 FROM memory_embeddings normalized
                           WHERE normalized.session_id = sessions.id
                             AND normalized.session_kind = 'session'
                       )
                     THEN 0 ELSE 1 END,
                updated_at DESC,
                created_at DESC
            LIMIT 1
            "#,
            params![character_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            },
        )
        .optional()
        .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;

    let Some((
        session_id,
        memories_json,
        legacy_embeddings_json,
        memory_summary,
        memory_summary_token_count,
        memory_tool_events_json,
        memory_status,
        memory_error,
        memory_progress_step,
    )) = source
    else {
        return Ok(None);
    };

    let mut embeddings =
        super::memory_embeddings::load_for_session(conn, &session_id, SessionKind::Session)?;
    if embeddings.is_empty() {
        embeddings = super::memory_embeddings::parse_legacy_json(&legacy_embeddings_json);
    }

    if shared_state_exists && existing_shared_count >= embeddings.len() as i64 {
        return Ok(None);
    }

    super::memory_embeddings::replace_all(
        conn,
        character_id,
        SessionKind::CompanionShared,
        &embeddings,
    )?;
    upsert_state(
        conn,
        character_id,
        &SharedMemoryState {
            memories_json,
            memory_summary,
            memory_summary_token_count: memory_summary_token_count.max(0),
            memory_tool_events_json,
            memory_status,
            memory_error,
            memory_progress_step,
        },
    )?;

    Ok(Some(session_id))
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

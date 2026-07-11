use rusqlite::OptionalExtension;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;
use uuid::Uuid;

use super::db::{now_ms, open_db};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMemoryJob {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub window_size: u32,
    pub next_window_start: usize,
    pub window_index: usize,
    pub total_windows: usize,
    pub processed_messages: usize,
    pub total_messages: usize,
    pub current_window_start: Option<usize>,
    pub current_window_end: Option<usize>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn active_jobs() -> &'static Mutex<HashSet<String>> {
    static ACTIVE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn try_claim(session_id: &str) -> Result<bool, String> {
    let mut jobs = active_jobs()
        .lock()
        .map_err(|_| "Imported memory job lock is poisoned".to_string())?;
    Ok(jobs.insert(session_id.to_string()))
}

pub fn release(session_id: &str) {
    if let Ok(mut jobs) = active_jobs().lock() {
        jobs.remove(session_id);
    }
}

pub fn get(app: &AppHandle, session_id: &str) -> Result<Option<ImportedMemoryJob>, String> {
    let conn = open_db(app)?;
    conn.query_row(
        "SELECT id, session_id, status, window_size, next_window_start, window_index,
                total_windows, processed_messages, total_messages, current_window_start,
                current_window_end, last_error, created_at, updated_at
         FROM imported_memory_jobs WHERE session_id = ?1",
        [session_id],
        |row| {
            Ok(ImportedMemoryJob {
                id: row.get(0)?,
                session_id: row.get(1)?,
                status: row.get(2)?,
                window_size: row.get::<_, i64>(3)? as u32,
                next_window_start: row.get::<_, i64>(4)? as usize,
                window_index: row.get::<_, i64>(5)? as usize,
                total_windows: row.get::<_, i64>(6)? as usize,
                processed_messages: row.get::<_, i64>(7)? as usize,
                total_messages: row.get::<_, i64>(8)? as usize,
                current_window_start: row.get::<_, Option<i64>>(9)?.map(|v| v as usize),
                current_window_end: row.get::<_, Option<i64>>(10)?.map(|v| v as usize),
                last_error: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        },
    )
    .optional()
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))
}

pub fn start_or_resume(
    app: &AppHandle,
    session_id: &str,
    window_size: u32,
    total_messages: usize,
    total_windows: usize,
) -> Result<ImportedMemoryJob, String> {
    let now = now_ms() as i64;
    let conn = open_db(app)?;
    let existing = get(app, session_id)?;
    let id = existing
        .as_ref()
        .map(|job| job.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let persisted_window_size = existing
        .as_ref()
        .map(|job| job.window_size)
        .filter(|value| *value > 0)
        .unwrap_or(window_size);
    let persisted_total_messages = existing
        .as_ref()
        .map(|job| job.total_messages)
        .filter(|value| *value > 0)
        .unwrap_or(total_messages);
    let next_start = existing
        .as_ref()
        .map(|job| job.next_window_start)
        .unwrap_or(0);
    let persisted_total_windows = existing
        .as_ref()
        .map(|job| job.total_windows)
        .filter(|value| *value > 0)
        .unwrap_or(total_windows);
    let window_index = existing
        .as_ref()
        .map(|job| job.window_index.max(1))
        .unwrap_or(1);
    let processed = next_start.min(persisted_total_messages);

    conn.execute(
        "INSERT INTO imported_memory_jobs
         (id, session_id, status, window_size, next_window_start, window_index,
          total_windows, processed_messages, total_messages, current_window_start,
          current_window_end, last_error, created_at, updated_at)
         VALUES (?1, ?2, 'running', ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL,
                 COALESCE((SELECT created_at FROM imported_memory_jobs WHERE session_id = ?2), ?9), ?9)
         ON CONFLICT(session_id) DO UPDATE SET
           status = 'running', window_size = excluded.window_size,
           total_windows = excluded.total_windows, total_messages = excluded.total_messages,
           processed_messages = excluded.processed_messages, last_error = NULL,
           updated_at = excluded.updated_at",
        rusqlite::params![
            id,
            session_id,
            persisted_window_size as i64,
            next_start as i64,
            window_index as i64,
            persisted_total_windows as i64,
            processed as i64,
            persisted_total_messages as i64,
            now,
        ],
    )
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    get(app, session_id)?.ok_or_else(|| "Failed to create imported memory job".to_string())
}

pub fn mark_window_started(
    app: &AppHandle,
    session_id: &str,
    start: usize,
    end: usize,
    index: usize,
) -> Result<(), String> {
    let conn = open_db(app)?;
    conn.execute(
        "UPDATE imported_memory_jobs SET status = 'running', window_index = ?1,
         current_window_start = ?2, current_window_end = ?3, updated_at = ?4
         WHERE session_id = ?5",
        rusqlite::params![
            index as i64,
            start as i64,
            end as i64,
            now_ms() as i64,
            session_id
        ],
    )
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(())
}

pub fn mark_window_completed(
    app: &AppHandle,
    session_id: &str,
    next_start: usize,
    next_index: usize,
) -> Result<(), String> {
    let conn = open_db(app)?;
    conn.execute(
        "UPDATE imported_memory_jobs SET status = 'running', next_window_start = ?1,
         window_index = ?2, processed_messages = ?1, current_window_start = NULL,
         current_window_end = NULL, last_error = NULL, updated_at = ?3
         WHERE session_id = ?4",
        rusqlite::params![
            next_start as i64,
            next_index as i64,
            now_ms() as i64,
            session_id
        ],
    )
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(())
}

pub fn mark_status(
    app: &AppHandle,
    session_id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let conn = open_db(app)?;
    conn.execute(
        "UPDATE imported_memory_jobs SET status = ?1, last_error = ?2, updated_at = ?3 WHERE session_id = ?4",
        rusqlite::params![status, error, now_ms() as i64, session_id],
    )
    .map_err(|e| crate::utils::err_to_string(module_path!(), line!(), e))?;
    Ok(())
}

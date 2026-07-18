use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::chat_manager::types::MemoryEmbedding;

use super::companion_shared_memory::{
    load_state, resolve_effective_memory_owner_for_session, upsert_state, EffectiveMemoryOwner,
};
use super::memory_embeddings::{load_for_session, parse_legacy_json, replace_all_in_connection};

#[derive(Debug, Default)]
pub struct MemoryRewindOutcome {
    pub reverted_events: usize,
    pub removed_memories: usize,
    pub restored_memories: usize,
    pub unrestorable_actions: usize,
    pub shared: bool,
}

#[derive(Debug)]
pub struct MemoryHistoryRepairOutcome {
    pub rewind: MemoryRewindOutcome,
    pub missing_message_ids: usize,
    pub anchor_window_end: usize,
}

struct RewindState {
    memories: Vec<MemoryEmbedding>,
    events: Vec<Value>,
    summary: Option<String>,
    summary_token_count: i64,
}

fn event_is_active(event: &Value) -> bool {
    event.get("revertedAt").map(Value::is_null).unwrap_or(true)
        && event.get("status").and_then(Value::as_str) != Some("error")
}

fn event_belongs_to_session(event: &Value, session_id: &str, shared: bool) -> bool {
    match event.get("sourceSessionId").and_then(Value::as_str) {
        Some(source_session_id) => source_session_id == session_id,
        None => !shared,
    }
}

fn event_belongs_to_rewind(event: &Value, session_id: &str, deleted_ids: &HashSet<&str>) -> bool {
    if let Some(source_session_id) = event.get("sourceSessionId").and_then(Value::as_str) {
        if source_session_id != session_id {
            return false;
        }
    }

    event
        .get("windowMessageIds")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .any(|id| deleted_ids.contains(id))
        })
        .unwrap_or(false)
}

fn action_memory_id(action: &Value) -> Option<&str> {
    action
        .get("arguments")
        .and_then(|args| args.get("id"))
        .and_then(Value::as_str)
        .or_else(|| action.get("memoryId").and_then(Value::as_str))
        .or_else(|| action.get("deletedMemoryId").and_then(Value::as_str))
}

fn repair_removed_memory_links(memories: &mut [MemoryEmbedding], removed_id: &str) {
    for memory in memories {
        if memory.superseded_by.as_deref() == Some(removed_id) {
            memory.superseded_by = None;
            memory.superseded_at = None;
        }
        memory.supersedes.retain(|id| id != removed_id);
    }
}

fn remove_memory(
    memories: &mut Vec<MemoryEmbedding>,
    memory_id: &str,
    outcome: &mut MemoryRewindOutcome,
) {
    if let Some(index) = memories.iter().position(|memory| memory.id == memory_id) {
        memories.remove(index);
        repair_removed_memory_links(memories, memory_id);
        outcome.removed_memories += 1;
    }
}

fn restore_memory(
    memories: &mut Vec<MemoryEmbedding>,
    action: &Value,
    outcome: &mut MemoryRewindOutcome,
) {
    let Some(snapshot) = action.get("memorySnapshot").cloned() else {
        outcome.unrestorable_actions += 1;
        return;
    };
    let Ok(memory) = serde_json::from_value::<MemoryEmbedding>(snapshot) else {
        outcome.unrestorable_actions += 1;
        return;
    };

    if let Some(index) = memories.iter().position(|item| item.id == memory.id) {
        memories[index] = memory;
    } else {
        memories.push(memory);
    }
    outcome.restored_memories += 1;
}

fn revert_event_actions(
    memories: &mut Vec<MemoryEmbedding>,
    event: &Value,
    outcome: &mut MemoryRewindOutcome,
) {
    let Some(actions) = event.get("actions").and_then(Value::as_array) else {
        return;
    };

    for action in actions.iter().rev() {
        if action.get("skipped").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        match action.get("name").and_then(Value::as_str) {
            Some("create_memory") => {
                if let Some(memory_id) = action_memory_id(action) {
                    remove_memory(memories, memory_id, outcome);
                } else {
                    outcome.unrestorable_actions += 1;
                }
            }
            Some("delete_memory") => restore_memory(memories, action, outcome),
            Some("pin_memory") | Some("unpin_memory") => {
                let Some(memory_id) = action_memory_id(action) else {
                    outcome.unrestorable_actions += 1;
                    continue;
                };
                if let Some(memory) = memories.iter_mut().find(|memory| memory.id == memory_id) {
                    memory.is_pinned = action
                        .get("previousIsPinned")
                        .and_then(Value::as_bool)
                        .unwrap_or_else(|| {
                            action.get("name").and_then(Value::as_str) == Some("unpin_memory")
                        });
                    if let Some(score) = action
                        .get("previousImportanceScore")
                        .and_then(Value::as_f64)
                    {
                        memory.importance_score = score as f32;
                    }
                }
            }
            _ => {}
        }
    }
}

fn apply_rewind(
    state: &mut RewindState,
    session_id: &str,
    retained_message_id: Option<&str>,
    retained_conversation_count: usize,
    deleted_message_ids: &[String],
    now: u64,
    reason: &str,
) -> MemoryRewindOutcome {
    let deleted_ids: HashSet<&str> = deleted_message_ids.iter().map(String::as_str).collect();
    let mut outcome = MemoryRewindOutcome::default();
    let affected_indices: Vec<usize> = state
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event_is_active(event) && event_belongs_to_rewind(event, session_id, &deleted_ids)
        })
        .map(|(index, _)| index)
        .collect();

    for index in affected_indices.iter().rev().copied() {
        let event = state.events[index].clone();
        revert_event_actions(&mut state.memories, &event, &mut outcome);
        if let Some(object) = state.events[index].as_object_mut() {
            object.insert("revertedAt".into(), json!(now));
            object.insert("revertReason".into(), json!(reason));
            if let Some(retained_message_id) = retained_message_id {
                object.insert("revertedByMessageId".into(), json!(retained_message_id));
            }
        }
        outcome.reverted_events += 1;
    }

    let source_linked_ids: Vec<String> = state
        .memories
        .iter()
        .filter(|memory| {
            memory
                .source_message_id
                .as_deref()
                .map(|id| deleted_ids.contains(id))
                .unwrap_or(false)
        })
        .map(|memory| memory.id.clone())
        .collect();
    let removed_source_linked_memory = !source_linked_ids.is_empty();
    for memory_id in source_linked_ids {
        remove_memory(&mut state.memories, &memory_id, &mut outcome);
    }

    if let Some(first_affected) = affected_indices.first().copied() {
        let checkpoint = state.events[..first_affected]
            .iter()
            .rev()
            .find(|event| event_is_active(event))
            .cloned();
        state.summary = checkpoint
            .as_ref()
            .and_then(|event| event.get("summary"))
            .and_then(Value::as_str)
            .filter(|summary| !summary.is_empty())
            .map(str::to_string);
        state.summary_token_count = checkpoint
            .as_ref()
            .and_then(|event| event.get("summaryTokenCount"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
    } else if removed_source_linked_memory {
        state.summary = None;
        state.summary_token_count = 0;
    }

    let anchor_message_ids = retained_message_id
        .map(|id| vec![id.to_string()])
        .unwrap_or_default();
    state.events.push(json!({
        "id": Uuid::new_v4().to_string(),
        "type": "rewind_anchor",
        "sourceSessionId": session_id,
        "windowStart": retained_conversation_count,
        "windowEnd": retained_conversation_count,
        "windowMessageIds": anchor_message_ids,
        "summary": state.summary.clone().unwrap_or_default(),
        "summaryTokenCount": state.summary_token_count,
        "actions": [],
        "reason": reason,
        "createdAt": now,
    }));
    if state.events.len() > 50 {
        let excess = state.events.len() - 50;
        state.events.drain(0..excess);
    }

    outcome
}

fn load_rewind_state(
    conn: &Connection,
    session_id: &str,
) -> Result<(EffectiveMemoryOwner, RewindState), String> {
    let owner = resolve_effective_memory_owner_for_session(conn, session_id)?;
    let state = if owner.shared {
        let shared = load_state(conn, &owner.owner_id)?;
        RewindState {
            memories: load_for_session(conn, &owner.owner_id, owner.kind)?,
            events: serde_json::from_str(&shared.memory_tool_events_json).map_err(|error| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to parse shared memory tool events: {error}"),
                )
            })?,
            summary: shared.memory_summary,
            summary_token_count: shared.memory_summary_token_count,
        }
    } else {
        let (legacy_embeddings, summary, token_count, events): (
            String,
            Option<String>,
            i64,
            String,
        ) = conn
            .query_row(
                "SELECT memory_embeddings, memory_summary, memory_summary_token_count, memory_tool_events FROM sessions WHERE id = ?1",
                params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?
            .ok_or_else(|| "Session not found".to_string())?;
        let mut memories = load_for_session(conn, &owner.owner_id, owner.kind)?;
        if memories.is_empty() {
            memories = parse_legacy_json(&legacy_embeddings);
        }
        RewindState {
            memories,
            events: serde_json::from_str(&events).map_err(|error| {
                crate::utils::err_msg(
                    module_path!(),
                    line!(),
                    format!("Failed to parse session memory tool events: {error}"),
                )
            })?,
            summary,
            summary_token_count: token_count,
        }
    };
    Ok((owner, state))
}

fn persist_rewind_state(
    conn: &Connection,
    session_id: &str,
    owner: &EffectiveMemoryOwner,
    state: &RewindState,
) -> Result<(), String> {
    replace_all_in_connection(conn, &owner.owner_id, owner.kind, &state.memories)?;
    let events_json = serde_json::to_string(&state.events)
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    if owner.shared {
        let mut shared = load_state(conn, &owner.owner_id)?;
        shared.memory_summary = state.summary.clone();
        shared.memory_summary_token_count = state.summary_token_count;
        shared.memory_tool_events_json = events_json;
        shared.memory_status = Some("idle".to_string());
        shared.memory_error = None;
        shared.memory_progress_step = None;
        upsert_state(conn, &owner.owner_id, &shared)
    } else {
        let embeddings_json = serde_json::to_string(&state.memories)
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
        conn.execute(
            "UPDATE sessions SET memory_embeddings = ?1, memory_summary = ?2, memory_summary_token_count = ?3, memory_tool_events = ?4, memory_status = 'idle', memory_error = NULL, memory_progress_step = NULL WHERE id = ?5",
            params![embeddings_json, state.summary, state.summary_token_count, events_json, session_id],
        )
        .map(|_| ())
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
    }
}

fn conversation_message_ids(conn: &Connection, session_id: &str) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id FROM messages
             WHERE session_id = ?1 AND (role = 'user' OR role = 'assistant')
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    let rows = statement
        .query_map(params![session_id], |row| row.get::<_, String>(0))
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

fn missing_event_message_ids(
    events: &[Value],
    session_id: &str,
    shared: bool,
    live_message_ids: &HashSet<&str>,
) -> HashSet<String> {
    events
        .iter()
        .filter(|event| {
            event_is_active(event) && event_belongs_to_session(event, session_id, shared)
        })
        .filter_map(|event| event.get("windowMessageIds").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .filter(|id| !live_message_ids.contains(*id))
        .map(str::to_string)
        .collect()
}

pub fn repair_orphaned_history(
    conn: &Connection,
    session_id: &str,
    recent_window_size: usize,
    now: u64,
) -> Result<MemoryHistoryRepairOutcome, String> {
    let (owner, mut state) = load_rewind_state(conn, session_id)?;
    let live_message_ids = conversation_message_ids(conn, session_id)?;
    let live_set: HashSet<&str> = live_message_ids.iter().map(String::as_str).collect();
    let missing_message_ids =
        missing_event_message_ids(&state.events, session_id, owner.shared, &live_set);

    let anchor_window_end = live_message_ids.len().saturating_sub(recent_window_size);
    let retained_message_id = anchor_window_end
        .checked_sub(1)
        .and_then(|index| live_message_ids.get(index))
        .map(String::as_str);
    let deleted_message_ids: Vec<String> = missing_message_ids.iter().cloned().collect();
    let mut rewind = apply_rewind(
        &mut state,
        session_id,
        retained_message_id,
        anchor_window_end,
        &deleted_message_ids,
        now,
        "orphaned_history_repair",
    );
    rewind.shared = owner.shared;
    persist_rewind_state(conn, session_id, &owner, &state)?;

    Ok(MemoryHistoryRepairOutcome {
        rewind,
        missing_message_ids: missing_message_ids.len(),
        anchor_window_end,
    })
}

pub fn rewind_for_deleted_messages(
    conn: &Connection,
    session_id: &str,
    retained_message_id: &str,
    retained_conversation_count: usize,
    deleted_message_ids: &[String],
    now: u64,
) -> Result<MemoryRewindOutcome, String> {
    let (owner, mut state) = load_rewind_state(conn, session_id)?;

    let mut outcome = apply_rewind(
        &mut state,
        session_id,
        Some(retained_message_id),
        retained_conversation_count,
        deleted_message_ids,
        now,
        "chat_rewind",
    );
    outcome.shared = owner.shared;

    persist_rewind_state(conn, session_id, &owner, &state)?;

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(id: &str, source_message_id: Option<&str>) -> MemoryEmbedding {
        serde_json::from_value(json!({
            "id": id,
            "text": format!("memory-{id}"),
            "embedding": [],
            "sourceMessageId": source_message_id,
        }))
        .unwrap()
    }

    #[test]
    fn rewinds_affected_actions_and_adds_cursor_anchor() {
        let mut old = memory("old", Some("kept"));
        old.superseded_by = Some("new".to_string());
        old.superseded_at = Some(20);
        let mut new = memory("new", Some("deleted"));
        new.supersedes = vec!["old".to_string()];
        let mut state = RewindState {
            memories: vec![old, new],
            events: vec![
                json!({"id":"e1","windowMessageIds":["kept"],"summary":"before","actions":[]}),
                json!({"id":"e2","windowMessageIds":["deleted"],"summary":"after","actions":[{"name":"create_memory","memoryId":"new"}]}),
            ],
            summary: Some("after".to_string()),
            summary_token_count: 42,
        };

        let outcome = apply_rewind(
            &mut state,
            "session",
            Some("kept"),
            1,
            &["deleted".to_string()],
            100,
            "chat_rewind",
        );

        assert_eq!(outcome.reverted_events, 1);
        assert_eq!(state.memories.len(), 1);
        assert_eq!(state.memories[0].superseded_by, None);
        assert_eq!(state.summary.as_deref(), Some("before"));
        assert_eq!(state.summary_token_count, 0);
        assert_eq!(state.events.last().unwrap()["type"], "rewind_anchor");
    }

    #[test]
    fn restores_deleted_memory_snapshot() {
        let snapshot = serde_json::to_value(memory("restored", Some("kept"))).unwrap();
        let mut state = RewindState {
            memories: vec![],
            events: vec![json!({
                "windowMessageIds":["deleted"],
                "actions":[{"name":"delete_memory","deletedMemoryId":"restored","memorySnapshot":snapshot}]
            })],
            summary: Some("after".to_string()),
            summary_token_count: 10,
        };

        let outcome = apply_rewind(
            &mut state,
            "session",
            Some("kept"),
            1,
            &["deleted".to_string()],
            100,
            "chat_rewind",
        );

        assert_eq!(outcome.restored_memories, 1);
        assert_eq!(state.memories[0].id, "restored");
        assert_eq!(state.summary, None);
    }

    #[test]
    fn shared_event_from_another_session_is_not_reverted() {
        let mut state = RewindState {
            memories: vec![memory("other", None)],
            events: vec![json!({
                "sourceSessionId":"other-session",
                "windowMessageIds":["deleted"],
                "actions":[{"name":"create_memory","memoryId":"other"}]
            })],
            summary: None,
            summary_token_count: 0,
        };

        let outcome = apply_rewind(
            &mut state,
            "session",
            Some("kept"),
            1,
            &["deleted".to_string()],
            100,
            "chat_rewind",
        );

        assert_eq!(outcome.reverted_events, 0);
        assert_eq!(state.memories.len(), 1);
    }

    #[test]
    fn shared_repair_only_collects_current_session_missing_ids() {
        let events = vec![
            json!({
                "sourceSessionId": "current",
                "windowMessageIds": ["kept", "deleted-current"]
            }),
            json!({
                "sourceSessionId": "other",
                "windowMessageIds": ["deleted-other"]
            }),
            json!({ "windowMessageIds": ["legacy-unscoped"] }),
        ];
        let live = HashSet::from(["kept"]);

        let missing = missing_event_message_ids(&events, "current", true, &live);

        assert_eq!(missing, HashSet::from(["deleted-current".to_string()]));
    }

    #[test]
    fn repair_anchor_keeps_recent_window_pending() {
        let mut state = RewindState {
            memories: vec![],
            events: vec![json!({
                "sourceSessionId": "session",
                "windowMessageIds": ["deleted"],
                "actions": []
            })],
            summary: None,
            summary_token_count: 0,
        };

        apply_rewind(
            &mut state,
            "session",
            Some("anchor"),
            20,
            &["deleted".to_string()],
            100,
            "orphaned_history_repair",
        );

        let anchor = state.events.last().unwrap();
        assert_eq!(anchor["windowEnd"], 20);
        assert_eq!(anchor["windowMessageIds"], json!(["anchor"]));
        assert_eq!(anchor["reason"], "orphaned_history_repair");
    }
}

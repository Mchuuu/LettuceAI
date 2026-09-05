use rusqlite::params;
use tauri::AppHandle;

use super::cache_metadata::TtsCacheReference;
use crate::storage_manager::db::open_db;

pub(crate) fn record_tts_characters(
    app: &AppHandle,
    reference: &TtsCacheReference,
    characters: u64,
) -> Result<(), String> {
    let value = i64::try_from(characters).unwrap_or(i64::MAX);
    let mut conn = open_db(app)?;
    let tx = conn
        .transaction()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;

    match reference.conversation_kind.as_str() {
        "session" => {
            let variant_id = reference
                .variant_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty());
            tx.execute(
                r#"UPDATE messages
                   SET tts_characters = ?1
                   WHERE id = ?2 AND session_id = ?3
                     AND (?4 IS NULL OR COALESCE(
                       selected_variant_id,
                       (SELECT id FROM message_variants
                        WHERE message_id = messages.id
                        ORDER BY created_at DESC, id DESC LIMIT 1)
                     ) = ?4)"#,
                params![
                    value,
                    reference.message_id,
                    reference.conversation_id,
                    variant_id,
                ],
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
            if let Some(variant_id) = variant_id {
                tx.execute(
                    "UPDATE message_variants SET tts_characters = ?1 WHERE id = ?2 AND message_id = ?3",
                    params![value, variant_id, reference.message_id],
                )
                .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
            }
        }
        "group_session" => {
            let variant_id = reference
                .variant_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty());
            tx.execute(
                r#"UPDATE group_messages
                   SET tts_characters = ?1
                   WHERE id = ?2 AND session_id = ?3
                     AND (?4 IS NULL OR COALESCE(
                       selected_variant_id,
                       (SELECT id FROM group_message_variants
                        WHERE message_id = group_messages.id
                        ORDER BY created_at DESC, id DESC LIMIT 1)
                     ) = ?4)"#,
                params![
                    value,
                    reference.message_id,
                    reference.conversation_id,
                    variant_id,
                ],
            )
            .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
            if let Some(variant_id) = variant_id {
                tx.execute(
                    "UPDATE group_message_variants SET tts_characters = ?1 WHERE id = ?2 AND message_id = ?3",
                    params![value, variant_id, reference.message_id],
                )
                .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))?;
            }
        }
        other => return Err(format!("Unsupported TTS usage conversation kind: {other}")),
    }

    tx.commit()
        .map_err(|error| crate::utils::err_to_string(module_path!(), line!(), error))
}

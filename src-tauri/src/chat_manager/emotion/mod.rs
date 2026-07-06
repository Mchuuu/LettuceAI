mod llm;
mod local_classifier;
mod mapper;
pub(crate) mod types;

use tauri::AppHandle;

#[cfg(test)]
pub(crate) use mapper::map_extraction_to_companion;
pub(crate) use types::{EmotionContextMessage, MappedEmotionSignals, MappedRelationshipDelta};

pub(crate) async fn extract_mapped_signals(
    app: &AppHandle,
    message: &str,
    
    context_messages: &[EmotionContextMessage],
) -> Result<MappedEmotionSignals, String> {
    if let Some(extraction) = llm::extract(app, message, context_messages).await? {
        return Ok(mapper::map_extraction_to_companion(&extraction));
    }

    if let Some(extraction) = local_classifier::extract(app, message).await? {
        return Ok(mapper::map_extraction_to_companion(&extraction));
    }

    Ok(mapper::neutral_signals(0.2))
}

pub(crate) fn neutral_signals(confidence: f64) -> MappedEmotionSignals {
    mapper::neutral_signals(confidence)
}

#[allow(dead_code)]
pub(crate) async fn extract_assistant_mapped_signals_placeholder(
    app: &AppHandle,
    message: &str,
    context_messages: &[EmotionContextMessage],
) -> Result<MappedEmotionSignals, String> {
    // Placeholder for future assistant-message emotion extraction. That path
    // should apply the mapped delta with a lower weight before updating soul.
    extract_mapped_signals(app, message, context_messages).await
}

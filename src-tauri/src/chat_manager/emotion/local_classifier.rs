use super::types::{EmotionExtraction, ExtractedAffect, ExtractedEmotionLabel};
use tauri::AppHandle;

pub(crate) async fn extract(
    app: &AppHandle,
    message: &str,
) -> Result<Option<EmotionExtraction>, String> {
    let Some(classification) = crate::embedding::emotion::classify_text(app, message).await? else {
        return Ok(None);
    };

    Ok(Some(EmotionExtraction {
        provider: "local_classifier".to_string(),
        labels: classification
            .labels
            .into_iter()
            .map(|item| ExtractedEmotionLabel {
                label: item.label,
                score: item.score as f64,
            })
            .collect(),
        affect: ExtractedAffect::default(),
        confidence: classification.confidence,
        evidence: Vec::new(),
    }))
}

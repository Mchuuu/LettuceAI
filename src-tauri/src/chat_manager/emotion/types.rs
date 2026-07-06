use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct EmotionContextMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmotionExtraction {
    #[serde(default)]
    pub(crate) provider: String,
    #[serde(default)]
    pub(crate) labels: Vec<ExtractedEmotionLabel>,
    #[serde(default)]
    pub(crate) affect: ExtractedAffect,
    #[serde(default)]
    pub(crate) confidence: f64,
    #[serde(default)]
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractedEmotionLabel {
    pub(crate) label: String,
    pub(crate) score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtractedAffect {
    #[serde(default)]
    pub(crate) warmth: f64,
    #[serde(default)]
    pub(crate) trust: f64,
    #[serde(default)]
    pub(crate) calm: f64,
    #[serde(default)]
    pub(crate) vulnerability: f64,
    #[serde(default)]
    pub(crate) longing: f64,
    #[serde(default)]
    pub(crate) hurt: f64,
    #[serde(default)]
    pub(crate) tension: f64,
    #[serde(default)]
    pub(crate) irritation: f64,
    #[serde(default)]
    pub(crate) affection_intensity: f64,
    #[serde(default)]
    pub(crate) reassurance_need: f64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MappedEmotionSignals {
    pub(crate) signals: Vec<String>,
    pub(crate) delta: crate::chat_manager::companion::EmotionVector,
    pub(crate) relationship_delta: MappedRelationshipDelta,
    pub(crate) confidence: f64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MappedRelationshipDelta {
    pub(crate) closeness: f64,
    pub(crate) trust: f64,
    pub(crate) affection: f64,
    pub(crate) tension: f64,
    pub(crate) stability: f64,
}

use super::types::{
    EmotionExtraction, ExtractedEmotionLabel, MappedEmotionSignals, MappedRelationshipDelta,
};
use crate::chat_manager::companion::EmotionVector;

const DIRECT_AFFECT_WEIGHT: f64 = 0.08;

pub(crate) fn neutral_signals(confidence: f64) -> MappedEmotionSignals {
    MappedEmotionSignals {
        relationship_delta: MappedRelationshipDelta {
            stability: 0.01,
            ..MappedRelationshipDelta::default()
        },
        confidence,
        ..MappedEmotionSignals::default()
    }
}

pub(crate) fn map_extraction_to_companion(extraction: &EmotionExtraction) -> MappedEmotionSignals {
    let mut mapped = MappedEmotionSignals::default();
    let mut applied_score = 0.0_f64;

    let mut labels = extraction.labels.clone();
    labels.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for item in labels.iter().take(8) {
        if item.score < label_threshold(item.label.as_str()) {
            continue;
        }
        applied_score = applied_score.max(item.score);
        apply_emotion_label(item, &mut mapped);
    }

    apply_direct_affect(extraction, &mut mapped);

    if mapped.signals.is_empty() {
        mapped.relationship_delta.stability += 0.01;
    }

    mapped.confidence = if mapped.signals.is_empty() {
        0.25
    } else {
        clamp01((extraction.confidence * 0.75) + (applied_score * 0.25))
    };
    mapped.delta = clamp_signed_vector(mapped.delta);
    mapped
}

fn apply_direct_affect(extraction: &EmotionExtraction, mapped: &mut MappedEmotionSignals) {
    let confidence = clamp01(extraction.confidence);
    if confidence <= 0.0 {
        return;
    }

    let scale = DIRECT_AFFECT_WEIGHT * confidence;
    let affect = &extraction.affect;
    mapped.delta.warmth += signed_axis(affect.warmth) * scale;
    mapped.delta.trust += signed_axis(affect.trust) * scale;
    mapped.delta.calm += signed_axis(affect.calm) * scale;
    mapped.delta.vulnerability += signed_axis(affect.vulnerability) * scale;
    mapped.delta.longing += signed_axis(affect.longing) * scale;
    mapped.delta.hurt += signed_axis(affect.hurt) * scale;
    mapped.delta.tension += signed_axis(affect.tension) * scale;
    mapped.delta.irritation += signed_axis(affect.irritation) * scale;
    mapped.delta.affection_intensity += signed_axis(affect.affection_intensity) * scale;
    mapped.delta.reassurance_need += signed_axis(affect.reassurance_need) * scale;
}

fn signed_axis(value: f64) -> f64 {
    clamp01(value) * 2.0 - 1.0
}

fn apply_emotion_label(item: &ExtractedEmotionLabel, mapped: &mut MappedEmotionSignals) {
    let score = clamp01(item.score);
    let label = normalize_label(item.label.as_str());

    match label.as_str() {
        "love" => {
            push_signal(&mut mapped.signals, "emotion:love");
            mapped.delta.warmth += 0.10 * score;
            mapped.delta.affection_intensity += 0.15 * score;
            mapped.delta.longing += 0.06 * score;
            mapped.delta.trust += 0.04 * score;
            mapped.relationship_delta.closeness += 0.035 * score;
            mapped.relationship_delta.affection += 0.055 * score;
        }
        "caring" => {
            push_signal(&mut mapped.signals, "emotion:caring");
            mapped.delta.warmth += 0.11 * score;
            mapped.delta.trust += 0.05 * score;
            mapped.delta.calm += 0.04 * score;
            mapped.relationship_delta.closeness += 0.025 * score;
            mapped.relationship_delta.trust += 0.025 * score;
        }
        "gratitude" | "admiration" | "approval" => {
            push_signal(&mut mapped.signals, "emotion:appreciation");
            mapped.delta.warmth += 0.08 * score;
            mapped.delta.trust += 0.07 * score;
            mapped.delta.calm += 0.035 * score;
            mapped.relationship_delta.trust += 0.03 * score;
            mapped.relationship_delta.stability += 0.025 * score;
        }
        "joy" | "amusement" | "excitement" | "optimism" => {
            push_signal(&mut mapped.signals, "emotion:positive");
            mapped.delta.warmth += 0.07 * score;
            mapped.delta.calm += 0.035 * score;
            mapped.delta.affection_intensity += 0.04 * score;
            mapped.relationship_delta.closeness += 0.018 * score;
            mapped.relationship_delta.stability += 0.015 * score;
        }
        "desire" => {
            push_signal(&mut mapped.signals, "emotion:desire");
            mapped.delta.longing += 0.12 * score;
            mapped.delta.affection_intensity += 0.08 * score;
            mapped.delta.vulnerability += 0.035 * score;
            mapped.relationship_delta.closeness += 0.025 * score;
            mapped.relationship_delta.affection += 0.03 * score;
        }
        "relief" => {
            push_signal(&mut mapped.signals, "emotion:relief");
            mapped.delta.calm += 0.08 * score;
            mapped.delta.trust += 0.04 * score;
            mapped.delta.tension -= 0.05 * score;
            mapped.delta.hurt -= 0.035 * score;
            mapped.relationship_delta.stability += 0.03 * score;
            mapped.relationship_delta.tension -= 0.025 * score;
        }
        "remorse" => {
            push_signal(&mut mapped.signals, "emotion:remorse");
            mapped.delta.warmth += 0.04 * score;
            mapped.delta.trust += 0.035 * score;
            mapped.delta.hurt -= 0.06 * score;
            mapped.delta.tension -= 0.05 * score;
            mapped.relationship_delta.trust += 0.025 * score;
            mapped.relationship_delta.tension -= 0.025 * score;
            mapped.relationship_delta.stability += 0.02 * score;
        }
        "sadness" | "grief" | "disappointment" => {
            push_signal(&mut mapped.signals, "emotion:distress");
            mapped.delta.warmth += 0.035 * score;
            mapped.delta.vulnerability += 0.10 * score;
            mapped.delta.reassurance_need += 0.09 * score;
            mapped.delta.hurt += 0.045 * score;
            mapped.delta.calm -= 0.035 * score;
            mapped.relationship_delta.closeness += 0.012 * score;
        }
        "fear" | "nervousness" => {
            push_signal(&mut mapped.signals, "emotion:anxiety");
            mapped.delta.vulnerability += 0.09 * score;
            mapped.delta.reassurance_need += 0.10 * score;
            mapped.delta.tension += 0.04 * score;
            mapped.delta.calm -= 0.06 * score;
            mapped.relationship_delta.stability -= 0.015 * score;
        }
        "anger" | "annoyance" | "disapproval" | "disgust" | "contempt" | "frustration" => {
            push_signal(&mut mapped.signals, "emotion:conflict");
            mapped.delta.hurt += 0.08 * score;
            mapped.delta.irritation += 0.10 * score;
            mapped.delta.tension += 0.12 * score;
            mapped.delta.calm -= 0.08 * score;
            mapped.delta.warmth -= 0.06 * score;
            mapped.delta.trust -= 0.045 * score;
            mapped.relationship_delta.tension += 0.07 * score;
            mapped.relationship_delta.trust -= 0.045 * score;
            mapped.relationship_delta.affection -= 0.06 * score;
            mapped.relationship_delta.closeness -= 0.03 * score;
            mapped.relationship_delta.stability -= 0.035 * score;
        }
        "embarrassment" => {
            push_signal(&mut mapped.signals, "emotion:embarrassment");
            mapped.delta.vulnerability += 0.07 * score;
            mapped.delta.reassurance_need += 0.045 * score;
            mapped.delta.tension += 0.02 * score;
            mapped.delta.warmth += 0.02 * score;
        }
        "confusion" => {
            push_signal(&mut mapped.signals, "emotion:uncertainty");
            mapped.delta.tension += 0.025 * score;
            mapped.delta.reassurance_need += 0.035 * score;
            mapped.delta.calm -= 0.025 * score;
        }
        "curiosity" | "realization" | "surprise" => {
            push_signal(&mut mapped.signals, "emotion:engagement");
            mapped.delta.warmth += 0.025 * score;
            mapped.delta.vulnerability += 0.02 * score;
            mapped.relationship_delta.closeness += 0.01 * score;
        }
        "pride" => {
            push_signal(&mut mapped.signals, "emotion:pride");
            mapped.delta.calm += 0.035 * score;
            mapped.delta.warmth += 0.025 * score;
            mapped.relationship_delta.stability += 0.015 * score;
        }
        "neutral" => {
            push_signal(&mut mapped.signals, "emotion:neutral");
            mapped.relationship_delta.stability += 0.01 * score;
        }
        _ => {}
    }
}

fn normalize_label(label: &str) -> String {
    label.trim().to_ascii_lowercase().replace(' ', "_")
}

fn label_threshold(label: &str) -> f64 {
    match normalize_label(label).as_str() {
        "neutral" => 0.65,
        "love" | "desire" | "anger" | "fear" | "sadness" | "hurt" => 0.25,
        _ => 0.30,
    }
}

fn push_signal(signals: &mut Vec<String>, signal: &str) {
    if !signals.iter().any(|item| item == signal) {
        signals.push(signal.to_string());
    }
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn clamp_signed(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

fn clamp_signed_vector(mut value: EmotionVector) -> EmotionVector {
    value.warmth = clamp_signed(value.warmth);
    value.trust = clamp_signed(value.trust);
    value.calm = clamp_signed(value.calm);
    value.vulnerability = clamp_signed(value.vulnerability);
    value.longing = clamp_signed(value.longing);
    value.hurt = clamp_signed(value.hurt);
    value.tension = clamp_signed(value.tension);
    value.irritation = clamp_signed(value.irritation);
    value.affection_intensity = clamp_signed(value.affection_intensity);
    value.reassurance_need = clamp_signed(value.reassurance_need);
    value
}

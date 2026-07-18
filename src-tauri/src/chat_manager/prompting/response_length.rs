use crate::chat_manager::types::{AdvancedModelSettings, Model, Session, Settings};

pub const PLACEHOLDER: &str = "{{response_length_rules}}";

const SHORT_CHARS: u32 = 80;
const MEDIUM_CHARS: u32 = 150;
const LONG_CHARS: u32 = 300;
const DEFAULT_CUSTOM_CHARS: u32 = SHORT_CHARS;
const MIN_CUSTOM_CHARS: u32 = 20;
const MAX_CUSTOM_CHARS: u32 = 2000;

fn limit_for_settings(settings: &AdvancedModelSettings) -> Option<u32> {
    match settings.response_length_preset.as_deref()? {
        "auto" => None,
        "short" => Some(SHORT_CHARS),
        "medium" => Some(MEDIUM_CHARS),
        "long" => Some(LONG_CHARS),
        "custom" => Some(
            settings
                .response_length_chars
                .unwrap_or(DEFAULT_CUSTOM_CHARS)
                .clamp(MIN_CUSTOM_CHARS, MAX_CUSTOM_CHARS),
        ),
        _ => None,
    }
}

fn resolve_from_layers<'a>(
    layers: impl IntoIterator<Item = Option<&'a AdvancedModelSettings>>,
) -> Option<u32> {
    for settings in layers.into_iter().flatten() {
        if settings.response_length_preset.is_some() {
            return limit_for_settings(settings);
        }
    }
    None
}

pub fn resolve_limit(session: &Session, model: &Model, settings: &Settings) -> Option<u32> {
    resolve_from_layers([
        session.advanced_model_settings.as_ref(),
        model.advanced_model_settings.as_ref(),
        Some(&settings.advanced_model_settings),
    ])
}

pub fn render_rules(session: &Session, model: &Model, settings: &Settings) -> String {
    resolve_limit(session, model, settings)
        .map(|limit| format!("请将本轮回复控制在 {limit} 个汉字以内。"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advanced(preset: Option<&str>, chars: Option<u32>) -> AdvancedModelSettings {
        AdvancedModelSettings {
            response_length_preset: preset.map(str::to_string),
            response_length_chars: chars,
            ..AdvancedModelSettings::default()
        }
    }

    #[test]
    fn resolves_first_explicit_layer() {
        let session = advanced(Some("auto"), None);
        let model = advanced(Some("long"), None);
        assert_eq!(resolve_from_layers([Some(&session), Some(&model)]), None);
    }

    #[test]
    fn missing_field_inherits_from_next_layer() {
        let session = advanced(None, None);
        let model = advanced(Some("medium"), None);
        assert_eq!(
            resolve_from_layers([Some(&session), Some(&model)]),
            Some(150)
        );
    }

    #[test]
    fn custom_value_is_clamped() {
        assert_eq!(
            limit_for_settings(&advanced(Some("custom"), Some(10))),
            Some(20)
        );
        assert_eq!(
            limit_for_settings(&advanced(Some("custom"), Some(3000))),
            Some(2000)
        );
    }
}

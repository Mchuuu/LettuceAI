use serde_json::{json, Map, Value};

use crate::chat_manager::openai_responses;
use crate::chat_manager::reasoning::ReasoningMode;
use crate::chat_manager::types::ProviderCredential;
use crate::utils::log_debug_global;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsesDialect {
    OpenAi,
    VolcengineArk,
}
pub fn dialect_for(credential: &ProviderCredential) -> ResponsesDialect {
    let Some(config) = credential.config.as_ref() else {
        return ResponsesDialect::OpenAi;
    };

    match config.get("responsesDialect").and_then(Value::as_str) {
        Some("volcengine-ark") => ResponsesDialect::VolcengineArk,
        Some("openai") => ResponsesDialect::OpenAi,
        _ => {
            // Compatibility for providers created while Ark Web Search was the only Ark marker.
            let legacy_ark_search = config
                .get("webSearch")
                .and_then(Value::as_object)
                .and_then(|search| search.get("mode"))
                .and_then(Value::as_str)
                == Some("volcengine-ark");
            if legacy_ark_search {
                ResponsesDialect::VolcengineArk
            } else {
                ResponsesDialect::OpenAi
            }
        }
    }
}

fn set_reasoning(body: &mut Map<String, Value>, effort: Option<&str>) {
    if let Some(effort) = effort {
        body.insert("reasoning".to_string(), json!({ "effort": effort }));
    } else {
        body.remove("reasoning");
    }
}

pub fn apply_reasoning(
    body: &mut Value,
    credential: &ProviderCredential,
    mode: ReasoningMode,
    effort: Option<&str>,
) {
    if !openai_responses::is_provider_id(&credential.provider_id) {
        return;
    }
    let Some(body) = body.as_object_mut() else {
        return;
    };

    let dialect = dialect_for(credential);
    body.remove("thinking");
    match dialect {
        ResponsesDialect::OpenAi => match mode {
            ReasoningMode::ProviderDefault | ReasoningMode::Auto => body.remove("reasoning"),
            ReasoningMode::Enabled => {
                set_reasoning(body, effort);
                None
            }
            ReasoningMode::Disabled => {
                body.insert("reasoning".to_string(), json!({ "effort": "minimal" }))
            }
        },
        ResponsesDialect::VolcengineArk => match mode {
            ReasoningMode::ProviderDefault => body.remove("reasoning"),
            ReasoningMode::Auto => {
                body.remove("reasoning");
                body.insert("thinking".to_string(), json!({ "type": "auto" }))
            }
            ReasoningMode::Enabled => {
                set_reasoning(body, effort);
                body.insert("thinking".to_string(), json!({ "type": "enabled" }))
            }
            ReasoningMode::Disabled => {
                body.insert("reasoning".to_string(), json!({ "effort": "minimal" }));
                body.insert("thinking".to_string(), json!({ "type": "disabled" }))
            }
        },
    };

    let applied_effort = body
        .get("reasoning")
        .and_then(Value::as_object)
        .and_then(|reasoning| reasoning.get("effort"))
        .and_then(Value::as_str)
        .unwrap_or("omitted");
    let applied_thinking = body
        .get("thinking")
        .and_then(Value::as_object)
        .and_then(|thinking| thinking.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("omitted");
    log_debug_global(
        "responses_compatibility",
        format!(
            "Applied Responses reasoning dialect={:?} mode={} requested_effort={} applied_effort={} thinking={}",
            dialect,
            mode.as_str(),
            effort.unwrap_or("provider-default"),
            applied_effort,
            applied_thinking,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential(dialect: Option<&str>) -> ProviderCredential {
        ProviderCredential {
            id: "credential-1".to_string(),
            provider_id: openai_responses::CUSTOM_PROVIDER_ID.to_string(),
            label: "Responses".to_string(),
            api_key: None,
            base_url: None,
            default_model: None,
            headers: None,
            config: Some(match dialect {
                Some(dialect) => json!({ "responsesDialect": dialect }),
                None => json!({}),
            }),
        }
    }

    #[test]
    fn ark_enabled_sends_thinking_and_effort() {
        let mut body = json!({ "reasoning": { "effort": "minimal" } });
        apply_reasoning(
            &mut body,
            &credential(Some("volcengine-ark")),
            ReasoningMode::Enabled,
            Some("max"),
        );
        assert_eq!(body.pointer("/thinking/type"), Some(&json!("enabled")));
        assert_eq!(body.pointer("/reasoning/effort"), Some(&json!("max")));
    }

    #[test]
    fn provider_default_omits_control_fields() {
        let mut body = json!({ "reasoning": { "effort": "minimal" } });
        apply_reasoning(
            &mut body,
            &credential(None),
            ReasoningMode::ProviderDefault,
            None,
        );
        assert!(body.get("reasoning").is_none());
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn ark_disabled_forces_minimal_effort() {
        let mut body = json!({ "reasoning": { "effort": "max" } });
        apply_reasoning(
            &mut body,
            &credential(Some("volcengine-ark")),
            ReasoningMode::Disabled,
            Some("max"),
        );
        assert_eq!(body.pointer("/thinking/type"), Some(&json!("disabled")));
        assert_eq!(body.pointer("/reasoning/effort"), Some(&json!("minimal")));
    }
}

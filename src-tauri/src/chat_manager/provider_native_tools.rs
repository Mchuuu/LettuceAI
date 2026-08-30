use serde_json::{json, Value};

use super::openai_responses;
use super::provider_adapter::responses_compatibility::{dialect_for, ResponsesDialect};
use super::types::ProviderCredential;
use crate::utils::{log_info_global, log_warn_global};

const ARK_WEB_SEARCH_MODE: &str = "volcengine-ark";
const DEFAULT_MAX_KEYWORD: u64 = 2;
const DEFAULT_LIMIT: u64 = 10;
const DEFAULT_MAX_TOOL_CALLS: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedNativeTools {
    pub web_search: bool,
}

impl AppliedNativeTools {
    fn none() -> Self {
        Self { web_search: false }
    }
}

fn bounded_u64(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn web_search_config(credential: &ProviderCredential) -> Option<&serde_json::Map<String, Value>> {
    credential
        .config
        .as_ref()?
        .get("webSearch")?
        .as_object()
        .filter(|config| config.get("mode").and_then(Value::as_str) == Some(ARK_WEB_SEARCH_MODE))
}

/// Adds provider-executed tools to an already-adapted request body.
///
/// This stays separate from canonical function tools because the provider executes these tools
/// inside a single response lifecycle; LettuceAI must never dispatch them through the local tool
/// loop.
pub fn apply(
    body: &mut Value,
    credential: &ProviderCredential,
    web_search_enabled: bool,
) -> AppliedNativeTools {
    if !web_search_enabled {
        return AppliedNativeTools::none();
    }

    let Some(config) = web_search_config(credential) else {
        return AppliedNativeTools::none();
    };

    if !openai_responses::is_provider_id(&credential.provider_id) {
        log_warn_global(
            "provider_native_tools",
            format!(
                "Ignoring Volcengine Ark Web Search for non-Responses provider {}",
                credential.provider_id
            ),
        );
        return AppliedNativeTools::none();
    }
    if dialect_for(credential) != ResponsesDialect::VolcengineArk {
        return AppliedNativeTools::none();
    }

    let max_keyword = bounded_u64(config.get("maxKeyword"), DEFAULT_MAX_KEYWORD, 1, 50);
    let limit = bounded_u64(config.get("limit"), DEFAULT_LIMIT, 1, 50);
    let max_tool_calls = bounded_u64(config.get("maxToolCalls"), DEFAULT_MAX_TOOL_CALLS, 1, 10);

    let Some(body) = body.as_object_mut() else {
        log_warn_global(
            "provider_native_tools",
            "Unable to add Volcengine Ark Web Search to a non-object request body",
        );
        return AppliedNativeTools::none();
    };

    let tools = body
        .entry("tools".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(tools) = tools.as_array_mut() else {
        log_warn_global(
            "provider_native_tools",
            "Unable to merge Volcengine Ark Web Search because tools is not an array",
        );
        return AppliedNativeTools::none();
    };

    if !tools
        .iter()
        .any(|tool| tool.get("type").and_then(Value::as_str) == Some("web_search"))
    {
        tools.push(json!({
            "type": "web_search",
            "max_keyword": max_keyword,
            "limit": limit,
        }));
    }
    body.entry("tool_choice".to_string())
        .or_insert_with(|| json!("auto"));
    body.insert("max_tool_calls".to_string(), json!(max_tool_calls));

    log_info_global(
        "provider_native_tools",
        format!(
            "Applied Volcengine Ark Web Search: provider={} max_keyword={} limit={} max_tool_calls={}",
            credential.provider_id, max_keyword, limit, max_tool_calls
        ),
    );

    AppliedNativeTools { web_search: true }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential(config: Value) -> ProviderCredential {
        ProviderCredential {
            id: "credential-1".to_string(),
            provider_id: openai_responses::CUSTOM_PROVIDER_ID.to_string(),
            label: "Responses".to_string(),
            api_key: None,
            base_url: None,
            default_model: None,
            headers: None,
            config: Some(config),
        }
    }

    #[test]
    fn applies_ark_web_search_with_bounded_settings() {
        let credential = credential(json!({
            "webSearch": {
                "mode": "volcengine-ark",
                "maxKeyword": 99,
                "limit": 0,
                "maxToolCalls": 4
            }
        }));
        let mut body = json!({ "model": "doubao", "tools": [] });

        let applied = apply(&mut body, &credential, true);

        assert!(applied.web_search);
        assert_eq!(body.pointer("/tools/0/type"), Some(&json!("web_search")));
        assert_eq!(body.pointer("/tools/0/max_keyword"), Some(&json!(50)));
        assert_eq!(body.pointer("/tools/0/limit"), Some(&json!(1)));
        assert_eq!(body.get("max_tool_calls"), Some(&json!(4)));
        assert_eq!(body.get("tool_choice"), Some(&json!("auto")));
    }

    #[test]
    fn requires_both_provider_configuration_and_model_enablement() {
        let enabled_credential = credential(json!({
            "webSearch": { "mode": "volcengine-ark" }
        }));
        let mut body = json!({ "model": "doubao" });

        assert!(!apply(&mut body, &enabled_credential, false).web_search);
        assert!(body.get("tools").is_none());

        let mut unconfigured = json!({ "model": "doubao" });
        let disabled_credential = credential(json!({ "webSearch": { "mode": "none" } }));
        assert!(!apply(&mut unconfigured, &disabled_credential, true).web_search);
        assert!(unconfigured.get("tools").is_none());
    }
}

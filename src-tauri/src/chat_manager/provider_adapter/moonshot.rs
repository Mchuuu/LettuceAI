use std::collections::HashMap;

use serde_json::{json, Value};

use super::{OpenAIChatRequest, ProviderAdapter};
use crate::chat_manager::tooling::{openai_tool_choice, openai_tools, ToolConfig};

pub struct MoonshotAdapter;

fn moonshot_thinking_type(model_name: &str, reasoning_enabled: bool) -> Option<&'static str> {
    let normalized = model_name.to_ascii_lowercase();
    if normalized.contains("kimi-k2.7-code") {
        return Some("enabled");
    }
    if normalized.contains("kimi-k2.6") || normalized.contains("kimi-k2.5") {
        return Some(if reasoning_enabled {
            "enabled"
        } else {
            "disabled"
        });
    }
    None
}

impl ProviderAdapter for MoonshotAdapter {
    fn endpoint(&self, base_url: &str) -> String {
        // Moonshot base: https://api.moonshot.ai, endpoint: /v1/chat/completions
        let trimmed = base_url.trim_end_matches('/');
        if trimmed.ends_with("/v1") {
            format!("{}/chat/completions", trimmed)
        } else {
            format!("{}/v1/chat/completions", trimmed)
        }
    }

    fn system_role(&self) -> std::borrow::Cow<'static, str> {
        "system".into()
    }

    fn required_auth_headers(&self) -> &'static [&'static str] {
        &["Authorization"]
    }

    fn default_headers_template(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        out.insert("Authorization".into(), "Bearer <apiKey>".into());
        out.insert("Content-Type".into(), "application/json".into());
        out.insert("Accept".into(), "text/event-stream".into());
        out
    }

    fn headers(
        &self,
        api_key: &str,
        extra: Option<&HashMap<String, String>>,
    ) -> HashMap<String, String> {
        let mut out: HashMap<String, String> = HashMap::new();
        out.insert("Authorization".into(), format!("Bearer {}", api_key));
        out.insert("Content-Type".into(), "application/json".into());
        out.insert("Accept".into(), "text/event-stream".into());
        out.entry("User-Agent".into())
            .or_insert_with(|| "LettuceAI/0.1".into());
        if let Some(extra) = extra {
            for (k, v) in extra.iter() {
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }

    fn body(
        &self,
        model_name: &str,
        messages_for_api: &Vec<Value>,
        _system_prompt: Option<String>,
        temperature: Option<f64>,
        top_p: Option<f64>,
        max_tokens: u32,
        context_length: Option<u32>,
        should_stream: bool,
        frequency_penalty: Option<f64>,
        presence_penalty: Option<f64>,
        _top_k: Option<u32>,
        tool_config: Option<&ToolConfig>,
        reasoning_enabled: bool,
        _reasoning_effort: Option<String>,
        reasoning_budget: Option<u32>,
    ) -> Value {
        let (tools, tool_choice) = if let Some(cfg) = tool_config {
            let tools = openai_tools(cfg);
            let choice = if tools.is_some() {
                openai_tool_choice(cfg.choice.as_ref())
            } else {
                None
            };
            (tools, choice)
        } else {
            (None, None)
        };

        let total_tokens = max_tokens + reasoning_budget.unwrap_or(0);
        let body = OpenAIChatRequest {
            model: model_name,
            messages: messages_for_api,
            stream: should_stream,
            temperature,
            top_p,
            max_tokens: Some(total_tokens),
            context_length,
            frequency_penalty,
            presence_penalty,
            max_completion_tokens: None,
            reasoning_effort: None,
            reasoning: None,
            tools,
            tool_choice,
        };

        let mut value = serde_json::to_value(body).unwrap_or_else(|_| json!({}));
        if let (Some(map), Some(thinking_type)) = (
            value.as_object_mut(),
            moonshot_thinking_type(model_name, reasoning_enabled),
        ) {
            map.insert(
                "thinking".to_string(),
                json!({
                    "type": thinking_type
                }),
            );
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moonshot_body(model_name: &str, reasoning_enabled: bool) -> Value {
        MoonshotAdapter.body(
            model_name,
            &vec![json!({"role": "user", "content": "hello"})],
            None,
            Some(0.7),
            Some(0.9),
            1024,
            None,
            true,
            None,
            None,
            None,
            None,
            reasoning_enabled,
            Some("medium".to_string()),
            Some(2048),
        )
    }

    #[test]
    fn kimi_k27_code_never_sends_disabled_thinking() {
        let body = moonshot_body("kimi-k2.7-code", false);

        assert_eq!(body.pointer("/thinking/type"), Some(&json!("enabled")));
        assert!(body.get("enable_thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn kimi_k26_can_disable_thinking() {
        let body = moonshot_body("kimi-k2.6", false);

        assert_eq!(body.pointer("/thinking/type"), Some(&json!("disabled")));
    }

    #[test]
    fn unknown_moonshot_model_omits_thinking() {
        let body = moonshot_body("moonshot-v1-8k", false);

        assert!(body.get("thinking").is_none());
    }
}

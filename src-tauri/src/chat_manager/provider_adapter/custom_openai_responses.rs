use std::borrow::Cow;
use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Value};

use super::ProviderAdapter;
use crate::chat_manager::openai_responses;
use crate::chat_manager::tooling::ToolConfig;
use crate::chat_manager::types::ProviderCredential;

pub struct CustomOpenAIResponsesAdapter {
    credential_config: Option<Value>,
}

impl CustomOpenAIResponsesAdapter {
    pub fn new(credential: &ProviderCredential) -> Self {
        Self {
            credential_config: credential.config.clone(),
        }
    }

    fn config_value(&self, key: &str) -> Option<String> {
        self.credential_config
            .as_ref()
            .and_then(|value| value.get(key))
            .and_then(Value::as_str)
            .map(str::to_string)
    }

    fn auth_mode(&self) -> String {
        self.config_value("authMode")
            .unwrap_or_else(|| "bearer".to_string())
            .to_ascii_lowercase()
    }

    fn auth_header_name(&self) -> String {
        self.config_value("authHeaderName")
            .unwrap_or_else(|| "x-api-key".to_string())
    }

    fn auth_query_param_name(&self) -> String {
        self.config_value("authQueryParamName")
            .unwrap_or_else(|| "api_key".to_string())
    }

    fn resolve_endpoint(&self, base_url: &str, config_key: &str, default_path: &str) -> String {
        let path = self
            .config_value(config_key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| default_path.to_string());
        if path.starts_with("http://") || path.starts_with("https://") {
            path
        } else if path.starts_with('/') {
            format!("{}{}", base_url.trim_end_matches('/'), path)
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), path)
        }
    }

    fn append_query_auth(&self, url: String, api_key: &str) -> String {
        if self.auth_mode() != "query" || api_key.trim().is_empty() {
            return url;
        }
        let separator = if url.contains('?') { '&' } else { '?' };
        format!(
            "{}{}{}={}",
            url,
            separator,
            self.auth_query_param_name(),
            api_key
        )
    }

    fn tool_choice_mode(&self) -> String {
        self.config_value("toolChoiceMode")
            .unwrap_or_else(|| "auto".to_string())
            .to_ascii_lowercase()
    }
}

#[derive(Serialize)]
struct ResponsesRequest {
    model: String,
    input: Vec<Value>,
    stream: bool,
    store: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<Value>,
}

impl ProviderAdapter for CustomOpenAIResponsesAdapter {
    fn endpoint(&self, base_url: &str) -> String {
        self.resolve_endpoint(base_url, "chatEndpoint", "/v1/responses")
    }

    fn build_url(
        &self,
        base_url: &str,
        _model_name: &str,
        api_key: &str,
        _should_stream: bool,
    ) -> String {
        self.append_query_auth(self.endpoint(base_url), api_key)
    }

    fn system_role(&self) -> Cow<'static, str> {
        Cow::Borrowed("system")
    }

    fn supports_stream(&self) -> bool {
        self.credential_config
            .as_ref()
            .and_then(|value| value.get("supportsStream"))
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    fn required_auth_headers(&self) -> &'static [&'static str] {
        &["Authorization"]
    }

    fn default_headers_template(&self) -> HashMap<String, String> {
        HashMap::from([("Authorization".to_string(), "Bearer $API_KEY".to_string())])
    }

    fn headers(
        &self,
        api_key: &str,
        extra: Option<&HashMap<String, String>>,
    ) -> HashMap<String, String> {
        let mut headers = HashMap::from([
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "text/event-stream".to_string()),
        ]);

        match self.auth_mode().as_str() {
            "none" | "query" => {}
            "header" => {
                if !api_key.trim().is_empty() {
                    headers.insert(self.auth_header_name(), api_key.to_string());
                }
            }
            _ => {
                if !api_key.trim().is_empty() {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
                }
            }
        }

        if let Some(extra) = extra {
            headers.extend(extra.clone());
        }
        headers
    }

    fn list_models_endpoint(&self, base_url: &str) -> String {
        self.resolve_endpoint(base_url, "modelsEndpoint", "/v1/models")
    }

    fn body(
        &self,
        model_name: &str,
        messages_for_api: &Vec<Value>,
        system_prompt: Option<String>,
        temperature: Option<f64>,
        top_p: Option<f64>,
        max_tokens: u32,
        _context_length: Option<u32>,
        should_stream: bool,
        _frequency_penalty: Option<f64>,
        _presence_penalty: Option<f64>,
        _top_k: Option<u32>,
        tool_config: Option<&ToolConfig>,
        reasoning_enabled: bool,
        reasoning_effort: Option<String>,
        _reasoning_budget: Option<u32>,
    ) -> Value {
        let tools = tool_config.and_then(openai_responses::tools);
        let tool_choice = if tools.is_some() {
            match self.tool_choice_mode().as_str() {
                "auto" => Some(json!("auto")),
                "none" => Some(json!("none")),
                "required" => Some(json!("required")),
                "omit" => None,
                "passthrough" => tool_config
                    .and_then(|config| openai_responses::tool_choice(config.choice.as_ref())),
                _ => Some(json!("auto")),
            }
        } else {
            None
        };
        let reasoning = if reasoning_enabled {
            reasoning_effort.map(|effort| json!({ "effort": effort }))
        } else {
            Some(json!({ "effort": "minimal" }))
        };

        serde_json::to_value(ResponsesRequest {
            model: model_name.to_string(),
            input: openai_responses::messages_to_input(messages_for_api),
            stream: should_stream,
            store: false,
            instructions: system_prompt,
            temperature,
            top_p,
            max_output_tokens: Some(max_tokens),
            reasoning,
            tools,
            tool_choice,
        })
        .unwrap_or_else(|_| json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential() -> ProviderCredential {
        ProviderCredential {
            id: "credential-1".to_string(),
            provider_id: openai_responses::CUSTOM_PROVIDER_ID.to_string(),
            label: "Responses".to_string(),
            api_key: None,
            base_url: Some("https://example.com".to_string()),
            default_model: None,
            headers: None,
            config: None,
        }
    }

    #[test]
    fn builds_standard_responses_request() {
        let adapter = CustomOpenAIResponsesAdapter::new(&credential());
        let body = adapter.body(
            "model-1",
            &vec![json!({ "role": "user", "content": "hello" })],
            Some("system".to_string()),
            Some(0.7),
            Some(0.9),
            1024,
            None,
            true,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
        );

        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["store"], json!(false));
        assert_eq!(body["max_output_tokens"], json!(1024));
        assert_eq!(body.pointer("/reasoning/effort"), Some(&json!("minimal")));
        assert!(body.get("messages").is_none());
    }
}

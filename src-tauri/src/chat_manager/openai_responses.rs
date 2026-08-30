use std::collections::HashSet;

use serde_json::{json, Value};

use super::tooling::{ToolCall, ToolChoice, ToolConfig};

pub const CUSTOM_PROVIDER_ID: &str = "custom-openai-responses";

pub fn is_provider_id(provider_id: &str) -> bool {
    provider_id.eq_ignore_ascii_case(CUSTOM_PROVIDER_ID)
}

pub fn looks_like_response(value: &Value) -> bool {
    value.get("object").and_then(Value::as_str) == Some("response")
        || value
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|event_type| event_type.starts_with("response."))
}

pub fn messages_to_input(messages: &[Value]) -> Vec<Value> {
    let mut input = Vec::new();

    for message in messages {
        let Some(object) = message.as_object() else {
            input.push(message.clone());
            continue;
        };

        let role = object.get("role").and_then(Value::as_str).unwrap_or("user");

        if role == "tool" {
            if let Some(call_id) = object.get("tool_call_id").and_then(Value::as_str) {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": tool_output_text(object.get("content")),
                }));
            }
            continue;
        }

        if let Some(content) = object.get("content") {
            let converted = convert_message_content(content);
            if has_message_content(&converted) {
                input.push(json!({
                    "type": "message",
                    "role": role,
                    "content": converted,
                }));
            }
        }

        if role == "assistant" {
            if let Some(calls) = object.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    if let Some(item) = chat_tool_call_to_response_item(call) {
                        input.push(item);
                    }
                }
            }
        }
    }

    input
}

pub fn tools(config: &ToolConfig) -> Option<Vec<Value>> {
    if config.tools.is_empty() {
        return None;
    }

    Some(
        config
            .tools
            .iter()
            .map(|tool| {
                let mut item = json!({
                    "type": "function",
                    "name": tool.name,
                    "parameters": tool.parameters,
                });
                if let Some(description) = tool.description.as_ref() {
                    item["description"] = Value::String(description.clone());
                }
                item
            })
            .collect(),
    )
}

pub fn tool_choice(choice: Option<&ToolChoice>) -> Option<Value> {
    match choice {
        None => None,
        Some(ToolChoice::Auto) => Some(json!("auto")),
        Some(ToolChoice::None) => Some(json!("none")),
        Some(ToolChoice::Required) | Some(ToolChoice::Any) => Some(json!("required")),
        Some(ToolChoice::Tool { name }) => Some(json!({
            "type": "function",
            "name": name,
        })),
    }
}

pub fn extract_output_text(value: &Value) -> Option<String> {
    let response = response_object(value);
    let output = response.get("output").and_then(Value::as_array)?;
    let mut text = String::new();

    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in content {
            let fragment = match part.get("type").and_then(Value::as_str) {
                Some("output_text") => part.get("text").and_then(Value::as_str),
                Some("refusal") => part.get("refusal").and_then(Value::as_str),
                _ => None,
            };
            if let Some(fragment) = fragment {
                text.push_str(fragment);
            }
        }
    }

    (!text.is_empty()).then_some(text)
}

pub fn extract_reasoning_summary(value: &Value) -> Option<String> {
    let response = response_object(value);
    let output = response.get("output").and_then(Value::as_array)?;
    let mut summary = String::new();

    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("reasoning") {
            continue;
        }
        let Some(parts) = item.get("summary").and_then(Value::as_array) else {
            continue;
        };
        for part in parts {
            if let Some(fragment) = part.get("text").and_then(Value::as_str) {
                summary.push_str(fragment);
            }
        }
    }

    (!summary.is_empty()).then_some(summary)
}

pub fn extract_tool_calls(value: &Value) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut seen = HashSet::new();
    collect_tool_calls(value, &mut calls, &mut seen);
    calls
}

pub fn stream_text_delta(value: &Value) -> Option<String> {
    matches!(
        event_type(value),
        Some("response.output_text.delta") | Some("response.refusal.delta")
    )
    .then(|| {
        value
            .get("delta")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
    .flatten()
}

pub fn stream_reasoning_delta(value: &Value) -> Option<String> {
    matches!(
        event_type(value),
        Some("response.reasoning_summary_text.delta")
            | Some("response.reasoning_summary.delta")
            | Some("response.reasoning_text.delta")
    )
    .then(|| {
        value
            .get("delta")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
    .flatten()
}

pub fn stream_tool_call(value: &Value) -> Option<ToolCall> {
    if event_type(value) != Some("response.output_item.done") {
        return None;
    }
    response_item_to_tool_call(value.get("item")?)
}

pub fn stream_error_message(value: &Value) -> Option<String> {
    match event_type(value) {
        Some("response.failed") => value
            .get("response")
            .and_then(|response| response.get("error"))
            .and_then(error_message),
        Some("error") => value.get("error").and_then(error_message).or_else(|| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        _ => None,
    }
}

pub fn is_terminal_stream_event(value: &Value) -> bool {
    matches!(
        event_type(value),
        Some("response.completed") | Some("response.incomplete") | Some("response.failed")
    )
}

pub fn event_type(value: &Value) -> Option<&str> {
    value.get("type").and_then(Value::as_str)
}

fn response_object(value: &Value) -> &Value {
    value.get("response").unwrap_or(value)
}

fn convert_message_content(content: &Value) -> Value {
    match content {
        Value::Array(parts) => Value::Array(parts.iter().map(convert_content_part).collect()),
        other => other.clone(),
    }
}

fn convert_content_part(part: &Value) -> Value {
    let Some(object) = part.as_object() else {
        return part.clone();
    };
    match object.get("type").and_then(Value::as_str) {
        Some("text") => json!({
            "type": "input_text",
            "text": object.get("text").cloned().unwrap_or_default(),
        }),
        Some("image_url") => convert_image_part(object).unwrap_or_else(|| part.clone()),
        _ => part.clone(),
    }
}

fn convert_image_part(object: &serde_json::Map<String, Value>) -> Option<Value> {
    let image = object.get("image_url")?;
    let detail = object
        .get("detail")
        .and_then(Value::as_str)
        .or_else(|| image.get("detail").and_then(Value::as_str));

    let mut converted = json!({ "type": "input_image" });
    if let Some(file_id) = image.get("file_id").and_then(Value::as_str) {
        converted["file_id"] = Value::String(file_id.to_string());
    } else if let Some(url) = image
        .as_str()
        .or_else(|| image.get("url").and_then(Value::as_str))
    {
        converted["image_url"] = Value::String(url.to_string());
    } else {
        return None;
    }
    if let Some(detail) = detail {
        converted["detail"] = Value::String(detail.to_string());
    }
    Some(converted)
}

fn has_message_content(content: &Value) -> bool {
    match content {
        Value::Null => false,
        Value::String(text) => !text.is_empty(),
        Value::Array(parts) => !parts.is_empty(),
        _ => true,
    }
}

fn chat_tool_call_to_response_item(call: &Value) -> Option<Value> {
    let function = call.get("function")?;
    let name = function.get("name").and_then(Value::as_str)?;
    let call_id = call
        .get("id")
        .or_else(|| call.get("call_id"))
        .and_then(Value::as_str)?;
    let arguments = arguments_as_string(function.get("arguments"));
    Some(json!({
        "type": "function_call",
        "call_id": call_id,
        "name": name,
        "arguments": arguments,
    }))
}

fn tool_output_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(other) => serde_json::to_string(other).unwrap_or_default(),
        None => String::new(),
    }
}

fn arguments_as_string(arguments: Option<&Value>) -> String {
    match arguments {
        Some(Value::String(raw)) => raw.clone(),
        Some(value) => serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
        None => "{}".to_string(),
    }
}

fn arguments_from_string(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn response_item_to_tool_call(item: &Value) -> Option<ToolCall> {
    if item.get("type").and_then(Value::as_str) != Some("function_call") {
        return None;
    }
    let call_id = item
        .get("call_id")
        .or_else(|| item.get("id"))
        .and_then(Value::as_str)?;
    let name = item.get("name").and_then(Value::as_str)?;
    let raw_arguments = arguments_as_string(item.get("arguments"));
    Some(ToolCall {
        id: call_id.to_string(),
        name: name.to_string(),
        arguments: arguments_from_string(&raw_arguments),
        raw_arguments: Some(raw_arguments),
        thought_signature: None,
    })
}

fn collect_tool_calls(value: &Value, calls: &mut Vec<ToolCall>, seen: &mut HashSet<String>) {
    match value {
        Value::String(raw) if raw.contains("data:") => {
            for line in raw.lines() {
                let Some(payload) = line.trim().strip_prefix("data:") else {
                    continue;
                };
                let payload = payload.trim();
                if payload.is_empty() || payload == "[DONE]" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<Value>(payload) {
                    collect_tool_calls(&parsed, calls, seen);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_tool_calls(item, calls, seen);
            }
        }
        Value::Object(map) => {
            if let Some(call) = response_item_to_tool_call(value) {
                if seen.insert(call.id.clone()) {
                    calls.push(call);
                }
                return;
            }
            if let Some(event_type) = map.get("type").and_then(Value::as_str) {
                match event_type {
                    "response.output_item.done" => {
                        if let Some(item) = map.get("item") {
                            collect_tool_calls(item, calls, seen);
                        }
                    }
                    "response.completed" => {
                        if let Some(response) = map.get("response") {
                            collect_tool_calls(response, calls, seen);
                        }
                    }
                    _ if event_type.starts_with("response.") => {}
                    _ => {}
                }
                if event_type.starts_with("response.") {
                    return;
                }
            }
            if let Some(item) = map.get("item") {
                collect_tool_calls(item, calls, seen);
            }
            if let Some(output) = map.get("output") {
                collect_tool_calls(output, calls, seen);
            }
            if let Some(response) = map.get("response") {
                collect_tool_calls(response, calls, seen);
            }
        }
        _ => {}
    }
}

fn error_message(error: &Value) -> Option<String> {
    error
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| error.as_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_chat_messages_images_and_tool_history() {
        let input = messages_to_input(&[
            json!({
                "role": "user",
                "content": [
                    { "type": "text", "text": "look" },
                    { "type": "image_url", "image_url": { "file_id": "file-1" } }
                ]
            }),
            json!({
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": { "name": "remember", "arguments": "{\"value\":1}" }
                }]
            }),
            json!({ "role": "tool", "tool_call_id": "call-1", "content": "ok" }),
        ]);

        assert_eq!(
            input[0].pointer("/content/0/type"),
            Some(&json!("input_text"))
        );
        assert_eq!(
            input[0].pointer("/content/1/file_id"),
            Some(&json!("file-1"))
        );
        assert_eq!(input[1]["type"], json!("function_call"));
        assert_eq!(input[2]["type"], json!("function_call_output"));
    }

    #[test]
    fn extracts_only_visible_response_text() {
        let response = json!({
            "object": "response",
            "output": [
                { "type": "reasoning", "summary": [{ "type": "summary_text", "text": "why" }] },
                { "type": "message", "content": [{ "type": "output_text", "text": "hello" }] }
            ]
        });

        assert_eq!(extract_output_text(&response).as_deref(), Some("hello"));
        assert_eq!(extract_reasoning_summary(&response).as_deref(), Some("why"));
    }

    #[test]
    fn extracts_refusal_as_visible_text() {
        let response = json!({
            "object": "response",
            "output": [{
                "type": "message",
                "content": [{ "type": "refusal", "refusal": "cannot comply" }]
            }]
        });

        assert_eq!(
            extract_output_text(&response).as_deref(),
            Some("cannot comply")
        );
        assert_eq!(
            stream_text_delta(&json!({
                "type": "response.refusal.delta",
                "delta": "cannot comply"
            }))
            .as_deref(),
            Some("cannot comply")
        );
    }

    #[test]
    fn extracts_function_calls_using_call_id() {
        let response = json!({
            "object": "response",
            "output": [{
                "type": "function_call",
                "id": "fc-1",
                "call_id": "call-1",
                "name": "remember",
                "arguments": "{\"value\":1}"
            }]
        });

        let calls = extract_tool_calls(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call-1");
        assert_eq!(calls[0].arguments, json!({ "value": 1 }));
    }
}

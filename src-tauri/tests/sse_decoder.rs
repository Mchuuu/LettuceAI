//! Integration tests for the SSE decoder and helpers.

use lettuceai_lib::chat_manager::sse::{
    accumulate_text_from_sse, usage_from_sse, usage_from_value, SseDecoder,
};
use lettuceai_lib::chat_manager::types::NormalizedEvent;
use serde_json::json;

fn collect_text(events: &[NormalizedEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            NormalizedEvent::Delta { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn collect_reasoning(events: &[NormalizedEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            NormalizedEvent::Reasoning { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn decode_openai_delta() {
    let mut dec = SseDecoder::new();
    let chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n";
    let evts = dec.feed(chunk, Some("openai"));
    assert!(!evts.is_empty());
    let text = collect_text(&evts);
    assert!(text.contains("hello"), "got {text:?}");
}

#[test]
fn decode_openai_multi_chunk_assembly() {
    let mut dec = SseDecoder::new();
    let mut all_events = Vec::new();
    all_events.extend(dec.feed(
        "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n",
        Some("openai"),
    ));
    all_events.extend(dec.feed(
        "data: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}\n\n",
        Some("openai"),
    ));
    all_events.extend(dec.feed(
        "data: {\"choices\":[{\"delta\":{\"content\":\"c\"}}]}\n\n",
        Some("openai"),
    ));
    let text = collect_text(&all_events);
    assert!(text.contains("a") && text.contains("b") && text.contains("c"));
}

#[test]
fn decode_partial_chunk_buffered() {
    let mut dec = SseDecoder::new();
    let first = dec.feed("data: {\"choi", Some("openai"));
    assert!(first.is_empty(), "incomplete line should not emit events");
    let second = dec.feed(
        "ces\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
        Some("openai"),
    );
    let text = collect_text(&second);
    assert!(text.contains("ok"));
}

#[test]
fn decode_anthropic_content_block_delta_no_panic() {
    let mut dec = SseDecoder::new();
    let chunk = "event: content_block_delta\ndata: {\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n";
    let _ = dec.feed(chunk, Some("anthropic"));
}

#[test]
fn decode_gemini_candidate_text() {
    let mut dec = SseDecoder::new();
    let chunk = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"hello\"}]}}]}\n\n";
    let evts = dec.feed(chunk, Some("gemini"));
    let text = collect_text(&evts);
    assert!(text.contains("hello"));
}

#[test]
fn decode_done_marker() {
    let mut dec = SseDecoder::new();
    let evts = dec.feed("data: [DONE]\n\n", Some("openai"));
    let has_done = evts.iter().any(|e| matches!(e, NormalizedEvent::Done));
    assert!(has_done, "expected Done event");
}

#[test]
fn decode_reasoning_field() {
    let mut dec = SseDecoder::new();
    let chunk = "data: {\"choices\":[{\"delta\":{\"reasoning\":\"thinking...\"}}]}\n\n";
    let evts = dec.feed(chunk, Some("openai"));
    let reasoning = collect_reasoning(&evts);
    assert!(reasoning.contains("thinking"), "got: {evts:?}");
}

#[test]
fn decode_openai_responses_lifecycle() {
    let mut dec = SseDecoder::new();
    let chunk = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"delta\":\"why\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"web_search_call\",\"id\":\"ws-1\",\"status\":\"completed\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"remember\",\"arguments\":\"{\\\"value\\\":1}\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"object\":\"response\",\"usage\":{\"input_tokens\":10,\"output_tokens\":5,\"total_tokens\":15,\"tool_usage\":2}}}\n\n",
    );

    let events = dec.feed(chunk, Some("custom-openai-responses"));
    assert_eq!(collect_text(&events), "hello");
    assert_eq!(collect_reasoning(&events), "why");
    assert!(events.iter().any(|event| matches!(
        event,
        NormalizedEvent::ToolCall { calls }
            if calls.len() == 1 && calls[0].id == "call-1" && calls[0].name == "remember"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        NormalizedEvent::Usage { usage }
            if usage.prompt_tokens == Some(10)
                && usage.completion_tokens == Some(5)
                && usage.web_search_requests == Some(2)
    )));
    assert!(events
        .iter()
        .any(|event| matches!(event, NormalizedEvent::Done)));
}

#[test]
fn openai_responses_accumulator_uses_deltas_only() {
    let raw = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
        "data: {\"type\":\"response.output_text.done\",\"text\":\"hello\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"object\":\"response\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]}]}}\n\n",
    );

    assert_eq!(
        accumulate_text_from_sse(raw, Some("custom-openai-responses")).as_deref(),
        Some("hello")
    );
}

#[test]
fn decode_malformed_json_skipped() {
    let mut dec = SseDecoder::new();
    let evts = dec.feed("data: {not_valid_json\n\n", Some("openai"));
    // Should not panic; events may be empty.
    let _ = evts;
}

#[test]
fn decode_empty_input_no_events() {
    let mut dec = SseDecoder::new();
    let evts = dec.feed("", Some("openai"));
    assert!(evts.is_empty());
}

#[test]
fn usage_extracted_from_openai_stream() {
    let raw = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}],\"usage\":null}\ndata: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n";
    let usage = usage_from_sse(raw).expect("should find usage");
    assert_eq!(usage.prompt_tokens, Some(10));
    assert_eq!(usage.completion_tokens, Some(5));
}

#[test]
fn usage_from_value_openai_shape() {
    let v = json!({
        "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 }
    });
    let usage = usage_from_value(&v).expect("should extract");
    assert_eq!(usage.prompt_tokens, Some(100));
    assert_eq!(usage.completion_tokens, Some(50));
}

#[test]
fn usage_from_value_gemini_shape() {
    let v = json!({
        "usageMetadata": {
            "promptTokenCount": 80,
            "candidatesTokenCount": 40,
            "totalTokenCount": 120
        }
    });
    let usage = usage_from_value(&v).expect("should extract gemini usage");
    assert_eq!(usage.prompt_tokens, Some(80));
}

#[test]
fn usage_from_value_anthropic_shape() {
    let v = json!({
        "usage": {
            "input_tokens": 60,
            "output_tokens": 30
        }
    });
    let usage = usage_from_value(&v).expect("anthropic usage");
    assert_eq!(usage.prompt_tokens, Some(60));
    assert_eq!(usage.completion_tokens, Some(30));
}

#[test]
fn usage_from_value_openai_responses_details() {
    let v = json!({
        "response": {
            "usage": {
                "input_tokens": 100,
                "input_tokens_details": { "cached_tokens": 40 },
                "output_tokens": 50,
                "output_tokens_details": { "reasoning_tokens": 20 },
                "total_tokens": 150
            }
        }
    });
    let usage = usage_from_value(&v).expect("responses usage");
    assert_eq!(usage.prompt_tokens, Some(100));
    assert_eq!(usage.completion_tokens, Some(50));
    assert_eq!(usage.reasoning_tokens, Some(20));
    assert_eq!(usage.cached_prompt_tokens, Some(40));
}

#[test]
fn usage_from_value_returns_none_when_absent() {
    let v = json!({"data": "ok"});
    assert!(usage_from_value(&v).is_none());
}

#[test]
fn usage_from_value_handles_string_encoded_numbers() {
    let v = json!({
        "usage": {
            "prompt_tokens": "100",
            "completion_tokens": "50"
        }
    });
    let _ = usage_from_value(&v);
}

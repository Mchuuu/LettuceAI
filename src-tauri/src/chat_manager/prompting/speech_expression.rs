use crate::chat_manager::types::SystemPromptEntry;

pub const SPEECH_EXPRESSION_ENTRY_ID: &str = "entry_speech_expression_protocol";
const OPEN_TAG: &str = "<voice_exp>";
const CLOSE_TAG: &str = "</voice_exp>";
const MAX_CONTEXT_CHARS: usize = 200;

pub struct ParsedSpeechExpression {
    pub content: String,
    pub context_text: Option<String>,
}

pub fn remove_protocol(mut entries: Vec<SystemPromptEntry>) -> Vec<SystemPromptEntry> {
    entries.retain(|entry| entry.id != SPEECH_EXPRESSION_ENTRY_ID);
    entries
}

pub fn parse(content: &str) -> ParsedSpeechExpression {
    let mut visible = String::with_capacity(content.len());
    let mut remaining = content;
    let mut context_text = None;

    while let Some(open_index) = remaining.find(OPEN_TAG) {
        visible.push_str(&remaining[..open_index]);
        let tagged = &remaining[open_index + OPEN_TAG.len()..];
        let Some(close_index) = tagged.find(CLOSE_TAG) else {
            remaining = "";
            break;
        };

        if let Some(normalized) = normalize_context_text(&tagged[..close_index]) {
            context_text = Some(normalized);
        }
        remaining = &tagged[close_index + CLOSE_TAG.len()..];
    }
    visible.push_str(remaining);

    ParsedSpeechExpression {
        content: visible.trim().to_string(),
        context_text,
    }
}

fn normalize_context_text(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return None;
    }
    Some(normalized.chars().take(MAX_CONTEXT_CHARS).collect())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn extracts_voice_expression_without_leaking_tag() {
        let parsed = parse(
            "怎么，现在才知道听话呀？\n<voice_exp>请用成熟慵懒、略带戏谑的御姐音，语速稍慢。</voice_exp>",
        );
        assert_eq!(parsed.content, "怎么，现在才知道听话呀？");
        assert_eq!(
            parsed.context_text.as_deref(),
            Some("请用成熟慵懒、略带戏谑的御姐音，语速稍慢。")
        );
    }

    #[test]
    fn preserves_other_output_protocols() {
        let parsed =
            parse("正文<voice_exp>请用四川话，语气轻快。</voice_exp><img>scene prompt</img>");
        assert_eq!(parsed.content, "正文<img>scene prompt</img>");
    }

    #[test]
    fn removes_unclosed_protocol_tail() {
        let parsed = parse("正文<voice_exp>请用悲伤的语气");
        assert_eq!(parsed.content, "正文");
        assert!(parsed.context_text.is_none());
    }
}

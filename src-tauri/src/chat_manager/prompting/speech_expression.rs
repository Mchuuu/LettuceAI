use crate::chat_manager::types::{
    Character, PromptEntryPosition, PromptEntryRole, SystemPromptEntry,
};

pub const SPEECH_EXPRESSION_ENTRY_ID: &str = "entry_speech_expression_protocol";
const OPEN_TAG: &str = "<voice_exp>";
const CLOSE_TAG: &str = "</voice_exp>";
const MAX_CONTEXT_CHARS: usize = 200;

pub struct ParsedSpeechExpression {
    pub content: String,
    pub context_text: Option<String>,
}

pub fn protocol_entry() -> SystemPromptEntry {
    SystemPromptEntry {
        id: SPEECH_EXPRESSION_ENTRY_ID.to_string(),
        name: "语音表现力协议".to_string(),
        role: PromptEntryRole::System,
        content: r#"# 豆包 TTS 语音表现力
完成正常回复正文后，必须在回复最末尾追加一条中文语音指令，格式严格如下：
<voice_exp>请用符合本轮回复的具体语气朗读。</voice_exp>

语音指令用于指导 TTS 如何演绎正文，不会展示给用户，也不是角色说出的台词。

规则：
- 结合角色性格、当前情境、对话上下文和本轮正文，给出一句自然、明确、可执行的中文指令。
- 优先描述最能提升本轮表现力的要素，可包含：整体情绪（如悲伤、生气、开心、害羞）、方言或口音（如四川话、北京话）、语气与表演风格（如撒娇、暧昧、争吵、夹子音、御姐音）、语速快慢、音调高低、音量、停顿或气声。
- 只选择当前回复确实需要的要素，并说明合理的程度，例如“略带”“明显”“压抑着”“逐渐加快”；不要机械地罗列全部维度。
- 方言或特殊声线仅在角色设定或当前情境明确需要时使用，不要凭空添加。
- 指令应增强角色一致性和情绪层次，避免过度表演；普通场景也要给出自然、贴合角色的读法。
- 指令保持精炼，建议不超过 80 个汉字；不要复述正文，不要解释标签，不要使用代码块。
- 每次回复只输出一组完整标签，并且必须放在所有可见正文之后。"#
            .to_string(),
        enabled: true,
        injection_position: PromptEntryPosition::Relative,
        injection_depth: 0,
        conditional_min_messages: None,
        interval_turns: None,
        system_prompt: false,
        conditions: None,
        prompt_entry_payload: None,
    }
}

pub fn is_enabled(character: &Character) -> bool {
    let Some(config) = character
        .voice_config
        .as_ref()
        .and_then(|value| value.as_object())
    else {
        return false;
    };
    if config.get("source").and_then(|value| value.as_str()) != Some("provider") {
        return false;
    }
    let Some(settings) = config
        .get("doubaoVoiceSettings")
        .and_then(|value| value.as_object())
    else {
        return false;
    };
    settings
        .get("speechExpressionEnabled")
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

pub fn apply_protocol(
    mut entries: Vec<SystemPromptEntry>,
    character: &Character,
) -> Vec<SystemPromptEntry> {
    if !is_enabled(character) {
        entries.retain(|entry| entry.id != SPEECH_EXPRESSION_ENTRY_ID);
        return entries;
    }
    if !entries
        .iter()
        .any(|entry| entry.id == SPEECH_EXPRESSION_ENTRY_ID)
    {
        entries.push(protocol_entry());
    }
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

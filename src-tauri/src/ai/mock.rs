use crate::ai::{ChatMessage, ChatOutcome, ChatRequest, ToolCall};
use crate::error::Result;
use crate::tools::ToolSink;
use serde_json::{json, Value};

/// Offline provider used for development, UI verification and tests — no key,
/// no network. It is deliberately simple but exercises the real tool loop:
///
/// * `USE_TOOL:<name> {json args}` → asks to call that tool
/// * `echo: ...` → streams the rest back
/// * anything else → a helpful canned reply that explains the backend is live.
pub async fn chat(req: &ChatRequest<'_>, sink: &ToolSink) -> Result<ChatOutcome> {
    let last_user = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .and_then(|m| m.content.clone())
        .unwrap_or_default();

    if let Some(rest) = last_user.trim().strip_prefix("USE_TOOL:") {
        let rest = rest.trim();
        let (name, args_raw) = match rest.find('{') {
            Some(idx) => (rest[..idx].trim().to_string(), rest[idx..].trim().to_string()),
            None => (rest.to_string(), "{}".to_string()),
        };
        let arguments = if args_raw.is_empty() { "{}".to_string() } else { args_raw };
        let call = ToolCall {
            id: format!("mock_call_{}", short_id()),
            name,
            arguments,
        };
        return Ok(ChatOutcome {
            content: String::new(),
            tool_calls: vec![call],
            finish_reason: "tool_calls".into(),
            model: "mock-1".into(),
            usage: None,
        });
    }

    // A tool result is the most recent message → summarize it.
    if let Some(last) = req.messages.last() {
        if last.role == "tool" {
            let body = last.content.clone().unwrap_or_default();
            let summary = format!(
                "Here is what that returned:\n\n{}\n\n(Offline test provider — connect a real model in Settings → AI.)",
                truncate(&body, 800)
            );
            stream_words(&summary, sink);
            return Ok(ChatOutcome {
                content: summary,
                tool_calls: vec![],
                finish_reason: "stop".into(),
                model: "mock-1".into(),
                usage: None,
            });
        }
    }

    let text = if let Some(rest) = last_user.trim().strip_prefix("echo:") {
        rest.trim().to_string()
    } else if last_user.trim().is_empty() {
        "I'm running in offline test mode. Set an API key in Settings → AI to talk to a real model.".to_string()
    } else {
        format!(
            "I heard you say: \"{}\".\n\nI'm the offline test provider — the whole app (chat, tools, permissions, voice) is working, but no real model is connected yet.",
            truncate(last_user.trim(), 400)
        )
    };
    stream_words(&text, sink);
    let _ = ChatMessage::system("");
    Ok(ChatOutcome {
        content: text,
        tool_calls: vec![],
        finish_reason: "stop".into(),
        model: "mock-1".into(),
        usage: Some(json!({"prompt_tokens": 0, "completion_tokens": 0})),
    })
}

fn stream_words(text: &str, sink: &ToolSink) {
    for word in text.split_inclusive(' ') {
        sink.emit_token(word);
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n).collect();
        out.push('…');
        out
    }
}

fn short_id() -> String {
    let full = uuid::Uuid::new_v4().to_string();
    full.chars().take(8).collect()
}

#[allow(dead_code)]
fn _unused(_v: Value) {}

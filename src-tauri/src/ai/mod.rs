pub mod mock;
pub mod stream;

use crate::error::{AppError, Result};
use crate::settings::AiSettings;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    Image { mime: String, data_base64: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<ContentPart>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(text: impl Into<String>) -> Self {
        ChatMessage { role: "system".into(), content: Some(text.into()), parts: vec![], tool_calls: vec![], tool_call_id: None }
    }
    pub fn user(text: impl Into<String>) -> Self {
        ChatMessage { role: "user".into(), content: Some(text.into()), parts: vec![], tool_calls: vec![], tool_call_id: None }
    }
    pub fn user_with_image(text: impl Into<String>, mime: String, data_base64: String) -> Self {
        ChatMessage {
            role: "user".into(),
            content: Some(text.into()),
            parts: vec![ContentPart::Image { mime, data_base64 }],
            tool_calls: vec![],
            tool_call_id: None,
        }
    }
    pub fn assistant_tool_calls(calls: Vec<ToolCall>) -> Self {
        ChatMessage { role: "assistant".into(), content: None, parts: vec![], tool_calls: calls, tool_call_id: None }
    }
    pub fn tool_result(id: impl Into<String>, text: impl Into<String>) -> Self {
        ChatMessage {
            role: "tool".into(),
            content: Some(text.into()),
            parts: vec![],
            tool_calls: vec![],
            tool_call_id: Some(id.into()),
        }
    }

    pub fn to_openai(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("role".into(), json!(self.role));
        let has_image = self.parts.iter().any(|p| matches!(p, ContentPart::Image { .. }));
        if has_image {
            let mut content: Vec<Value> = Vec::new();
            if let Some(text) = &self.content {
                if !text.is_empty() {
                    content.push(json!({"type": "text", "text": text}));
                }
            }
            for part in &self.parts {
                if let ContentPart::Image { mime, data_base64 } = part {
                    content.push(json!({
                        "type": "image_url",
                        "image_url": {"url": format!("data:{mime};base64,{data_base64}")}
                    }));
                }
            }
            obj.insert("content".into(), Value::Array(content));
        } else if let Some(text) = &self.content {
            obj.insert("content".into(), json!(text));
        } else if self.tool_calls.is_empty() {
            obj.insert("content".into(), json!(""));
        }
        if !self.tool_calls.is_empty() {
            obj.insert(
                "tool_calls".into(),
                Value::Array(
                    self.tool_calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": {"name": c.name, "arguments": c.arguments}
                            })
                        })
                        .collect(),
                ),
            );
        }
        if let Some(id) = &self.tool_call_id {
            obj.insert("tool_call_id".into(), json!(id));
        }
        Value::Object(obj)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct ChatOutcome {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: String,
    pub model: String,
    pub usage: Option<Value>,
}

pub struct ChatRequest<'a> {
    pub cfg: &'a AiSettings,
    pub messages: &'a [ChatMessage],
    pub tools: Vec<Value>,
    pub api_key: Option<String>,
    pub stream: bool,
}

pub fn api_key_account(provider: &str) -> String {
    format!("api_key/{provider}")
}

/// Resolve the effective URL endpoint for a provider+base_url.
pub(super) fn endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

fn is_local_url(base_url: &str) -> bool {
    let lower = base_url.to_lowercase();
    lower.contains("localhost") || lower.contains("127.0.0.1") || lower.contains("::1")
}

/// Run a chat completion, optionally streaming tokens into `sink`.
pub async fn chat(
    req: ChatRequest<'_>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    sink: &super::tools::ToolSink,
) -> Result<ChatOutcome> {
    match req.cfg.provider.as_str() {
        "mock" => mock::chat(&req, sink).await,
        "groq" | "openai" | "openai_compat" | "ollama" | "custom" => {
            stream::chat_streaming(req, cancel, sink).await
        }
        other => Err(AppError::Precondition(format!("unknown ai provider '{other}'"))),
    }
}

/// Non-streaming helper used by vision + title generation.
pub async fn chat_once(req: ChatRequest<'_>, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<ChatOutcome> {
    let sink = super::tools::ToolSink::none("vision");
    let mut req = req;
    req.stream = false;
    chat(req, cancel, &sink).await
}

pub struct ProviderProbe {
    pub ok: bool,
    pub models: Vec<String>,
    pub message: String,
}

pub async fn probe(cfg: &AiSettings) -> ProviderProbe {
    if cfg.provider == "mock" {
        return ProviderProbe { ok: true, models: vec!["mock-1".into()], message: "Offline test provider is active.".into() };
    }
    let key = crate::credentials::get(&api_key_account(&cfg.provider)).ok().flatten();
    if key.is_none() && !is_local_url(&cfg.base_url) {
        return ProviderProbe {
            ok: false,
            models: vec![],
            message: "No API key stored for this provider yet. Add one in Settings \u{2192} AI.".into(),
        };
    }
    let url = format!("{}/models", cfg.base_url.trim().trim_end_matches('/'));
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build() {
        Ok(c) => c,
        Err(e) => return ProviderProbe { ok: false, models: vec![], message: format!("client error: {e}") },
    };
    let mut rb = client.get(&url);
    if let Some(k) = &key {
        rb = rb.bearer_auth(k);
    }
    match rb.send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return ProviderProbe {
                    ok: false,
                    models: vec![],
                    message: format!("provider returned {status}: {}", truncate(&body, 300)),
                };
            }
            let models: Vec<String> = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("data").and_then(|d| d.as_array()).cloned())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            ProviderProbe {
                ok: true,
                message: format!("Connected. {} models available.", models.len()),
                models,
            }
        }
        Err(e) => ProviderProbe {
            ok: false,
            models: vec![],
            message: format!("could not reach provider: {e}"),
        },
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n).collect();
        format!("{cut}\u{2026}")
    }
}

pub fn parse_tool_arguments(raw: &str) -> Value {
    if raw.trim().is_empty() {
        return json!({});
    }
    serde_json::from_str(raw).unwrap_or_else(|_| json!({ "_raw": raw }))
}

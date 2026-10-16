use crate::ai::{ChatOutcome, ChatRequest, ToolCall};
use crate::error::{AppError, Result};
use crate::tools::ToolSink;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub async fn chat_streaming(
    req: ChatRequest<'_>,
    cancel: Arc<AtomicBool>,
    sink: &ToolSink,
) -> Result<ChatOutcome> {
    let cfg = req.cfg;
    let url = endpoint_for(cfg);
    let mut api_key = req.api_key.clone();
    if api_key.is_none() {
        api_key = crate::credentials::get(&crate::ai::api_key_account(&cfg.provider))
            .ok()
            .flatten();
    }
    if api_key.is_none() && !is_local_url(&cfg.base_url) {
        return Err(AppError::Precondition(format!(
            "No API key configured for provider '{}'. Add one in Settings → AI.",
            cfg.provider
        )));
    }

    let mut body = json!({
        "model": cfg.chat_model,
        "messages": req.messages.iter().map(|m| m.to_openai_value()).collect::<Vec<_>>(),
        "temperature": cfg.temperature,
        "max_tokens": cfg.max_tokens,
        "stream": true,
    });
    if !req.tools.is_empty() {
        body["tools"] = Value::Array(req.tools.clone());
        body["tool_choice"] = json!("auto");
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_sec.max(10)))
        .build()
        .map_err(|e| AppError::msg(format!("http client: {e}")))?;

    let mut rb = client.post(&url).json(&body);
    if let Some(k) = &api_key {
        rb = rb.bearer_auth(k);
    }
    let resp = rb.send().await.map_err(|e| {
        if e.is_timeout() {
            AppError::Timeout(format!("provider did not respond within {}s", cfg.timeout_sec))
        } else {
            AppError::Provider(format!("request failed: {e}"))
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.as_u16() == 429 {
            return Err(AppError::Provider(format!(
                "rate limited by provider (429). Try again shortly. {}",
                truncate(&text, 300)
            )));
        }
        return Err(AppError::Provider(format!(
            "provider returned {status}: {}",
            truncate(&text, 500)
        )));
    }

    let mut acc_content = String::new();
    // index -> (id, name, arguments)
    let mut tool_acc: std::collections::BTreeMap<u64, (String, String, String)> =
        std::collections::BTreeMap::new();
    let mut finish = "stop".to_string();
    let mut usage: Option<Value> = None;

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            return Err(AppError::Denied("generation cancelled".into()));
        }
        let chunk = chunk.map_err(|e| AppError::Provider(format!("stream error: {e}")))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);
            let Some(data) = line.strip_prefix("data:") else { continue };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(parsed) = serde_json::from_str::<Value>(data) else { continue };
            if let Some(u) = parsed.get("usage") {
                if !u.is_null() {
                    usage = Some(u.clone());
                }
            }
            let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) else { continue };
            let Some(choice) = choices.first() else { continue };
            if let Some(fr) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                finish = fr.to_string();
            }
            let Some(delta) = choice.get("delta") else { continue };
            if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                if !text.is_empty() {
                    acc_content.push_str(text);
                    sink.emit_token(text);
                }
            }
            if let Some(calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                for c in calls {
                    let idx = c.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                    let entry = tool_acc.entry(idx).or_insert_with(|| (String::new(), String::new(), String::new()));
                    if let Some(id) = c.get("id").and_then(|i| i.as_str()) {
                        if !id.is_empty() {
                            entry.0 = id.to_string();
                        }
                    }
                    if let Some(f) = c.get("function") {
                        if let Some(n) = f.get("name").and_then(|n| n.as_str()) {
                            entry.1.push_str(n);
                        }
                        if let Some(a) = f.get("arguments").and_then(|a| a.as_str()) {
                            entry.2.push_str(a);
                        }
                    }
                }
            }
        }
    }

    let tool_calls: Vec<ToolCall> = tool_acc
        .into_values()
        .enumerate()
        .map(|(i, (id, name, args))| ToolCall {
            id: if id.is_empty() { format!("call_{i}") } else { id },
            name,
            arguments: args,
        })
        .filter(|c| !c.name.is_empty())
        .collect();

    if !tool_calls.is_empty() && finish == "stop" {
        finish = "tool_calls".into();
    }

    Ok(ChatOutcome {
        content: acc_content,
        tool_calls,
        finish_reason: finish,
        model: cfg.chat_model.clone(),
        usage,
    })
}

impl crate::ai::ChatMessage {
    pub fn to_openai_value(&self) -> Value {
        self.to_openai()
    }
}

fn endpoint_for(cfg: &crate::settings::AiSettings) -> String {
    super::endpoint(&cfg.base_url)
}

fn is_local_url(base_url: &str) -> bool {
    let l = base_url.to_lowercase();
    l.contains("localhost") || l.contains("127.0.0.1") || l.contains("::1")
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}


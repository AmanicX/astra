use crate::error::{AppError, Result};
use crate::tools::ExecutionOutput;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Web search via DuckDuckGo's lite HTML endpoint (no API keys, no tracking
/// beyond the request itself). Results are parsed defensively.
pub async fn search(query: &str, count: u32, cancel: Arc<AtomicBool>) -> Result<ExecutionOutput> {
    let query = query.trim();
    if query.is_empty() {
        return Err(AppError::Precondition("empty search query".into()));
    }
    let client = client()?;
    let resp = client
        .post("https://lite.duckduckgo.com/lite/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("q={}", urlencode(query)))
        .send()
        .await
        .map_err(|e| AppError::msg(format!("search request failed: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::msg(format!("search returned {}", resp.status())));
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(AppError::Denied("search cancelled".into()));
    }
    let html = resp.text().await.map_err(|e| AppError::msg(format!("search body: {e}")))?;
    let results = parse_ddg_lite(&html, count.max(1) as usize);
    let text = if results.is_empty() {
        format!("No results for \"{query}\".")
    } else {
        results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}\n   {}\n   {}", i + 1, r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(ExecutionOutput {
        text,
        data: Some(json!({
            "query": query,
            "results": results.iter().map(|r| json!({"title": r.title, "url": r.url, "snippet": r.snippet})).collect::<Vec<_>>(),
        })),
    })
}

pub async fn fetch(url: &str, max_chars: u32, cancel: Arc<AtomicBool>) -> Result<ExecutionOutput> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(AppError::Precondition("only http/https URLs can be fetched".into()));
    }
    let client = client()?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::msg(format!("fetch failed: {e}")))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(AppError::msg(format!("page returned {status}")));
    }
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp.text().await.map_err(|e| AppError::msg(format!("read body: {e}")))?;
    if cancel.load(Ordering::SeqCst) {
        return Err(AppError::Denied("fetch cancelled".into()));
    }

    let text = if ctype.contains("html") || body.trim_start().starts_with('<') {
        html_to_text(&body)
    } else {
        body.clone()
    };
    let limit = max_chars.clamp(500, 200_000) as usize;
    let clipped: String = text.chars().take(limit).collect();
    Ok(ExecutionOutput {
        text: format!("Fetched {url} ({status}).\n\n{clipped}"),
        data: Some(json!({"url": url, "status": status.as_u16(), "content_type": ctype, "text": clipped})),
    })
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (compatible; Astra/0.1; +local assistant)")
        .build()
        .map_err(|e| AppError::msg(format!("client: {e}")))
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_ddg_lite(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut out = Vec::new();
    // Result rows are <a rel="nofollow" href="...">title</a> followed by a snippet cell.
    let mut rest = html;
    while out.len() < limit {
        let Some(anchor_start) = rest.find("<a ") else { break };
        let after = &rest[anchor_start..];
        let Some(href_idx) = after.find("href=\"") else { break };
        let href_val_start = href_idx + 6;
        let Some(href_end) = after[href_val_start..].find('"') else { break };
        let href = &after[href_val_start..href_val_start + href_end];
        let Some(open_end) = after.find('>') else { break };
        let Some(close) = after[open_end..].find("</a>") else { break };
        let title_html = &after[open_end + 1..open_end + close];
        let title = strip_tags(title_html);
        let remainder = &after[open_end + close + 4..];
        let snippet = extract_snippet(remainder);
        if href.starts_with("http") && !title.is_empty() {
            out.push(SearchResult {
                title,
                url: href.to_string(),
                snippet,
            });
        }
        rest = remainder;
    }
    out
}

fn extract_snippet(html_after_anchor: &str) -> String {
    let Some(cell_start) = html_after_anchor.find("result-snippet") else { return String::new() };
    let window = &html_after_anchor[cell_start..];
    let Some(gt) = window.find('>') else { return String::new() };
    let Some(end) = window[gt..].find("</td>") else { return String::new() };
    strip_tags(&window[gt + 1..gt + end])
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(out.trim())
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 100).unwrap_or_default()
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

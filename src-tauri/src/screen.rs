use crate::error::{AppError, Result};
use crate::settings::Settings;
use crate::tools::ExecutionOutput;
use base64::Engine;
use serde::Serialize;
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Captured {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub monitor: usize,
    pub captured_at: String,
}

static LAST_CAPTURE: Mutex<Option<Captured>> = Mutex::new(None);

fn store(cap: Captured) {
    *LAST_CAPTURE.lock().unwrap() = Some(cap);
}

pub fn last_capture() -> Option<Captured> {
    LAST_CAPTURE.lock().unwrap().clone()
}

#[derive(Serialize)]
pub struct CaptureResult {
    pub monitor: usize,
    pub width: u32,
    pub height: u32,
    pub captured_at: String,
    pub data_url: String,
    pub saved_to: Option<String>,
}

pub async fn capture_tool(
    monitor: Option<usize>,
    save: bool,
    data_dir: &Path,
) -> Result<ExecutionOutput> {
    let result = capture(monitor, save, data_dir).await?;
    Ok(ExecutionOutput {
        text: format!(
            "Captured monitor {} ({}×{}) at {}.",
            result.monitor, result.width, result.height, result.captured_at
        ),
        data: Some(serde_json::json!({
            "monitor": result.monitor,
            "width": result.width,
            "height": result.height,
            "captured_at": result.captured_at,
            "saved_to": result.saved_to,
        })),
    })
}

pub async fn capture(
    monitor: Option<usize>,
    save: bool,
    data_dir: &Path,
) -> Result<CaptureResult> {
    let monitor_idx = monitor.unwrap_or(0);
    let dir = data_dir.to_path_buf();
    let (png, width, height) = tokio::task::spawn_blocking(move || grab_png(monitor_idx))
        .await
        .map_err(|e| AppError::msg(format!("capture task: {e}")))??;

    let captured_at = chrono::Utc::now().to_rfc3339();
    let saved_to = if save {
        let dir = dir.join("screenshots");
        std::fs::create_dir_all(&dir).ok();
        let file = dir.join(format!(
            "shot-{}.png",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        std::fs::write(&file, &png).ok();
        Some(file.to_string_lossy().to_string())
    } else {
        None
    };

    let cap = Captured {
        png: png.clone(),
        width,
        height,
        monitor: monitor_idx,
        captured_at: captured_at.clone(),
    };
    store(cap);

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Ok(CaptureResult {
        monitor: monitor_idx,
        width,
        height,
        captured_at,
        data_url: format!("data:image/png;base64,{b64}"),
        saved_to,
    })
}

fn grab_png(monitor_idx: usize) -> Result<(Vec<u8>, u32, u32)> {
    use xcap::Monitor;
    let monitors = Monitor::all().map_err(|e| AppError::msg(format!("enumerate monitors: {e}")))?;
    if monitors.is_empty() {
        return Err(AppError::msg("no monitors found"));
    }
    let monitor = monitors
        .get(monitor_idx)
        .or_else(|| monitors.first())
        .ok_or_else(|| AppError::msg("monitor index out of range"))?;
    let image = monitor
        .capture_image()
        .map_err(|e| AppError::msg(format!("capture screen: {e}")))?;
    let width = image.width();
    let height = image.height();
    let mut buf = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| AppError::msg(format!("encode png: {e}")))?;
    Ok((buf, width, height))
}

pub async fn vision_tool(
    _app: &tauri::AppHandle,
    settings: &Settings,
    question: &str,
    cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<ExecutionOutput> {
    if settings.privacy.local_only {
        return Err(AppError::Denied(
            "Local-only mode is on, so screenshots are never sent to a cloud model. Turn it off in Settings → Privacy to use vision analysis.".into(),
        ));
    }
    if !settings.privacy.allow_screen_to_cloud {
        return Err(AppError::Denied(
            "Sending screens to the cloud model is disabled. Enable 'Allow screen to cloud' in Settings → Privacy, or attach the image directly in chat.".into(),
        ));
    }
    let cap = last_capture().ok_or_else(|| {
        AppError::Precondition("no screenshot yet — capture the screen first".into())
    })?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&cap.png);
    let mut cfg = settings.ai.clone();
    cfg.chat_model = settings.ai.vision_model.clone();
    cfg.max_tokens = cfg.max_tokens.min(2048);

    let messages = vec![
        crate::ai::ChatMessage::system(
            "You are Astra's vision assistant. Describe what is relevant to the user's question, precisely and without inventing details you cannot see.",
        ),
        crate::ai::ChatMessage::user_with_image(question, "image/png".into(), b64),
    ];
    let req = crate::ai::ChatRequest {
        cfg: &cfg,
        messages: &messages,
        tools: vec![],
        api_key: None,
        stream: false,
    };
    let outcome = crate::ai::chat_once(req, cancel.clone()).await?;
    Ok(ExecutionOutput {
        text: outcome.content.clone(),
        data: Some(serde_json::json!({
            "answer": outcome.content,
            "model": outcome.model,
            "monitor": cap.monitor,
            "width": cap.width,
            "height": cap.height,
            "captured_at": cap.captured_at,
        })),
    })
}

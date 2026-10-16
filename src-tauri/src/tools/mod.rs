pub mod apps;
pub mod audio;
pub mod files;
pub mod mem;
pub mod net;
pub mod shell;
pub mod sys;

use crate::error::{AppError, Result};
use crate::permissions::{PermissionEngine, Risk};
use crate::settings::Settings;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Streams incremental tool output to the UI. Cloned freely.
#[derive(Clone)]
pub struct ToolSink {
    chan: Option<tauri::ipc::Channel<crate::turn::TurnEvent>>,
    tool_id: String,
    turn_id: String,
}

impl ToolSink {
    pub fn new(
        chan: tauri::ipc::Channel<crate::turn::TurnEvent>,
        turn_id: String,
        tool_id: String,
    ) -> Self {
        ToolSink { chan: Some(chan), tool_id, turn_id }
    }
    pub fn none(tool_id: impl Into<String>) -> Self {
        ToolSink { chan: None, tool_id: tool_id.into(), turn_id: String::new() }
    }
    pub fn emit_output(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(c) = &self.chan {
            use crate::turn::TurnEvent;
            let _ = c.send(TurnEvent::ToolOutput {
                tool_id: self.tool_id.clone(),
                text: text.to_string(),
            });
        }
    }
    pub fn emit_token(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(c) = &self.chan {
            use crate::turn::TurnEvent;
            let _ = c.send(TurnEvent::Token {
                turn_id: self.turn_id.clone(),
                text: text.to_string(),
            });
        }
    }
    pub fn tool_id(&self) -> &str {
        &self.tool_id
    }
}

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    pub why: &'static str,
    pub params: Value,
}

pub struct ToolCtx<'a> {
    pub app: &'a tauri::AppHandle,
    pub state: &'a AppState,
    pub settings: &'a Settings,
    pub cancel: Arc<AtomicBool>,
}

pub fn catalog() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "system_info",
            description: "OS, CPU, memory, battery and uptime snapshot of this machine.",
            risk: Risk::Low,
            why: "Reads hardware and OS information (no personal files).",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "time_info",
            description: "Current local date/time, weekday and timezone.",
            risk: Risk::Low,
            why: "Reads the system clock.",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "app_discover",
            description: "List installed applications (Start Menu + installed programs).",
            risk: Risk::Low,
            why: "Lists installed programs (names and paths, no files are read).",
            params: json!({"type":"object","properties":{"query":{"type":"string"}},"additionalProperties":false}),
        },
        ToolDef {
            name: "app_launch",
            description: "Open an installed app, file or URL.",
            risk: Risk::Low,
            why: "Opens an application the user can see (URLs open in the default browser).",
            params: json!({"type":"object","properties":{"target":{"type":"string"}},"required":["target"],"additionalProperties":false}),
        },
        ToolDef {
            name: "audio_get",
            description: "Current system volume percentage and mute state.",
            risk: Risk::Low,
            why: "Reads the current volume and mute state.",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "audio_set",
            description: "Set system volume (0-100).",
            risk: Risk::Moderate,
            why: "Changes the system volume (audible, easily reversible).",
            params: json!({"type":"object","properties":{"level":{"type":"number","minimum":0,"maximum":100}},"required":["level"],"additionalProperties":false}),
        },
        ToolDef {
            name: "audio_mute",
            description: "Mute or unmute the system audio.",
            risk: Risk::Moderate,
            why: "Toggles system mute (audible, easily reversible).",
            params: json!({"type":"object","properties":{"mute":{"type":"boolean"}},"required":["mute"],"additionalProperties":false}),
        },
        ToolDef {
            name: "brightness_get",
            description: "Current display brightness percentage (laptops).",
            risk: Risk::Low,
            why: "Reads display brightness.",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "brightness_set",
            description: "Set display brightness (0-100).",
            risk: Risk::Moderate,
            why: "Changes display brightness (reversible).",
            params: json!({"type":"object","properties":{"level":{"type":"number","minimum":0,"maximum":100}},"required":["level"],"additionalProperties":false}),
        },
        ToolDef {
            name: "wifi_status",
            description: "Wi-Fi radio state, connected SSID, signal and IP info.",
            risk: Risk::Low,
            why: "Reads network status (SSID and signal, no traffic).",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "wifi_set",
            description: "Turn Wi-Fi radio on/off, or connect to a saved network profile by name.",
            risk: Risk::Moderate,
            why: "Changes network connectivity (may drop the AI's own connection).",
            params: json!({"type":"object","properties":{"action":{"type":"string","enum":["on","off","connect"]},"ssid":{"type":"string"}},"required":["action"],"additionalProperties":false}),
        },
        ToolDef {
            name: "wifi_scan",
            description: "Scan for nearby Wi-Fi networks.",
            risk: Risk::Moderate,
            why: "Performs an active Wi-Fi scan (reveals nearby network names).",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "bluetooth_status",
            description: "Bluetooth adapter radio state.",
            risk: Risk::Low,
            why: "Reads Bluetooth radio state.",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "bluetooth_set",
            description: "Turn the Bluetooth radio on or off.",
            risk: Risk::Moderate,
            why: "Toggles the Bluetooth radio (affects connected devices).",
            params: json!({"type":"object","properties":{"on":{"type":"boolean"}},"required":["on"],"additionalProperties":false}),
        },
        ToolDef {
            name: "wallpaper_get",
            description: "Current desktop wallpaper file path.",
            risk: Risk::Low,
            why: "Reads the current wallpaper path.",
            params: json!({"type":"object","properties":{},"additionalProperties":false}),
        },
        ToolDef {
            name: "wallpaper_set",
            description: "Set the desktop wallpaper to an image file. Keeps a backup of the previous wallpaper.",
            risk: Risk::Moderate,
            why: "Changes the desktop wallpaper (previous wallpaper is backed up and restorable).",
            params: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        },
        ToolDef {
            name: "file_list",
            description: "List files and folders in a directory.",
            risk: Risk::Low,
            why: "Lists names in a directory (does not read file contents).",
            params: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        },
        ToolDef {
            name: "file_read",
            description: "Read a text file. Respects size limits; refuses binary files.",
            risk: Risk::Low,
            why: "Reads one text file the user asked about.",
            params: json!({"type":"object","properties":{"path":{"type":"string"},"max_bytes":{"type":"integer"}},"required":["path"],"additionalProperties":false}),
        },
        ToolDef {
            name: "file_write",
            description: "Write text content to a file. Overwriting an existing file requires approval.",
            risk: Risk::Moderate,
            why: "Writes or overwrites a file on disk (a backup is kept when replacing).",
            params: json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"],"additionalProperties":false}),
        },
        ToolDef {
            name: "file_mkdir",
            description: "Create a directory (and parents).",
            risk: Risk::Moderate,
            why: "Creates new folders on disk.",
            params: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        },
        ToolDef {
            name: "file_delete",
            description: "Delete a file. Moves it to Astra's trash folder instead of permanent deletion.",
            risk: Risk::High,
            why: "Deletes a file (moved to Astra's recoverable trash, not permanent).",
            params: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        },
        ToolDef {
            name: "shell_run",
            description: "Run a shell command and return its output. Prefer dedicated tools first.",
            risk: Risk::High,
            why: "Runs an arbitrary shell command with full user privileges.",
            params: json!({"type":"object","properties":{"command":{"type":"string"},"cwd":{"type":"string"}},"required":["command"],"additionalProperties":false}),
        },
        ToolDef {
            name: "web_search",
            description: "Search the web (DuckDuckGo) and get titles, URLs and snippets.",
            risk: Risk::Low,
            why: "Fetches public web search results.",
            params: json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}),
        },
        ToolDef {
            name: "web_fetch",
            description: "Fetch a public URL and return its readable text content.",
            risk: Risk::Low,
            why: "Downloads one public web page (URL must be http/https).",
            params: json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"],"additionalProperties":false}),
        },
        ToolDef {
            name: "screen_capture",
            description: "Capture the screen (or a monitor) as a PNG image for vision analysis.",
            risk: Risk::Moderate,
            why: "Takes a screenshot of what is currently on screen.",
            params: json!({"type":"object","properties":{"monitor":{"type":"integer"}},"additionalProperties":false}),
        },
        ToolDef {
            name: "vision_analyze",
            description: "Analyze the most recent screenshot with the vision model (sends it to the configured cloud provider unless local-only mode forbids it).",
            risk: Risk::Moderate,
            why: "Sends a screenshot to the AI provider for analysis.",
            params: json!({"type":"object","properties":{"question":{"type":"string"}},"required":["question"],"additionalProperties":false}),
        },
        ToolDef {
            name: "memory_read",
            description: "Read stored memories (facts and preferences about the user).",
            risk: Risk::Low,
            why: "Reads Astra's own memory notes.",
            params: json!({"type":"object","properties":{"category":{"type":"string"}},"additionalProperties":false}),
        },
        ToolDef {
            name: "memory_write",
            description: "Persist a memory (fact or preference) about the user or their work.",
            risk: Risk::Moderate,
            why: "Saves a persistent memory note about the user.",
            params: json!({"type":"object","properties":{"category":{"type":"string"},"key":{"type":"string"},"value":{"type":"string"}},"required":["category","key","value"],"additionalProperties":false}),
        },
        ToolDef {
            name: "memory_delete",
            description: "Delete a stored memory by id.",
            risk: Risk::Moderate,
            why: "Removes a saved memory note.",
            params: json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}),
        },
        ToolDef {
            name: "coding_agent_run",
            description: "Run an installed coding agent (opencode, claude, or a configured CLI) with a prompt in a project directory. Streams its output.",
            risk: Risk::High,
            why: "Runs a coding agent that can modify files and execute commands in a project directory.",
            params: json!({"type":"object","properties":{
                "agent":{"type":"string"},
                "prompt":{"type":"string"},
                "cwd":{"type":"string"}
            },"required":["prompt"],"additionalProperties":false}),
        },
    ]
}

pub fn lookup(name: &str) -> Result<&'static ToolDef> {
    static CATALOG: std::sync::OnceLock<Vec<ToolDef>> = std::sync::OnceLock::new();
    let catalog: &'static Vec<ToolDef> = CATALOG.get_or_init(catalog);
    catalog
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| AppError::NotFound(format!("tool {name}")))
}

/// Risk can escalate at execution time (e.g. overwriting an existing file).
pub fn dynamic_risk(def: &ToolDef, args: &Value, ctx_paths: &Path) -> Risk {
    let mut risk = def.risk;
    match def.name {
        "file_write" => {
            if let Some(p) = args.get("path").and_then(|v| v.as_str()) {
                if Path::new(p).exists() {
                    return Risk::High;
                }
            }
            // Writing outside the home folder is more consequential.
            if let Some(p) = args.get("path").and_then(|v| v.as_str()) {
                if !is_inside(p, ctx_paths) {
                    return Risk::High;
                }
            }
        }
        "file_read" | "file_list" | "file_delete" => {
            if let Some(p) = args.get("path").and_then(|v| v.as_str()) {
                if !is_inside(p, ctx_paths) {
                    risk = Risk::High;
                }
            }
        }
        "app_launch" => {
            // Launching an arbitrary executable is not "open a known app".
            if let Some(t) = args.get("target").and_then(|v| v.as_str()) {
                let lower = t.to_lowercase();
                let benign = lower.starts_with("http://")
                    || lower.starts_with("https://")
                    || lower.contains("\\microsoft\\windows\\start menu\\")
                    || !lower.ends_with(".exe");
                if !benign {
                    return Risk::High;
                }
            }
        }
        _ => {}
    }
    risk
}

fn is_inside(path_str: &str, root: &Path) -> bool {
    let p = PathBuf::from(path_str);
    match p.canonicalize() {
        Ok(c) => match root.canonicalize() {
            Ok(r) => c.starts_with(&r),
            Err(_) => false,
        },
        Err(_) => match p.strip_prefix(root) {
            Ok(_) => true,
            Err(_) => p.is_absolute() == false || path_str.starts_with(root.to_string_lossy().as_ref()),
        },
    }
}

/// Resolve + validate a user/model supplied path. Rejects traversal tricks.
pub fn resolve_path(raw: &str) -> Result<PathBuf> {
    if raw.trim().is_empty() {
        return Err(AppError::Precondition("empty path".into()));
    }
    if raw.contains('\0') {
        return Err(AppError::Precondition("invalid path".into()));
    }
    let p = PathBuf::from(raw);
    // Reject `..` components before any filesystem access.
    for c in p.components() {
        if let std::path::Component::ParentDir = c {
            return Err(AppError::Precondition("path traversal ('..') is not allowed".into()));
        }
    }
    Ok(p)
}

pub fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub struct ExecutionOutput {
    pub text: String,
    pub data: Option<Value>,
}

pub async fn execute(
    ctx: &ToolCtx<'_>,
    name: &str,
    args: Value,
    sink: &ToolSink,
) -> Result<ExecutionOutput> {
    let def = lookup(name)?;
    let home = home_dir();
    let risk = dynamic_risk(def, &args, &home);
    let auth = ctx
        .state
        .permissions
        .authorize(name, risk, ctx.settings, def.why);
    if auth.requires_approval {
        return Err(AppError::Denied(format!(
            "{name} requires approval: {}",
            auth.reason
        )));
    }
    run_impl(ctx, name, &args, sink, def).await
}

/// Run a tool whose permission gate already passed (approved by the user
/// inside `turn.rs`). Still validates args the same way.
pub async fn execute_approved(
    ctx: &ToolCtx<'_>,
    name: &str,
    args: Value,
    sink: &ToolSink,
) -> Result<ExecutionOutput> {
    let def = lookup(name)?;
    run_impl(ctx, name, &args, sink, def).await
}

async fn run_impl(
    ctx: &ToolCtx<'_>,
    name: &str,
    args: &Value,
    sink: &ToolSink,
    _def: &ToolDef,
) -> Result<ExecutionOutput> {
    match name {
        "system_info" => sys::system_info(),
        "time_info" => sys::time_info(),
        "app_discover" => apps::discover(args.get("query").and_then(|v| v.as_str())),
        "app_launch" => apps::launch(args.get("target").and_then(|v| v.as_str()).unwrap_or("")),
        "audio_get" => audio::get(),
        "audio_set" => audio::set_level(args.get("level").and_then(|v| v.as_f64()).unwrap_or(50.0).clamp(0.0, 100.0) as u8),
        "audio_mute" => audio::set_mute(args.get("mute").and_then(|v| v.as_bool()).unwrap_or(true)),
        "brightness_get" => audio::brightness_get(),
        "brightness_set" => audio::brightness_set(args.get("level").and_then(|v| v.as_f64()).unwrap_or(50.0).clamp(0.0, 100.0) as u8),
        "wifi_status" => net::wifi_status(),
        "wifi_set" => net::wifi_set(
            args.get("action").and_then(|v| v.as_str()).unwrap_or(""),
            args.get("ssid").and_then(|v| v.as_str()),
        ).await,
        "wifi_scan" => net::wifi_scan().await,
        "bluetooth_status" => net::bt_status(),
        "bluetooth_set" => net::bt_set(args.get("on").and_then(|v| v.as_bool()).unwrap_or(true)).await,
        "wallpaper_get" => audio::wallpaper_get(),
        "wallpaper_set" => audio::wallpaper_set(args.get("path").and_then(|v| v.as_str()).unwrap_or("")),
        "file_list" => files::list(args.get("path").and_then(|v| v.as_str()).unwrap_or(".")),
        "file_read" => files::read(
            args.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            args.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(64 * 1024) as usize,
        ),
        "file_write" => files::write(
            args.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            args.get("content").and_then(|v| v.as_str()).unwrap_or(""),
            &ctx.state.data_dir,
        ),
        "file_mkdir" => files::mkdir(args.get("path").and_then(|v| v.as_str()).unwrap_or("")),
        "file_delete" => files::delete(
            args.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            &ctx.state.data_dir,
        ),
        "shell_run" => {
            shell::run(
                args.get("command").and_then(|v| v.as_str()).unwrap_or(""),
                args.get("cwd").and_then(|v| v.as_str()),
                &ctx.cancel,
                sink,
            )
            .await
        }
        "web_search" => crate::research::search(
            args.get("query").and_then(|v| v.as_str()).unwrap_or(""),
            ctx.settings.research.results_count,
            ctx.cancel.clone(),
        ).await,
        "web_fetch" => crate::research::fetch(
            args.get("url").and_then(|v| v.as_str()).unwrap_or(""),
            ctx.settings.research.max_page_chars,
            ctx.cancel.clone(),
        ).await,
        "screen_capture" => crate::screen::capture_tool(
            args.get("monitor").and_then(|v| v.as_i64()).map(|m| m as usize),
            ctx.settings.privacy.save_screenshots,
            &ctx.state.data_dir,
        ).await,
        "vision_analyze" => {
            crate::screen::vision_tool(
                ctx.app,
                ctx.settings,
                args.get("question").and_then(|v| v.as_str()).unwrap_or("Describe this screenshot."),
                &ctx.cancel,
            )
            .await
        }
        "memory_read" => mem::read(
            ctx.state,
            args.get("category").and_then(|v| v.as_str()),
        ),
        "memory_write" => mem::write(
            ctx.state,
            args.get("category").and_then(|v| v.as_str()).unwrap_or("general"),
            args.get("key").and_then(|v| v.as_str()).unwrap_or(""),
            args.get("value").and_then(|v| v.as_str()).unwrap_or(""),
        ),
        "memory_delete" => mem::delete(ctx.state, args.get("id").and_then(|v| v.as_str()).unwrap_or("")),
        "coding_agent_run" => crate::agents::run_as_tool(
            ctx,
            args.get("agent").and_then(|v| v.as_str()),
            args.get("prompt").and_then(|v| v.as_str()).unwrap_or(""),
            args.get("cwd").and_then(|v| v.as_str()),
            sink,
        ).await,
        other => Err(AppError::NotFound(format!("tool {other}"))),
    }
}

/// JSON tool definitions for the model, in OpenAI function-calling format.
pub fn openai_tool_schemas() -> Vec<Value> {
    catalog()
        .into_iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.params,
                }
            })
        })
        .collect()
}

/// Re-check used by the permission dialog UI before showing a request.
pub fn tool_why(name: &str) -> String {
    lookup(name).map(|t| t.why.to_string()).unwrap_or_else(|_| name.to_string())
}

pub fn engine_used(settings: &Settings, engine: &PermissionEngine, tool: &str, risk: Risk) -> bool {
    engine.authorize(tool, risk, settings, "").allowed
}

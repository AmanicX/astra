use crate::ai::{self, ChatMessage, ToolCall};
use crate::error::{AppError, Result};
use crate::permissions::{Authorization, Risk};
use crate::settings::Settings;
use crate::state::{ActivityItem, AppState, PendingApproval};
use crate::tools::{self, ToolCtx, ToolSink};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;

const MAX_STEPS: usize = 8;
const HISTORY_TURNS: u32 = 24;
const APPROVAL_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnEvent {
    TurnStarted {
        turn_id: String,
        conversation_id: String,
    },
    Token {
        turn_id: String,
        text: String,
    },
    AssistantMessage {
        turn_id: String,
        message: crate::db::Message,
    },
    ToolStarted {
        turn_id: String,
        tool_id: String,
        tool: String,
        label: String,
        args: Value,
    },
    ToolOutput {
        tool_id: String,
        text: String,
    },
    ToolFinished {
        turn_id: String,
        tool_id: String,
        tool: String,
        status: String,
        summary: String,
        data: Option<Value>,
    },
    ApprovalRequest {
        approval_id: String,
        turn_id: String,
        tool: String,
        tool_id: String,
        risk: String,
        why: String,
        args: Value,
    },
    ApprovalResolved {
        approval_id: String,
        approved: bool,
    },
    TurnFinished {
        turn_id: String,
        status: String,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatAttachment {
    pub mime: String,
    pub data_base64: String,
}

pub struct TurnSpec {
    pub conversation_id: String,
    pub text: String,
    pub images: Vec<ChatAttachment>,
}

pub struct TurnRunner {
    pub app: tauri::AppHandle,
    pub turn_id: String,
    pub cancel: Arc<AtomicBool>,
    pub chan: Channel<TurnEvent>,
}

impl TurnRunner {
    fn emit(&self, event: TurnEvent) {
        let _ = self.chan.send(event);
    }

    fn check_cancel(&self) -> Result<()> {
        if self.cancel.load(Ordering::SeqCst) {
            Err(AppError::Denied("cancelled".into()))
        } else {
            Ok(())
        }
    }
}

pub async fn run_turn(
    app: tauri::AppHandle,
    state: &AppState,
    chan: Channel<TurnEvent>,
    spec: TurnSpec,
    cancel: Arc<AtomicBool>,
    turn_id: String,
) {
    let runner = TurnRunner {
        app: app.clone(),
        turn_id: turn_id.clone(),
        cancel: cancel.clone(),
        chan: chan.clone(),
    };
    runner.emit(TurnEvent::TurnStarted {
        turn_id: turn_id.clone(),
        conversation_id: spec.conversation_id.clone(),
    });

    let outcome = run_turn_inner(&runner, state, &spec).await;

    {
        let mut active = state.active_turns.lock().unwrap();
        active.remove(&turn_id);
    }

    match outcome {
        Ok(()) => runner.emit(TurnEvent::TurnFinished {
            turn_id,
            status: "completed".into(),
            error: None,
        }),
        Err(e) => {
            let msg = e.to_string();
            let status = if matches!(e, AppError::Denied(_)) { "cancelled" } else { "failed" };
            runner.emit(TurnEvent::TurnFinished {
                turn_id,
                status: status.into(),
                error: Some(msg),
            });
        }
    }
}

async fn run_turn_inner(runner: &TurnRunner, state: &AppState, spec: &TurnSpec) -> Result<()> {
    let settings = state.get_settings();
    let turn_id = runner.turn_id.clone();

    // Persist the incoming user message.
    let user_meta = if spec.images.is_empty() {
        json!({})
    } else {
        json!({"images": spec.images.len()})
    };
    {
        let db = state.db.lock().unwrap();
        db.get_conversation(&spec.conversation_id)?;
        db.insert_message(&spec.conversation_id, "user", &spec.text, &user_meta)?;
    }

    let mut messages = build_messages(state, &settings, &spec.conversation_id, spec)?;
    let tool_schemas = tools::openai_tool_schemas();
    let correction_budget = 2usize;
    let mut corrections = 0usize;

    for _step in 0..MAX_STEPS {
        runner.check_cancel()?;

        let messages_snapshot = messages.clone();
        let chat_req = ai::ChatRequest {
            cfg: &settings.ai,
            messages: &messages_snapshot,
            tools: tool_schemas.clone(),
            api_key: None,
            stream: true,
        };
        // Token streaming goes through a tool-less sink that carries turn_id.
        let stream_sink = ToolSink::new(runner.chan.clone(), turn_id.clone(), String::new());
        let outcome = ai::chat(chat_req, runner.cancel.clone(), &stream_sink).await;

        let outcome = match outcome {
            Ok(o) => o,
            Err(e) => return Err(e),
        };
        log::debug!(
            "turn {}: finish={} model={} tool_calls={}",
            turn_id,
            outcome.finish_reason,
            outcome.model,
            outcome.tool_calls.len()
        );

        // Record the assistant turn (content + any tool calls) for the model.
        if !outcome.content.trim().is_empty() || !outcome.tool_calls.is_empty() {
            if outcome.content.trim().is_empty() {
                messages.push(ChatMessage::assistant_tool_calls(outcome.tool_calls.clone()));
            } else {
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: Some(outcome.content.clone()),
                    parts: vec![],
                    tool_calls: outcome.tool_calls.clone(),
                    tool_call_id: None,
                });
            }
        }

        if outcome.tool_calls.is_empty() {
            // Validate: an answer that claims tool use but has no calls, and
            // no tools were ever run this turn, gets one gentle correction.
            if corrections < correction_budget && looks_like_false_claim(&outcome.content, &messages) {
                corrections += 1;
                messages.push(ChatMessage::user(
                    "Reminder: you claimed to perform an action but did not call a tool. If an action is needed, call the tool now. Otherwise, answer directly and honestly.",
                ));
                continue;
            }
            finalize_assistant_message(state, runner, &spec.conversation_id, &outcome.content, &outcome.usage)?;
            return Ok(());
        }

        // Execute each tool call, gating through the permission engine.
        let assistant_calls = outcome.tool_calls.clone();
        let mut tool_results: Vec<ChatMessage> = Vec::new();
        for call in &assistant_calls {
            runner.check_cancel()?;
            let (result_text, tool_message) =
                execute_tool_call(runner, state, &settings, call).await;
            tool_results.push(tool_message);
            let _ = result_text;
        }
        messages.extend(tool_results);
    }

    // Ran out of steps.
    let fallback = "I reached the limit of actions I can chain in one turn. Here's where things stand — tell me how you'd like to continue.";
    finalize_assistant_message(state, runner, &spec.conversation_id, fallback, &None)?;
    Ok(())
}

async fn execute_tool_call(
    runner: &TurnRunner,
    state: &AppState,
    settings: &Settings,
    call: &ToolCall,
) -> (String, ChatMessage) {
    let tool_id = uuid::Uuid::new_v4().to_string();
    let args = ai::parse_tool_arguments(&call.arguments);

    let def = match tools::lookup(&call.name) {
        Ok(d) => d,
        Err(_) => {
            let msg = format!("Unknown tool '{}'.", call.name);
            runner.emit(TurnEvent::ToolFinished {
                turn_id: runner.turn_id.clone(),
                tool_id: tool_id.clone(),
                tool: call.name.clone(),
                status: "failed".into(),
                summary: msg.clone(),
                data: None,
            });
            return (msg.clone(), ChatMessage::tool_result(call.id.clone(), msg));
        }
    };

    let home = tools::home_dir();
    let risk = tools::dynamic_risk(def, &args, &home);
    let why = def.why.to_string();

    runner.emit(TurnEvent::ToolStarted {
        turn_id: runner.turn_id.clone(),
        tool_id: tool_id.clone(),
        tool: call.name.clone(),
        label: humanize_tool(&call.name),
        args: args.clone(),
    });

    let auth: Authorization = state.permissions.authorize(&call.name, risk, settings, &why);
    let mut approved_by_user = false;
    if auth.requires_approval {
        let approved = request_approval(runner, state, &call.name, &tool_id, risk, &why, &args).await;
        if !approved {
            let msg = format!("The user declined to run {}.", call.name);
            runner.emit(TurnEvent::ToolFinished {
                turn_id: runner.turn_id.clone(),
                tool_id: tool_id.clone(),
                tool: call.name.clone(),
                status: "denied".into(),
                summary: "Declined by user.".into(),
                data: None,
            });
            push_activity(
                state,
                ActivityItem {
                    id: tool_id,
                    turn_id: runner.turn_id.clone(),
                    tool: call.name.clone(),
                    label: humanize_tool(&call.name),
                    status: "denied".into(),
                    detail: None,
                    started_at: chrono::Utc::now().to_rfc3339(),
                    ended_at: Some(chrono::Utc::now().to_rfc3339()),
                },
            );
            return (msg.clone(), ChatMessage::tool_result(call.id.clone(), msg));
        }
        approved_by_user = true;
        state.permissions.approve_session(&call.name, risk);
    }

    push_activity(
        state,
        ActivityItem {
            id: tool_id.clone(),
            turn_id: runner.turn_id.clone(),
            tool: call.name.clone(),
            label: humanize_tool(&call.name),
            status: "running".into(),
            detail: None,
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
        },
    );

    let sink = ToolSink::new(runner.chan.clone(), runner.turn_id.clone(), tool_id.clone());
    let ctx = ToolCtx {
        app: &runner.app,
        state,
        settings,
        cancel: runner.cancel.clone(),
    };

    let result = if approved_by_user {
        tools::execute_approved(&ctx, &call.name, args, &sink).await
    } else {
        tools::execute(&ctx, &call.name, args, &sink).await
    };
    log::debug!(
        "tool {} [{}] -> {}",
        call.name,
        sink.tool_id(),
        if result.is_ok() { "ok" } else { "failed" }
    );

    match result {
        Ok(out) => {
            push_activity(
                state,
                ActivityItem {
                    id: tool_id.clone(),
                    turn_id: runner.turn_id.clone(),
                    tool: call.name.clone(),
                    label: humanize_tool(&call.name),
                    status: "done".into(),
                    detail: Some(truncate(&out.text, 160)),
                    started_at: chrono::Utc::now().to_rfc3339(),
                    ended_at: Some(chrono::Utc::now().to_rfc3339()),
                },
            );
            runner.emit(TurnEvent::ToolFinished {
                turn_id: runner.turn_id.clone(),
                tool_id,
                tool: call.name.clone(),
                status: "done".into(),
                summary: truncate(&out.text, 400),
                data: out.data.clone(),
            });
            let payload = serde_json::to_string(&json!({
                "ok": true,
                "result": out.text,
                "data": out.data,
            }))
            .unwrap_or_else(|_| out.text.clone());
            (out.text.clone(), ChatMessage::tool_result(call.id.clone(), payload))
        }
        Err(e) => {
            let msg = e.to_string();
            push_activity(
                state,
                ActivityItem {
                    id: tool_id.clone(),
                    turn_id: runner.turn_id.clone(),
                    tool: call.name.clone(),
                    label: humanize_tool(&call.name),
                    status: "failed".into(),
                    detail: Some(msg.clone()),
                    started_at: chrono::Utc::now().to_rfc3339(),
                    ended_at: Some(chrono::Utc::now().to_rfc3339()),
                },
            );
            runner.emit(TurnEvent::ToolFinished {
                turn_id: runner.turn_id.clone(),
                tool_id,
                tool: call.name.clone(),
                status: "failed".into(),
                summary: msg.clone(),
                data: None,
            });
            let payload = serde_json::to_string(&json!({"ok": false, "error": msg}))
                .unwrap_or_else(|_| msg.clone());
            (msg.clone(), ChatMessage::tool_result(call.id.clone(), payload))
        }
    }
}

async fn request_approval(
    runner: &TurnRunner,
    state: &AppState,
    tool: &str,
    tool_id: &str,
    risk: Risk,
    why: &str,
    args: &Value,
) -> bool {
    let approval_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut approvals = state.approvals.lock().unwrap();
        approvals.insert(
            approval_id.clone(),
            PendingApproval {
                tool: tool.to_string(),
                sender: tx,
            },
        );
    }
    runner.emit(TurnEvent::ApprovalRequest {
        approval_id: approval_id.clone(),
        turn_id: runner.turn_id.clone(),
        tool: tool.to_string(),
        tool_id: tool_id.to_string(),
        risk: risk_str(risk).into(),
        why: why.to_string(),
        args: args.clone(),
    });

    let wait = tokio::time::timeout(std::time::Duration::from_secs(APPROVAL_TIMEOUT_SECS), rx).await;
    let approved = match wait {
        Ok(Ok(v)) => v,
        Ok(Err(_)) => false,
        Err(_) => false, // timed out → deny
    };
    {
        let mut approvals = state.approvals.lock().unwrap();
        approvals.remove(&approval_id);
    }
    if !approved {
        runner.emit(TurnEvent::ApprovalResolved {
            approval_id,
            approved: false,
        });
    }
    approved
}

pub fn risk_str(risk: Risk) -> &'static str {
    match risk {
        Risk::Low => "low",
        Risk::Moderate => "moderate",
        Risk::High => "high",
    }
}

fn finalize_assistant_message(
    state: &AppState,
    runner: &TurnRunner,
    conversation_id: &str,
    content: &str,
    usage: &Option<Value>,
) -> Result<()> {
    let meta = usage
        .as_ref()
        .map(|u| json!({"usage": u}))
        .unwrap_or_else(|| json!({}));
    let message = {
        let db = state.db.lock().unwrap();
        db.insert_message(conversation_id, "assistant", content, &meta)?
    };
    runner.emit(TurnEvent::AssistantMessage {
        turn_id: runner.turn_id.clone(),
        message,
    });
    Ok(())
}

fn build_messages(
    state: &AppState,
    settings: &Settings,
    conversation_id: &str,
    spec: &TurnSpec,
) -> Result<Vec<ChatMessage>> {
    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::system(build_system_prompt(state, settings)));

    let history = {
        let db = state.db.lock().unwrap();
        db.recent_history(conversation_id, HISTORY_TURNS)?
    };
    // All but the just-saved user message (added below with images).
    let mut history = history;
    if history.last().map(|m| m.role.as_str()) == Some("user") {
        history.pop();
    }
    for m in history {
        match m.role.as_str() {
            "user" => messages.push(ChatMessage::user(m.content)),
            "assistant" => messages.push(ChatMessage {
                role: "assistant".into(),
                content: Some(m.content),
                parts: vec![],
                tool_calls: vec![],
                tool_call_id: None,
            }),
            _ => {}
        }
    }

    let mut user = ChatMessage::user(spec.text.clone());
    for img in &spec.images {
        user.parts.push(ai::ContentPart::Image {
            mime: img.mime.clone(),
            data_base64: img.data_base64.clone(),
        });
    }
    messages.push(user);
    Ok(messages)
}

fn build_system_prompt(state: &AppState, settings: &Settings) -> String {
    let a = &settings.assistant;
    let mut s = String::new();
    s.push_str(&format!(
        "You are {}, a desktop AI assistant running locally on the user's {} machine.\n",
        a.name,
        std::env::consts::OS
    ));
    s.push_str("You are an agent: you can act on this computer through tools, not just talk.\n\n");

    s.push_str("## Style\n");
    s.push_str(&format!(
        "- Personality: {}. Tone: {}.\n- Formality: {}/100, conciseness: {}/100, humor: {}/100.\n",
        a.personality, a.tone, a.formality, a.conciseness, a.humor
    ));
    s.push_str(&format!("- Explanation depth: {}.\n", a.explanation_depth));
    s.push_str(&format!("- Reply in language code '{}' unless the user switches.\n", a.language));
    if a.proactive {
        s.push_str("- Be proactive: anticipate the next useful step and offer it briefly.\n");
    }
    if a.conciseness >= 60 {
        s.push_str("- Prefer short, direct answers. No filler, no restating the question.\n");
    }

    s.push_str("\n## Acting on the computer\n");
    s.push_str("- Use tools to actually do things (volume, apps, files, network, screenshots, web, memories, coding agents).\n");
    s.push_str("- Prefer the most specific tool over shell_run. Never run a destructive shell command without a clear reason the user asked for.\n");
    s.push_str("- Every consequential action is gated by a permission dialog. Explain plainly what you are about to do before you do it.\n");
    s.push_str("- NEVER claim you performed an action unless a tool actually returned success. If a tool is denied, say so plainly.\n");
    s.push_str("- If a request is ambiguous or risky, ask one clarifying question instead of guessing.\n");
    s.push_str("- When you show results, be concrete (paths, values, names) and keep them readable.\n");

    if settings.memory.enabled && settings.memory.inject_preferences {
        let memories = {
            let db = state.db.lock().unwrap();
            db.list_memories(None, settings.memory.max_injected.min(50)).unwrap_or_default()
        };
        if !memories.is_empty() {
            s.push_str("\n## What you remember about the user\n");
            for m in memories {
                s.push_str(&format!("- [{}] {} = {}\n", m.category, m.key, m.value));
            }
            s.push_str("Use these naturally; update them with memory_write when you learn something durable.\n");
        }
    }

    s.push_str(&format!(
        "\n## Environment\n- Local time: {}\n- OS: {}\n",
        chrono::Local::now().format("%A %Y-%m-%d %H:%M"),
        std::env::consts::OS
    ));
    if settings.privacy.local_only {
        s.push_str("- PRIVACY: local-only mode is ON. Do not send screen or files to cloud services; say so if asked to.\n");
    }

    if !a.custom_system_prompt.trim().is_empty() {
        s.push_str("\n## User instructions\n");
        s.push_str(a.custom_system_prompt.trim());
        s.push('\n');
    }
    s
}

fn looks_like_false_claim(content: &str, messages: &[ChatMessage]) -> bool {
    let ran_tool = messages.iter().any(|m| m.role == "tool");
    if ran_tool {
        return false;
    }
    let c = content.to_lowercase();
    let claims = [
        "i've set", "i have set", "i set the", "i've changed", "i have changed", "i changed the",
        "i've opened", "i have opened", "i opened", "i've turned", "i have turned", "i turned",
        "i've created", "i have created", "i created", "i've deleted", "i have deleted",
        "i deleted", "i've run", "i have run", "i ran the", "done —", "done -", "i've captured",
        "i captured", "i've muted", "i muted",
    ];
    claims.iter().any(|p| c.contains(p))
}

fn truncate(s: &str, n: usize) -> String {
    let clean = s.trim();
    if clean.chars().count() <= n {
        clean.to_string()
    } else {
        let mut out: String = clean.chars().take(n).collect();
        out.push('…');
        out
    }
}

pub fn humanize_tool(name: &str) -> String {
    match name {
        "system_info" => "Reading system information",
        "time_info" => "Checking the clock",
        "app_discover" => "Looking through installed apps",
        "app_launch" => "Opening an application",
        "audio_get" => "Reading the volume",
        "audio_set" => "Changing the volume",
        "audio_mute" => "Muting audio",
        "brightness_get" => "Reading brightness",
        "brightness_set" => "Changing brightness",
        "wifi_status" => "Checking Wi-Fi",
        "wifi_set" => "Changing Wi-Fi",
        "wifi_scan" => "Scanning for networks",
        "bluetooth_status" => "Checking Bluetooth",
        "bluetooth_set" => "Toggling Bluetooth",
        "wallpaper_get" => "Reading the wallpaper",
        "wallpaper_set" => "Changing the wallpaper",
        "file_list" => "Listing a folder",
        "file_read" => "Reading a file",
        "file_write" => "Writing a file",
        "file_mkdir" => "Creating a folder",
        "file_delete" => "Deleting a file",
        "shell_run" => "Running a command",
        "web_search" => "Searching the web",
        "web_fetch" => "Reading a web page",
        "screen_capture" => "Taking a screenshot",
        "vision_analyze" => "Looking at the screen",
        "memory_read" => "Reading memories",
        "memory_write" => "Saving a memory",
        "memory_delete" => "Removing a memory",
        "coding_agent_run" => "Delegating to a coding agent",
        other => other,
    }
    .to_string()
}

fn push_activity(state: &AppState, item: ActivityItem) {
    state.push_activity(item);
}

/// Called by the `approval_respond` command.
pub fn resolve_approval(state: &AppState, approval_id: &str, approved: bool) -> Result<()> {
    let pending = {
        let mut approvals = state.approvals.lock().unwrap();
        approvals.remove(approval_id)
    };
    match pending {
        Some(p) => {
            let _ = p.sender.send(approved);
            Ok(())
        }
        None => Err(AppError::NotFound(format!("approval {approval_id}"))),
    }
}

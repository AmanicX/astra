use crate::ai;
use crate::db::{Conversation, Message};
use crate::error::{AppError, Result};
use crate::settings::Settings;
use crate::state::{ActivityItem, AppState};
use crate::turn::{self, ChatAttachment, TurnEvent, TurnSpec};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

// ---------------- settings & credentials ----------------

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.get_settings()
}

#[tauri::command]
pub fn settings_set(state: State<'_, AppState>, settings: Settings) -> Result<Settings> {
    state.save_settings(settings)
}

#[tauri::command]
pub fn settings_reset(state: State<'_, AppState>) -> Result<Settings> {
    state.save_settings(Settings::default())
}

#[tauri::command]
pub fn credential_set(state: State<'_, AppState>, account: String, secret: String) -> Result<()> {
    let _ = &state;
    crate::credentials::set(&account, &secret)
}

#[tauri::command]
pub fn credential_has(state: State<'_, AppState>, account: String) -> bool {
    let _ = &state;
    crate::credentials::has(&account)
}

#[tauri::command]
pub fn credential_delete(state: State<'_, AppState>, account: String) -> Result<()> {
    let _ = &state;
    crate::credentials::delete(&account)
}

#[derive(Serialize)]
pub struct ProviderProbeResult {
    ok: bool,
    models: Vec<String>,
    message: String,
}

#[tauri::command]
pub async fn provider_test(state: State<'_, AppState>, provider: Option<String>) -> Result<ProviderProbeResult> {
    let mut cfg = state.get_settings().ai;
    if let Some(p) = provider {
        cfg.provider = p;
    }
    let probe = ai::probe(&cfg).await;
    Ok(ProviderProbeResult {
        ok: probe.ok,
        models: probe.models,
        message: probe.message,
    })
}

// ---------------- conversations & memory ----------------

#[tauri::command]
pub fn conversation_list(state: State<'_, AppState>, query: Option<String>) -> Result<Vec<Conversation>> {
    let db = state.db.lock().unwrap();
    db.list_conversations(query.as_deref())
}

#[tauri::command]
pub fn conversation_create(
    state: State<'_, AppState>,
    title: Option<String>,
    project: Option<String>,
) -> Result<Conversation> {
    let db = state.db.lock().unwrap();
    db.create_conversation(title.as_deref().unwrap_or("New conversation"), project.as_deref())
}

#[tauri::command]
pub fn conversation_rename(
    state: State<'_, AppState>,
    id: String,
    title: String,
    pinned: Option<bool>,
) -> Result<()> {
    let db = state.db.lock().unwrap();
    db.rename_conversation(&id, &title, pinned)
}

#[tauri::command]
pub fn conversation_delete(state: State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.lock().unwrap();
    db.delete_conversation(&id)
}

#[tauri::command]
pub fn conversation_messages(
    state: State<'_, AppState>,
    id: String,
    limit: Option<u32>,
) -> Result<Vec<Message>> {
    let db = state.db.lock().unwrap();
    db.messages_for(&id, limit.unwrap_or(500))
}

#[tauri::command]
pub fn message_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<Message>> {
    let db = state.db.lock().unwrap();
    db.search_messages(&query, limit.unwrap_or(50))
}

#[tauri::command]
pub fn memory_list(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<crate::db::Memory>> {
    let db = state.db.lock().unwrap();
    db.list_memories(category.as_deref(), 500)
}

#[tauri::command]
pub fn memory_set(
    state: State<'_, AppState>,
    category: String,
    key: String,
    value: String,
) -> Result<crate::db::Memory> {
    let db = state.db.lock().unwrap();
    db.upsert_memory(&category, &key, &value, Some("user"))
}

#[tauri::command]
pub fn memory_delete(state: State<'_, AppState>, id: String) -> Result<()> {
    let db = state.db.lock().unwrap();
    db.delete_memory(&id)
}

// ---------------- turns ----------------

#[tauri::command]
pub async fn turn_run(
    app: AppHandle,
    state: State<'_, AppState>,
    on_event: Channel<TurnEvent>,
    conversation_id: String,
    text: String,
    images: Option<Vec<ChatAttachment>>,
) -> Result<()> {
    let turn_id = uuid::Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut active = state.active_turns.lock().unwrap();
        active.insert(turn_id.clone(), cancel.clone());
    }

    let spec = TurnSpec {
        conversation_id,
        text,
        images: images.unwrap_or_default(),
    };
    let state_ref: &AppState = state.inner();
    turn::run_turn(app, state_ref, on_event, spec, cancel, turn_id).await;
    Ok(())
}

#[tauri::command]
pub fn turn_cancel(state: State<'_, AppState>, turn_id: String) -> Result<()> {
    let active = state.active_turns.lock().unwrap();
    if let Some(flag) = active.get(&turn_id) {
        flag.store(true, Ordering::SeqCst);
    } else {
        // Cancel everything if the id is unknown.
        for flag in active.values() {
            flag.store(true, Ordering::SeqCst);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn approval_respond(
    state: State<'_, AppState>,
    approval_id: String,
    approved: bool,
    remember: Option<String>,
) -> Result<()> {
    if approved {
        if let Some(mode) = remember {
            let mode = mode.trim().to_lowercase();
            if mode == "always" || mode == "auto" {
                let mut settings = state.get_settings();
                // Look up the tool for this approval to persist an override.
                let tool = state
                    .approvals
                    .lock()
                    .unwrap()
                    .get(&approval_id)
                    .map(|p| p.tool.clone());
                if let Some(tool) = tool {
                    settings
                        .permissions
                        .tool_overrides
                        .retain(|(t, _)| t != &tool);
                    settings.permissions.tool_overrides.push((tool, mode));
                    state.save_settings(settings)?;
                }
            }
        }
    }
    turn::resolve_approval(&state, &approval_id, approved)
}

#[tauri::command]
pub fn activity_list(state: State<'_, AppState>) -> Vec<ActivityItem> {
    state.activity_snapshot()
}

// ---------------- system & tools ----------------

#[tauri::command]
pub fn system_discover(state: State<'_, AppState>) -> Vec<crate::platform::Capability> {
    let _ = &state;
    crate::platform::discover()
}

#[tauri::command]
pub async fn app_discover(state: State<'_, AppState>, query: Option<String>) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(move || crate::tools::apps::discover(query.as_deref()))
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn app_launch(state: State<'_, AppState>, target: String) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(move || crate::tools::apps::launch(&target))
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn audio_get(state: State<'_, AppState>) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(crate::tools::audio::get)
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn audio_set(state: State<'_, AppState>, level: u8) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(move || crate::tools::audio::set_level(level))
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn audio_mute(state: State<'_, AppState>, mute: bool) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(move || crate::tools::audio::set_mute(mute))
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn brightness_get(state: State<'_, AppState>) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(crate::tools::audio::brightness_get)
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn brightness_set(state: State<'_, AppState>, level: u8) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(move || crate::tools::audio::brightness_set(level))
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn network_wifi_status(state: State<'_, AppState>) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(crate::tools::net::wifi_status)
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn network_wifi_set(
    state: State<'_, AppState>,
    action: String,
    ssid: Option<String>,
) -> Result<Value> {
    let _ = &state;
    let out = crate::tools::net::wifi_set(&action, ssid.as_deref()).await?;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn network_bluetooth_status(state: State<'_, AppState>) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(crate::tools::net::bt_status)
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn network_bluetooth_set(state: State<'_, AppState>, on: bool) -> Result<Value> {
    let _ = &state;
    let out = crate::tools::net::bt_set(on).await?;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn wallpaper_get(state: State<'_, AppState>) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(crate::tools::audio::wallpaper_get)
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn wallpaper_set(state: State<'_, AppState>, path: String) -> Result<Value> {
    let _ = &state;
    let out = tokio::task::spawn_blocking(move || crate::tools::audio::wallpaper_set(&path))
        .await
        .map_err(|e| AppError::msg(format!("task: {e}")))??;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub fn tool_catalog() -> Vec<Value> {
    crate::tools::catalog()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "risk": turn::risk_str(t.risk),
                "why": t.why,
            })
        })
        .collect()
}

// ---------------- screen, research, agents ----------------

#[tauri::command]
pub async fn screen_capture(
    state: State<'_, AppState>,
    monitor: Option<usize>,
) -> Result<crate::screen::CaptureResult> {
    let settings = state.get_settings();
    crate::screen::capture(monitor, settings.privacy.save_screenshots, &state.data_dir).await
}

#[tauri::command]
pub async fn research_search(state: State<'_, AppState>, query: String, count: Option<u32>) -> Result<Value> {
    let _ = &state;
    let settings = state.get_settings();
    let cancel = Arc::new(AtomicBool::new(false));
    let out = crate::research::search(&query, count.unwrap_or(settings.research.results_count), cancel).await?;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn research_fetch(state: State<'_, AppState>, url: String) -> Result<Value> {
    let _ = &state;
    let settings = state.get_settings();
    let cancel = Arc::new(AtomicBool::new(false));
    let out = crate::research::fetch(&url, settings.research.max_page_chars, cancel).await?;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub fn agent_discover(state: State<'_, AppState>) -> Vec<crate::agents::AgentInfo> {
    let _ = &state;
    crate::agents::discover()
}

#[tauri::command]
pub async fn agent_run(
    state: State<'_, AppState>,
    on_event: Channel<TurnEvent>,
    agent: Option<String>,
    prompt: String,
    cwd: Option<String>,
) -> Result<Value> {
    let settings = state.get_settings();
    let info = crate::agents::resolve(&settings.agents, agent.as_deref())?;
    let cwd = cwd.unwrap_or_else(|| crate::agents::default_project_dir(&settings.agents));
    let tool_id = uuid::Uuid::new_v4().to_string();
    let sink = crate::tools::ToolSink::new(on_event, "agent-run".into(), tool_id);
    let cancel = Arc::new(AtomicBool::new(false));
    let out = crate::agents::run(&state, &info, &prompt, &cwd, &cancel, &sink).await?;
    Ok(out.data.unwrap_or(Value::Null))
}

#[tauri::command]
pub fn agent_cancel(state: State<'_, AppState>) -> Result<()> {
    let mut children = state.agent_children.lock().unwrap();
    children.clear();
    Ok(())
}

#[tauri::command]
pub fn agent_history(state: State<'_, AppState>) -> Result<Vec<Value>> {
    let db = state.db.lock().unwrap();
    db.list_agent_runs(100)
}

// ---------------- voice ----------------

#[tauri::command]
pub fn tts_voices(state: State<'_, AppState>) -> Vec<crate::voice::VoiceInfo> {
    let mut voices = crate::voice::curated_voices();
    let settings = state.get_settings();
    if settings.speech.custom_voice_enabled && settings.speech.custom_voice_path.is_some() {
        voices.insert(
            0,
            crate::voice::VoiceInfo {
                short_name: "custom".into(),
                locale: "local".into(),
                gender: "custom".into(),
                friendly: format!("Custom — {}", settings.speech.custom_voice_name),
            },
        );
    }
    voices
}

#[derive(Serialize)]
pub struct TtsResult {
    audio_base64: String,
    mime: String,
    engine: String,
    voice: String,
}

#[tauri::command]
pub async fn tts_speak(state: State<'_, AppState>, text: String, voice: Option<String>) -> Result<TtsResult> {
    let mut speech = state.get_settings().speech;
    if let Some(v) = voice {
        if !v.trim().is_empty() {
            speech.voice = v;
        }
    }
    let audio = crate::voice::speak(&text, &speech).await?;
    let b64 = crate::voice::audio_b64(&audio);
    Ok(TtsResult {
        audio_base64: b64,
        mime: audio.mime,
        engine: audio.engine,
        voice: audio.voice,
    })
}

#[tauri::command]
pub async fn stt_transcribe(
    state: State<'_, AppState>,
    audio_base64: String,
    mime: Option<String>,
    filename: Option<String>,
) -> Result<String> {
    let _ = &state;
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_base64.trim())
        .map_err(|e| AppError::msg(format!("bad audio payload: {e}")))?;
    let mime = mime.unwrap_or_else(|| "audio/webm".into());
    let name = filename.unwrap_or_else(|| "audio.webm".into());
    crate::voice::transcribe(bytes, &name, &mime).await
}

// ---------------- window & app ----------------

#[tauri::command]
pub fn overlay_toggle(app: AppHandle) -> Result<()> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| AppError::NotFound("overlay window".into()))?;
    if overlay.is_visible().unwrap_or(false) {
        overlay.hide().map_err(|e| AppError::msg(format!("hide overlay: {e}")))?;
    } else {
        overlay.show().map_err(|e| AppError::msg(format!("show overlay: {e}")))?;
        overlay.set_focus().map_err(|e| AppError::msg(format!("focus overlay: {e}")))?;
    }
    Ok(())
}

#[tauri::command]
pub fn overlay_hide(app: AppHandle) -> Result<()> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay.hide().map_err(|e| AppError::msg(format!("hide overlay: {e}")))?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_minimize(app: AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.minimize().map_err(|e| AppError::msg(format!("minimize: {e}")))?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_toggle_maximize(app: AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_maximized().unwrap_or(false) {
            w.unmaximize().map_err(|e| AppError::msg(format!("unmaximize: {e}")))?;
        } else {
            w.maximize().map_err(|e| AppError::msg(format!("maximize: {e}")))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn window_close(app: AppHandle) -> Result<()> {
    let settings = app.state::<AppState>().get_settings();
    if settings.background.minimize_to_tray {
        if let Some(w) = app.get_webview_window("main") {
            w.hide().map_err(|e| AppError::msg(format!("hide: {e}")))?;
        }
    } else {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
pub fn logs_recent(state: State<'_, AppState>) -> Vec<crate::logging::LogLine> {
    let _ = &state;
    crate::logging::recent()
}

#[tauri::command]
pub fn permissions_clear_session(state: State<'_, AppState>) -> Result<()> {
    state.permissions.clear_session();
    Ok(())
}

/// Why a tool wants approval + whether the engine currently allows it.
#[tauri::command]
pub fn tool_meta(state: State<'_, AppState>, name: String, risk: String) -> Result<Value> {
    let settings = state.get_settings();
    let tier = match risk.trim().to_lowercase().as_str() {
        "high" => crate::permissions::Risk::High,
        "moderate" => crate::permissions::Risk::Moderate,
        _ => crate::permissions::Risk::Low,
    };
    let engine_allows = crate::tools::engine_used(&settings, &state.permissions, &name, tier);
    Ok(json!({
        "why": crate::tools::tool_why(&name),
        "engine_allows": engine_allows,
        "risk": risk,
    }))
}

#[tauri::command]
pub fn remember_decision(
    state: State<'_, AppState>,
    tool: String,
    policy: String,
) -> Result<Settings> {
    let mut settings = state.get_settings();
    let policy = policy.trim().to_lowercase();
    settings.permissions.tool_overrides.retain(|(t, _)| t != &tool);
    if policy == "auto" || policy == "always" {
        settings.permissions.tool_overrides.push((tool, policy));
    }
    state.save_settings(settings)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

// ---------------- shortcuts ----------------

pub fn overlay_shortcut() -> tauri_plugin_global_shortcut::Shortcut {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyA)
}

pub fn ptt_shortcut() -> tauri_plugin_global_shortcut::Shortcut {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space)
}

pub fn stop_speak_shortcut() -> tauri_plugin_global_shortcut::Shortcut {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS)
}

/// Global shortcut handler, installed with the plugin at build time.
pub fn shortcut_handler(
    app: &AppHandle,
    shortcut: &tauri_plugin_global_shortcut::Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    use tauri::Emitter;
    use tauri_plugin_global_shortcut::ShortcutState;

    if shortcut == &overlay_shortcut() && event.state == ShortcutState::Pressed {
        if let Some(overlay) = app.get_webview_window("overlay") {
            let visible = overlay.is_visible().unwrap_or(false);
            if visible {
                let _ = overlay.hide();
            } else {
                let _ = overlay.show();
                let _ = overlay.set_focus();
            }
        }
    } else if shortcut == &ptt_shortcut() {
        let phase = if event.state == ShortcutState::Pressed { "start" } else { "stop" };
        let _ = app.emit("astra://ptt", phase);
    } else if shortcut == &stop_speak_shortcut() && event.state == ShortcutState::Pressed {
        let _ = app.emit("astra://stop-speaking", ());
    }
}

pub fn register_shortcuts(app: &AppHandle) -> Result<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    for sc in [overlay_shortcut(), ptt_shortcut(), stop_speak_shortcut()] {
        if let Err(e) = gs.register(sc) {
            log::warn!("could not register shortcut: {e}");
        }
    }
    Ok(())
}

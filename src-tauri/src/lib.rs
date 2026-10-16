mod agents;
mod ai;
mod commands;
mod credentials;
mod db;
mod error;
mod logging;
mod permissions;
mod platform;
mod research;
mod screen;
mod settings;
mod state;
mod tools;
mod turn;
mod voice;

use state::AppState;
use tauri::Manager;

pub use error::{AppError, Result};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(commands::shortcut_handler)
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState::init(&handle)?;
            app.manage(state);
            let logger = logging::FileLogger::init(&app.path().app_log_dir()?);
            let level = std::env::var("ASTRA_LOG")
                .ok()
                .and_then(|v| match v.trim().to_lowercase().as_str() {
                    "error" => Some(log::LevelFilter::Error),
                    "warn" => Some(log::LevelFilter::Warn),
                    "info" => Some(log::LevelFilter::Info),
                    "debug" => Some(log::LevelFilter::Debug),
                    "trace" => Some(log::LevelFilter::Trace),
                    _ => None,
                })
                .unwrap_or(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                });
            logger.set_level(level);
            commands::register_shortcuts(&handle)?;
            tray::init(&handle)?;
            log::info!("astra started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // settings & credentials
            commands::settings_get,
            commands::settings_set,
            commands::settings_reset,
            commands::credential_set,
            commands::credential_has,
            commands::credential_delete,
            commands::provider_test,
            // conversations & memory
            commands::conversation_list,
            commands::conversation_create,
            commands::conversation_rename,
            commands::conversation_delete,
            commands::conversation_messages,
            commands::message_search,
            commands::memory_list,
            commands::memory_set,
            commands::memory_delete,
            commands::permissions_clear_session,
            commands::tool_meta,
            // turns
            commands::turn_run,
            commands::turn_cancel,
            commands::approval_respond,
            commands::activity_list,
            // system & tools
            commands::system_discover,
            commands::app_discover,
            commands::app_launch,
            commands::audio_get,
            commands::audio_set,
            commands::audio_mute,
            commands::brightness_get,
            commands::brightness_set,
            commands::network_wifi_status,
            commands::network_wifi_set,
            commands::network_bluetooth_status,
            commands::network_bluetooth_set,
            commands::wallpaper_get,
            commands::wallpaper_set,
            commands::tool_catalog,
            // screen, research, agents
            commands::screen_capture,
            commands::research_search,
            commands::research_fetch,
            commands::agent_discover,
            commands::agent_run,
            commands::agent_cancel,
            commands::agent_history,
            // voice
            commands::tts_voices,
            commands::tts_speak,
            commands::stt_transcribe,
            // window & app
            commands::overlay_toggle,
            commands::overlay_hide,
            commands::window_minimize,
            commands::window_toggle_maximize,
            commands::window_close,
            commands::logs_recent,
            commands::remember_decision,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running astra");
}

mod tray {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::{AppHandle, Manager, Wry};

    pub fn init(app: &AppHandle) -> tauri::Result<()> {
        let show = MenuItem::with_id(app, "show", "Show Astra", true, None::<&str>)?;
        let hide = MenuItem::with_id(app, "hide", "Hide to tray", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&show, &hide, &quit])?;

        let mut builder = TrayIconBuilder::with_id("astra-tray")
            .tooltip("Astra")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "show" => show_main(app),
                "hide" => {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_main(tray.app_handle());
                }
            });

        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        let _ = builder.build(app);
        Ok(())
    }

    fn show_main(app: &AppHandle<Wry>) {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
        }
    }
}

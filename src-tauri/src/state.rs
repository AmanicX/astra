use crate::db::Db;
use crate::error::Result;
use crate::permissions::PermissionEngine;
use crate::settings::{Settings, SettingsStore};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// A single pending approval request awaiting the user's decision.
pub struct PendingApproval {
    pub tool: String,
    pub sender: tokio::sync::oneshot::Sender<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActivityItem {
    pub id: String,
    pub turn_id: String,
    pub tool: String,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

pub struct AppState {
    pub db: Mutex<Db>,
    pub settings: Mutex<SettingsStore>,
    pub permissions: PermissionEngine,
    pub approvals: Mutex<HashMap<String, PendingApproval>>,
    pub active_turns: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub activity: Mutex<Vec<ActivityItem>>,
    pub data_dir: PathBuf,
    pub agent_children: Mutex<HashMap<String, u32>>,
}

impl AppState {
    pub fn init(app: &tauri::AppHandle) -> Result<AppState> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir).ok();
        let db = Db::open(&data_dir.join("astra.db"))?;
        let raw = db.get_setting("settings")?;
        let store = SettingsStore::load(raw)?;
        Ok(AppState {
            db: Mutex::new(db),
            settings: Mutex::new(store),
            permissions: PermissionEngine::default(),
            approvals: Mutex::new(HashMap::new()),
            active_turns: Mutex::new(HashMap::new()),
            activity: Mutex::new(Vec::new()),
            data_dir,
            agent_children: Mutex::new(HashMap::new()),
        })
    }

    pub fn get_settings(&self) -> Settings {
        self.settings.lock().unwrap().settings.clone()
    }

    pub fn save_settings(&self, next: Settings) -> Result<Settings> {
        let saved = self.settings.lock().unwrap().update(next)?;
        let json = serde_json::to_string(&saved)
            .map_err(|e| crate::AppError::msg(format!("serialize settings: {e}")))?;
        self.db.lock().unwrap().set_setting("settings", &json)?;
        Ok(saved)
    }

    pub fn push_activity(&self, mut item: ActivityItem) {
        let mut act = self.activity.lock().unwrap();
        if let Some(existing) = act.iter_mut().find(|a| a.id == item.id) {
            existing.status = item.status.clone();
            existing.detail = item.detail.clone();
            if item.ended_at.is_some() {
                existing.ended_at = item.ended_at.take();
            }
            return;
        }
        if act.len() > 200 {
            act.remove(0);
        }
        act.push(item);
    }

    pub fn activity_snapshot(&self) -> Vec<ActivityItem> {
        self.activity.lock().unwrap().clone()
    }
}

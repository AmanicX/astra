use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const SETTINGS_VERSION: u32 = 1;

/// Typed, versioned application settings. Secrets never live here —
/// API keys go through the OS credential store (`credentials.rs`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub assistant: AssistantSettings,
    pub ai: AiSettings,
    pub speech: SpeechSettings,
    pub shortcuts: ShortcutSettings,
    pub permissions: PermissionSettings,
    pub privacy: PrivacySettings,
    pub memory: MemorySettings,
    pub appearance: AppearanceSettings,
    pub background: BackgroundSettings,
    pub agents: AgentSettings,
    pub research: ResearchSettings,
    pub diagnostics: DiagnosticsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AssistantSettings {
    pub name: String,
    pub preset: String,
    pub personality: String,
    pub tone: String,
    pub formality: u32,
    pub conciseness: u32,
    pub humor: u32,
    pub proactive: bool,
    pub explanation_depth: String,
    pub language: String,
    pub custom_system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AiSettings {
    pub provider: String,
    pub base_url: String,
    pub chat_model: String,
    pub vision_model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_sec: u64,
    /// Optional per-role model overrides (planner/designer/…).
    pub role_models: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SpeechSettings {
    pub stt_provider: String,
    pub tts_provider: String,
    pub voice: String,
    pub rate: f32,
    pub volume: f32,
    pub auto_speak: bool,
    pub max_spoken_chars: u32,
    pub wake_word: WakeWordSettings,
    pub input_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WakeWordSettings {
    pub enabled: bool,
    pub engine: String,
    pub energy_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ShortcutSettings {
    pub overlay: String,
    pub push_to_talk: String,
    pub stop_speaking: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PermissionSettings {
    /// Per-tool overrides: tool name -> "auto" | "ask" | "always".
    /// Session-scoped approvals are never persisted (they live in memory).
    pub tool_overrides: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PrivacySettings {
    pub local_only: bool,
    pub allow_screen_to_cloud: bool,
    pub save_screenshots: bool,
    pub save_recordings: bool,
    pub telemetry: bool,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MemorySettings {
    pub enabled: bool,
    pub inject_preferences: bool,
    pub max_injected: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub accent: String,
    pub reduced_motion: String,
    pub font_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BackgroundSettings {
    pub minimize_to_tray: bool,
    pub autostart: bool,
    pub notifications: bool,
    pub quiet_hours_enabled: bool,
    pub quiet_from: String,
    pub quiet_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AgentSettings {
    pub preferred: String,
    pub default_project_dir: String,
    pub extra_agents: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ResearchSettings {
    pub results_count: u32,
    pub max_page_chars: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DiagnosticsSettings {
    pub log_level: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            assistant: AssistantSettings::default(),
            ai: AiSettings::default(),
            speech: SpeechSettings::default(),
            shortcuts: ShortcutSettings::default(),
            permissions: PermissionSettings::default(),
            privacy: PrivacySettings::default(),
            memory: MemorySettings::default(),
            appearance: AppearanceSettings::default(),
            background: BackgroundSettings::default(),
            agents: AgentSettings::default(),
            research: ResearchSettings::default(),
            diagnostics: DiagnosticsSettings::default(),
        }
    }
}

impl Default for AssistantSettings {
    fn default() -> Self {
        AssistantSettings {
            name: "Astra".into(),
            preset: "calm_professional".into(),
            personality: "calm, direct, warm".into(),
            tone: "professional".into(),
            formality: 50,
            conciseness: 65,
            humor: 15,
            proactive: true,
            explanation_depth: "adaptive".into(),
            language: "en".into(),
            custom_system_prompt: String::new(),
        }
    }
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            chat_model: "openai/gpt-oss-20b".into(),
            vision_model: "meta-llama/llama-4-scout-17b-16e-instruct".into(),
            temperature: 0.6,
            max_tokens: 4096,
            timeout_sec: 120,
            role_models: vec![],
        }
    }
}

impl Default for SpeechSettings {
    fn default() -> Self {
        SpeechSettings {
            stt_provider: "groq_whisper".into(),
            tts_provider: "edge".into(),
            voice: "en-US-AriaNeural".into(),
            rate: 1.0,
            volume: 1.0,
            auto_speak: false,
            max_spoken_chars: 700,
            wake_word: WakeWordSettings {
                enabled: false,
                engine: "off".into(),
                energy_threshold: 0.03,
            },
            input_device: None,
        }
    }
}

impl Default for WakeWordSettings {
    fn default() -> Self {
        WakeWordSettings { enabled: false, engine: "off".into(), energy_threshold: 0.03 }
    }
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        ShortcutSettings {
            overlay: "Alt+Shift+A".into(),
            push_to_talk: "CommandOrControl+Shift+Space".into(),
            stop_speaking: "CommandOrControl+Shift+S".into(),
        }
    }
}

impl Default for PermissionSettings {
    fn default() -> Self {
        PermissionSettings { tool_overrides: vec![] }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        PrivacySettings {
            local_only: false,
            allow_screen_to_cloud: true,
            save_screenshots: false,
            save_recordings: false,
            telemetry: false,
            log_level: "info".into(),
        }
    }
}

impl Default for MemorySettings {
    fn default() -> Self {
        MemorySettings { enabled: true, inject_preferences: true, max_injected: 12 }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        AppearanceSettings {
            theme: "dark".into(),
            accent: "vermilion".into(),
            reduced_motion: "system".into(),
            font_scale: 1.0,
        }
    }
}

impl Default for BackgroundSettings {
    fn default() -> Self {
        BackgroundSettings {
            minimize_to_tray: true,
            autostart: false,
            notifications: true,
            quiet_hours_enabled: false,
            quiet_from: "22:00".into(),
            quiet_to: "08:00".into(),
        }
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        AgentSettings {
            preferred: "auto".into(),
            default_project_dir: dirs::home_dir()
                .map(|h| h.join("projects").to_string_lossy().into_owned())
                .unwrap_or_default(),
            extra_agents: vec![],
        }
    }
}

impl Default for ResearchSettings {
    fn default() -> Self {
        ResearchSettings { results_count: 6, max_page_chars: 40_000 }
    }
}

impl Default for DiagnosticsSettings {
    fn default() -> Self {
        DiagnosticsSettings { log_level: "info".into() }
    }
}

/// Applies migrations to a raw settings JSON document.
pub fn migrate(raw: &mut serde_json::Value) -> Result<()> {
    let mut version = raw
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(SETTINGS_VERSION as u64) as u32;
    if version > SETTINGS_VERSION {
        return Err(AppError::msg(format!(
            "settings version {version} is newer than this build supports (max {SETTINGS_VERSION})"
        )));
    }
    while version < SETTINGS_VERSION {
        match version {
            1 => {}
            _ => {
                return Err(AppError::msg(format!(
                    "settings version {version} is newer than this build supports"
                )))
            }
        }
        version += 1;
        raw["version"] = json!(version);
    }
    Ok(())
}

pub struct SettingsStore {
    pub settings: Settings,
}

impl SettingsStore {
    pub fn load(raw: Option<String>) -> Result<Self> {
        let mut value: serde_json::Value = match raw {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| AppError::msg(format!("settings corrupt: {e}")))?,
            None => serde_json::to_value(Settings::default()).unwrap(),
        };
        migrate(&mut value)?;
        let settings: Settings = serde_json::from_value(value)
            .map_err(|e| AppError::msg(format!("settings invalid: {e}")))?;
        Ok(SettingsStore { settings })
    }

    pub fn update(&mut self, next: Settings) -> Result<Settings> {
        let mut value = serde_json::to_value(&next)
            .map_err(|e| AppError::msg(format!("settings serialize: {e}")))?;
        migrate(&mut value)?;
        let migrated: Settings = serde_json::from_value(value)
            .map_err(|e| AppError::msg(format!("settings invalid: {e}")))?;
        self.settings = migrated.clone();
        Ok(migrated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_versioned() {
        let s = Settings::default();
        assert_eq!(s.version, SETTINGS_VERSION);
        assert!(!s.ai.chat_model.is_empty());
    }

    #[test]
    fn corrupt_settings_rejected() {
        assert!(SettingsStore::load(Some("{not json".into())).is_err());
    }

    #[test]
    fn roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back = SettingsStore::load(Some(json)).unwrap();
        assert_eq!(back.settings, s);
    }

    #[test]
    fn future_version_rejected() {
        let mut v = serde_json::to_value(Settings::default()).unwrap();
        v["version"] = json!(SETTINGS_VERSION + 5);
        assert!(migrate(&mut v).is_err());
    }
}

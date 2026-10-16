use crate::settings::Settings;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Mutex;

/// Risk classification for every tool invocation. The language model never
/// sees this decision logic as something it can override: every dispatch in
/// `tools::execute` is gated through `PermissionEngine::authorize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Moderate,
    High,
}

#[derive(Debug, Clone, Serialize)]
pub struct Authorization {
    pub allowed: bool,
    pub risk: Risk,
    pub requires_approval: bool,
    pub reason: String,
}

pub enum Policy {
    /// Run automatically (subject to risk tier defaults).
    Auto,
    /// Ask once per app session, then remember for this session only.
    AskSession,
    /// Always ask, action-specific.
    AskAlways,
}

/// High-risk actions always require fresh, action-specific approval.
/// Moderate actions ask unless the user narrows them to auto.
/// Low actions run automatically under the default policy.
pub fn policy_for(tool: &str, risk: Risk, settings: &Settings) -> Policy {
    if let Some((_, override_v)) = settings
        .permissions
        .tool_overrides
        .iter()
        .find(|(t, _)| t == tool)
    {
        return match override_v.as_str() {
            "auto" => {
                if risk == Risk::High {
                    // A tool override can never silently downgrade High risk.
                    Policy::AskAlways
                } else {
                    Policy::Auto
                }
            }
            "always" => Policy::AskAlways,
            _ => Policy::AskSession,
        };
    }
    match risk {
        Risk::Low => Policy::Auto,
        Risk::Moderate => Policy::AskSession,
        Risk::High => Policy::AskAlways,
    }
}

pub struct PermissionEngine {
    /// Approved for this app session only — never persisted.
    session_allowed: Mutex<HashSet<String>>,
}

impl Default for PermissionEngine {
    fn default() -> Self {
        PermissionEngine { session_allowed: Mutex::new(HashSet::new()) }
    }
}

impl PermissionEngine {
    pub fn authorize(&self, tool: &str, risk: Risk, settings: &Settings, why: &str) -> Authorization {
        let policy = policy_for(tool, risk, settings);
        let (allowed, needs, reason) = match policy {
            Policy::Auto => (true, false, format!("{why} (auto: low risk)")),
            Policy::AskAlways => (false, true, why.to_string()),
            Policy::AskSession => {
                let hit = self
                    .session_allowed
                    .lock()
                    .unwrap()
                    .contains(&session_key(tool));
                if hit {
                    (true, false, format!("{why} (approved this session)"))
                } else {
                    (false, true, why.to_string())
                }
            }
        };
        Authorization { allowed, risk, requires_approval: needs, reason }
    }

    pub fn approve_session(&self, tool: &str, risk: Risk) {
        // Session memory only applies to non-high risk tiers.
        if risk != Risk::High {
            self.session_allowed.lock().unwrap().insert(session_key(tool));
        }
    }

    pub fn clear_session(&self) {
        self.session_allowed.lock().unwrap().clear();
    }
}

fn session_key(tool: &str) -> String {
    tool.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_risk_runs_automatically() {
        let engine = PermissionEngine::default();
        let settings = Settings::default();
        let a = engine.authorize("system_info", Risk::Low, &settings, "read info");
        assert!(a.allowed && !a.requires_approval);
    }

    #[test]
    fn moderate_asks_then_allows_for_session() {
        let engine = PermissionEngine::default();
        let settings = Settings::default();
        let a = engine.authorize("volume_set", Risk::Moderate, &settings, "change volume");
        assert!(!a.allowed && a.requires_approval);
        engine.approve_session("volume_set", Risk::Moderate);
        let b = engine.authorize("volume_set", Risk::Moderate, &settings, "change volume");
        assert!(b.allowed && !b.requires_approval);
    }

    #[test]
    fn high_risk_never_session_allowed() {
        let engine = PermissionEngine::default();
        let settings = Settings::default();
        engine.approve_session("file_delete", Risk::High);
        let a = engine.authorize("file_delete", Risk::High, &settings, "delete file");
        assert!(!a.allowed && a.requires_approval);
    }

    #[test]
    fn override_cannot_downgrade_high() {
        let mut settings = Settings::default();
        settings.permissions.tool_overrides = vec![("file_delete".into(), "auto".into())];
        let engine = PermissionEngine::default();
        let a = engine.authorize("file_delete", Risk::High, &settings, "delete");
        assert!(!a.allowed && a.requires_approval);
    }

    #[test]
    fn override_allows_moderate_auto() {
        let mut settings = Settings::default();
        settings.permissions.tool_overrides = vec![("volume_set".into(), "auto".into())];
        let engine = PermissionEngine::default();
        let a = engine.authorize("volume_set", Risk::Moderate, &settings, "set");
        assert!(a.allowed);
    }
}

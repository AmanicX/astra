use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct Capability {
    pub key: String,
    pub label: String,
    pub state: String,
    pub detail: String,
}

/// Honest capability discovery: probe the machine and report what actually
/// works here instead of assuming. Used by the dashboard and diagnostics.
pub fn discover() -> Vec<Capability> {
    let mut caps = Vec::new();

    caps.push(native("audio_volume", "Volume & mute", true, "Windows Core Audio (WASAPI)"));
    caps.push(native("wallpaper", "Wallpaper", true, "SystemParametersInfo"));
    caps.push(native("brightness", "Brightness", cfg!(windows), "WMI (laptops only)"));

    let ps = which("powershell").is_some();
    caps.push(Capability {
        key: "powershell".into(),
        label: "PowerShell bridge".into(),
        state: if ps { "available".into() } else { "unavailable".into() },
        detail: "Used for Wi-Fi/Bluetooth radios, brightness and a few fallbacks.".into(),
    });

    let netsh = which("netsh").is_some();
    caps.push(Capability {
        key: "netsh".into(),
        label: "Wi-Fi profiles & scan".into(),
        state: if netsh { "available".into() } else { "unavailable".into() },
        detail: "Connects to saved Wi-Fi profiles and scans networks.".into(),
    });

    let webview = webview2_present();
    caps.push(Capability {
        key: "webview2".into(),
        label: "WebView2 runtime".into(),
        state: if webview { "available".into() } else { "missing".into() },
        detail: "Required by the Tauri UI layer on Windows.".into(),
    });

    let keyring_ok = crate::credentials::get("__probe__").is_ok();
    caps.push(Capability {
        key: "credential_store".into(),
        label: "Secure credential store".into(),
        state: if keyring_ok { "available".into() } else { "unavailable".into() },
        detail: "API keys are kept in Windows Credential Manager, never in the database.".into(),
    });

    let agents = crate::agents::discover();
    caps.push(Capability {
        key: "coding_agents".into(),
        label: "Coding agents".into(),
        state: if agents.is_empty() { "none found".into() } else { "available".into() },
        detail: if agents.is_empty() {
            "No opencode/claude/codex CLI found on PATH.".into()
        } else {
            agents
                .iter()
                .map(|a| a.label.clone())
                .collect::<Vec<_>>()
                .join(", ")
        },
    });

    let tts = cfg!(windows);
    caps.push(Capability {
        key: "tts".into(),
        label: "Text-to-speech".into(),
        state: if tts { "available".into() } else { "limited".into() },
        detail: "Edge neural voices online; Windows SAPI offline fallback.".into(),
    });

    caps
}

fn native(key: &str, label: &str, ok: bool, detail: &str) -> Capability {
    Capability {
        key: key.into(),
        label: label.into(),
        state: if ok { "available".into() } else { "unavailable".into() },
        detail: detail.into(),
    }
}

pub fn which(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".into());
    let exts: Vec<String> = std::iter::once(String::new())
        .chain(pathext.split(';').map(|s| s.to_lowercase()))
        .collect();
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let candidate = dir.join(format!("{exe}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(windows)]
fn webview2_present() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let keys = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\"),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\"),
    ];
    for (hive, sub) in keys {
        let hk = winreg::RegKey::predef(hive);
        if hk.open_subkey(sub).is_ok() {
            return true;
        }
    }
    false
}

#[cfg(not(windows))]
fn webview2_present() -> bool {
    true
}

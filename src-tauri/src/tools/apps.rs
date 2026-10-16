use crate::error::{AppError, Result};
use crate::tools::ExecutionOutput;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct AppEntry {
    pub name: String,
    pub path: String,
}

pub fn discover(query: Option<&str>) -> Result<ExecutionOutput> {
    let mut apps = collect_start_menu();
    apps.extend(collect_registry_apps());
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    if let Some(q) = query {
        let ql = q.to_lowercase();
        apps.retain(|a| a.name.to_lowercase().contains(&ql) || a.path.to_lowercase().contains(&ql));
    }
    let total = apps.len();
    let top: Vec<Value> = apps
        .iter()
        .take(60)
        .map(|a| json!({"name": a.name, "path": a.path}))
        .collect();

    Ok(ExecutionOutput {
        text: if total == 0 {
            "No installed applications matched.".into()
        } else {
            format!(
                "Found {total} applications. First {}: {}",
                top.len(),
                top.iter()
                    .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        data: Some(json!({"total": total, "apps": top})),
    })
}

fn collect_start_menu() -> Vec<AppEntry> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots.push(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    roots.push(PathBuf::from(
        r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs",
    ));
    let mut out = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        if let Ok(entries) = walk_lnk(&root) {
            out.extend(entries);
        }
    }
    out
}

fn walk_lnk(dir: &Path) -> std::io::Result<Vec<AppEntry>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(mut nested) = walk_lnk(&path) {
                out.append(&mut nested);
            }
        } else if path.extension().map(|e| e.eq_ignore_ascii_case("lnk")).unwrap_or(false) {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let target = resolve_lnk(&path).unwrap_or_else(|| path.to_string_lossy().to_string());
            out.push(AppEntry { name, path: target });
        }
    }
    Ok(out)
}

/// Resolve a .lnk via the Windows shell's WScript.Shell COM object.
fn resolve_lnk(lnk: &Path) -> Option<String> {
    let lnk_esc = lnk.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$s = New-Object -ComObject WScript.Shell; \
         $l = $s.CreateShortcut('{lnk_esc}'); \
         if ($l.TargetPath) {{ $l.TargetPath }} else {{ '' }}"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    let target = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if target.is_empty() {
        None
    } else {
        Some(target)
    }
}

fn collect_registry_apps() -> Vec<AppEntry> {
    let mut out = Vec::new();
    for (hive, sub) in registry_hives() {
        for view in [true, false] {
            if let Some(items) = read_uninstall(&hive, sub, view) {
                out.extend(items);
            }
        }
    }
    out
}

fn registry_hives() -> Vec<(winreg::RegKey, &'static str)> {
    use winreg::enums::*;
    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    vec![(hkcu, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"), (hklm, r"Software\Microsoft\Windows\CurrentVersion\Uninstall")]
}

fn read_uninstall(hive: &winreg::RegKey, sub: &str, view64: bool) -> Option<Vec<AppEntry>> {
    use winreg::enums::*;
    let access = if view64 {
        KEY_READ | KEY_WOW64_64KEY
    } else {
        KEY_READ | KEY_WOW64_32KEY
    };
    let key = hive.open_subkey_with_flags(sub, access).ok()?;
    let mut out = Vec::new();
    for name in key.enum_keys().filter_map(|k| k.ok()) {
        let subkey = match key.open_subkey_with_flags(&name, KEY_READ) {
            Ok(k) => k,
            Err(_) => continue,
        };
        let display: String = subkey.get_value("DisplayName").unwrap_or_default();
        if display.is_empty() {
            continue;
        }
        let icon: String = subkey.get_value("DisplayIcon").unwrap_or_default();
        let loc: String = subkey.get_value("InstallLocation").unwrap_or_default();
        let path = if !icon.is_empty() {
            icon.split(',').next().unwrap_or(&icon).to_string()
        } else if !loc.is_empty() {
            loc
        } else {
            continue;
        };
        if Path::new(&path).exists() {
            out.push(AppEntry { name: display, path });
        }
    }
    Some(out)
}

pub fn launch(target: &str) -> Result<ExecutionOutput> {
    let t = target.trim();
    if t.is_empty() {
        return Err(AppError::Precondition("empty launch target".into()));
    }
    // Known app by name first.
    let apps = discover(Some(t))?;
    if let Some(list) = apps.data.as_ref().and_then(|d| d.get("apps")).and_then(|a| a.as_array()) {
        if let Some(first) = list.first() {
            if let Some(p) = first.get("path").and_then(|p| p.as_str()) {
                return spawn_or_open(p, t);
            }
        }
    }
    spawn_or_open(t, t)
}

fn spawn_or_open(path: &str, original: &str) -> Result<ExecutionOutput> {
    if path.starts_with("http://") || path.starts_with("https://") {
        open::that_detached(path)
            .map_err(|e| AppError::msg(format!("open url: {e}")))?;
        return Ok(ExecutionOutput {
            text: format!("Opened {path} in the default browser."),
            data: Some(json!({"opened": path})),
        });
    }
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(AppError::NotFound(format!("nothing named '{original}' and '{path}' does not exist")));
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()
            .map_err(|e| AppError::msg(format!("launch: {e}")))?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(path)
            .spawn()
            .map_err(|e| AppError::msg(format!("launch: {e}")))?;
    }
    Ok(ExecutionOutput {
        text: format!("Launched {path}."),
        data: Some(json!({"opened": path})),
    })
}

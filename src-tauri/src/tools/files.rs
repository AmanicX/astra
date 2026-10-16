use crate::error::{AppError, Result};
use crate::tools::{resolve_path, ExecutionOutput};
use serde_json::json;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn list(path: &str) -> Result<ExecutionOutput> {
    let p = resolve_path(path)?;
    let dir = if p.as_os_str().is_empty() { PathBuf::from(".") } else { p };
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let mut dirs = 0;
    let mut files = 0;
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| AppError::msg(format!("cannot list {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| AppError::msg(format!("dir entry: {e}")))?;
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        if is_dir {
            dirs += 1;
        } else {
            files += 1;
        }
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if entries.len() < 400 {
            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "dir": is_dir,
                "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                "modified_unix": modified,
            }));
        }
    }
    entries.sort_by(|a, b| {
        let da = a.get("dir").and_then(|v| v.as_bool()).unwrap_or(false);
        let db = b.get("dir").and_then(|v| v.as_bool()).unwrap_or(false);
        db.cmp(&da).then_with(|| {
            a.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase()
                .cmp(&b.get("name").and_then(|v| v.as_str()).unwrap_or("").to_lowercase())
        })
    });
    Ok(ExecutionOutput {
        text: format!(
            "{} contains {dirs} folders and {files} files.",
            dir.display()
        ),
        data: Some(json!({"path": dir.to_string_lossy(), "entries": entries, "dirs": dirs, "files": files})),
    })
}

pub fn read(path: &str, max_bytes: usize) -> Result<ExecutionOutput> {
    let p = resolve_path(path)?;
    let meta = std::fs::metadata(&p)
        .map_err(|_| AppError::NotFound(format!("file {}", p.display())))?;
    if meta.is_dir() {
        return Err(AppError::Precondition(format!("{} is a directory; use file_list", p.display())));
    }
    let cap = max_bytes.clamp(1, 1024 * 1024);
    let mut f = std::fs::File::open(&p).map_err(|e| AppError::msg(format!("open {}: {e}", p.display())))?;
    let mut buf = Vec::with_capacity(cap.min(meta.len() as usize));
    f.by_ref()
        .take(cap as u64)
        .read_to_end(&mut buf)
        .map_err(|e| AppError::msg(format!("read {}: {e}", p.display())))?;
    if buf.contains(&0u8) {
        return Err(AppError::Precondition(format!(
            "{} looks like a binary file; refusing to read it as text",
            p.display()
        )));
    }
    let content = String::from_utf8_lossy(&buf).to_string();
    let truncated = meta.len() as usize > content.len();
    Ok(ExecutionOutput {
        text: format!(
            "Read {} ({} bytes{}).\n\n{}",
            p.display(),
            content.len(),
            if truncated { ", truncated" } else { "" },
            content
        ),
        data: Some(json!({
            "path": p.to_string_lossy(),
            "content": content,
            "truncated": truncated,
            "size": meta.len(),
        })),
    })
}

pub fn write(path: &str, content: &str, data_dir: &Path) -> Result<ExecutionOutput> {
    let p = resolve_path(path)?;
    if p.as_os_str().is_empty() {
        return Err(AppError::Precondition("empty path".into()));
    }
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::msg(format!("create {}: {e}", parent.display())))?;
        }
    }
    let mut backup = None;
    if p.exists() {
        backup = Some(snapshot_backup(&p, data_dir)?);
    }
    std::fs::write(&p, content).map_err(|e| AppError::msg(format!("write {}: {e}", p.display())))?;
    Ok(ExecutionOutput {
        text: format!(
            "Wrote {} bytes to {}{}",
            content.len(),
            p.display(),
            backup
                .as_ref()
                .map(|b| format!(" (previous version backed up to {})", b.display()))
                .unwrap_or_default()
        ),
        data: Some(json!({
            "path": p.to_string_lossy(),
            "bytes": content.len(),
            "backup": backup.map(|b| b.to_string_lossy().to_string()),
        })),
    })
}

pub fn mkdir(path: &str) -> Result<ExecutionOutput> {
    let p = resolve_path(path)?;
    let existed = p.exists();
    std::fs::create_dir_all(&p).map_err(|e| AppError::msg(format!("mkdir {}: {e}", p.display())))?;
    Ok(ExecutionOutput {
        text: if existed {
            format!("{} already existed.", p.display())
        } else {
            format!("Created {}", p.display())
        },
        data: Some(json!({"path": p.to_string_lossy(), "existed": existed})),
    })
}

pub fn delete(path: &str, data_dir: &Path) -> Result<ExecutionOutput> {
    let p = resolve_path(path)?;
    let meta = std::fs::metadata(&p).map_err(|_| AppError::NotFound(format!("{}", p.display())))?;
    if meta.is_dir() {
        return Err(AppError::Precondition(
            "refusing to delete directories; delete files individually".into(),
        ));
    }
    let trash = data_dir.join("trash");
    std::fs::create_dir_all(&trash).ok();
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
    let dest = trash.join(format!("{stamp}-{name}"));
    std::fs::rename(&p, &dest)
        .or_else(|_| {
            std::fs::copy(&p, &dest).and_then(|_| std::fs::remove_file(&p))
        })
        .map_err(|e| AppError::msg(format!("move to trash failed: {e}")))?;
    Ok(ExecutionOutput {
        text: format!("Deleted {} (recoverable from {}).", p.display(), dest.display()),
        data: Some(json!({"path": p.to_string_lossy(), "trash": dest.to_string_lossy()})),
    })
}

fn snapshot_backup(path: &Path, data_dir: &Path) -> Result<PathBuf> {
    let backups = data_dir.join("backups");
    std::fs::create_dir_all(&backups).ok();
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let dest = backups.join(format!("{stamp}-{name}"));
    std::fs::copy(path, &dest).map_err(|e| AppError::msg(format!("backup failed: {e}")))?;
    // Keep the backups folder from growing without bound.
    if let Ok(entries) = std::fs::read_dir(&backups) {
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
        if files.len() > 60 {
            files.sort();
            for old in files.iter().take(files.len() - 60) {
                let _ = std::fs::remove_file(old);
            }
        }
    }
    Ok(dest)
}

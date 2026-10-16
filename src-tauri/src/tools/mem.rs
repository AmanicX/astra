use crate::error::{AppError, Result};
use crate::state::AppState;
use crate::tools::ExecutionOutput;
use serde_json::json;

pub fn read(state: &AppState, category: Option<&str>) -> Result<ExecutionOutput> {
    let db = state.db.lock().unwrap();
    let memories = db.list_memories(category, 200)?;
    let text = if memories.is_empty() {
        "No memories stored yet.".to_string()
    } else {
        memories
            .iter()
            .map(|m| format!("- [{}] {} = {} (id {})", m.category, m.key, m.value, m.id))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(ExecutionOutput {
        text,
        data: Some(json!({"memories": memories})),
    })
}

pub fn write(state: &AppState, category: &str, key: &str, value: &str) -> Result<ExecutionOutput> {
    if key.trim().is_empty() || value.trim().is_empty() {
        return Err(AppError::Precondition("memory needs a key and a value".into()));
    }
    let db = state.db.lock().unwrap();
    let m = db.upsert_memory(category, key, value, Some("assistant"))?;
    Ok(ExecutionOutput {
        text: format!("Remembered [{}] {} = {}", m.category, m.key, m.value),
        data: Some(json!(m)),
    })
}

pub fn delete(state: &AppState, id: &str) -> Result<ExecutionOutput> {
    let db = state.db.lock().unwrap();
    db.delete_memory(id)?;
    Ok(ExecutionOutput {
        text: format!("Deleted memory {id}."),
        data: None,
    })
}

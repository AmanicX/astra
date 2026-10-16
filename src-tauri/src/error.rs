use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Precondition(String),
    #[error("permission denied: {0}")]
    Denied(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        AppError::Message(s.into())
    }
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Message(_) => "error",
            AppError::Precondition(_) => "precondition",
            AppError::Denied(_) => "permission_denied",
            AppError::NotFound(_) => "not_found",
            AppError::Provider(_) => "provider",
            AppError::Timeout(_) => "timeout",
            AppError::Other(_) => "error",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 3)?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field("kind", self.kind())?;
        s.end()
    }
}

use serde::Serializer;

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Message(s)
    }
}
impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Message(s.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Message(format!("tauri: {e}"))
    }
}

use crate::error::{AppError, Result};

/// Secrets are stored in the OS credential store — Windows Credential Manager
/// on Windows, Secret Service on Linux. They never touch the database, logs,
/// settings exports, or the frontend bundle.
const SERVICE: &str = "astra";

fn entry(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, account)
        .map_err(|e| AppError::msg(format!("credential store unavailable: {e}")))
}

pub fn set(account: &str, secret: &str) -> Result<()> {
    if secret.trim().is_empty() {
        return delete(account);
    }
    entry(account)?
        .set_password(secret)
        .map_err(|e| AppError::msg(format!("store secret failed: {e}")))
}

pub fn get(account: &str) -> Result<Option<String>> {
    match entry(account)?.get_password() {
        Ok(s) if s.is_empty() => Ok(None),
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::msg(format!("read secret failed: {e}"))),
    }
}

pub fn delete(account: &str) -> Result<()> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::msg(format!("delete secret failed: {e}"))),
    }
}

pub fn has(account: &str) -> bool {
    get(account).map(|v| v.is_some()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    // Credential-store tests need a real backend (Windows session / D-Bus),
    // so they are intentionally not run in unit tests. Presence is verified
    // by the diagnostics "credential store" probe instead.
}

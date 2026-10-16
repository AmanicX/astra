use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

static BUFFER: Mutex<Vec<LogLine>> = Mutex::new(Vec::new());

/// Append-only file logger plus an in-memory ring buffer for the diagnostics
/// screen. Never logs secrets: callers redact before logging.
pub struct FileLogger {
    file: Mutex<std::fs::File>,
    level: Mutex<log::LevelFilter>,
}

impl FileLogger {
    pub fn init(dir: &PathBuf) -> &'static FileLogger {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join("astra.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .or_else(|_| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("astra-fallback.log"))
            })
            .unwrap_or_else(|_| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(std::env::temp_dir().join("astra.log"))
                    .expect("fallback log file")
            });
        let logger = Box::leak(Box::new(FileLogger {
            file: Mutex::new(file),
            level: Mutex::new(log::LevelFilter::Info),
        }));
        let _ = log::set_logger(logger);
        log::set_max_level(log::LevelFilter::Info);
        logger
    }

    pub fn set_level(&self, level: log::LevelFilter) {
        *self.level.lock().unwrap() = level;
        log::set_max_level(level);
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, md: &log::Metadata) -> bool {
        md.level() <= *self.level.lock().unwrap()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let level = record.level().to_string();
        let target = record.target().to_string();
        let message = redact(&record.args().to_string());
        let display = format!("{timestamp} [{level:>5}] {target}: {message}");

        {
            let mut buf = BUFFER.lock().unwrap();
            buf.push(LogLine {
                timestamp,
                level,
                target,
                message,
            });
            if buf.len() > 500 {
                buf.remove(0);
            }
        }
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{display}");
            let _ = f.flush();
        }
    }

    fn flush(&self) {}
}

/// Cheap guard so an accidental key in a log line is never persisted verbatim.
fn redact(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for token in s.split(' ') {
        let lower = token.to_lowercase();
        if lower.starts_with("sk-") || lower.starts_with("gsk_") || lower.contains("bearer") {
            out.push_str("[redacted]");
        } else {
            out.push_str(token);
        }
        out.push(' ');
    }
    out.trim_end().to_string()
}

pub fn recent() -> Vec<LogLine> {
    BUFFER.lock().unwrap().clone()
}

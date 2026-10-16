use crate::error::{AppError, Result};
use crate::tools::{ExecutionOutput, ToolSink};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_OUTPUT: usize = 32 * 1024;

/// Run a shell command, streaming combined output to the sink. Cancellable.
pub async fn run(
    command: &str,
    cwd: Option<&str>,
    cancel: &Arc<AtomicBool>,
    sink: &ToolSink,
) -> Result<ExecutionOutput> {
    let command = command.trim();
    if command.is_empty() {
        return Err(AppError::Precondition("empty command".into()));
    }

    #[cfg(windows)]
    let mut cmd = {
        let mut c = tokio::process::Command::new("cmd");
        c.arg("/C").arg(command);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = tokio::process::Command::new("sh");
        c.arg("-c").arg(command);
        c
    };

    if let Some(dir) = cwd {
        if !dir.trim().is_empty() {
            cmd.current_dir(dir);
        }
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdin(std::process::Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::msg(format!("spawn command: {e}")))?;

    sink.emit_output(&format!("$ {command}\n"));

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut collected = String::new();
    let mut out_buf = String::new();

    let reader_sink = sink.clone();
    let reader = {
        let collect = Arc::new(std::sync::Mutex::new(String::new()));
        let collect_clone = collect.clone();
        let handle = tokio::spawn(async move {
            use tokio::io::BufReader;
            let mut handles = Vec::new();
            if let Some(o) = stdout {
                let s = reader_sink.clone();
                let c = collect_clone.clone();
                handles.push(tokio::spawn(read_stream(
                    BufReader::new(o),
                    s,
                    c,
                )));
            }
            if let Some(e) = stderr {
                let s = reader_sink.clone();
                let c = collect_clone.clone();
                handles.push(tokio::spawn(read_stream(
                    BufReader::new(e),
                    s,
                    c,
                )));
            }
            for h in handles {
                let _ = h.await;
            }
            collect
        });
        handle
    };

    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            let _ = reader.await;
            return Err(AppError::Denied("command cancelled by user".into()));
        }
        match tokio::time::timeout(std::time::Duration::from_millis(200), child.wait()).await {
            Ok(Ok(status)) => break status,
            Ok(Err(e)) => return Err(AppError::msg(format!("wait command: {e}"))),
            Err(_) => continue,
        }
    };

    let collect = reader.await.map_err(|e| AppError::msg(format!("reader: {e}")))?;
    if let Ok(g) = collect.lock() {
        collected = g.clone();
    }

    if collected.len() > MAX_OUTPUT {
        out_buf.push_str(&collected[collected.len() - MAX_OUTPUT..]);
        out_buf.insert_str(0, "…(output truncated)\n");
    } else {
        out_buf.push_str(&collected);
    }

    let code = status.code().unwrap_or(-1);
    sink.emit_output(&format!("\n[exit code {code}]\n"));

    Ok(ExecutionOutput {
        text: format!(
            "Command exited with code {code}.\n\n{}",
            if out_buf.trim().is_empty() { "(no output)" } else { out_buf.trim() }
        ),
        data: Some(json!({"exit_code": code, "output": out_buf})),
    })
}

async fn read_stream<R>(
    reader: tokio::io::BufReader<R>,
    sink: ToolSink,
    collect: Arc<std::sync::Mutex<String>>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncBufReadExt;
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        sink.emit_output(&format!("{line}\n"));
        if let Ok(mut g) = collect.lock() {
            g.push_str(&line);
            g.push('\n');
        }
    }
}

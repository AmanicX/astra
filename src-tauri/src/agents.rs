use crate::error::{AppError, Result};
use crate::settings::AgentSettings;
use crate::tools::{ExecutionOutput, ToolCtx, ToolSink};
use crate::tools::resolve_path;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub label: String,
    pub command: String,
    /// argv template; `{prompt}` is substituted.
    pub template: Vec<String>,
    pub found: bool,
    pub path: Option<String>,
    pub note: String,
}

/// Detect installed coding-agent CLIs and configured extras.
pub fn discover() -> Vec<AgentInfo> {
    let mut out = Vec::new();
    out.push(probe("opencode", "opencode", "opencode", &["run", "{prompt}"],
        "Open agentic coding tool (opencode.ai)."));
    out.push(probe("claude", "Claude Code", "claude", &["-p", "{prompt}"],
        "Anthropic's Claude Code CLI."));
    out.push(probe("codex", "Codex CLI", "codex", &["exec", "{prompt}"],
        "OpenAI Codex CLI."));
    out.push(probe("aider", "Aider", "aider", &["--message", "{prompt}", "--yes-always"],
        "Aider pair-programming CLI."));
    out
}

fn probe(id: &str, label: &str, exe: &str, template: &[&str], note: &str) -> AgentInfo {
    let path = crate::platform::which(exe);
    AgentInfo {
        id: id.into(),
        label: label.into(),
        command: exe.into(),
        template: template.iter().map(|s| s.to_string()).collect(),
        found: path.is_some(),
        path: path.map(|p| p.to_string_lossy().to_string()),
        note: note.into(),
    }
}

pub fn resolve(settings: &AgentSettings, requested: Option<&str>) -> Result<AgentInfo> {
    let mut all = discover();
    for (id, cmd) in &settings.extra_agents {
        let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
        if let Some((exe, rest)) = parts.split_first() {
            let path = crate::platform::which(exe);
            all.push(AgentInfo {
                id: id.clone(),
                label: id.clone(),
                command: exe.clone(),
                template: {
                    let mut t: Vec<String> = rest.to_vec();
                    if !t.iter().any(|s| s.contains("{prompt}")) {
                        t.push("{prompt}".into());
                    }
                    t
                },
                found: path.is_some(),
                path: path.map(|p| p.to_string_lossy().to_string()),
                note: "User-configured agent.".into(),
            });
        }
    }

    let candidate = if let Some(req) = requested {
        if !req.trim().is_empty() {
            all.iter().find(|a| a.id == req).cloned()
        } else if !settings.preferred.trim().is_empty() {
            all.iter().find(|a| a.id == settings.preferred).cloned()
        } else {
            all.iter().find(|a| a.found).cloned()
        }
    } else if !settings.preferred.trim().is_empty() {
        all.iter().find(|a| a.id == settings.preferred).cloned()
    } else {
        all.iter().find(|a| a.found).cloned()
    };

    match candidate {
        Some(a) if a.found => Ok(a),
        Some(a) => Err(AppError::Precondition(format!(
            "'{}' was requested but is not installed (command '{}' not found on PATH).",
            a.label, a.command
        ))),
        None => Err(AppError::Precondition(
            "no coding agent found. Install opencode, Claude Code, Codex or Aider, or configure one in Settings → Agents.".into(),
        )),
    }
}

pub fn default_project_dir(settings: &AgentSettings) -> String {
    if !settings.default_project_dir.trim().is_empty() {
        return settings.default_project_dir.clone();
    }
    crate::tools::home_dir().to_string_lossy().to_string()
}

/// Spawn an agent, streaming its output. Cancellable; writes a run record.
pub async fn run(
    app_state: &crate::state::AppState,
    agent: &AgentInfo,
    prompt: &str,
    cwd: &str,
    cancel: &Arc<std::sync::atomic::AtomicBool>,
    sink: &ToolSink,
) -> Result<ExecutionOutput> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let preview: String = prompt.chars().take(200).collect();
    {
        let db = app_state.db.lock().unwrap();
        db.record_agent_start(&run_id, &agent.id, &preview, cwd)?;
    }

    let mut args: Vec<String> = agent
        .template
        .iter()
        .map(|t| t.replace("{prompt}", prompt))
        .collect();
    if args.is_empty() {
        args.push(prompt.to_string());
    }

    sink.emit_output(&format!("» {} starting in {cwd}\n", agent.label));

    let mut cmd = tokio::process::Command::new(&agent.command);
    cmd.args(&args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::msg(format!("could not start {}: {e}", agent.label)))?;

    {
        let mut children = app_state.agent_children.lock().unwrap();
        if let Some(pid) = child.id() {
            children.insert(run_id.clone(), pid);
        }
    }

    let mut tail = String::new();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let collect = Arc::new(std::sync::Mutex::new(String::new()));

    let mut readers = Vec::new();
    if let Some(o) = stdout {
        readers.push(tokio::spawn(read_into(
            o,
            sink.clone(),
            collect.clone(),
        )));
    }
    if let Some(e) = stderr {
        readers.push(tokio::spawn(read_into(
            e,
            sink.clone(),
            collect.clone(),
        )));
    }

    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            for r in readers {
                let _ = r.await;
            }
            {
                let mut children = app_state.agent_children.lock().unwrap();
                children.remove(&run_id);
            }
            let db = app_state.db.lock().unwrap();
            db.record_agent_end(&run_id, "cancelled", None);
            return Err(AppError::Denied(format!("{} was cancelled", agent.label)));
        }
        match tokio::time::timeout(std::time::Duration::from_millis(300), child.wait()).await {
            Ok(Ok(s)) => break s,
            Ok(Err(e)) => {
                let db = app_state.db.lock().unwrap();
                db.record_agent_end(&run_id, "failed", None);
                return Err(AppError::msg(format!("{} error: {e}", agent.label)));
            }
            Err(_) => {
                // Also watch for death by signal.
                continue;
            }
        }
    };

    for r in readers {
        let _ = r.await;
    }
    {
        let mut children = app_state.agent_children.lock().unwrap();
        children.remove(&run_id);
    }

    if let Ok(g) = collect.lock() {
        tail = g.clone();
    }
    if tail.len() > 16_000 {
        tail = tail[tail.len() - 16_000..].to_string();
    }

    let code = status.code().unwrap_or(-1);
    {
        let db = app_state.db.lock().unwrap();
        let state = if code == 0 { "completed" } else { "failed" };
        db.record_agent_end(&run_id, state, Some(code));
    }

    Ok(ExecutionOutput {
        text: format!(
            "{} finished with exit code {code}.\n\n{}",
            agent.label,
            if tail.trim().is_empty() { "(no output captured)" } else { tail.trim() }
        ),
        data: Some(serde_json::json!({"agent": agent.id, "exit_code": code, "output": tail})),
    })
}

async fn read_into<R>(
    stream: R,
    sink: ToolSink,
    collect: Arc<std::sync::Mutex<String>>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        sink.emit_output(&format!("{line}\n"));
        if let Ok(mut g) = collect.lock() {
            g.push_str(&line);
            g.push('\n');
        }
    }
}

/// Tool-facing entry point.
pub async fn run_as_tool(
    ctx: &ToolCtx<'_>,
    agent_id: Option<&str>,
    prompt: &str,
    cwd: Option<&str>,
    sink: &ToolSink,
) -> Result<ExecutionOutput> {
    let agent = resolve(&ctx.settings.agents, agent_id)?;
    let cwd = match cwd {
        Some(c) if !c.trim().is_empty() => resolve_path(c)?
            .to_string_lossy()
            .to_string(),
        _ => default_project_dir(&ctx.settings.agents),
    };
    if !std::path::Path::new(&cwd).is_dir() {
        return Err(AppError::Precondition(format!("project directory does not exist: {cwd}")));
    }
    run(ctx.state, &agent, prompt, &cwd, &ctx.cancel, sink).await
}

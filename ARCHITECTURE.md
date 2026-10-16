# Architecture

Astra is two programs in one process: a **Rust core** that owns every capability
and a **React front end** that is a thin, typed client. Nothing privileged lives
in JavaScript — the webview can only ask the core to do things through named
commands, and the core decides what is actually allowed to run.

```
  +----------------------------- WebView (React 19 + TS) -----------------------------+
  |  src/lib/store.ts        zustand store (workspace) / separate store (overlay)      |
  |  src/lib/api.ts          typed wrappers over invoke() + event channels            |
  |  src/lib/types.ts        DTO mirrors of the Rust types                            |
  |  src/components/*        UI                                                        |
  +------------------------------------|-----------------------------------------------+
                     invoke(cmd, args) | Channel<TurnEvent>  ^  emit (activity, toast)
  +------------------------------------v-----------------------------------------------+
  |                              Tauri 2 command bridge                                |
  +------------------------------------|-----------------------------------------------+
  |  src-tauri/src/commands.rs   ~60 #[tauri::command] entry points                       |
  |  turn.rs     agentic loop: model -> tool calls -> approvals -> results               |
  |  permissions.rs  Risk + Policy + PermissionEngine (authorize every dispatch)         |
  |  ai/         provider clients + SSE streaming + offline mock                         |
  |  tools/      tool implementations (sys, apps, audio, files, shell, net, mem)          |
  |  platform.rs OS adapters (Windows today; Linux interfaces present, unverified)       |
  |  screen.rs / research.rs / agents.rs / voice.rs                                      |
  |  db.rs (SQLite) / credentials.rs (OS keyring) / settings.rs / state.rs / logging.rs   |
  +-------------------------------------------------------------------------------------+
```

## The turn loop (`turn.rs`)

A user message starts a **turn**. The loop (bounded by `MAX_STEPS = 8`, carrying
`HISTORY_TURNS = 24` of context) streams the model's response and, when the model
calls a tool, dispatches it through the permission engine and feeds the result
back. Progress is streamed to the UI as a tagged `TurnEvent`:

| Event               | Meaning                                             |
| ------------------- | --------------------------------------------------- |
| `turn_started`      | Turn begins                                         |
| `token`             | A streamed chunk of assistant text                  |
| `assistant_message` | The completed assistant message                     |
| `tool_started`      | A tool call began                                   |
| `tool_output`       | Incremental tool output                             |
| `tool_finished`     | Tool result + status + summary                      |
| `approval_request`  | A tool needs the user's decision (blocking)         |
| `approval_resolved` | The user approved or denied                         |
| `turn_finished`     | Turn ended with a status (and optional error)       |

Events travel over a per-turn `Channel<TurnEvent>`. `ToolSink` routes output to
the right stream: `Token` when it has no tool id, otherwise `ToolOutput`.

## Permissions (`permissions.rs`)

Every tool declares a `Risk` (`low` / `moderate` / `high`). `policy_for`
resolves a `Policy`:

- **Low** → `Auto` (runs immediately).
- **Moderate** → `AskSession` (ask once, then remember for this app session).
- **High** → `AskAlways` (fresh, action-specific approval every time).

Per-tool user overrides exist, but **an override can never silently downgrade a
high-risk tool** — it still becomes `AskAlways`. Session approvals live in memory
only and are never persisted. The model's output is never a permission grant:
web page content and model text cannot authorize anything.

## AI providers (`ai/`)

- `ai/mod.rs` — provider config, request shaping, `truncate()` (char-safe), and
  the system prompt that describes available tools.
- `ai/stream.rs` — OpenAI-compatible SSE streaming client.
- `ai/mock.rs` — offline scripting provider for key-free development and tests:
  `USE_TOOL:<name> {json}` emits a tool call, `echo: …` streams text.

Supported providers: `groq`, `openai`, `openai_compat`, `ollama`, `custom`,
`mock`. Defaults: chat `openai/gpt-oss-20b`, vision
`meta-llama/llama-4-scout-17b-16e-instruct`.

## Tools (`tools/`)

Grouped by domain; the registry in `tools/mod.rs` declares each tool's name,
description, risk, "why it may prompt", and JSON parameter schema. Execution is
funelled through `tools::execute`, which authorizes first, then dispatches, and
streams output via `ToolSink`.

- `sys.rs` — system info, time, audio, brightness, Wi-Fi/Bluetooth, wallpaper
- `apps.rs` — app discovery + launch
- `files.rs` — list/read/write/mkdir/delete (writes take backups)
- `shell.rs` — authorized command execution
- `net.rs` — web search + page fetch/extract
- `mem.rs` — durable memory read/write/delete
- `audio.rs` — audio/voice helpers

Screen capture and vision analysis are surfaced as tools too (`screen_capture`,
`vision_analyze`).

## Platform adapters (`platform.rs`)

OS-specific behaviour hides behind adapter functions with a discoverable
capability map. Windows is implemented and verified (Core Audio via COM, `netsh`
for radios, WinRT, desktop capture). Linux adapters (Hyprland, KDE, GNOME) are
implemented behind the same interface but **not yet verified on hardware** —
`system_discover` reports honest capability state.

## Persistence & secrets

- `db.rs` — local SQLite: conversations, messages, memories, activity, logs.
- `credentials.rs` — API keys in the OS credential store (Windows Credential
  Manager / libsecret). **Never** in the DB or logs.
- `settings.rs` — typed, versioned settings with migration and forward-version
  rejection.
- `state.rs` — `AppState` wiring shared across commands.
- `logging.rs` — file logger (level via `ASTRA_LOG`).

## Front end (`src/`)

- `src/lib/api.ts` — one typed wrapper per command; casing matches the Rust
  `snake_case` params via camelCase.
- `src/lib/store.ts` — zustand store. **The overlay window has its own store
  instance** (separate webview), so state does not auto-share across windows.
- `src/components/*` — TitleBar, Sidebar, Conversation, Message, Composer,
  ActivityRail, ActivityPage, ApprovalDialog, Settings, Icons.
- `src/App.tsx` (workspace) and `src/Overlay.tsx` (compact overlay) are the two
  entry points, wired in `vite.config.ts` to `index.html` and `overlay.html`.

## Windows / packaging

`src-tauri/tauri.conf.json` defines two windows (`main` decorated-less with a
custom titlebar, `overlay` always-on-top and hidden by default), a strict CSP,
and `nsis` + `msi` bundle targets. A tray icon and global shortcut are registered
at startup.

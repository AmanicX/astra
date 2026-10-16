# SECURITY.md

Astra is designed to **operate the computer**. The safety model is deliberately
simple: the **native shell decides what may run**, the **model only proposes**.

## Core principles

1. **The permission boundary is native.** Every tool invocation is routed
   through `permissions.rs`. The user approval logic is in Rust code, not
   prompt-tunable in a way that lets content "grant" itself permissions. Web
   page content or model text can never authorize a tool.
2. **Risk-driven gating.**
   - `Low` → run automatically
   - `Moderate` → ask once, remember for this app session
   - `High` → always ask (fresh, action-specific)
   - Tool overrides can never downgrade `High` risk.
3. **Secrets stay in the credential store.** API keys live only via
   `credentials.rs` (Windows Credential Manager / libsecret). They are **never**
   written to the SQLite database, never logged, and never included in tool
   results.
4. **Local-first by default.** Conversations, messages, memory, activity, and
   logs are stored in the local SQLite database (`db.rs`). Network calls only
   occur when the user has a non-`mock` provider configured or when they use
   web research (if enabled/used).
5. **Least surface.** Commands are an explicit allowlist in the Tauri
   `invoke_handler` (`src-tauri/src/lib.rs`). Tool metadata (`tool_meta`) is
   declarative and reports `risk` and a plain-English `why` so the UI can be
   honest about prompts.

## Implementation details

- **Turn safety.** The turn loop (`turn.rs`) only dispatches tools when the
  plan is coherent with the schema. Steps are bounded (`MAX_STEPS = 8`), and
  `HISTORY_TURNS = 24` to keep context under control.
- **Tool execution.** `tools::execute` authorizes before dispatch. `ToolSink`
  streams output but never redacts secrets — tooling must not output secrets.
- **Shell access.** `shell.rs` executes commands the user has asked for, but
  it must follow the same permission policy and is subject to the "high-risk
  needs approval" tiers as declared in the tool registry.
- **Screen capture & vision.** Screen images are captured into memory/temp as
  declared by `screen.rs`; whether they are sent to a remote provider depends
  on which provider the user selected.
- **Network**. `net.rs` contains search + page extraction. The "research" flow is
  a fixed, declared set of endpoints — no broad browsing with arbitrary
  execution.
- **Settings & migration.** `settings.rs` enforces typed schemas and rejects
  future-version settings (prevents accidental cross-version trust).
- **Logging.** `logging.rs` writes to the app log directory; log level is
  controlled via `ASTRA_LOG`. Code must not log keys, tokens, or raw images.

## Recommendations for users

- Use the **mock** provider first to explore.
- Prefer local providers (`ollama`) when privacy is paramount.
- Treat high-risk prompts: read the "why" and what the tool will do.
- Do not paste raw `curl`/secrets into conversation (type them in Settings or
  the OS credential store as intended).
- Review recent logs (`Logs → Recent`) if anything unexpected happens.

## Known considerations

- **Linux path**: platform adapters are implemented but not hardware-verified.
- **Global shortcut**: can summon the overlay; do not bind it to a key you
  expect for another tool without testing.
- **Windows COM**: Core Audio uses Windows COM; the code must follow COM rules
  (set only where safety-critical).
- **Tool output size.** Large outputs are streamed, not dumped.
- **Backups**: file writes create backups where applicable (see `files.rs`).

## Disclaimer

This is research/engineering software under active development. Use at your own
risk on a machine you control. Review and test changes, especially around shell
execution, network, and file writes.

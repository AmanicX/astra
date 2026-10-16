# Astra

A local-first, permission-safe desktop AI assistant for Windows (Linux adapters
present, not yet hardware-verified). Astra is not a chat window: it understands
natural language and **executes real work** through a typed, risk-gated tool
system — OS control, app launching, screen awareness, web research, and coding
agent orchestration — with every consequential action approved by a native
permission engine in the shell, never by the model.

> Status: **v0.1.0**, pre-release. Backend compiles and tests green on Windows;
> the UI is built and passes typecheck/build/tests. The Groq API key is supplied
> by the user post-build through in-app settings; offline operation is available
> through the built-in `mock` provider.

## Highlights

- **Conversation workspace** — streaming replies, live tool activity, approval
  prompts, Markdown + syntax-highlighted code, persistent searchable history.
- **Real tools, real verification** — system info, app launch, volume, brightness,
  Wi-Fi/Bluetooth, wallpaper/theme, file read/write with backups, authorized
  shell, web search + page extraction, screen capture + vision analysis, memory.
- **Permission engine** — tool calls are declared with a risk tier
  (`low` / `moderate` / `high`); high-risk actions always require an
  action-specific approval. Web page content can never grant permissions.
- **Voice** — push-to-talk (plus global shortcut), Groq Whisper transcription,
  Edge neural TTS with an offline system-voice fallback.
- **Local-first privacy** — history and memory live in a local SQLite database;
  API keys live in the OS credential store, never in the DB or logs.
- **Dual window** — a full workspace window plus a compact always-on-top overlay
  summoned by a global shortcut.

See [PRODUCT.md](PRODUCT.md) for the product definition and [DESIGN.md](DESIGN.md)
for the visual direction.

## Requirements

- Windows 11 (primary target). Linux adapters are implemented behind platform
  interfaces but not yet verified on real hardware.
- Node.js (v20+) and npm.
- A Rust toolchain. This project is developed against a **user-space GNU
  toolchain** (`stable-x86_64-pc-windows-gnu`) with a portable LLVM-MinGW
  linker, so no admin/UAC rights are required.
- The Microsoft **WebView2 runtime** (installed on most Windows 11 machines).

## Getting started

```powershell
npm install
```

Run the app in development (Vite + Tauri, hot reload):

```powershell
npm run tauri dev
```

Build a release bundle (`nsis` + `msi`):

```powershell
npm run tauri build
```

### Rust commands go through the env wrapper

On this machine the Rust toolchain is not on the default `PATH`. All
`cargo` / `tauri` invocations must be routed through the wrapper script, run from
the `src-tauri` directory:

```powershell
# from D:\Ai-Assist\src-tauri
& "D:\Ai-Assist\scripts\astra-env.ps1" cargo check
& "D:\Ai-Assist\scripts\astra-env.ps1" cargo test
```

The wrapper sets the toolchain `PATH`, adds the build-output directories, and
regenerates the linker shims the GNU toolchain needs.

## Scripts

| Command             | What it does                                    |
| ------------------- | ----------------------------------------------- |
| `npm run dev`       | Vite dev server for the web front end           |
| `npm run build`     | Typecheck + production web build                |
| `npm run typecheck` | `tsc -b` across all TypeScript projects         |
| `npm test`          | Vitest unit tests                               |
| `npm run tauri`     | Tauri CLI (e.g. `npm run tauri dev`)            |
| `npm run icon`      | Regenerate app icons from `scripts/astra-icon-src.png` |

## Configuration

Astra reads a typed, versioned settings document. The AI provider can be
`groq`, `openai`, `openai_compat`, `ollama`, `custom`, or `mock`. The default
chat model is `openai/gpt-oss-20b`; the default vision model is
`meta-llama/llama-4-scout-17b-16e-instruct`.

API keys are entered in **Settings → AI** and stored in the OS credential store
(Windows Credential Manager / Secret Service), never written to the database or
to logs. Set the provider to **Offline test provider** (`mock`) to try the app
without any key.

## Safety

Astra can operate the computer, so its safety model matters. Read
[SECURITY.md](SECURITY.md). In short: the native shell enforces permissions, the
model only proposes; secrets never leave the credential store; and the network
surface is a fixed, disclosed set of provider and research endpoints.

## Architecture

The Rust core owns all capability; the React front end is a thin, typed client
over a command/event bridge. See [ARCHITECTURE.md](ARCHITECTURE.md) for the
module map and data flow.

## Repository layout

```
src/                 React front end (workspace window + overlay)
src-tauri/           Rust core, Tauri config, capabilities, icons
scripts/             Toolchain wrapper + icon generation
tests/               Vitest front-end tests
```

## License

No license has been selected yet. All rights reserved by the author until one is
added.

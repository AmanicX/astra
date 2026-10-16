# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

delegated: Tauri 2 (Rust) + React + TypeScript + Vite. An Electron + TypeScript stack was offered and declined in favor of Tauri; the build host had no Rust/MSVC toolchain, so a user-space Rust GNU toolchain (rustup stable-x86_64-pc-windows-gnu + portable LLVM-MinGW + per-user WebView2 runtime) is installed as part of setup, avoiding any admin/UAC requirement.

## Users

The machine owner: a developer and power user on Windows 11 (primary), with Arch Linux + Hyprland as a deliberate secondary target, who wants a voice- and chat-driven personal assistant that can genuinely operate the computer, research the web, and coordinate coding agents — not another chat window.

## Product Purpose

Astra is a local-first, cross-platform desktop AI assistant that understands natural language and executes real work through a typed, permission-controlled tool system: OS control, app launching, screen awareness, internet research, browser interaction, coding-agent orchestration, and durable memory. Success means the user can speak or type an intent ("set volume to 30%", "research X and write a report", "create this app with OpenCode") and see honest, verified results — with consequential actions gated by an explicit permission engine.

## Positioning

A chatbot frontend cannot copy Astra's mechanism: a native permission engine in the desktop shell enforces every tool invocation against a typed risk model, backed by real OS integrations (Core Audio, netsh, WinRT radios, desktop capture) and real verification of outcomes. The assistant reasons; the shell decides what may actually run.

## Operating Context

- Windows 11 on this machine is the verified development target; Linux (Arch/Hyprland, KDE, GNOME) adapters are implemented deliberately via platform interfaces but not yet verified on real Linux hardware in this session.
- Initial AI provider is Groq (user supplies the API key post-build via in-app settings); the provider layer is extensible to OpenAI-compatible endpoints and local servers (Ollama).
- Voice: push-to-talk plus optional wake word; Groq Whisper for transcription (requires the same key); Edge neural feminine voices for TTS with a documented offline fallback.
- Storage is local-only (SQLite); secrets go to the OS credential store (Windows Credential Manager / libsecret), never to the database or logs.
- The user's own browser session (not scraped credentials) is the channel for interacting with AI websites.

## Capabilities and Constraints

- Conversation workspace with streaming responses, tool execution activity, approval prompts, Markdown/code rendering, persistent searchable history.
- Tool execution: system info, app launching, volume, brightness (where hardware supports it), Wi-Fi/Bluetooth, wallpaper/theme (Windows personalization; Hyprland/Caelestia adapters when present), file read/write with backups, authorized shell commands, web search + page extraction, screen capture + vision analysis via configured model, coding-agent invocation (OpenCode first).
- Central permission engine with low/moderate/high risk tiers; high-risk actions always require action-specific approval; webpage content can never grant permissions.
- Settings are typed, versioned, persisted; API keys stored encrypted in the OS credential store.
- Global shortcuts, system tray, optional autostart (explicit opt-in only), compact overlay window.
- Configurable assistant name, personality, tone; security policy is not editable by personality changes.
- Not in v1 (recorded honestly): verified Linux hardware testing, local Whisper models, full browser DOM automation parity, Windows theme switching beyond documented personalization APIs.

## Brand Commitments

- Temporary product name **Astra**; name, voice, personality, and identity are user-configurable.
- Default assistant voice is feminine, natural, conversational (configurable).
- The interface is dark-first, sophisticated, restrained — near-black/charcoal surfaces, intentional accent color, excellent typography; explicitly no generic AI purple-gradient aesthetic, no decorative RGB/glass excess.

## Evidence on Hand

None. No existing code, assets, customers, or claims. The master build brief is the specification. No performance, pricing, or capability claims may be invented; any demo content in the app must be labeled synthetic in code and docs.

## Product Principles

1. Real tools over simulated UI — every button and tool performs the actual operation or says why it cannot.
2. Permission-first safety — the native shell, not the language model, enforces what may run.
3. Local-first privacy — history, memory, and secrets stay on the machine; cloud calls are explicit and disclosed.
4. Honest capability reporting — unavailable features are explained with the missing dependency; outcomes are verified, never assumed.
5. Cross-platform by construction — OS differences live behind platform adapters with discoverable capability maps.

## Accessibility & Inclusion

Keyboard-reachable controls throughout, visible focus styling, contrast-compliant tokens, reduced-motion support honored globally, spoken and typed interaction both first-class.

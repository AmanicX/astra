# Changelog

All notable changes to Astra are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-pre.1] - 2026-10-17

### Added

- Initial public pre-release of Astra, a local-first, permission-safe desktop AI assistant.
- Streaming conversation workspace with live tool activity, approval prompts, and Markdown.
- Risk-gated tool system: system info, app launch, volume, brightness, Wi-Fi/Bluetooth,
  wallpaper/theme, file read/write, authorized shell, web search, screen capture, memory.
- Native permission engine: high-risk actions require action-specific approval.
- Voice: push-to-talk, Whisper transcription, Edge neural TTS with system-voice fallback.
- Windows installer (NSIS `.exe` and `.msi`).
- Linux packages: `.deb`, `.rpm`, and portable `.AppImage`.
- One-shot Linux installer script at `scripts/install-linux.sh`.

[Unreleased]: https://github.com/AmanicX/astra/compare/v0.1.0-pre.1...HEAD
[0.1.0-pre.1]: https://github.com/AmanicX/astra/releases/tag/v0.1.0-pre.1

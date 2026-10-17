# Contributing to Astra

Thanks for your interest in improving Astra.

## Prerequisites

- Node.js 20 or newer
- The Rust stable toolchain, installed via [rustup](https://rustup.rs)
- The Tauri system dependencies for your platform (see the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/))

On Windows this project is developed against a user-space LLVM-MinGW toolchain,
so Cargo commands are routed through a wrapper and run from `src-tauri`:

```powershell
& "scripts\astra-env.ps1" cargo check
```

## Development

```bash
npm install
npm run dev         # Vite dev server for the web front end
npm run tauri dev   # full desktop app with hot reload
```

## Before opening a pull request

Run the full verification suite:

```bash
npm run typecheck
npm run build
npm test
```

and, from `src-tauri`:

```bash
cargo check
cargo test
```

To build and install the app on Linux from source, run:

```bash
./scripts/install-linux.sh
```

## Conventions

- Source files are ASCII. Use escapes or HTML entities for special characters.
- Style components with the design tokens in `src/styles/tokens.css`.
- Accessibility is required: label icon-only buttons, name form controls, and
  honor reduced motion.
- Never commit secrets. Credentials live in the OS keychain, not the repo or logs.

## License

By contributing, you agree that your contributions are licensed under the
Apache License 2.0. See [LICENSE](LICENSE).

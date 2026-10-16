# AGENTS.md

Guidance for AI coding agents working in this repository. Keep it current.

## What this is

Astra — a Tauri 2 (Rust) + React 19 + TypeScript + Vite desktop AI assistant.
The Rust core owns all capability; the React front end is a typed client over a
command/event bridge. See [ARCHITECTURE.md](ARCHITECTURE.md).

## Toolchain — read this first

The Rust toolchain on this machine is a **user-space GNU toolchain** and is **not
on the default `PATH`**. Never call `cargo` or `tauri` directly.

- Route every Rust command through the wrapper and run it **from `src-tauri`**:

  ```powershell
  # workdir: D:\Ai-Assist\src-tauri
  & "D:\Ai-Assist\scripts\astra-env.ps1" cargo check
  & "D:\Ai-Assist\scripts\astra-env.ps1" cargo test
  ```

- The wrapper prints cargo's stderr as red PowerShell errors. That is cosmetic —
  look for `Finished \`dev\` profile` for success, not for the absence of red.
- `cargo` commands **must** run from `D:\Ai-Assist\src-tauri`, not the repo root.
- Windows-GNU specifics: the `[lib]` crate type is `["rlib"]` only, and
  `build.rs` emits an unsuffixed `cargo:rustc-link-arg=…/libresource.a` so test
  harnesses get the Common-Controls manifest (otherwise `0xC0000139` from
  `comctl32.dll`).

For front-end work, the normal `npm` scripts apply.

## Verify before you finish

Run all of these; do not claim a change works without them.

```powershell
npm run typecheck          # tsc -b across all TS projects
npm run build              # tsc -b && vite build
npm test                   # vitest run (front-end unit tests)
```

```powershell
# workdir: D:\Ai-Assist\src-tauri
& "D:\Ai-Assist\scripts\astra-env.ps1" cargo check
& "D:\Ai-Assist\scripts\astra-env.ps1" cargo test
```

### Front-end design checks

If you changed `src/styles`, `src/components`, `src/App.tsx`, or
`src/Overlay.tsx`, the design detector must report **no** findings:

```powershell
& "$env:USERPROFILE\.config\opencode\skills\impeccable\scripts\impeccable.cmd" detect --json --target src/styles src/components src/App.tsx src/Overlay.tsx
```

(Expect `[]`.)

## Conventions

- **Source is ASCII.** Non-ASCII that must appear (arrows, ellipses) goes in as
  Rust escapes (`\u{2192}`, `\u{2026}`) or HTML entities — never as literal
  bytes. `Set-Content -Encoding utf8` in this environment mangles Unicode; for
  source edits use the editor tooling, not shell redirection.
- **No comments unless asked.** Match the surrounding style.
- **Style the design system, not ad hoc values.** Consume tokens from
  `src/styles/tokens.css`; follow [DESIGN.md](DESIGN.md).
- **Accessibility is a requirement**, not an enhancement: label icon-only
  buttons, name form controls, keep the toast `aria-live`, honor reduced motion.
- **Tauri v2 arg casing:** Rust `snake_case` params are passed from JS as
  `camelCase`.
- **Secrets never touch the DB or logs.** They live in the OS credential store
  via `credentials.rs`. See [SECURITY.md](SECURITY.md).
- **Do not add a git remote or push.** Local commits are fine.

## Layout

```
src/                 React front end (workspace window + overlay)
src/styles/          Design tokens + CSS (see DESIGN.md)
src/lib/             Store, typed invoke wrappers, DTO mirrors, helpers
src/components/       UI components
src-tauri/src/        Rust core modules (see ARCHITECTURE.md)
src-tauri/src/tools/  Tool implementations grouped by domain
scripts/              astra-env.ps1 wrapper, icon generation
tests/                Vitest tests
```

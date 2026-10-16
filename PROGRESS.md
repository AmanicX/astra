# PROGRESS.md

## Snapshot — 2026-10-16 (current)

**State:** Core + UI built, typechecked, built, tested. Design review issues
found and fixed; detector clean. Git initialized (fb0e80a) with backend + root
scaffolding; frontend + tests + docs still pending commit.

### What works

- **Rust core** — compiles clean (`cargo check`), tests pass (13 passed, 1
  ignored; includes GEC v2 reference vectors and version guard). `cargo build`
  succeeds. Turn loop + events, permissions, tools, providers (including mock),
  DB, credential store, voice (Edge DRM + Whisper), platform + screen + research
  + agents.
- **Front end** — all components, stores, typings, markdown renderer. `tsc -b`
  clean; `vite build` produces `dist/index.html` and `dist/overlay.html`; Vitest
  green (13 tests).
- **Toolchain** — `astra-env.ps1` wrapper for the user-space GNU + LLVM-MinGW
  toolchain; `[lib] crate-type=["rlib"]` + unsuffixed link-arg in `build.rs`;
  `.linker` shims + PATH handling fixed.
- **Design** — tokens adjusted for contrast (ink-2/ink-3), `--accent-fill`
  added, bounce easing removed. Seams over shadows. Button press scale + tap
  highlight removed. Prose uses Literata + correct size. Overlay selectors
  fixed. Markdown blockquote reduced to 1px side-tab. `impeccable detect`
  returns `[]`. Added `theme-color` and `color-scheme` to both HTML shells.
- **Accessibility** — icon-only buttons labelled, form controls named, toast
  `aria-live`, reduced motion respected, text-wrap balance, checkbox exclusions
  from generic input styles.

### Recent fixes (this session)

- Model default → `openai/gpt-oss-20b` in `settings.rs`
- Mojibake fixed in `ai/mod.rs` (→ `\u{2192}`, … `\u{2026}`) + char-safe
  `truncate()`
- Cargo.toml removed unused `sha2 = "0.11"`
- `vite.config.ts` → `import.meta.dirname`
- Fonts + code inline styling in `markdown.css`
- Missing UI classes added in `app.css` (sidebar-empty, msg-attach, tool-label,
  rail-idle, probe-models, panel-error, state/color variants)
- Settings unsaved-changes guard (dirty check + confirm on Close)
- A11y: names, aria-labels, `autoComplete="off"` on auth-like fields, toast
  aria-live, `theme-color`

### Known work

- **Docs** — README, DESIGN, AGENTS, ARCHITECTURE, SECURITY, PROGRESS written in
  this pass (all committed in git? next step).
- **Final polish** — emil-design-eng Before/After table against the changed
  motion/visual surface; surface polish review.
- **Smoketest** — quick manual run of `npm run tauri dev` (window "Astra"
  confirmed earlier; optional now).

### Validation status

| Check                | Status | Notes                    |
| -------------------- | ------ | ------------------------ |
| `tsc -b`            | OK     | No type errors          |
| `vite build`        | OK     | 502 modules             |
| `vitest run`        | OK     | 13 passed               |
| `cargo check`       | OK     | Finished dev profile    |
| `cargo test`        | OK     | 13 passed, 1 ignored    |
| `impeccable detect` | OK     | []                      |

### Git

- Branch: `master`, no remotes
- Head: `fb0e80a` — Backend: Astra local-first assistant
- Uncommitted: `src/` (entire frontend), `tests/`, root HTML/package/vite
  tweaks, `scripts/astra-env.ps1`, `src-tauri/Cargo.toml/Cargo.lock/src`, docs
  (when written), `*.tsbuildinfo` (to ignore)

### Next step (recommendation)

1. `git add -A` (with .gitignore updated to exclude tsbuildinfo)
2. `git commit -m "Frontend: UI, styles, a11y, design fixes; docs; default model gpt-oss-20b; ASCII-safe truncation"`
3. Run the verification suite one more time (or spot-check)
4. Optionally run `emil-design-eng` Before/After table for the key changes

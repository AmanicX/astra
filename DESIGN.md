# Design

Astra's interface is **dark-first, sophisticated, restrained**. The direction is
a **letterpress composing room**: warm ink-black grounds, an ivory reading
surface, one saturated ink — vermilion — reserved for the single most important
action on screen, and a bottle green for affirmative/active states. Hierarchy is
carried by weight, measure, and hairline seams, not by shadows, gradients, or
glow.

This file is the contract for anyone (human or agent) changing the look. If a
change conflicts with a rule here, change the rule deliberately and say so.

## Principles

1. **Seams over shadows.** Structure is drawn with 1px warm hairlines
   (`--seam`, `--seam-strong`). Resting surfaces do not float on drop shadows.
   Shadows, if any, are reserved for genuinely elevated, transient layers
   (menus, dialogs).
2. **Weight first.** Hierarchy comes from type weight and scale before size
   jumps and color. Headings are heavier, not just bigger.
3. **One ink.** Vermilion is scarce. It marks the primary action and live/active
   state. Everything else is grounds, seams, and the ink ramp.
4. **The reading surface is serif.** Assistant prose is set in **Literata**; the
   chrome (labels, controls, UI copy) is **Schibsted Grotesk**. This split is
   intentional — do not unify the two fonts.
5. **Quiet chrome, loud content.** Controls recede; the message measure is
   capped at `min(760px, 100%)` so lines stay readable at any window width.
6. **Accessible by construction.** Contrast, focus visibility, reduced motion,
   and keyboard reachability are not afterthoughts; a change that breaks them is
   a bug.

## Tokens

All tokens live in `src/styles/tokens.css`. Components must consume tokens, not
raw color values.

| Role            | Token                 | Dark (default) |
| --------------- | --------------------- | -------------- |
| Page ground     | `--ground-0`          | `#12100e`      |
| Raised ground   | `--ground-1`/`-2`/`-3`| warm charcoal  |
| Primary text    | `--ink-0`             | ivory `#f2ede3`|
| Secondary/tertiary text | `--ink-2` / `--ink-3` | tuned to meet ≥ 4.5:1 |
| Accent (quiet)  | `--vermilion`         | `#d9482b`      |
| Accent (filled) | `--accent-fill`       | `#b63a1e`      |
| Affirmative     | `--green`             | bottle green   |
| Seam            | `--seam` / `--seam-strong` | warm hairlines |

Spacing follows a `--sp-*` scale; radii follow `--r-*`; type sizes follow
`--fs-*`. Motion uses `--ease-*` and `--dur-*` and is disabled under
`prefers-reduced-motion: reduce` (and the user's "Reduced motion" setting).

### Contrast

Text tokens are chosen to clear **WCAG AA** against their intended grounds. Two
decisions worth remembering:

- The muted ink ramps (`--ink-2`, `--ink-3`) were lightened from an earlier
  draft because the original `#6e6659` failed AA (~3.24:1). They are now tuned
  to ≥ 4.5:1 on dark grounds and adjusted separately for the light theme.
- Filled primary buttons use `--accent-fill` (a deeper vermilion), not the
  brighter `--vermilion`, so that white button text clears AA (~4.5:1).

## Interaction & motion

- Press feedback is a small scale (`.97`) with a short transition; it is a
  tactile acknowledgement, not a performance.
- Transient layers (scrim, approval dialog, toast, overlay) animate *in* with
  `@starting-style`; they never animate in ways that delay the user.
- Motion is composite-only where it matters (transform/opacity); respect
  `prefers-reduced-motion`.
- **No bounce easings.** A spring/bounce curve was removed during review; the
  set is now limited to a standard/entrance pair.

## Anti-patterns (rejected on sight)

- Generic "AI" purple/blue gradients, glassmorphism, RGB glow.
- Thick accent `border-left`/`border-right` on cards and callouts (the
  "side-tab" tell). **Hairline seams only** — a 2px blockquote rule was caught
  by the design detector and reduced to 1px.
- Resting shadows on cards, inputs, and the composer.
- Centered paragraph text in the reading surface; body copy is left-aligned.
- Emoji as interface iconography. Icons are stroked SVGs in `src/components/Icons.tsx`.

## Where things are enforced

- Tokens: `src/styles/tokens.css`
- Primitives (buttons, fields, focus, `.level`): `src/styles/base.css`
- App shell, chat, settings, activity: `src/styles/app.css`
- Assistant prose / Markdown: `src/styles/markdown.css`
- Overlay window: `src/styles/overlay.css`

The design-skill detector (`impeccable detect`) is expected to return **zero**
findings against `src/styles`, `src/components`, `src/App.tsx`, and
`src/Overlay.tsx`.

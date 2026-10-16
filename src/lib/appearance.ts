import type { AppearanceSettings } from "./types";

export function applyAppearance(a: AppearanceSettings | null | undefined): void {
  const root = document.documentElement;
  if (!a) return;

  const theme =
    a.theme === "system"
      ? window.matchMedia("(prefers-color-scheme: light)").matches
        ? "light"
        : "dark"
      : a.theme;
  root.dataset.theme = theme;

  if (a.accent && a.accent !== "vermilion") root.dataset.accent = a.accent;
  else delete root.dataset.accent;

  if (a.reduced_motion === "on") root.dataset.motion = "reduced";
  else if (a.reduced_motion === "off") root.dataset.motion = "full";
  else delete root.dataset.motion;

  root.style.setProperty("--font-scale", String(a.font_scale || 1));
}

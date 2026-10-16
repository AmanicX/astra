import type { Risk } from "./types";

export function cn(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}

export function clock(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function dayLabel(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const now = new Date();
  const ms = now.getTime() - d.getTime();
  if (ms < 60_000) return "just now";
  if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m ago`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h ago`;
  if (ms < 7 * 86_400_000) return `${Math.floor(ms / 86_400_000)}d ago`;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function dayKey(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const now = new Date();
  const sameDay = d.toDateString() === now.toDateString();
  const y = new Date(now.getTime() - 86_400_000);
  if (sameDay) return "Today";
  if (d.toDateString() === y.toDateString()) return "Yesterday";
  return d.toLocaleDateString(undefined, {
    month: "long",
    day: "numeric",
    year: d.getFullYear() === now.getFullYear() ? undefined : "numeric",
  });
}

export function titleFromText(text: string): string {
  const line = text.trim().split("\n")[0].replace(/^[#>*\-\s]+/, "");
  const cut = line.length > 52 ? `${line.slice(0, 52).trimEnd()}…` : line;
  return cut || "New conversation";
}

export const RISK_LABEL: Record<Risk, string> = {
  low: "Low risk",
  moderate: "Moderate risk",
  high: "High risk",
};

export function riskOf(value: string): Risk {
  const v = value.toLowerCase();
  if (v === "high" || v === "moderate") return v;
  return "low";
}

export function providerLabel(provider: string): string {
  const map: Record<string, string> = {
    groq: "Groq",
    openai: "OpenAI",
    openai_compat: "OpenAI-compatible",
    ollama: "Ollama (local)",
    custom: "Custom endpoint",
    mock: "Offline test provider",
  };
  return map[provider] ?? provider;
}

export function credentialAccount(provider: string): string {
  return `api_key/${provider}`;
}

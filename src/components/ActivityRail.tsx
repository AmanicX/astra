import { useState } from "react";
import { cn } from "../lib/format";
import type { ToolActivity } from "../lib/types";
import { IconChevron } from "./Icons";

const STATUS_LABEL: Record<string, string> = {
  running: "running",
  ok: "done",
  error: "failed",
  denied: "declined",
  cancelled: "cancelled",
};

export function ActivityRail({ activity }: { activity: ToolActivity[] }) {
  const [open, setOpen] = useState<Record<string, boolean>>({});

  if (activity.length === 0) {
    return (
      <aside className="rail rail-idle">
        <h3 className="caps rail-title">Actions</h3>
        <p className="rail-empty">
          When Astra runs a tool — opening apps, changing a setting, reading the screen — it appears
          here in real time.
        </p>
      </aside>
    );
  }

  const running = activity.filter((a) => a.status === "running").length;

  return (
    <aside className="rail">
      <h3 className="caps rail-title">
        Actions
        <span className="rail-count tnum">
          {running > 0 ? `${running} running` : `${activity.length}`}
        </span>
      </h3>

      <div className="rail-list scroll">
        {activity.map((item) => {
          const expanded = !!open[item.toolId];
          const args = summarize(item.args);
          return (
            <div key={item.toolId} className={cn("tool", `tool-${item.status}`)}>
              <button
                className="tool-head"
                type="button"
                onClick={() => setOpen((s) => ({ ...s, [item.toolId]: !expanded }))}
              >
                <span className="tool-dot" />
                <span className="tool-label">{item.label}</span>
                <span className={cn("tool-status", `s-${item.status}`)}>
                  {STATUS_LABEL[item.status] ?? item.status}
                </span>
                <IconChevron
                  size={13}
                  className={cn("tool-caret", expanded && "tool-caret-open")}
                />
              </button>

              <div className="tool-sub">
                <code className="mono tool-name">{item.tool}</code>
                {args && <span className="tool-args mono">{args}</span>}
              </div>

              {item.status !== "running" && item.summary && !expanded && (
                <p className="tool-summary">{item.summary}</p>
              )}

              {expanded && (
                <div className="tool-detail">
                  {item.args != null && (
                    <>
                      <span className="caps tool-detail-label">Arguments</span>
                      <pre className="tool-pre mono">{pretty(item.args)}</pre>
                    </>
                  )}
                  {item.output && (
                    <>
                      <span className="caps tool-detail-label">Output</span>
                      <pre className="tool-pre mono">{item.output}</pre>
                    </>
                  )}
                  {item.data != null && (
                    <>
                      <span className="caps tool-detail-label">Result</span>
                      <pre className="tool-pre mono">{pretty(item.data)}</pre>
                    </>
                  )}
                  {!item.output && item.args == null && item.data == null && (
                    <p className="tool-summary">{item.summary || "No detail recorded."}</p>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </aside>
  );
}

function summarize(args: unknown): string {
  if (args == null) return "";
  if (typeof args === "object") {
    const entries = Object.entries(args as Record<string, unknown>).filter(
      ([, v]) => v !== null && v !== undefined && v !== "",
    );
    if (entries.length === 0) return "";
    return entries
      .slice(0, 3)
      .map(([k, v]) => `${k}=${short(v)}`)
      .join("  ");
  }
  return short(args);
}

function short(value: unknown): string {
  const text = typeof value === "string" ? value : JSON.stringify(value);
  return text.length > 42 ? `${text.slice(0, 42)}…` : text;
}

function pretty(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

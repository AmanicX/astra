import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { cn, clock } from "../lib/format";
import { useStore } from "../lib/store";
import type { ActivityItem } from "../lib/types";
import { IconRefresh } from "./Icons";

const STATUS_LABEL: Record<string, string> = {
  done: "done",
  failed: "failed",
  denied: "declined",
  running: "running",
  cancelled: "cancelled",
  ok: "done",
  error: "failed",
};

export function ActivityPage() {
  const setView = useStore((s) => s.setView);
  const live = useStore((s) => s.activity);
  const [history, setHistory] = useState<ActivityItem[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = async () => {
    setLoading(true);
    setHistory(await api.activityList());
    setLoading(false);
  };

  useEffect(() => {
    void refresh();
  }, []);

  const rows: ActivityItem[] = [
    ...live.map((a) => ({
      id: a.toolId,
      turn_id: a.turnId,
      tool: a.tool,
      label: a.label,
      status: a.status,
      detail: a.summary || null,
      started_at: new Date(a.startedAt).toISOString(),
      ended_at: a.endedAt ? new Date(a.endedAt).toISOString() : null,
    })),
    ...history.filter((h) => !live.some((l) => l.toolId === h.id)),
  ];

  return (
    <div className="settings">
      <div className="settings-head">
        <h1 className="settings-title">Activity</h1>
        <div className="settings-actions">
          <button className="btn btn-ghost" type="button" onClick={() => setView("chat")}>
            Close
          </button>
          <button className="btn btn-quiet" type="button" onClick={() => void refresh()} disabled={loading}>
            <IconRefresh size={15} /> {loading ? "Loading…" : "Refresh"}
          </button>
        </div>
      </div>

      <div className="settings-body scroll">
        <p className="ssec-note">
          Every tool Astra has run this session, newest first. Session approvals are in memory only.
        </p>
        <div className="list">
          {rows.length === 0 && <p className="ssec-note">Nothing has run yet.</p>}
          {rows.map((r) => (
            <div key={r.id} className={cn("list-row", `row-${r.status}`)}>
              <code className="mono">{r.tool}</code>
              <span className="label">{r.label}</span>
              <span className={cn("caps", `s-${r.status}`)}>{STATUS_LABEL[r.status] ?? r.status}</span>
              <span className="tnum mono">{clock(r.started_at)}</span>
              {r.detail && <span className="detail">{r.detail}</span>}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

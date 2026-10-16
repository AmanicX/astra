import { useEffect } from "react";
import { cn, RISK_LABEL } from "../lib/format";
import { useStore } from "../lib/store";
import { IconAlert, IconShield } from "./Icons";

export function ApprovalDialog() {
  const approval = useStore((s) => s.approval);
  const respond = useStore((s) => s.respondApproval);

  useEffect(() => {
    if (!approval) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void respond(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [approval, respond]);

  if (!approval) return null;

  const args = prettyArgs(approval.args);

  return (
    <div className="scrim" role="dialog" aria-modal="true" aria-label="Permission request">
      <div className={cn("approval", `risk-${approval.risk}`)}>
        <header className="approval-head">
          <span className="approval-badge">
            {approval.risk === "high" ? <IconAlert size={16} /> : <IconShield size={16} />}
          </span>
          <div>
            <h2 className="approval-title">Astra wants to act</h2>
            <p className="approval-risk caps">{RISK_LABEL[approval.risk]}</p>
          </div>
        </header>

        <div className="approval-body">
          <div className="approval-row">
            <span className="caps">Tool</span>
            <code className="mono">{approval.tool}</code>
          </div>
          {approval.why && (
            <div className="approval-row">
              <span className="caps">Why</span>
              <p>{approval.why}</p>
            </div>
          )}
          {args && (
            <div className="approval-row">
              <span className="caps">Arguments</span>
              <pre className="tool-pre mono">{args}</pre>
            </div>
          )}
        </div>

        <footer className="approval-foot">
          <button className="btn btn-ghost" type="button" onClick={() => void respond(false)}>
            Decline
          </button>
          <div className="approval-allow">
            <button
              className="btn btn-quiet"
              type="button"
              onClick={() => void respond(true, "always")}
            >
              Always allow {approval.tool}
            </button>
            <button
              className="btn btn-primary"
              type="button"
              onClick={() => void respond(true)}
              autoFocus
            >
              Allow once
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
}

function prettyArgs(args: unknown): string {
  if (args == null) return "";
  if (typeof args === "object" && Object.keys(args as object).length === 0) return "";
  try {
    return JSON.stringify(args, null, 2);
  } catch {
    return String(args);
  }
}

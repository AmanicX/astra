import { useState } from "react";
import { Markdown } from "../lib/markdown";
import { cn, clock } from "../lib/format";
import { useStore } from "../lib/store";
import type { UIMessage } from "../lib/types";
import { IconCopy, IconVolume, IconUser, IconSparkle } from "./Icons";

export function Message({ message }: { message: UIMessage }) {
  const speakText = useStore((s) => s.speakText);
  const speaking = useStore((s) => s.speaking);
  const [copied, setCopied] = useState(false);

  const isUser = message.role === "user";
  const attachments = countAttachments(message.meta);

  const copy = () => {
    navigator.clipboard?.writeText(message.content).then(
      () => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1400);
      },
      () => {},
    );
  };

  return (
    <article className={cn("msg", isUser ? "msg-user" : "msg-assistant")}>
      <div className="msg-gutter">
        <span className="msg-avatar">{isUser ? <IconUser size={15} /> : <IconSparkle size={15} />}</span>
        <span className="msg-rule" />
      </div>

      <div className="msg-main">
        <header className="msg-head">
          <span className="msg-who">{isUser ? "You" : "Astra"}</span>
          <span className="msg-time tnum">{clock(message.created_at)}</span>
          {attachments > 0 && (
            <span className="msg-attach">
              {attachments} image{attachments > 1 ? "s" : ""}
            </span>
          )}
          <div className="msg-tools">
            <button
              className="icon-btn"
              type="button"
              title="Copy"
              aria-label={copied ? "Copied" : "Copy message"}
              onClick={copy}
            >
              {copied ? <span className="icon-check">Copied</span> : <IconCopy size={14} />}
            </button>
            {!isUser && message.content.trim() && (
              <button
                className={cn("icon-btn", speaking && "icon-btn-live")}
                type="button"
                title="Read aloud"
                aria-label="Read message aloud"
                onClick={() => void speakText(message.content)}
              >
                <IconVolume size={14} />
              </button>
            )}
          </div>
        </header>

        <div className="msg-content">
          {isUser ? (
            <p className="msg-text-user">{message.content}</p>
          ) : message.content ? (
            <Markdown>{message.content}</Markdown>
          ) : (
            <span className="thinking">
              <span className="dot" data-live="true" />
              <span className="dot" data-live="true" />
              <span className="dot" data-live="true" />
            </span>
          )}
          {message.streaming && message.content && <span className="caret" aria-hidden />}
        </div>
      </div>
    </article>
  );
}

function countAttachments(meta: unknown): number {
  if (meta && typeof meta === "object" && "images" in meta) {
    const value = (meta as { images?: unknown }).images;
    if (typeof value === "number") return value;
  }
  return 0;
}

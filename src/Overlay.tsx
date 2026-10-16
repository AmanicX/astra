import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText } from "./lib/api";
import { blobToBase64, startRecording } from "./lib/audio";
import { cn } from "./lib/format";
import { IconClose, IconMic, IconSend, IconSparkle } from "./components/Icons";
import "./styles/overlay.css";

export default function Overlay() {
  const [text, setText] = useState("");
  const [output, setOutput] = useState<Array<{ role: "user" | "assistant" | "system"; text: string }>>([]);
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [level, setLevel] = useState(0);
  const recRef = useRef<{ stop: () => Promise<Blob>; cancel: () => void } | null>(null);
  const convRef = useRef<string | null>(null);

  const ensureConversation = async () => {
    if (convRef.current) return convRef.current;
    const created = await api.conversationCreate("Quick capture");
    convRef.current = created.id;
    return created.id;
  };

  const submit = async () => {
    const value = text.trim();
    if (!value || busy) return;
    setOutput((prev) => [...prev, { role: "user", text: value }]);
    setText("");
    setBusy(true);
    setError(null);
    try {
      const conversationId = await ensureConversation();
      let turnId = "";
      await api.turnRun(conversationId, value, [], (event) => {
        if (event.type === "turn_started") turnId = event.turn_id;
        if (event.type === "token" && event.turn_id === turnId) {
          setOutput((prev) => {
            const last = prev[prev.length - 1];
            if (last?.role === "assistant") {
              const next = [...prev];
              next[next.length - 1] = { ...last, text: last.text + event.text };
              return next;
            }
            return [...prev, { role: "assistant", text: event.text }];
          });
        }
        if (event.type === "turn_finished") {
          setBusy(false);
        }
      });
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  };

  const begin = () => {
    if (recording || busy) return;
    try {
      recRef.current = startRecording(setLevel);
      setRecording(true);
    } catch (e) {
      setError("Microphone unavailable.");
    }
  };

  const finish = async () => {
    const r = recRef.current;
    recRef.current = null;
    setRecording(false);
    setLevel(0);
    if (!r) return;
    setBusy(true);
    try {
      const clip = await r.stop();
      if (clip.size === 0) return;
      const base64 = await blobToBase64(clip);
      const t = await api.sttTranscribe(base64, clip.type || "audio/webm", "clip.webm");
      if (t.trim()) setText(t.trim());
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    const un = listen<string>("astra://ptt", (e) => {
      if (e.payload === "start") begin();
      if (e.payload === "stop") void finish();
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  return (
    <div className="overlay">
      <header className="overlay-drag" data-tauri-drag-region>
        <div className="overlay-brand">
          <IconSparkle size={14} />
          <span>Astra</span>
        </div>
        <div className="overlay-actions">
          <button
            className="overlay-close"
            type="button"
            title="Hide"
            aria-label="Hide overlay"
            onClick={() => void api.overlayHide()}
          >
            <IconClose size={14} />
          </button>
        </div>
      </header>

      <main className="overlay-main">
        <div className="overlay-results scroll">
          {output.length === 0 && <span className="overlay-line overlay-line-system">Say something…</span>}
          {output.map((o, i) => (
            <span key={i} className={cn("overlay-line", `overlay-line-${o.role}`)}>
              {o.text}
            </span>
          ))}
          {busy && <span className="overlay-line overlay-line-system">…</span>}
          {error && <span className="overlay-line overlay-line-system">{error}</span>}
        </div>

        {recording && (
          <div className="overlay-row">
            <span className="overlay-status">Listening</span>
            <span className="level" style={{ flex: 1 }}>
              <span style={{ transform: `scaleX(${Math.max(0.04, level)})` }} />
            </span>
          </div>
        )}

        <div className="overlay-row">
          <input
            className="overlay-input"
            name="overlay-message"
            aria-label="Ask Astra"
            value={text}
            placeholder="Ask Astra"
            autoComplete="off"
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void submit();
              }
            }}
          />
          <button
            className={cn("overlay-mic", recording && "overlay-mic-live")}
            type="button"
            title="Hold to talk"
            aria-label="Hold to talk"
            onPointerDown={(e) => {
              e.preventDefault();
              begin();
            }}
            onPointerUp={() => void finish()}
          >
            <IconMic size={16} />
          </button>
          <button
            className="overlay-mic"
            type="button"
            title="Send"
            aria-label="Send"
            onClick={submit}
            disabled={!text.trim()}
          >
            <IconSend size={16} />
          </button>
        </div>
      </main>
    </div>
  );
}

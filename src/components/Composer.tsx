import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText } from "../lib/api";
import { blobToBase64, imageToAttachment, startRecording } from "../lib/audio";
import { cn, providerLabel } from "../lib/format";
import { useStore } from "../lib/store";
import type { ChatAttachment } from "../lib/types";
import { IconClose, IconImage, IconMic, IconSend, IconStop } from "./Icons";

let nextAttachmentId = 0;
interface Attachment extends ChatAttachment {
  id: number;
  name: string;
}

export function Composer() {
  const streaming = useStore((s) => s.streaming);
  const send = useStore((s) => s.send);
  const cancelTurn = useStore((s) => s.cancelTurn);
  const settings = useStore((s) => s.settings);
  const setView = useStore((s) => s.setView);

  const [text, setText] = useState("");
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [recording, setRecording] = useState(false);
  const [level, setLevel] = useState(0);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const areaRef = useRef<HTMLTextAreaElement>(null);
  const recorderRef = useRef<{ stop: () => Promise<Blob>; cancel: () => void } | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const pttRef = useRef(false);

  useEffect(() => {
    const el = areaRef.current;
    if (!el) return;
    el.style.height = "0px";
    el.style.height = `${Math.min(el.scrollHeight, 260)}px`;
  }, [text]);

  const submit = () => {
    const value = text.trim();
    if (!value || streaming) return;
    void send(
      value,
      attachments.map(({ mime, data_base64 }) => ({ mime, data_base64 })),
    );
    setText("");
    setAttachments([]);
  };

  const beginRecording = async () => {
    if (recording || streaming) return;
    setNotice(null);
    try {
      recorderRef.current = startRecording(setLevel);
      setRecording(true);
    } catch {
      setNotice("Microphone unavailable — check Windows privacy settings.");
    }
  };

  const finishRecording = async () => {
    const rec = recorderRef.current;
    recorderRef.current = null;
    setRecording(false);
    setLevel(0);
    if (!rec) return;
    setBusy(true);
    try {
      const clip = await rec.stop();
      if (clip.size === 0) return;
      const base64 = await blobToBase64(clip);
      const transcript = await api.sttTranscribe(base64, clip.type || "audio/webm", "clip.webm");
      if (transcript.trim()) {
        setText((prev) => (prev ? `${prev} ${transcript.trim()}` : transcript.trim()));
      }
    } catch (e) {
      setNotice(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  // Push-to-talk from the global shortcut (astra://ptt).
  useEffect(() => {
    const unlisten = listen<string>("astra://ptt", (event) => {
      if (event.payload === "start") {
        pttRef.current = true;
        void beginRecording();
      } else if (event.payload === "stop" && pttRef.current) {
        pttRef.current = false;
        void finishRecording();
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onPickFiles = async (files: FileList | null) => {
    if (!files) return;
    const next: Attachment[] = [];
    for (const file of Array.from(files).slice(0, 4)) {
      try {
        const attachment = await imageToAttachment(file);
        next.push({ ...attachment, id: nextAttachmentId++, name: file.name });
      } catch {
        /* skip unreadable file */
      }
    }
    setAttachments((prev) => [...prev, ...next].slice(0, 4));
  };

  const provider = settings ? providerLabel(settings.ai.provider) : "";
  const isMock = settings?.ai.provider === "mock";

  return (
    <div className="composer-wrap">
      {notice && <div className="composer-notice">{notice}</div>}

      {isMock && (
        <button className="composer-banner" type="button" onClick={() => setView("settings")}>
          Offline test provider is active — add a real API key in Settings to reach a live model.
        </button>
      )}

      <div className={cn("composer", recording && "composer-recording")}>
        {attachments.length > 0 && (
          <div className="attach-row">
            {attachments.map((a) => (
              <span className="attach-chip" key={a.id}>
                <IconImage size={13} />
                <span className="attach-name">{a.name}</span>
                <button
                  type="button"
                  className="attach-x"
                  title="Remove"
                  aria-label={`Remove ${a.name}`}
                  onClick={() => setAttachments((prev) => prev.filter((x) => x.id !== a.id))}
                >
                  <IconClose size={12} />
                </button>
              </span>
            ))}
          </div>
        )}

        <div className="composer-field">
          <textarea
            ref={areaRef}
            value={text}
            rows={1}
            name="message"
            aria-label="Message Astra"
            placeholder={recording ? "Listening…" : "Ask Astra, or describe a task to run on this machine"}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                submit();
              }
            }}
            spellCheck
          />

          <div className="composer-actions">
            <input
              ref={fileRef}
              type="file"
              accept="image/*"
              multiple
              hidden
              aria-label="Attach image"
              onChange={(e) => {
                void onPickFiles(e.target.files);
                e.target.value = "";
              }}
            />
            <button
              className="composer-icon"
              type="button"
              title="Attach image"
              aria-label="Attach image"
              onClick={() => fileRef.current?.click()}
              disabled={attachments.length >= 4}
            >
              <IconImage size={17} />
            </button>

            <button
              className={cn("composer-icon", recording && "composer-icon-live")}
              type="button"
              title="Hold to talk (Ctrl+Shift+Space)"
              aria-label="Hold to talk"
              onPointerDown={(e) => {
                e.preventDefault();
                void beginRecording();
              }}
              onPointerUp={() => void finishRecording()}
              onPointerLeave={() => recording && void finishRecording()}
              disabled={busy}
            >
              <IconMic size={17} />
            </button>
          </div>
        </div>

        {recording && (
          <div className="level" title="Input level">
            <span style={{ transform: `scaleX(${Math.max(0.04, level)})` }} />
          </div>
        )}

        <div className="composer-foot">
          <span className="composer-hint">
            {busy ? (
              "Transcribing…"
            ) : recording ? (
              "Release to send to speech-to-text"
            ) : (
              <>
                <kbd>Enter</kbd> to send · <kbd>Shift</kbd>+<kbd>Enter</kbd> for a new line
              </>
            )}
            <span className="composer-provider">via {provider}</span>
          </span>

          {streaming ? (
            <button className="send send-stop" type="button" onClick={() => void cancelTurn()}>
              <IconStop size={15} />
              <span>Stop</span>
            </button>
          ) : (
            <button
              className="send"
              type="button"
              onClick={submit}
              disabled={!text.trim()}
              title="Send"
            >
              <IconSend size={16} />
              <span>Send</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

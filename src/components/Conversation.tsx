import { useEffect, useRef } from "react";
import { useStore } from "../lib/store";
import { Message } from "./Message";
import { Composer } from "./Composer";

const PROMPTS = [
  "What can you do on this machine?",
  "Set the volume to 30% and turn on the Bluetooth.",
  "Take a screenshot and tell me what's on screen.",
  "Search the web for today's tech news and summarise it.",
  "Write a Python script that renames files by date and run it here.",
];

export function Conversation() {
  const messages = useStore((s) => s.messages);
  const streaming = useStore((s) => s.streaming);
  const send = useStore((s) => s.send);
  const settings = useStore((s) => s.settings);

  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: streaming ? "auto" : "smooth", block: "end" });
  }, [messages, streaming]);

  const empty = messages.length === 0;

  return (
    <div className="conversation">
      <div className="thread scroll">
        {empty ? (
          <div className="welcome">
            <div className="welcome-mark">
              <span className="welcome-glyph">A</span>
              <span className="welcome-rule" />
            </div>
            <h1 className="welcome-title">{settings?.assistant.name ?? "Astra"}</h1>
            <p className="welcome-lede">
              A local-first assistant that can actually <em>do</em> things on this computer — under
              your eye, with every action on the record.
            </p>
            <div className="welcome-prompts">
              <span className="caps">Try</span>
              {PROMPTS.map((p) => (
                <button key={p} className="prompt-chip" type="button" onClick={() => void send(p)}>
                  {p}
                </button>
              ))}
            </div>
          </div>
        ) : (
          messages.map((m) => <Message key={m.id} message={m} />)
        )}
        <div ref={bottomRef} />
      </div>

      <Composer />
    </div>
  );
}

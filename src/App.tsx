import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./lib/store";
import { TitleBar } from "./components/TitleBar";
import { Sidebar } from "./components/Sidebar";
import { Conversation } from "./components/Conversation";
import { ActivityRail } from "./components/ActivityRail";
import { ActivityPage } from "./components/ActivityPage";
import { ApprovalDialog } from "./components/ApprovalDialog";
import { Settings } from "./components/Settings";
import "./styles/app.css";
import "./styles/markdown.css";

export default function App() {
  const view = useStore((s) => s.view);
  const ready = useStore((s) => s.ready);
  const load = useStore((s) => s.load);
  const activity = useStore((s) => s.activity);
  const toast = useStore((s) => s.toast);
  const stopSpeaking = useStore((s) => s.stopSpeaking);

  const loadedRef = useRef(false);
  useEffect(() => {
    if (!loadedRef.current) {
      loadedRef.current = true;
      void load();
    }
  }, [load]);

  useEffect(() => {
    const unlisten = listen("astra://stop-speaking", () => {
      stopSpeaking();
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [stopSpeaking]);

  const showRail = activity.length > 0 && view === "chat";

  return (
    <div className="app" data-rail={showRail ? "on" : "off"}>
      <TitleBar />
      <Sidebar />

      <main className="main">
        {!ready ? (
          <div className="welcome">
            <span className="welcome-mark" aria-hidden="true" />
            <h1 className="welcome-title">Astra</h1>
            <p className="welcome-lede">Waking the composing room.</p>
          </div>
        ) : view === "settings" || view === "diagnostics" ? (
          <Settings initialTab={view === "diagnostics" ? "diagnostics" : "overview"} />
        ) : view === "activity" ? (
          <ActivityPage />
        ) : (
          <Conversation />
        )}
      </main>

      {showRail && <ActivityRail activity={activity} />}

      <ApprovalDialog />

      {toast && (
        <div className="toast" role="status" aria-live="polite">
          {toast}
        </div>
      )}
    </div>
  );
}

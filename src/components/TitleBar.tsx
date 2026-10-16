import { api } from "../lib/api";
import { providerLabel } from "../lib/format";
import { useStore } from "../lib/store";
import { IconClose, IconMaximize, IconMinimize, IconSparkle } from "./Icons";

export function TitleBar() {
  const settings = useStore((s) => s.settings);
  const streaming = useStore((s) => s.streaming);
  const setView = useStore((s) => s.setView);

  const provider = settings ? providerLabel(settings.ai.provider) : "";
  const model = settings?.ai.chat_model ?? "";

  return (
    <header className="titlebar" data-tauri-drag-region>
      <button
        className="brand"
        type="button"
        title="Astra"
        onClick={() => setView("chat")}
      >
        <IconSparkle size={16} />
        <span className="brand-name">Astra</span>
      </button>

      <div className="titlebar-recipe mono" data-tauri-drag-region>
        {streaming ? (
          <span className="recipe-live">
            <span className="dot" data-live="true" />
            working
          </span>
        ) : (
          <>
            <span>{provider}</span>
            {model && <span className="recipe-sep">/</span>}
            {model && <span className="recipe-model">{model}</span>}
          </>
        )}
      </div>

      <div className="window-controls">
        <button
          className="wc"
          type="button"
          title="Minimise"
          aria-label="Minimise window"
          onClick={() => void api.windowMinimize()}
        >
          <IconMinimize size={15} />
        </button>
        <button
          className="wc"
          type="button"
          title="Maximise"
          aria-label="Maximise window"
          onClick={() => void api.windowToggleMaximize()}
        >
          <IconMaximize size={13} />
        </button>
        <button
          className="wc wc-close"
          type="button"
          title="Close"
          aria-label="Close window"
          onClick={() => void api.windowClose()}
        >
          <IconClose size={15} />
        </button>
      </div>
    </header>
  );
}

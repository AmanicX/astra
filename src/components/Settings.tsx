import { useEffect, useState } from "react";
import { api, errorText } from "../lib/api";
import { cn, credentialAccount } from "../lib/format";
import { useStore } from "../lib/store";
import type { AgentInfo, Capability, LogLine, ToolCatalogEntry, VoiceInfo, ProviderProbeResult } from "../lib/types";
import { IconCpu, IconRefresh } from "./Icons";

const TABS = [
  { id: "overview", label: "Overview" },
  { id: "ai", label: "AI & Models" },
  { id: "voice", label: "Voice" },
  { id: "permissions", label: "Permissions" },
  { id: "privacy", label: "Privacy" },
  { id: "look", label: "Look & Feel" },
  { id: "agents", label: "Agents" },
  { id: "research", label: "Research" },
  { id: "diagnostics", label: "Diagnostics" },
] as const;

type TabId = (typeof TABS)[number]["id"];

export function Settings({ initialTab = "overview" }: { initialTab?: TabId }) {
  const settings = useStore((s) => s.settings);
  const applySettings = useStore((s) => s.applySettings);
  const showToast = useStore((s) => s.showToast);
  const setView = useStore((s) => s.setView);

  const [tab, setTab] = useState<TabId>(initialTab);
  const [draft, setDraft] = useState(settings ?? null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // AI
  const [providerKey, setProviderKey] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [probing, setProbing] = useState(false);
  const [probe, setProbe] = useState<ProviderProbeResult | null>(null);

  // Voice
  const [voices, setVoices] = useState<VoiceInfo[]>([]);

  // Diagnostics
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [catalog, setCatalog] = useState<ToolCatalogEntry[]>([]);
  const [meta, setMeta] = useState<Record<string, string>>({});
  const [capabilities, setCapabilities] = useState<Capability[]>([]);
  const [agents, setAgents] = useState<AgentInfo[]>([]);

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  useEffect(() => {
    if (!draft) return;
    void (async () => {
      const acc = credentialAccount(draft.ai.provider);
      setHasKey(await api.credentialHas(acc));
    })();
  }, [draft?.ai.provider]);

  useEffect(() => {
    void (async () => {
      setVoices(await api.ttsVoices());
      setLogs(await api.logsRecent());
      setCatalog(await api.toolCatalog());
      setCapabilities(await api.systemDiscover());
      setAgents(await api.agentDiscover());
    })();
  }, []);

  const save = async () => {
    if (!draft) return;
    setSaving(true);
    setError(null);
    try {
      const saved = await api.settingsSet(draft);
      applySettings(saved);
      showToast("Settings saved.");
      setView("chat");
    } catch (e) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  };

  const reset = async () => {
    if (!confirm("Reset all settings to defaults?")) return;
    const saved = await api.settingsReset();
    applySettings(saved);
    setDraft(saved);
    showToast("Settings reset.");
  };

  const saveKey = async () => {
    if (!draft || !providerKey.trim()) return;
    await api.credentialSet(credentialAccount(draft.ai.provider), providerKey.trim());
    setProviderKey("");
    const acc = credentialAccount(draft.ai.provider);
    setHasKey(await api.credentialHas(acc));
    showToast("API key saved.");
  };

  const testProvider = async () => {
    if (!draft) return;
    setProbing(true);
    setProbe(null);
    try {
      setProbe(await api.providerTest(draft.ai.provider));
    } catch (e) {
      setProbe({ ok: false, models: [], message: errorText(e) });
    } finally {
      setProbing(false);
    }
  };

  const reloadDiag = async () => {
    setLogs(await api.logsRecent());
    setCapabilities(await api.systemDiscover());
    setAgents(await api.agentDiscover());
    setCatalog(await api.toolCatalog());
  };

  const clearSessionPerms = async () => {
    await api.permissionsClearSession();
    showToast("Session permissions cleared.");
  };

  const getMeta = async (name: string, risk: string) => {
    const m = await api.toolMeta(name, risk);
    setMeta((prev) => ({ ...prev, [`${name}|${risk}`]: JSON.stringify(m) }));
  };

  const dirty = !!draft && !!settings && JSON.stringify(draft) !== JSON.stringify(settings);

  const close = () => {
    if (dirty && !confirm("Discard unsaved changes?")) return;
    setView("chat");
  };

  if (!draft) return null;

  return (
    <div className="settings">
      <div className="settings-head">
        <h1 className="settings-title">Settings</h1>
        <div className="settings-actions">
          <button className="btn btn-ghost" type="button" onClick={close}>
            Close
          </button>
          <button className="btn btn-quiet" type="button" onClick={reset}>
            Reset
          </button>
          <button className="btn btn-primary" type="button" onClick={save} disabled={saving}>
            {saving ? "Saving…" : "Save & return"}
          </button>
        </div>
      </div>

      <div className="settings-layout">
        <nav className="settings-nav">
          {TABS.map((t) => (
            <button
              key={t.id}
              className={cn("snav", tab === t.id && "snav-active")}
              type="button"
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </nav>

        <div className="settings-body scroll">
          {error && <div className="panel-error">{error}</div>}

          {/* Overview */}
          {tab === "overview" && (
            <section className="ssec">
              <h2 className="ssec-title">Overview</h2>
              <div className="grid-2">
                <label className="field">
                  <span className="field-label caps">Assistant name</span>
                  <input
                    name="assistant-name"
                    autoComplete="off"
                    spellCheck={false}
                    value={draft.assistant.name}
                    onChange={(e) => setDraft(p => ({ ...p!, assistant: { ...p!.assistant, name: e.target.value } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Tone</span>
                  <input
                    name="assistant-tone"
                    autoComplete="off"
                    value={draft.assistant.tone}
                    onChange={(e) => setDraft(p => ({ ...p!, assistant: { ...p!.assistant, tone: e.target.value } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Personality</span>
                  <input
                    name="assistant-personality"
                    autoComplete="off"
                    value={draft.assistant.personality}
                    onChange={(e) => setDraft(p => ({ ...p!, assistant: { ...p!.assistant, personality: e.target.value } }))}
                  />
                </label>
              </div>
            </section>
          )}

          {/* AI & Models */}
          {tab === "ai" && (
            <section className="ssec">
              <h2 className="ssec-title">AI & Models</h2>
              <div className="grid-2">
                <label className="field">
                  <span className="field-label caps">Provider</span>
                  <select
                    name="provider"
                    value={draft.ai.provider}
                    onChange={(e) => setDraft(p => ({ ...p!, ai: { ...p!.ai, provider: e.target.value } }))}
                  >
                    <option value="groq">Groq</option>
                    <option value="openai">OpenAI</option>
                    <option value="openai_compat">OpenAI-compatible</option>
                    <option value="ollama">Ollama (local)</option>
                    <option value="custom">Custom endpoint</option>
                    <option value="mock">Offline test provider</option>
                  </select>
                </label>
                <label className="field">
                  <span className="field-label caps">Base URL</span>
                  <input
                    name="base-url"
                    type="url"
                    inputMode="url"
                    autoComplete="off"
                    spellCheck={false}
                    value={draft.ai.base_url}
                    onChange={(e) => setDraft(p => ({ ...p!, ai: { ...p!.ai, base_url: e.target.value } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Chat model</span>
                  <input
                    name="chat-model"
                    autoComplete="off"
                    spellCheck={false}
                    value={draft.ai.chat_model}
                    onChange={(e) => setDraft(p => ({ ...p!, ai: { ...p!.ai, chat_model: e.target.value } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Vision model</span>
                  <input
                    name="vision-model"
                    autoComplete="off"
                    spellCheck={false}
                    value={draft.ai.vision_model}
                    onChange={(e) => setDraft(p => ({ ...p!, ai: { ...p!.ai, vision_model: e.target.value } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Temperature</span>
                  <input
                    name="temperature"
                    type="number"
                    inputMode="decimal"
                    step="0.1"
                    min="0"
                    max="2"
                    value={draft.ai.temperature}
                    onChange={(e) => setDraft(p => ({ ...p!, ai: { ...p!.ai, temperature: Number(e.target.value) } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Max tokens</span>
                  <input
                    name="max-tokens"
                    type="number"
                    inputMode="numeric"
                    min="256"
                    max="8192"
                    value={draft.ai.max_tokens}
                    onChange={(e) => setDraft(p => ({ ...p!, ai: { ...p!.ai, max_tokens: Number(e.target.value) } }))}
                  />
                </label>
              </div>

              <div className="subsec">
                <h3 className="subs-title">API key</h3>
                <p className="subs-note">
                  Keys stay in the OS credential store. Account: <code className="mono">{credentialAccount(draft.ai.provider)}</code>
                </p>
                <div className="row">
                  <input
                    type="password"
                    name="api-key"
                    autoComplete="off"
                    spellCheck={false}
                    aria-label="API key"
                    placeholder={hasKey ? "Key already stored — enter to replace" : "Paste API key"}
                    value={providerKey}
                    onChange={(e) => setProviderKey(e.target.value)}
                  />
                  <button className="btn btn-quiet" type="button" onClick={saveKey}>
                    Save key
                  </button>
                  <button className="btn btn-ghost" type="button" onClick={testProvider} disabled={probing}>
                    {probing ? "Testing…" : "Test connection"}
                  </button>
                </div>
                {probe && (
                  <div className={cn("probe", probe.ok ? "probe-ok" : "probe-bad")}>
                    <span className="caps">Status</span>
                    <span>{probe.ok ? "OK" : "Failed"}</span>
                    {probe.models.length > 0 && <span className="probe-models">Models: {probe.models.slice(0, 4).join(", ")}{probe.models.length > 4 ? "…" : ""}</span>}
                    <span>{probe.message}</span>
                  </div>
                )}
              </div>
            </section>
          )}

          {/* Voice */}
          {tab === "voice" && (
            <section className="ssec">
              <h2 className="ssec-title">Voice</h2>
              <div className="grid-2">
                <label className="field">
                  <span className="field-label caps">TTS voice</span>
                  <select
                    name="tts-voice"
                    value={draft.speech.voice}
                    onChange={(e) => setDraft(p => ({ ...p!, speech: { ...p!.speech, voice: e.target.value } }))}
                  >
                    {voices.map((v) => (
                      <option key={v.short_name} value={v.short_name}>
                        {v.friendly} · {v.locale} · {v.gender}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field checkbox">
                  <input
                    type="checkbox"
                    name="auto-speak"
                    checked={draft.speech.auto_speak}
                    onChange={(e) => setDraft(p => ({ ...p!, speech: { ...p!.speech, auto_speak: e.target.checked } }))}
                  />
                  <span>Auto-speak assistant replies</span>
                </label>
              </div>
            </section>
          )}

          {/* Permissions */}
          {tab === "permissions" && (
            <section className="ssec">
              <h2 className="ssec-title">Permissions</h2>
              <p className="ssec-note">Tool overrides persist. Session approvals can be cleared.</p>
              <button className="btn btn-quiet" type="button" onClick={clearSessionPerms}>
                Clear session permissions
              </button>
            </section>
          )}

          {/* Privacy */}
          {tab === "privacy" && (
            <section className="ssec">
              <h2 className="ssec-title">Privacy</h2>
              <div className="grid-1">
                <label className="field checkbox">
                  <input
                    type="checkbox"
                    name="local-only"
                    checked={draft.privacy.local_only}
                    onChange={(e) => setDraft(p => ({ ...p!, privacy: { ...p!.privacy, local_only: e.target.checked } }))}
                  />
                  <span>Prefer local providers (Ollama) when possible</span>
                </label>
                <label className="field checkbox">
                  <input
                    type="checkbox"
                    name="save-screenshots"
                    checked={draft.privacy.save_screenshots}
                    onChange={(e) => setDraft(p => ({ ...p!, privacy: { ...p!.privacy, save_screenshots: e.target.checked } }))}
                  />
                  <span>Save screenshots to disk</span>
                </label>
                <label className="field checkbox">
                  <input
                    type="checkbox"
                    name="save-recordings"
                    checked={draft.privacy.save_recordings}
                    onChange={(e) => setDraft(p => ({ ...p!, privacy: { ...p!.privacy, save_recordings: e.target.checked } }))}
                  />
                  <span>Save recordings to disk</span>
                </label>
              </div>
            </section>
          )}

          {/* Look & Feel */}
          {tab === "look" && (
            <section className="ssec">
              <h2 className="ssec-title">Look & Feel</h2>
              <div className="grid-2">
                <label className="field">
                  <span className="field-label caps">Theme</span>
                  <select
                    name="theme"
                    value={draft.appearance.theme}
                    onChange={(e) => setDraft(p => ({ ...p!, appearance: { ...p!.appearance, theme: e.target.value } }))}
                  >
                    <option value="dark">Dark</option>
                    <option value="light">Light</option>
                    <option value="system">System</option>
                  </select>
                </label>
                <label className="field">
                  <span className="field-label caps">Accent</span>
                  <select
                    name="accent"
                    value={draft.appearance.accent}
                    onChange={(e) => setDraft(p => ({ ...p!, appearance: { ...p!.appearance, accent: e.target.value } }))}
                  >
                    <option value="vermilion">Vermilion</option>
                    <option value="green">Bottle green</option>
                    <option value="amber">Amber</option>
                  </select>
                </label>
                <label className="field">
                  <span className="field-label caps">Reduced motion</span>
                  <select
                    name="reduced-motion"
                    value={draft.appearance.reduced_motion}
                    onChange={(e) => setDraft(p => ({ ...p!, appearance: { ...p!.appearance, reduced_motion: e.target.value } }))}
                  >
                    <option value="system">System</option>
                    <option value="on">On</option>
                    <option value="off">Off</option>
                  </select>
                </label>
                <label className="field">
                  <span className="field-label caps">Font scale</span>
                  <input
                    name="font-scale"
                    type="number"
                    inputMode="decimal"
                    step="0.05"
                    min="0.9"
                    max="1.2"
                    value={draft.appearance.font_scale}
                    onChange={(e) => setDraft(p => ({ ...p!, appearance: { ...p!.appearance, font_scale: Number(e.target.value) } }))}
                  />
                </label>
              </div>
            </section>
          )}

          {/* Agents */}
          {tab === "agents" && (
            <section className="ssec">
              <h2 className="ssec-title">Agents</h2>
              <p className="ssec-note">Discovered CLI agents on this machine.</p>
              <div className="list">
                {agents.map((a) => (
                  <div key={a.id} className="list-row">
                    <span className="mono">{a.label}</span>
                    <span className="caps">{a.found ? "found" : "missing"}</span>
                    <code className="mono">{a.command}</code>
                    <span>{a.note}</span>
                  </div>
                ))}
              </div>
            </section>
          )}

          {/* Research */}
          {tab === "research" && (
            <section className="ssec">
              <h2 className="ssec-title">Research</h2>
              <div className="grid-2">
                <label className="field">
                  <span className="field-label caps">Results count</span>
                  <input
                    name="results-count"
                    type="number"
                    inputMode="numeric"
                    min="3"
                    max="12"
                    value={draft.research.results_count}
                    onChange={(e) => setDraft(p => ({ ...p!, research: { ...p!.research, results_count: Number(e.target.value) } }))}
                  />
                </label>
                <label className="field">
                  <span className="field-label caps">Max page chars</span>
                  <input
                    name="max-page-chars"
                    type="number"
                    inputMode="numeric"
                    min="10000"
                    max="80000"
                    value={draft.research.max_page_chars}
                    onChange={(e) => setDraft(p => ({ ...p!, research: { ...p!.research, max_page_chars: Number(e.target.value) } }))}
                  />
                </label>
              </div>
            </section>
          )}

          {/* Diagnostics */}
          {tab === "diagnostics" && (
            <section className="ssec">
              <h2 className="ssec-title">Diagnostics</h2>
              <div className="row">
                <button className="btn btn-ghost" type="button" onClick={reloadDiag}>
                  <IconRefresh size={15} /> Reload
                </button>
              </div>

              <div className="subsec">
                <h3 className="subs-title">System capabilities</h3>
                <div className="list">
                  {capabilities.map((c) => (
                    <div key={c.key} className="list-row">
                      <span><IconCpu size={14} /> {c.label}</span>
                      <span className={cn("caps", `s-${c.state}`)}>{c.state}</span>
                      <span>{c.detail}</span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="subsec">
                <h3 className="subs-title">Tool catalog</h3>
                <div className="list">
                  {catalog.map((t) => (
                    <div key={t.name} className="list-row">
                      <code className="mono">{t.name}</code>
                      <span className="caps">{t.risk}</span>
                      <span>{t.description}</span>
                      <button
                        className="btn btn-tiny"
                        type="button"
                        onClick={() => getMeta(t.name, t.risk)}
                      >
                        meta
                      </button>
                      {meta[`${t.name}|${t.risk}`] && (
                        <pre className="mono meta">{meta[`${t.name}|${t.risk}`]}</pre>
                      )}
                    </div>
                  ))}
                </div>
              </div>

              <div className="subsec">
                <h3 className="subs-title">Recent logs</h3>
                <div className="log-table">
                  <div className="log-head caps">
                    <span>Time</span>
                    <span>Level</span>
                    <span>Target</span>
                    <span>Message</span>
                  </div>
                  {logs.map((l, idx) => (
                    <div key={idx} className="log-row">
                      <span className="mono tnum">{l.timestamp.slice(-8)}</span>
                      <span className={cn("caps", `lvl-${l.level}`)}>{l.level}</span>
                      <code className="mono">{l.target}</code>
                      <span>{l.message}</span>
                    </div>
                  ))}
                </div>
              </div>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}

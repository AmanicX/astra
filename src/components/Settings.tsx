import { useEffect, useRef, useState, type ReactNode } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { api, errorText } from "../lib/api";
import { applyAppearance } from "../lib/appearance";
import { cn, credentialAccount } from "../lib/format";
import { useStore } from "../lib/store";
import type {
  AgentInfo,
  Capability,
  LogLine,
  Memory,
  ProviderProbeResult,
  Settings as SettingsShape,
  ToolCatalogEntry,
  VoiceInfo,
} from "../lib/types";
import {
  IconCpu,
  IconFolder,
  IconInfo,
  IconRefresh,
  IconSearch,
  IconSettings,
  IconShield,
  IconSliders,
  IconSparkle,
  IconTerminal,
  IconTrash,
  IconUser,
  IconWave,
} from "./Icons";

const SECTIONS = [
  { id: "appearance", label: "Appearance", icon: <IconSliders size={15} /> },
  { id: "models", label: "AI & Models", icon: <IconSparkle size={15} /> },
  { id: "assistant", label: "Assistant", icon: <IconUser size={15} /> },
  { id: "voice", label: "Voice & Mic", icon: <IconWave size={15} /> },
  { id: "memory", label: "Memory & Privacy", icon: <IconFolder size={15} /> },
  { id: "permissions", label: "Permissions", icon: <IconShield size={15} /> },
  { id: "research", label: "Research & Web", icon: <IconSearch size={15} /> },
  { id: "agents", label: "Tools & Agents", icon: <IconTerminal size={15} /> },
  { id: "shortcuts", label: "Shortcuts", icon: <IconSettings size={15} /> },
  { id: "system", label: "System", icon: <IconCpu size={15} /> },
  { id: "about", label: "About", icon: <IconInfo size={15} /> },
] as const;

type SectionId = (typeof SECTIONS)[number]["id"];

function capTone(state: string): "ok" | "warn" | "off" {
  const s = state.toLowerCase();
  if (s.startsWith("available")) return "ok";
  if (s === "limited") return "warn";
  return "off";
}

export function Settings({ initialSection = "appearance" }: { initialSection?: SectionId }) {
  const settings = useStore((s) => s.settings);
  const applySettings = useStore((s) => s.applySettings);
  const showToast = useStore((s) => s.showToast);
  const setView = useStore((s) => s.setView);

  const [draft, setDraft] = useState<SettingsShape | null>(settings ?? null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [active, setActive] = useState<SectionId>(initialSection);
  const [flash, setFlash] = useState<SectionId | null>(null);
  const [appVersion, setAppVersion] = useState("");

  const [providerKey, setProviderKey] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [probing, setProbing] = useState(false);
  const [probe, setProbe] = useState<ProviderProbeResult | null>(null);

  const [voices, setVoices] = useState<VoiceInfo[]>([]);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [catalog, setCatalog] = useState<ToolCatalogEntry[]>([]);
  const [capabilities, setCapabilities] = useState<Capability[]>([]);
  const [agents, setAgents] = useState<AgentInfo[]>([]);

  const scrollRef = useRef<HTMLDivElement>(null);
  const flashTimer = useRef<number>(0);
  const didInit = useRef(false);

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  useEffect(() => {
    if (draft) applyAppearance(draft.appearance);
  }, [draft?.appearance]);

  useEffect(() => () => applyAppearance(useStore.getState().settings?.appearance), []);

  useEffect(() => {
    if (!draft) return;
    void (async () => {
      setHasKey(await api.credentialHas(credentialAccount(draft.ai.provider)));
    })();
  }, [draft?.ai.provider]);

  useEffect(() => {
    void (async () => {
      setVoices(await api.ttsVoices());
      setMemories(await api.memoryList());
      setLogs(await api.logsRecent());
      setCatalog(await api.toolCatalog());
      setCapabilities(await api.systemDiscover());
      setAgents(await api.agentDiscover());
      try {
        setAppVersion(await getVersion());
      } catch {
        setAppVersion("");
      }
    })();
  }, []);

  useEffect(() => {
    if (didInit.current || !draft) return;
    didInit.current = true;
    if (initialSection !== "appearance") {
      requestAnimationFrame(() => scrollTo(initialSection, false));
    }
  }, [draft]);

  useEffect(() => {
    const root = scrollRef.current;
    if (!root || !draft) return;
    const compute = () => {
      const rootTop = root.getBoundingClientRect().top;
      let current: SectionId = SECTIONS[0].id;
      for (const s of SECTIONS) {
        const el = document.getElementById(`sec-${s.id}`);
        if (el && el.getBoundingClientRect().top - rootTop <= 24) current = s.id;
      }
      if (root.scrollHeight - root.scrollTop - root.clientHeight <= 2) {
        current = SECTIONS[SECTIONS.length - 1].id;
      }
      setActive(current);
    };
    const io = new IntersectionObserver(compute, {
      root,
      rootMargin: "0px 0px -55% 0px",
      threshold: [0, 0.2, 0.5, 1],
    });
    for (const s of SECTIONS) {
      const el = document.getElementById(`sec-${s.id}`);
      if (el) io.observe(el);
    }
    let frame = 0;
    const onScroll = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(compute);
    };
    root.addEventListener("scroll", onScroll, { passive: true });
    compute();
    return () => {
      io.disconnect();
      root.removeEventListener("scroll", onScroll);
      cancelAnimationFrame(frame);
    };
  }, [draft]);

  const patch = <K extends keyof SettingsShape>(key: K, value: Partial<SettingsShape[K]>) => {
    setDraft((p) => (p ? ({ ...p, [key]: { ...(p[key] as object), ...value } } as SettingsShape) : p));
  };

  const motionReduced = () => {
    const a = draft?.appearance.reduced_motion ?? "system";
    if (a === "on") return true;
    if (a === "off") return false;
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  };

  const scrollTo = (id: SectionId, flashIt = true) => {
    const el = document.getElementById(`sec-${id}`);
    if (!el) return;
    el.scrollIntoView({ behavior: motionReduced() ? "auto" : "smooth", block: "start" });
    setActive(id);
    if (flashIt) {
      setFlash(id);
      window.clearTimeout(flashTimer.current);
      flashTimer.current = window.setTimeout(() => setFlash(null), 1100);
    }
  };

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
    setHasKey(await api.credentialHas(credentialAccount(draft.ai.provider)));
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

  const deleteMemory = async (id: string) => {
    await api.memoryDelete(id);
    setMemories(await api.memoryList());
  };

  const getMeta = async (name: string, risk: string) => {
    const m = await api.toolMeta(name, risk);
    showToast(`${name}: ${m.engine_allows ? "engine allows" : "engine blocks"} · ${m.why}`);
  };

  const dirty = !!draft && !!settings && JSON.stringify(draft) !== JSON.stringify(settings);

  const close = () => {
    if (dirty && !confirm("Discard unsaved changes?")) return;
    setView("chat");
  };

  if (!draft) return null;

  const a = draft;
  const foundAgents = agents.filter((x) => x.found);

  return (
    <div className="settings">
      <header className="settings-head">
        <div className="settings-head-main">
          <p className="caps settings-eyebrow">Preferences</p>
          <h1 className="settings-title">Settings</h1>
          <p className="settings-lede">
            Everything Astra does, in one place. Changes apply when you save.
          </p>
        </div>
        <div className="settings-actions">
          <span className={cn("settings-status", dirty ? "settings-status-dirty" : "settings-status-clean")}>
            {dirty ? "Unsaved changes" : "All changes saved"}
          </span>
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
      </header>

      <nav className="settings-nav" aria-label="Settings sections">
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            type="button"
            className={cn("snav", active === s.id && "snav-active")}
            aria-current={active === s.id ? "true" : undefined}
            onClick={() => scrollTo(s.id)}
          >
            <span className="snav-icon" aria-hidden="true">{s.icon}</span>
            <span>{s.label}</span>
          </button>
        ))}
      </nav>

      <div className="settings-scroll scroll" ref={scrollRef}>
        <div className="settings-inner">
          {error && <div className="panel-error">{error}</div>}

          {/* ---- Appearance ---- */}
          <Section id="appearance" title="Appearance" desc="Theme, accent and motion." icon={<IconSliders size={16} />} flash={flash}>
            <Group>
              <FieldRow id="set-theme" label="Theme" desc="Colour scheme for the whole interface.">
                <select id="set-theme" value={a.appearance.theme} onChange={(e) => patch("appearance", { theme: e.target.value })}>
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                  <option value="system">Match system</option>
                </select>
              </FieldRow>
              <FieldRow id="set-accent" label="Accent colour" desc="Used for emphasis, focus and the send action.">
                <select id="set-accent" value={a.appearance.accent} onChange={(e) => patch("appearance", { accent: e.target.value })}>
                  <option value="vermilion">Vermilion</option>
                  <option value="green">Bottle green</option>
                  <option value="amber">Amber</option>
                </select>
              </FieldRow>
              <RangeRow
                id="set-font-scale"
                label="Interface scale"
                desc="Sizes type across the app."
                value={a.appearance.font_scale}
                min={0.9}
                max={1.2}
                step={0.05}
                onChange={(v) => patch("appearance", { font_scale: v })}
                format={(v) => `${Math.round(v * 100)}%`}
              />
            </Group>
            <Group>
              <FieldRow id="set-motion" label="Motion" desc="Animations and smooth scrolling.">
                <select id="set-motion" value={a.appearance.reduced_motion} onChange={(e) => patch("appearance", { reduced_motion: e.target.value })}>
                  <option value="system">Follow system</option>
                  <option value="on">Reduce motion</option>
                  <option value="off">Full motion</option>
                </select>
              </FieldRow>
              <SwitchRow
                id="set-tray"
                label="Minimise to tray"
                desc="Keep Astra running in the tray when the window closes."
                checked={a.background.minimize_to_tray}
                onChange={(v) => patch("background", { minimize_to_tray: v })}
              />
            </Group>
          </Section>

          {/* ---- AI & Models ---- */}
          <Section id="models" title="AI & Models" desc="Provider, models and connection." icon={<IconSparkle size={16} />} flash={flash}>
            <Group>
              <FieldRow id="set-provider" label="Provider">
                <select id="set-provider" value={a.ai.provider} onChange={(e) => { setProbe(null); patch("ai", { provider: e.target.value }); }}>
                  <option value="groq">Groq</option>
                  <option value="openai">OpenAI</option>
                  <option value="openai_compat">OpenAI-compatible</option>
                  <option value="ollama">Ollama (local)</option>
                  <option value="custom">Custom endpoint</option>
                  <option value="mock">Offline test provider</option>
                </select>
              </FieldRow>
              <FieldRow id="set-base-url" label="Base URL" desc="Request endpoint for the selected provider.">
                <input id="set-base-url" type="url" inputMode="url" autoComplete="off" spellCheck={false} value={a.ai.base_url} onChange={(e) => patch("ai", { base_url: e.target.value })} />
              </FieldRow>
              <FieldRow id="set-chat-model" label="Chat model">
                <input id="set-chat-model" autoComplete="off" spellCheck={false} value={a.ai.chat_model} onChange={(e) => patch("ai", { chat_model: e.target.value })} />
              </FieldRow>
              <FieldRow id="set-vision-model" label="Vision model" desc="Used when an image is attached.">
                <input id="set-vision-model" autoComplete="off" spellCheck={false} value={a.ai.vision_model} onChange={(e) => patch("ai", { vision_model: e.target.value })} />
              </FieldRow>
            </Group>
            <Group>
              <RangeRow
                id="set-temperature"
                label="Temperature"
                desc="Lower is focused, higher is exploratory."
                value={a.ai.temperature}
                min={0}
                max={2}
                step={0.1}
                onChange={(v) => patch("ai", { temperature: v })}
                format={(v) => v.toFixed(1)}
              />
              <FieldRow id="set-max-tokens" label="Max response tokens">
                <input id="set-max-tokens" type="number" inputMode="numeric" min={256} max={8192} value={a.ai.max_tokens} onChange={(e) => patch("ai", { max_tokens: Number(e.target.value) })} />
              </FieldRow>
              <FieldRow id="set-timeout" label="Request timeout" desc="Seconds before a stalled request is abandoned.">
                <input id="set-timeout" type="number" inputMode="numeric" min={10} max={600} value={a.ai.timeout_sec} onChange={(e) => patch("ai", { timeout_sec: Number(e.target.value) })} />
              </FieldRow>
            </Group>
            <div className={cn("sgroup sgroup-block", hasKey && "sgroup-ok")}>
              <div className="sblock-head">
                <span className="sblock-title">API key</span>
                <span className={cn("caps", hasKey ? "s-ok" : "s-missing")}>{hasKey ? "Stored" : "Not set"}</span>
              </div>
              <p className="sblock-note">
                Keys are held in the OS credential store, never in the database or logs. Account{" "}
                <code className="mono">{credentialAccount(a.ai.provider)}</code>.
              </p>
              <div className="sblock-row">
                <input
                  type="password"
                  name="api-key"
                  autoComplete="off"
                  spellCheck={false}
                  aria-label="API key"
                  placeholder={hasKey ? "Enter a new key to replace the stored one" : "Paste API key"}
                  value={providerKey}
                  onChange={(e) => setProviderKey(e.target.value)}
                />
                <button className="btn btn-quiet" type="button" onClick={saveKey} disabled={!providerKey.trim()}>
                  Save key
                </button>
                <button className="btn btn-ghost" type="button" onClick={testProvider} disabled={probing}>
                  <IconRefresh size={14} /> {probing ? "Testing…" : "Test"}
                </button>
              </div>
              {probe && (
                <div className={cn("probe", probe.ok ? "probe-ok" : "probe-bad")}>
                  <span className="caps">Status</span>
                  <span>{probe.ok ? "Reachable" : "Unreachable"}</span>
                  {probe.models.length > 0 && (
                    <span className="probe-models">Models: {probe.models.slice(0, 4).join(", ")}{probe.models.length > 4 ? "…" : ""}</span>
                  )}
                  <span>{probe.message}</span>
                </div>
              )}
            </div>
          </Section>

          {/* ---- Assistant ---- */}
          <Section id="assistant" title="Assistant" desc="How Astra sounds and behaves." icon={<IconUser size={16} />} flash={flash}>
            <Group>
              <FieldRow id="set-name" label="Name">
                <input id="set-name" autoComplete="off" spellCheck={false} value={a.assistant.name} onChange={(e) => patch("assistant", { name: e.target.value })} />
              </FieldRow>
              <FieldRow id="set-tone" label="Tone">
                <input id="set-tone" autoComplete="off" value={a.assistant.tone} onChange={(e) => patch("assistant", { tone: e.target.value })} />
              </FieldRow>
              <FieldRow id="set-personality" label="Personality">
                <input id="set-personality" autoComplete="off" value={a.assistant.personality} onChange={(e) => patch("assistant", { personality: e.target.value })} />
              </FieldRow>
              <FieldRow id="set-depth" label="Explanation depth">
                <select id="set-depth" value={a.assistant.explanation_depth} onChange={(e) => patch("assistant", { explanation_depth: e.target.value })}>
                  <option value="adaptive">Adaptive</option>
                  <option value="brief">Brief</option>
                  <option value="detailed">Detailed</option>
                  <option value="technical">Technical</option>
                </select>
              </FieldRow>
              <FieldRow id="set-language" label="Reply language" desc="ISO code, e.g. en, de, ja.">
                <input id="set-language" autoComplete="off" spellCheck={false} value={a.assistant.language} onChange={(e) => patch("assistant", { language: e.target.value })} />
              </FieldRow>
            </Group>
            <Group>
              <RangeRow id="set-formality" label="Formality" value={a.assistant.formality} min={0} max={100} step={1} onChange={(v) => patch("assistant", { formality: v })} format={(v) => `${v}`} />
              <RangeRow id="set-conciseness" label="Conciseness" value={a.assistant.conciseness} min={0} max={100} step={1} onChange={(v) => patch("assistant", { conciseness: v })} format={(v) => `${v}`} />
              <RangeRow id="set-humor" label="Humor" value={a.assistant.humor} min={0} max={100} step={1} onChange={(v) => patch("assistant", { humor: v })} format={(v) => `${v}`} />
              <SwitchRow
                id="set-proactive"
                label="Proactive"
                desc="Offer the next useful step without being asked."
                checked={a.assistant.proactive}
                onChange={(v) => patch("assistant", { proactive: v })}
              />
            </Group>
            <FieldRow id="set-sysprompt" label="Custom system prompt" desc="Appended to Astra's built-in instructions. Leave empty to use defaults." stack>
              <textarea
                id="set-sysprompt"
                rows={4}
                value={a.assistant.custom_system_prompt}
                placeholder="e.g. Always answer in British English and cite sources."
                onChange={(e) => patch("assistant", { custom_system_prompt: e.target.value })}
              />
            </FieldRow>
          </Section>

          {/* ---- Voice ---- */}
          <Section id="voice" title="Voice & Microphone" desc="Speech output and dictation." icon={<IconWave size={16} />} flash={flash}>
            <Group>
              <FieldRow id="set-voice" label="Speech voice" desc={`${voices.length} voices available`}>
                <select id="set-voice" value={a.speech.voice} onChange={(e) => patch("speech", { voice: e.target.value })}>
                  {voices.map((v) => (
                    <option key={v.short_name} value={v.short_name}>
                      {v.friendly} · {v.locale} · {v.gender}
                    </option>
                  ))}
                </select>
              </FieldRow>
              <SwitchRow
                id="set-autospeak"
                label="Auto-speak replies"
                desc="Read each assistant answer aloud as it completes."
                checked={a.speech.auto_speak}
                onChange={(v) => patch("speech", { auto_speak: v })}
              />
            </Group>
            <Group>
              <RangeRow id="set-rate" label="Speech rate" value={a.speech.rate} min={0.5} max={2} step={0.05} onChange={(v) => patch("speech", { rate: v })} format={(v) => `${v.toFixed(2)}×`} />
              <RangeRow id="set-volume" label="Speech volume" value={a.speech.volume} min={0} max={2} step={0.05} onChange={(v) => patch("speech", { volume: v })} format={(v) => `${Math.round(v * 100)}%`} />
              <FieldRow id="set-spoken-chars" label="Max spoken characters" desc="Longer replies are clipped before speaking.">
                <input id="set-spoken-chars" type="number" inputMode="numeric" min={100} max={4000} value={a.speech.max_spoken_chars} onChange={(e) => patch("speech", { max_spoken_chars: Number(e.target.value) })} />
              </FieldRow>
            </Group>
          </Section>

          {/* ---- Memory & Privacy ---- */}
          <Section id="memory" title="Memory & Privacy" desc="What Astra remembers, and what stays on this machine." icon={<IconFolder size={16} />} flash={flash}>
            <Group>
              <SwitchRow
                id="set-memory"
                label="Remember preferences"
                desc="Let Astra store durable facts about how you like to work."
                checked={a.memory.enabled}
                onChange={(v) => patch("memory", { enabled: v })}
              />
              <SwitchRow
                id="set-inject"
                label="Use memory in replies"
                desc="Inject stored preferences into each conversation."
                checked={a.memory.inject_preferences}
                disabled={!a.memory.enabled}
                onChange={(v) => patch("memory", { inject_preferences: v })}
              />
              <FieldRow id="set-max-injected" label="Memories per reply" desc="How many stored facts can be injected at once.">
                <input id="set-max-injected" type="number" inputMode="numeric" min={0} max={50} disabled={!a.memory.enabled || !a.memory.inject_preferences} value={a.memory.max_injected} onChange={(e) => patch("memory", { max_injected: Number(e.target.value) })} />
              </FieldRow>
            </Group>
            <Group>
              <SwitchRow
                id="set-local"
                label="Prefer local providers"
                desc="Keep prompts on this machine where a local model can serve them."
                checked={a.privacy.local_only}
                onChange={(v) => patch("privacy", { local_only: v })}
              />
              <SwitchRow
                id="set-cloud-screen"
                label="Allow screen to cloud"
                desc="Permit screenshots to be sent to a cloud vision model."
                checked={a.privacy.allow_screen_to_cloud}
                onChange={(v) => patch("privacy", { allow_screen_to_cloud: v })}
              />
              <SwitchRow
                id="set-save-shots"
                label="Save screenshots to disk"
                checked={a.privacy.save_screenshots}
                onChange={(v) => patch("privacy", { save_screenshots: v })}
              />
            </Group>
            <div className="sgroup sgroup-list">
              <div className="sblock-head">
                <span className="sblock-title">Stored memories</span>
                <span className="caps">{memories.length} {memories.length === 1 ? "entry" : "entries"}</span>
              </div>
              {memories.length === 0 ? (
                <p className="sblock-note">Nothing remembered yet. Preferences Astra learns will appear here.</p>
              ) : (
                <div className="mem-list">
                  {memories.map((m) => (
                    <div key={m.id} className="mem-row">
                      <span className="caps mem-cat">{m.category}</span>
                      <span className="mem-key mono">{m.key}</span>
                      <span className="mem-value">{m.value}</span>
                      <button className="icon-btn danger" type="button" title="Forget" aria-label={`Forget ${m.key}`} onClick={() => void deleteMemory(m.id)}>
                        <IconTrash size={13} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </Section>

          {/* ---- Permissions ---- */}
          <Section id="permissions" title="Permissions & Safety" desc="What Astra may do on this machine, and how it asks first." icon={<IconShield size={16} />} flash={flash}>
            <Group>
              <div className="srow">
                <div className="srow-main">
                  <span className="srow-label">Remembered tool decisions</span>
                  <span className="srow-desc">{a.permissions.tool_overrides.length} saved override{a.permissions.tool_overrides.length === 1 ? "" : "s"}</span>
                </div>
                <div className="srow-control srow-control-wide">
                  {a.permissions.tool_overrides.length === 0 ? (
                    <span className="muted">None — every action asks first</span>
                  ) : (
                    <div className="chip-wrap">
                      {a.permissions.tool_overrides.map(([tool, mode]) => (
                        <span key={tool} className="mini-chip">
                          <code className="mono">{tool}</code>
                          <span className="caps">{mode}</span>
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
              <div className="srow">
                <div className="srow-main">
                  <span className="srow-label">Session approvals</span>
                  <span className="srow-desc">Approvals given this session only. Clearing restores prompting.</span>
                </div>
                <div className="srow-control">
                  <button className="btn btn-quiet" type="button" onClick={clearSessionPerms}>
                    Clear session
                  </button>
                </div>
              </div>
            </Group>
            <div className="sgroup sgroup-list">
              <div className="sblock-head">
                <span className="sblock-title">Tools by risk</span>
                <span className="caps">{catalog.length} tools</span>
              </div>
              <div className="tool-list">
                {catalog.map((t) => (
                  <div key={t.name} className="tool-row">
                    <span className={cn("risk-dot", `risk-${t.risk}`)} aria-hidden="true" />
                    <code className="mono tool-row-name">{t.name}</code>
                    <span className={cn("caps", `risk-text-${t.risk}`)}>{t.risk}</span>
                    <span className="tool-row-desc">{t.description}</span>
                    <button className="btn btn-tiny" type="button" onClick={() => void getMeta(t.name, t.risk)} aria-label={`Details for ${t.name}`}>
                      why
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </Section>

          {/* ---- Research ---- */}
          <Section id="research" title="Research & Web" desc="How Astra searches and reads the web." icon={<IconSearch size={16} />} flash={flash}>
            <Group>
              <FieldRow id="set-results" label="Results per search" desc="How many sources to gather before synthesising.">
                <input id="set-results" type="number" inputMode="numeric" min={3} max={12} value={a.research.results_count} onChange={(e) => patch("research", { results_count: Number(e.target.value) })} />
              </FieldRow>
              <FieldRow id="set-page-chars" label="Max page characters" desc="Reading budget per fetched page.">
                <input id="set-page-chars" type="number" inputMode="numeric" min={10000} max={80000} step={1000} value={a.research.max_page_chars} onChange={(e) => patch("research", { max_page_chars: Number(e.target.value) })} />
              </FieldRow>
            </Group>
          </Section>

          {/* ---- Agents ---- */}
          <Section id="agents" title="Tools & Agents" desc="Coding agents Astra can delegate to." icon={<IconTerminal size={16} />} flash={flash}>
            <Group>
              <FieldRow id="set-preferred" label="Preferred agent" desc="Used when no agent is named explicitly.">
                <select id="set-preferred" value={a.agents.preferred} onChange={(e) => patch("agents", { preferred: e.target.value })}>
                  <option value="auto">Auto</option>
                  {agents.map((ag) => (
                    <option key={ag.id} value={ag.id} disabled={!ag.found}>
                      {ag.label}{ag.found ? "" : " (unavailable)"}
                    </option>
                  ))}
                </select>
              </FieldRow>
              <FieldRow id="set-project-dir" label="Default project folder" desc="Working directory for agent runs.">
                <input id="set-project-dir" autoComplete="off" spellCheck={false} value={a.agents.default_project_dir} onChange={(e) => patch("agents", { default_project_dir: e.target.value })} />
              </FieldRow>
            </Group>
            <div className="sgroup sgroup-list">
              <div className="sblock-head">
                <span className="sblock-title">Detected agents</span>
                <span className="caps">{foundAgents.length} of {agents.length} available</span>
              </div>
              <div className="agent-list">
                {agents.map((ag) => (
                  <div key={ag.id} className={cn("agent-row", !ag.found && "agent-row-off")}>
                    <span className={cn("status-dot", ag.found ? "s-ok" : "s-missing")} aria-hidden="true" />
                    <code className="mono agent-name">{ag.label}</code>
                    <code className="mono agent-cmd">{ag.command}</code>
                    <span className="agent-note">{ag.note}</span>
                  </div>
                ))}
              </div>
            </div>
          </Section>

          {/* ---- Shortcuts ---- */}
          <Section id="shortcuts" title="Keyboard Shortcuts" desc="Registered globally at launch." icon={<IconSettings size={16} />} flash={flash}>
            <Group>
              <ShortcutRow label="Toggle overlay" keys={a.shortcuts.overlay} />
              <ShortcutRow label="Push to talk" keys={a.shortcuts.push_to_talk} />
              <ShortcutRow label="Stop speaking" keys={a.shortcuts.stop_speaking} />
              <ShortcutRow label="Send message" keys="Enter" />
              <ShortcutRow label="New line in composer" keys="Shift + Enter" />
            </Group>
          </Section>

          {/* ---- System ---- */}
          <Section id="system" title="System & Diagnostics" desc="Environment detection and recent activity." icon={<IconCpu size={16} />} flash={flash}>
            <div className="sblock-row">
              <button className="btn btn-ghost" type="button" onClick={reloadDiag}>
                <IconRefresh size={15} /> Reload
              </button>
            </div>
            <div className="sgroup sgroup-list">
              <div className="sblock-head">
                <span className="sblock-title">System capabilities</span>
                <span className="caps">{capabilities.length} checks</span>
              </div>
              <div className="cap-list">
                {capabilities.map((c) => (
                  <div key={c.key} className="cap-row">
                    <span className={cn("status-dot", `s-${capTone(c.state)}`)} aria-hidden="true" />
                    <span className="cap-label">{c.label}</span>
                    <span className={cn("caps", `s-${capTone(c.state)}`)}>{c.state}</span>
                    <span className="cap-detail">{c.detail}</span>
                  </div>
                ))}
              </div>
            </div>
            <div className="sgroup sgroup-list">
              <div className="sblock-head">
                <span className="sblock-title">Recent logs</span>
                <span className="caps">{logs.length} lines</span>
              </div>
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
          </Section>

          {/* ---- About ---- */}
          <Section id="about" title="About Astra" desc="Version and build information." icon={<IconInfo size={16} />} flash={flash}>
            <Group>
              <AboutRow label="Version" value={appVersion || "0.1.0"} />
              <AboutRow label="Assistant" value={a.assistant.name} />
              <AboutRow label="Provider" value={a.ai.provider} />
              <AboutRow label="Chat model" value={a.ai.chat_model} />
              <AboutRow label="Prompt cache" value="Local · no telemetry" />
            </Group>
            <p className="ssec-desc about-foot">
              Astra runs locally and reports nothing back. Credentials live in the OS credential store.
            </p>
          </Section>

          <div className="settings-foot">
            <button className="btn btn-ghost" type="button" onClick={() => scrollTo("appearance")}>
              Back to top
            </button>
            <span className="muted">Astra {appVersion || "0.1.0"}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function Section({
  id,
  title,
  desc,
  icon,
  flash,
  children,
}: {
  id: SectionId;
  title: string;
  desc?: string;
  icon: ReactNode;
  flash: SectionId | null;
  children: ReactNode;
}) {
  return (
    <section id={`sec-${id}`} className={cn("ssec", flash === id && "ssec-flash")} aria-labelledby={`sec-${id}-title`}>
      <header className="ssec-head">
        <span className="ssec-icon" aria-hidden="true">{icon}</span>
        <div className="ssec-head-text">
          <h2 id={`sec-${id}-title`} className="ssec-title">{title}</h2>
          {desc && <p className="ssec-desc">{desc}</p>}
        </div>
      </header>
      {children}
    </section>
  );
}

function Group({ children }: { children: ReactNode }) {
  return <div className="sgroup">{children}</div>;
}

function FieldRow({
  id,
  label,
  desc,
  stack,
  children,
}: {
  id: string;
  label: string;
  desc?: string;
  stack?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={cn("srow", stack && "srow-stack")}>
      <div className="srow-main">
        <label className="srow-label" htmlFor={id}>{label}</label>
        {desc && <span className="srow-desc">{desc}</span>}
      </div>
      <div className="srow-control">{children}</div>
    </div>
  );
}

function SwitchRow({
  id,
  label,
  desc,
  checked,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  desc?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className={cn("srow", disabled && "srow-disabled")}>
      <div className="srow-main">
        <label className="srow-label" htmlFor={id}>{label}</label>
        {desc && <span className="srow-desc">{desc}</span>}
      </div>
      <div className="srow-control">
        <input
          id={id}
          type="checkbox"
          className="switch"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
        />
      </div>
    </div>
  );
}

function RangeRow({
  id,
  label,
  desc,
  value,
  min,
  max,
  step,
  disabled,
  onChange,
  format,
}: {
  id: string;
  label: string;
  desc?: string;
  value: number;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  onChange: (value: number) => void;
  format: (value: number) => string;
}) {
  return (
    <div className={cn("srow", disabled && "srow-disabled")}>
      <div className="srow-main">
        <label className="srow-label" htmlFor={id}>{label}</label>
        {desc && <span className="srow-desc">{desc}</span>}
      </div>
      <div className="srow-control srow-range">
        <input
          id={id}
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          disabled={disabled}
          onChange={(e) => onChange(Number(e.target.value))}
        />
        <output className="srow-value tnum" htmlFor={id}>{format(value)}</output>
      </div>
    </div>
  );
}

function ShortcutRow({ label, keys }: { label: string; keys: string }) {
  return (
    <div className="srow">
      <div className="srow-main">
        <span className="srow-label">{label}</span>
      </div>
      <div className="srow-control">
        <span className="kbd-group">
          {keys.split("+").map((k, i) => (
            <kbd key={i}>{k.trim()}</kbd>
          ))}
        </span>
      </div>
    </div>
  );
}

function AboutRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="srow">
      <div className="srow-main">
        <span className="srow-label">{label}</span>
      </div>
      <div className="srow-control">
        <span className="mono about-value">{value}</span>
      </div>
    </div>
  );
}

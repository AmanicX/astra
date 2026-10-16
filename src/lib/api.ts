import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  AgentInfo,
  CaptureResult,
  Capability,
  ChatAttachment,
  Conversation,
  LogLine,
  Memory,
  Message,
  ProviderProbeResult,
  Settings,
  ToolCatalogEntry,
  ToolMeta,
  TurnEvent,
  TtsResult,
  VoiceInfo,
} from "./types";

/** Thin typed wrappers over the Tauri command surface (see `commands.rs`). */

export const api = {
  // ---- settings & credentials ----
  settingsGet: () => invoke<Settings>("settings_get"),
  settingsSet: (settings: Settings) => invoke<Settings>("settings_set", { settings }),
  settingsReset: () => invoke<Settings>("settings_reset"),
  credentialSet: (account: string, secret: string) =>
    invoke<void>("credential_set", { account, secret }),
  credentialHas: (account: string) => invoke<boolean>("credential_has", { account }),
  credentialDelete: (account: string) => invoke<void>("credential_delete", { account }),
  providerTest: (provider?: string) =>
    invoke<ProviderProbeResult>("provider_test", { provider: provider ?? null }),

  // ---- conversations & memory ----
  conversationList: (query?: string) =>
    invoke<Conversation[]>("conversation_list", { query: query ?? null }),
  conversationCreate: (title?: string, project?: string) =>
    invoke<Conversation>("conversation_create", {
      title: title ?? null,
      project: project ?? null,
    }),
  conversationRename: (id: string, title: string, pinned?: boolean) =>
    invoke<void>("conversation_rename", { id, title, pinned: pinned ?? null }),
  conversationDelete: (id: string) => invoke<void>("conversation_delete", { id }),
  conversationMessages: (id: string, limit?: number) =>
    invoke<Message[]>("conversation_messages", { id, limit: limit ?? null }),
  messageSearch: (query: string, limit?: number) =>
    invoke<Message[]>("message_search", { query, limit: limit ?? null }),

  memoryList: (category?: string) =>
    invoke<Memory[]>("memory_list", { category: category ?? null }),
  memorySet: (category: string, key: string, value: string) =>
    invoke<Memory>("memory_set", { category, key, value }),
  memoryDelete: (id: string) => invoke<void>("memory_delete", { id }),

  permissionsClearSession: () => invoke<void>("permissions_clear_session"),
  toolMeta: (name: string, risk: string) =>
    invoke<ToolMeta>("tool_meta", { name, risk }),

  // ---- turns ----
  turnRun: (
    conversationId: string,
    text: string,
    images: ChatAttachment[],
    onEvent: (e: TurnEvent) => void,
  ) => {
    const onEventChannel = new Channel<TurnEvent>();
    onEventChannel.onmessage = onEvent;
    return invoke<void>("turn_run", {
      onEvent: onEventChannel,
      conversationId,
      text,
      images: images.length ? images : null,
    });
  },
  turnCancel: (turnId: string) => invoke<void>("turn_cancel", { turnId }),
  approvalRespond: (approvalId: string, approved: boolean, remember?: string) =>
    invoke<void>("approval_respond", {
      approvalId,
      approved,
      remember: remember ?? null,
    }),
  activityList: () => invoke<ActivityItemList>("activity_list"),

  // ---- system & tools ----
  systemDiscover: () => invoke<Capability[]>("system_discover"),
  appDiscover: (query?: string) => invoke<unknown>("app_discover", { query: query ?? null }),
  appLaunch: (target: string) => invoke<unknown>("app_launch", { target }),
  audioGet: () => invoke<unknown>("audio_get"),
  audioSet: (level: number) => invoke<unknown>("audio_set", { level }),
  audioMute: (mute: boolean) => invoke<unknown>("audio_mute", { mute }),
  brightnessGet: () => invoke<unknown>("brightness_get"),
  brightnessSet: (level: number) => invoke<unknown>("brightness_set", { level }),
  wifiStatus: () => invoke<unknown>("network_wifi_status"),
  wifiSet: (action: string, ssid?: string) =>
    invoke<unknown>("network_wifi_set", { action, ssid: ssid ?? null }),
  bluetoothStatus: () => invoke<unknown>("network_bluetooth_status"),
  bluetoothSet: (on: boolean) => invoke<unknown>("network_bluetooth_set", { on }),
  wallpaperGet: () => invoke<unknown>("wallpaper_get"),
  wallpaperSet: (path: string) => invoke<unknown>("wallpaper_set", { path }),
  toolCatalog: () => invoke<ToolCatalogEntry[]>("tool_catalog"),

  // ---- screen, research, agents ----
  screenCapture: (monitor?: number) =>
    invoke<CaptureResult>("screen_capture", { monitor: monitor ?? null }),
  researchSearch: (query: string, count?: number) =>
    invoke<unknown>("research_search", { query, count: count ?? null }),
  researchFetch: (url: string) => invoke<unknown>("research_fetch", { url }),
  agentDiscover: () => invoke<AgentInfo[]>("agent_discover"),
  agentRun: (
    prompt: string,
    agent: string | undefined,
    cwd: string | undefined,
    onEvent: (e: TurnEvent) => void,
  ) => {
    const channel = new Channel<TurnEvent>();
    channel.onmessage = onEvent;
    return invoke<unknown>("agent_run", {
      onEvent: channel,
      agent: agent ?? null,
      prompt,
      cwd: cwd ?? null,
    });
  },
  agentCancel: () => invoke<void>("agent_cancel"),
  agentHistory: () => invoke<unknown[]>("agent_history"),

  // ---- voice ----
  ttsVoices: () => invoke<VoiceInfo[]>("tts_voices"),
  ttsSpeak: (text: string, voice?: string) =>
    invoke<TtsResult>("tts_speak", { text, voice: voice ?? null }),
  sttTranscribe: (audioBase64: string, mime?: string, filename?: string) =>
    invoke<string>("stt_transcribe", {
      audioBase64,
      mime: mime ?? null,
      filename: filename ?? null,
    }),

  // ---- window & app ----
  overlayToggle: () => invoke<void>("overlay_toggle"),
  overlayHide: () => invoke<void>("overlay_hide"),
  windowMinimize: () => invoke<void>("window_minimize"),
  windowToggleMaximize: () => invoke<void>("window_toggle_maximize"),
  windowClose: () => invoke<void>("window_close"),
  logsRecent: () => invoke<LogLine[]>("logs_recent"),
  rememberDecision: (tool: string, policy: string) =>
    invoke<Settings>("remember_decision", { tool, policy }),
  quitApp: () => invoke<void>("quit_app"),
};

type ActivityItemList = import("./types").ActivityItem[];

/** Normalize an unknown thrown value (Tauri rejects with strings) into text. */
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

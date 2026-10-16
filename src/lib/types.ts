// TypeScript mirrors of the Rust DTOs. Field names match the serde output
// (Rust snake_case), not the JS camelCase used for command *arguments*.

export type Risk = "low" | "moderate" | "high";
export type ToolPolicy = "auto" | "ask" | "always";
export type TurnStatus = "completed" | "cancelled" | "failed";
export type ToolStatus = "ok" | "error" | "denied" | "cancelled";

export interface AssistantSettings {
  name: string;
  preset: string;
  personality: string;
  tone: string;
  formality: number;
  conciseness: number;
  humor: number;
  proactive: boolean;
  explanation_depth: string;
  language: string;
  custom_system_prompt: string;
}

export interface AiSettings {
  provider: string;
  base_url: string;
  chat_model: string;
  vision_model: string;
  temperature: number;
  max_tokens: number;
  timeout_sec: number;
  role_models: [string, string][];
}

export interface WakeWordSettings {
  enabled: boolean;
  engine: string;
  energy_threshold: number;
}

export interface SpeechSettings {
  stt_provider: string;
  tts_provider: string;
  voice: string;
  rate: number;
  volume: number;
  auto_speak: boolean;
  max_spoken_chars: number;
  wake_word: WakeWordSettings;
  input_device: string | null;
}

export interface ShortcutSettings {
  overlay: string;
  push_to_talk: string;
  stop_speaking: string;
}

export interface PermissionSettings {
  tool_overrides: [string, string][];
}

export interface PrivacySettings {
  local_only: boolean;
  allow_screen_to_cloud: boolean;
  save_screenshots: boolean;
  save_recordings: boolean;
  telemetry: boolean;
  log_level: string;
}

export interface MemorySettings {
  enabled: boolean;
  inject_preferences: boolean;
  max_injected: number;
}

export interface AppearanceSettings {
  theme: string;
  accent: string;
  reduced_motion: string;
  font_scale: number;
}

export interface BackgroundSettings {
  minimize_to_tray: boolean;
  autostart: boolean;
  notifications: boolean;
  quiet_hours_enabled: boolean;
  quiet_from: string;
  quiet_to: string;
}

export interface AgentSettings {
  preferred: string;
  default_project_dir: string;
  extra_agents: [string, string][];
}

export interface ResearchSettings {
  results_count: number;
  max_page_chars: number;
}

export interface DiagnosticsSettings {
  log_level: string;
}

export interface Settings {
  version: number;
  assistant: AssistantSettings;
  ai: AiSettings;
  speech: SpeechSettings;
  shortcuts: ShortcutSettings;
  permissions: PermissionSettings;
  privacy: PrivacySettings;
  memory: MemorySettings;
  appearance: AppearanceSettings;
  background: BackgroundSettings;
  agents: AgentSettings;
  research: ResearchSettings;
  diagnostics: DiagnosticsSettings;
}

export interface Conversation {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  pinned: boolean;
  project: string | null;
  archived: boolean;
  message_count: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  meta: unknown;
  created_at: string;
}

export interface Memory {
  id: string;
  category: string;
  key: string;
  value: string;
  source: string | null;
  created_at: string;
  updated_at: string;
}

export interface ActivityItem {
  id: string;
  turn_id: string;
  tool: string;
  label: string;
  status: string;
  detail: string | null;
  started_at: string;
  ended_at: string | null;
}

export interface LogLine {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

export interface Capability {
  key: string;
  label: string;
  state: string;
  detail: string;
}

export interface AgentInfo {
  id: string;
  label: string;
  command: string;
  template: string[];
  found: boolean;
  path: string | null;
  note: string;
}

export interface VoiceInfo {
  short_name: string;
  locale: string;
  gender: string;
  friendly: string;
}

export interface CaptureResult {
  monitor: number;
  width: number;
  height: number;
  captured_at: string;
  data_url: string;
  saved_to: string | null;
}

export interface ProviderProbeResult {
  ok: boolean;
  models: string[];
  message: string;
}

export interface TtsResult {
  audio_base64: string;
  mime: string;
  engine: string;
  voice: string;
}

export interface ToolCatalogEntry {
  name: string;
  description: string;
  risk: Risk;
  why: string;
}

export interface ToolMeta {
  why: string;
  engine_allows: boolean;
  risk: string;
}

export interface ChatAttachment {
  mime: string;
  data_base64: string;
}

// ---- Turn events (serde tag = "type", snake_case variants) ----

export type TurnEvent =
  | { type: "turn_started"; turn_id: string; conversation_id: string }
  | { type: "token"; turn_id: string; text: string }
  | { type: "assistant_message"; turn_id: string; message: Message }
  | {
      type: "tool_started";
      turn_id: string;
      tool_id: string;
      tool: string;
      label: string;
      args: unknown;
    }
  | { type: "tool_output"; tool_id: string; text: string }
  | {
      type: "tool_finished";
      turn_id: string;
      tool_id: string;
      tool: string;
      status: string;
      summary: string;
      data: unknown;
    }
  | {
      type: "approval_request";
      approval_id: string;
      turn_id: string;
      tool: string;
      tool_id: string;
      risk: string;
      why: string;
      args: unknown;
    }
  | { type: "approval_resolved"; approval_id: string; approved: boolean }
  | { type: "turn_finished"; turn_id: string; status: string; error: string | null };

// ---- UI-only view models ----

export interface UIMessage extends Message {
  streaming?: boolean;
}

export interface ToolActivity {
  toolId: string;
  turnId: string;
  tool: string;
  label: string;
  args: unknown;
  status: ToolStatus | "running";
  summary: string;
  output: string;
  data: unknown;
  startedAt: number;
  endedAt: number | null;
}

export interface PendingApproval {
  approvalId: string;
  turnId: string;
  tool: string;
  toolId: string;
  risk: Risk;
  why: string;
  args: unknown;
}

import { create } from "zustand";
import { api, errorText } from "./api";
import { speak, stopSpeaking } from "./audio";
import { titleFromText } from "./format";
import type {
  ActivityItem,
  ChatAttachment,
  Conversation,
  PendingApproval,
  Settings,
  ToolActivity,
  TurnEvent,
  UIMessage,
} from "./types";

export type View = "chat" | "settings" | "activity" | "diagnostics";

interface AstraState {
  ready: boolean;
  view: View;
  settings: Settings | null;

  conversations: Conversation[];
  activeId: string | null;
  messages: UIMessage[];

  streaming: boolean;
  turnId: string | null;
  activity: ToolActivity[];
  approval: PendingApproval | null;
  error: string | null;

  toast: string | null;
  speaking: boolean;

  load: () => Promise<void>;
  setView: (view: View) => void;
  applySettings: (settings: Settings) => void;

  refreshConversations: (query?: string) => Promise<void>;
  selectConversation: (id: string) => Promise<void>;
  newConversation: () => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  renameConversation: (id: string, title: string, pinned?: boolean) => Promise<void>;
  hydrateActivity: () => Promise<void>;

  send: (text: string, images?: ChatAttachment[]) => Promise<void>;
  cancelTurn: () => Promise<void>;
  respondApproval: (approved: boolean, remember?: string) => Promise<void>;
  speakText: (text: string) => Promise<void>;
  speakLast: () => Promise<void>;
  stopSpeaking: () => void;
  clearError: () => void;
  showToast: (text: string) => void;
  applyEvent: (event: TurnEvent) => void;
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;

export const useStore = create<AstraState>((set, get) => ({
  ready: false,
  view: "chat",
  settings: null,
  conversations: [],
  activeId: null,
  messages: [],
  streaming: false,
  turnId: null,
  activity: [],
  approval: null,
  error: null,
  toast: null,
  speaking: false,

  load: async () => {
    try {
      const [settings, conversations] = await Promise.all([
        api.settingsGet(),
        api.conversationList(),
      ]);
      set({ settings, conversations, ready: true });
      if (conversations.length > 0) {
        await get().selectConversation(conversations[0].id);
      }
      await get().hydrateActivity();
    } catch (e) {
      set({ ready: true, error: errorText(e) });
    }
  },

  setView: (view) => set({ view, error: null }),

  applySettings: (settings) => set({ settings }),

  refreshConversations: async (query) => {
    const conversations = await api.conversationList(query);
    set({ conversations });
  },

  selectConversation: async (id) => {
    set({ activeId: id, messages: [], error: null, activity: [] });
    try {
      const messages = await api.conversationMessages(id);
      set({ messages });
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  newConversation: async () => {
    try {
      const conv = await api.conversationCreate();
      set((s) => ({
        conversations: [conv, ...s.conversations],
        activeId: conv.id,
        messages: [],
        activity: [],
        view: "chat",
        error: null,
      }));
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  deleteConversation: async (id) => {
    try {
      await api.conversationDelete(id);
      const remaining = get().conversations.filter((c) => c.id !== id);
      set({ conversations: remaining });
      if (get().activeId === id) {
        if (remaining.length > 0) await get().selectConversation(remaining[0].id);
        else set({ activeId: null, messages: [], activity: [] });
      }
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  renameConversation: async (id, title, pinned) => {
    try {
      await api.conversationRename(id, title, pinned);
      set((s) => ({
        conversations: s.conversations.map((c) =>
          c.id === id ? { ...c, title, pinned: pinned ?? c.pinned } : c,
        ),
      }));
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  hydrateActivity: async () => {
    try {
      const items = await api.activityList();
      set({ activity: items.map(fromActivityItem) });
    } catch {
      /* activity is best-effort */
    }
  },

  send: async (text, images = []) => {
    const clean = text.trim();
    if (!clean || get().streaming) return;

    let convId = get().activeId;
    if (!convId) {
      try {
        const conv = await api.conversationCreate(titleFromText(clean));
        convId = conv.id;
        set((s) => ({ conversations: [conv, ...s.conversations], activeId: conv.id }));
      } catch (e) {
        set({ error: errorText(e) });
        return;
      }
    }

    const userMessage: UIMessage = {
      id: `local-user-${Date.now()}`,
      conversation_id: convId,
      role: "user",
      content: clean,
      meta: images.length ? { images: images.length } : {},
      created_at: new Date().toISOString(),
    };

    set((s) => ({
      messages: [...s.messages, userMessage],
      streaming: true,
      error: null,
      activity: [],
      approval: null,
    }));

    try {
      await api.turnRun(convId, clean, images, (event) => get().applyEvent(event));
    } catch (e) {
      set({ error: errorText(e), streaming: false, turnId: null });
    } finally {
      set({ streaming: false, turnId: null });
      void get().refreshConversations();
    }
  },

  cancelTurn: async () => {
    try {
      await api.turnCancel(get().turnId ?? "");
    } catch {
      /* best effort */
    }
  },

  respondApproval: async (approved, remember) => {
    const approval = get().approval;
    if (!approval) return;
    set({ approval: null });
    try {
      await api.approvalRespond(approval.approvalId, approved, remember);
    } catch (e) {
      set({ error: errorText(e) });
    }
  },

  speakText: async (text) => {
    const settings = get().settings;
    if (!text.trim()) return;
    set({ speaking: true });
    try {
      await speak(text, settings?.speech.voice);
    } catch (e) {
      set({ error: errorText(e) });
    } finally {
      set({ speaking: false });
    }
  },

  speakLast: async () => {
    const last = [...get().messages].reverse().find((m) => m.role === "assistant" && m.content);
    if (!last) return;
    await get().speakText(last.content);
  },

  stopSpeaking: () => {
    stopSpeaking();
    set({ speaking: false });
  },

  clearError: () => set({ error: null }),

  showToast: (text) => {
    if (toastTimer) clearTimeout(toastTimer);
    set({ toast: text });
    toastTimer = setTimeout(() => set({ toast: null }), 2600);
  },

  // ---- TurnEvent reducer ----
  applyEvent: (event: TurnEvent) => {
    const s = get();
    switch (event.type) {
      case "turn_started":
        set({ turnId: event.turn_id });
        break;
      case "token":
        set((st) => ({ messages: appendToken(st.messages, event.turn_id, event.text) }));
        break;
      case "assistant_message":
        set((st) => ({ messages: replaceStreaming(st.messages, event.message) }));
        if (get().settings?.speech.auto_speak) {
          void get().speakLast();
        }
        break;
      case "tool_started":
        set((st) => ({
          activity: [
            ...st.activity,
            {
              toolId: event.tool_id,
              turnId: event.turn_id,
              tool: event.tool,
              label: event.label,
              args: event.args,
              status: "running",
              summary: "",
              output: "",
              data: null,
              startedAt: Date.now(),
              endedAt: null,
            },
          ],
        }));
        break;
      case "tool_output":
        set((st) => ({
          activity: st.activity.map((a) =>
            a.toolId === event.tool_id
              ? { ...a, output: a.output + event.text }
              : a,
          ),
        }));
        break;
      case "tool_finished":
        set((st) => ({
          activity: st.activity.map((a) =>
            a.toolId === event.tool_id
              ? {
                  ...a,
                  status: event.status as ToolActivity["status"],
                  summary: event.summary,
                  data: event.data,
                  endedAt: Date.now(),
                }
              : a,
          ),
        }));
        break;
      case "approval_request":
        set({
          approval: {
            approvalId: event.approval_id,
            turnId: event.turn_id,
            tool: event.tool,
            toolId: event.tool_id,
            risk: (event.risk.toLowerCase() as PendingApproval["risk"]) ?? "low",
            why: event.why,
            args: event.args,
          },
        });
        break;
      case "approval_resolved":
        if (s.approval?.approvalId === event.approval_id) set({ approval: null });
        break;
      case "turn_finished":
        set({ streaming: false, turnId: null });
        if (event.error && event.status !== "cancelled") set({ error: event.error });
        break;
    }
  },
}));

// ---- pure helpers ----

function appendToken(messages: UIMessage[], turnId: string, text: string): UIMessage[] {
  const id = `streaming-${turnId}`;
  const idx = messages.findIndex((m) => m.id === id);
  if (idx === -1) {
    return [
      ...messages,
      {
        id,
        conversation_id: "",
        role: "assistant",
        content: text,
        meta: {},
        created_at: new Date().toISOString(),
        streaming: true,
      },
    ];
  }
  const next = [...messages];
  next[idx] = { ...next[idx], content: next[idx].content + text };
  return next;
}

function replaceStreaming(messages: UIMessage[], final: UIMessage): UIMessage[] {
  const idx = messages.findIndex((m) => m.streaming && m.role === "assistant");
  if (idx === -1) {
    if (messages.some((m) => m.id === final.id)) return messages;
    return [...messages, final];
  }
  const next = [...messages];
  next[idx] = { ...final, streaming: false };
  return next;
}

function fromActivityItem(item: ActivityItem): ToolActivity {
  const ended = item.ended_at ? new Date(item.ended_at).getTime() : null;
  return {
    toolId: item.id,
    turnId: item.turn_id,
    tool: item.tool,
    label: item.label,
    args: null,
    status: normalizeStatus(item.status),
    summary: item.detail ?? "",
    output: "",
    data: null,
    startedAt: new Date(item.started_at).getTime(),
    endedAt: ended,
  };
}

function normalizeStatus(status: string): ToolActivity["status"] {
  switch (status) {
    case "done":
    case "ok":
      return "ok";
    case "failed":
    case "error":
      return "error";
    case "denied":
      return "denied";
    case "cancelled":
      return "cancelled";
    default:
      return "running";
  }
}

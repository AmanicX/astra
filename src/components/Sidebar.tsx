import { useMemo, useState } from "react";
import { useStore } from "../lib/store";
import { cn, dayKey, dayLabel } from "../lib/format";
import type { Conversation } from "../lib/types";
import { IconPin, IconPlus, IconSearch, IconSettings, IconPulse, IconTrash } from "./Icons";

export function Sidebar() {
  const conversations = useStore((s) => s.conversations);
  const activeId = useStore((s) => s.activeId);
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const selectConversation = useStore((s) => s.selectConversation);
  const newConversation = useStore((s) => s.newConversation);
  const deleteConversation = useStore((s) => s.deleteConversation);
  const renameConversation = useStore((s) => s.renameConversation);

  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<string | null>(null);

  const groups = useMemo(() => groupConversations(conversations, query), [conversations, query]);

  return (
    <aside className="sidebar">
      <div className="sidebar-top">
        <button className="btn-new" type="button" onClick={() => void newConversation()}>
          <IconPlus size={16} />
          <span>New conversation</span>
        </button>

        <div className="search">
          <IconSearch size={15} />
          <input
            type="search"
            name="conversation-search"
            aria-label="Search conversations"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search conversations"
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      </div>

      <div className="sidebar-list scroll">
        {groups.length === 0 && (
          <p className="sidebar-empty">No conversations yet.</p>
        )}
        {groups.map((group) => (
          <section key={group.label} className="conv-group">
            <h4 className="caps conv-group-label">{group.label}</h4>
            {group.items.map((conv) => (
              <div
                key={conv.id}
                className={cn("conv", activeId === conv.id && view === "chat" && "conv-active")}
                onClick={() => {
                  setView("chat");
                  void selectConversation(conv.id);
                }}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    setView("chat");
                    void selectConversation(conv.id);
                  }
                }}
              >
                <div className="conv-body">
                  {editing === conv.id ? (
                    <input
                      className="conv-rename"
                      autoFocus
                      aria-label="Rename conversation"
                      defaultValue={conv.title}
                      onClick={(e) => e.stopPropagation()}
                      onBlur={(e) => {
                        const title = e.target.value.trim();
                        if (title && title !== conv.title) {
                          void renameConversation(conv.id, title);
                        }
                        setEditing(null);
                      }}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                        if (e.key === "Escape") setEditing(null);
                      }}
                    />
                  ) : (
                    <span
                      className="conv-title"
                      onDoubleClick={(e) => {
                        e.stopPropagation();
                        setEditing(conv.id);
                      }}
                    >
                      {conv.title}
                    </span>
                  )}
                  <span className="conv-meta">
                    {conv.message_count > 0 && <span>{conv.message_count}</span>}
                    {conv.pinned && <IconPin size={11} />}
                    <span>{dayLabel(conv.updated_at)}</span>
                  </span>
                </div>
                <div className="conv-actions">
                  <button
                    className="icon-btn"
                    type="button"
                    title={conv.pinned ? "Unpin" : "Pin"}
                    aria-label={conv.pinned ? "Unpin conversation" : "Pin conversation"}
                    onClick={(e) => {
                      e.stopPropagation();
                      void renameConversation(conv.id, conv.title, !conv.pinned);
                    }}
                  >
                    <IconPin size={13} />
                  </button>
                  <button
                    className="icon-btn danger"
                    type="button"
                    title="Delete"
                    aria-label="Delete conversation"
                    onClick={(e) => {
                      e.stopPropagation();
                      void deleteConversation(conv.id);
                    }}
                  >
                    <IconTrash size={13} />
                  </button>
                </div>
              </div>
            ))}
          </section>
        ))}
      </div>

      <nav className="sidebar-nav">
        <button
          className={cn("nav-item", view === "activity" && "nav-active")}
          type="button"
          onClick={() => setView("activity")}
        >
          <IconPulse size={16} />
          <span>Activity</span>
        </button>
        <button
          className={cn("nav-item", view === "diagnostics" && "nav-active")}
          type="button"
          onClick={() => setView("diagnostics")}
        >
          <IconPulse size={16} />
          <span>Diagnostics</span>
        </button>
        <button
          className={cn("nav-item", view === "settings" && "nav-active")}
          type="button"
          onClick={() => setView("settings")}
        >
          <IconSettings size={16} />
          <span>Settings</span>
        </button>
      </nav>
    </aside>
  );
}

function groupConversations(conversations: Conversation[], query: string) {
  const q = query.trim().toLowerCase();
  const filtered = q
    ? conversations.filter((c) => c.title.toLowerCase().includes(q))
    : conversations;

  const sorted = [...filtered].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    return b.updated_at.localeCompare(a.updated_at);
  });

  const buckets = new Map<string, Conversation[]>();
  for (const conv of sorted) {
    const label = dayKey(conv.updated_at);
    const list = buckets.get(label) ?? [];
    list.push(conv);
    buckets.set(label, list);
  }
  return [...buckets.entries()].map(([label, items]) => ({ label, items }));
}

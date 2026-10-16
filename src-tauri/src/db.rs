use crate::error::{AppError, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS conversations (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        pinned INTEGER NOT NULL DEFAULT 0,
        project TEXT,
        archived INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        conversation_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        meta TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id, created_at)",
    "CREATE TABLE IF NOT EXISTS memories (
        id TEXT PRIMARY KEY,
        category TEXT NOT NULL,
        key TEXT NOT NULL,
        value TEXT NOT NULL,
        source TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        UNIQUE(category, key)
    )",
    "CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY,
        conversation_id TEXT,
        title TEXT NOT NULL,
        status TEXT NOT NULL,
        detail TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS agent_runs (
        id TEXT PRIMARY KEY,
        agent TEXT NOT NULL,
        prompt_preview TEXT NOT NULL,
        cwd TEXT NOT NULL,
        status TEXT NOT NULL,
        started_at TEXT NOT NULL,
        ended_at TEXT,
        exit_code INTEGER
    )",
    "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
];

pub struct Db {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub pinned: bool,
    pub project: Option<String>,
    pub archived: bool,
    #[serde(default)]
    pub message_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub meta: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub category: String,
    pub key: String,
    pub value: String,
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl Db {
    pub fn open(path: &Path) -> Result<Db> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| AppError::msg(format!("create data dir: {e}")))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| AppError::msg(format!("open database: {e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        conn.pragma_update(None, "busy_timeout", 5000).ok();
        for stmt in SCHEMA {
            conn.execute(stmt, []).map_err(|e| {
                AppError::msg(format!("schema migration failed for `{stmt}`: {e}"))
            })?;
        }
        conn.execute(
            "INSERT INTO meta(key, value) VALUES('schema_version', '1')
             ON CONFLICT(key) DO UPDATE SET value='1'",
            [],
        )
        .ok();
        Ok(Db { conn })
    }

    // ---- settings document ----
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT value FROM settings WHERE key = ?1")
            .map_err(sql)?;
        let mut rows = stmt.query(params![key]).map_err(sql)?;
        match rows.next().map_err(sql)? {
            Some(r) => Ok(Some(r.get(0).map_err(sql)?)),
            None => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO settings(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(sql)?;
        Ok(())
    }

    // ---- conversations ----
    pub fn create_conversation(&self, title: &str, project: Option<&str>) -> Result<Conversation> {
        let id = uuid::Uuid::new_v4().to_string();
        let t = now();
        self.conn
            .execute(
                "INSERT INTO conversations(id, title, created_at, updated_at, project)
                 VALUES(?1, ?2, ?3, ?3, ?4)",
                params![id, title, t, project],
            )
            .map_err(sql)?;
        Ok(Conversation {
            id,
            title: title.to_string(),
            created_at: t.clone(),
            updated_at: t,
            pinned: false,
            project: project.map(str::to_string),
            archived: false,
            message_count: 0,
        })
    }

    pub fn list_conversations(&self, query: Option<&str>) -> Result<Vec<Conversation>> {
        let base = "SELECT c.id, c.title, c.created_at, c.updated_at, c.pinned, c.project, c.archived,
                    (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id)
                    FROM conversations c";
        let rows = match query {
            Some(q) if !q.trim().is_empty() => {
                let like = format!("%{}%", q.trim());
                let mut stmt = self
                    .conn
                    .prepare_cached(&format!(
                        "{base} WHERE c.archived = 0 AND (c.title LIKE ?1 OR EXISTS(
                            SELECT 1 FROM messages m WHERE m.conversation_id = c.id AND m.content LIKE ?1
                        )) ORDER BY c.pinned DESC, c.updated_at DESC LIMIT 200"
                    ))
                    .map_err(sql)?;
                let mapped = stmt
                    .query_map(params![like], map_conv)
                    .map_err(sql)?;
                mapped.collect::<std::result::Result<Vec<_>, _>>().map_err(sql)?
            }
            _ => {
                let mut stmt = self
                    .conn
                    .prepare_cached(&format!(
                        "{base} WHERE c.archived = 0 ORDER BY c.pinned DESC, c.updated_at DESC LIMIT 200"
                    ))
                    .map_err(sql)?;
                let mapped = stmt.query_map([], map_conv).map_err(sql)?;
                mapped.collect::<std::result::Result<Vec<_>, _>>().map_err(sql)?
            }
        };
        Ok(rows)
    }

    pub fn rename_conversation(&self, id: &str, title: &str, pinned: Option<bool>) -> Result<()> {
        if let Some(p) = pinned {
            self.conn
                .execute(
                    "UPDATE conversations SET title = ?2, pinned = ?3, updated_at = ?4 WHERE id = ?1",
                    params![id, title, p as i64, now()],
                )
                .map_err(sql)?;
        } else {
            self.conn
                .execute(
                    "UPDATE conversations SET title = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, title, now()],
                )
                .map_err(sql)?;
        }
        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM messages WHERE conversation_id = ?1", params![id])
            .map_err(sql)?;
        self.conn
            .execute("DELETE FROM conversations WHERE id = ?1", params![id])
            .map_err(sql)?;
        Ok(())
    }

    pub fn touch_conversation(&self, id: &str) {
        let _ = self.conn.execute(
            "UPDATE conversations SET updated_at = ?2 WHERE id = ?1",
            params![id, now()],
        );
    }

    pub fn get_conversation(&self, id: &str) -> Result<Conversation> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, title, created_at, updated_at, pinned, project, archived, 0
                 FROM conversations WHERE id = ?1",
            )
            .map_err(sql)?;
        stmt.query_row(params![id], map_conv).map_err(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                AppError::NotFound(format!("conversation {id}"))
            } else {
                sql(e)
            }
        })
    }

    // ---- messages ----
    pub fn insert_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        meta: &serde_json::Value,
    ) -> Result<Message> {
        let id = uuid::Uuid::new_v4().to_string();
        let t = now();
        let meta_s = serde_json::to_string(meta).unwrap_or_else(|_| "{}".into());
        self.conn
            .execute(
                "INSERT INTO messages(id, conversation_id, role, content, meta, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, conversation_id, role, content, meta_s, t],
            )
            .map_err(sql)?;
        self.touch_conversation(conversation_id);
        Ok(Message {
            id,
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            meta: meta.clone(),
            created_at: t,
        })
    }

    pub fn messages_for(&self, conversation_id: &str, limit: u32) -> Result<Vec<Message>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, conversation_id, role, content, meta, created_at FROM messages
                 WHERE conversation_id = ?1 ORDER BY created_at ASC, rowid ASC LIMIT ?2",
            )
            .map_err(sql)?;
        let rows = stmt
            .query_map(params![conversation_id, limit as i64], map_message)
            .map_err(sql)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(sql)
    }

    pub fn recent_history(&self, conversation_id: &str, limit: u32) -> Result<Vec<Message>> {
        // last N messages, returned oldest-first
        let mut all = self.messages_for(conversation_id, 10_000)?;
        let len = all.len();
        if len > limit as usize {
            all.drain(..len - limit as usize);
        }
        Ok(all)
    }

    pub fn search_messages(&self, query: &str, limit: u32) -> Result<Vec<Message>> {
        let like = format!("%{}%", query.trim());
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, conversation_id, role, content, meta, created_at FROM messages
                 WHERE content LIKE ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(sql)?;
        let rows = stmt
            .query_map(params![like, limit as i64], map_message)
            .map_err(sql)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(sql)
    }

    // ---- memories ----
    pub fn upsert_memory(
        &self,
        category: &str,
        key: &str,
        value: &str,
        source: Option<&str>,
    ) -> Result<Memory> {
        let t = now();
        self.conn
            .execute(
                "INSERT INTO memories(id, category, key, value, source, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?6)
                 ON CONFLICT(category, key) DO UPDATE SET value = excluded.value,
                   updated_at = excluded.updated_at, source = excluded.source",
                params![uuid::Uuid::new_v4().to_string(), category, key, value, source, t],
            )
            .map_err(sql)?;
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, category, key, value, source, created_at, updated_at
                 FROM memories WHERE category = ?1 AND key = ?2",
            )
            .map_err(sql)?;
        stmt.query_row(params![category, key], map_memory).map_err(sql)
    }

    pub fn list_memories(&self, category: Option<&str>, limit: u32) -> Result<Vec<Memory>> {
        let mut out = Vec::new();
        let mut stmt = match category {
            Some(_) => self
                .conn
                .prepare_cached(
                    "SELECT id, category, key, value, source, created_at, updated_at
                     FROM memories WHERE category = ?1 ORDER BY updated_at DESC LIMIT ?2",
                )
                .map_err(sql)?,
            None => self
                .conn
                .prepare_cached(
                    "SELECT id, category, key, value, source, created_at, updated_at
                     FROM memories ORDER BY updated_at DESC LIMIT ?1",
                )
                .map_err(sql)?,
        };
        let rows = match category {
            Some(c) => stmt.query_map(params![c, limit as i64], map_memory).map_err(sql)?,
            None => stmt.query_map(params![limit as i64], map_memory).map_err(sql)?,
        };
        for r in rows {
            out.push(r.map_err(sql)?);
        }
        Ok(out)
    }

    pub fn delete_memory(&self, id: &str) -> Result<()> {
        let n = self
            .conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])
            .map_err(sql)?;
        if n == 0 {
            return Err(AppError::NotFound(format!("memory {id}")));
        }
        Ok(())
    }

    // ---- agent runs ----
    pub fn record_agent_start(
        &self,
        id: &str,
        agent: &str,
        preview: &str,
        cwd: &str,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO agent_runs(id, agent, prompt_preview, cwd, status, started_at)
                 VALUES(?1, ?2, ?3, ?4, 'running', ?5)",
                params![id, agent, preview, cwd, now()],
            )
            .map_err(sql)?;
        Ok(())
    }

    pub fn record_agent_end(&self, id: &str, status: &str, exit_code: Option<i32>) {
        let _ = self.conn.execute(
            "UPDATE agent_runs SET status = ?2, ended_at = ?3, exit_code = ?4 WHERE id = ?1",
            params![id, status, now(), exit_code],
        );
    }

    pub fn list_agent_runs(&self, limit: u32) -> Result<Vec<serde_json::Value>> {
        let mut stmt = self
            .conn
            .prepare_cached(
                "SELECT id, agent, prompt_preview, cwd, status, started_at, ended_at, exit_code
                 FROM agent_runs ORDER BY started_at DESC LIMIT ?1",
            )
            .map_err(sql)?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "agent": r.get::<_, String>(1)?,
                    "prompt_preview": r.get::<_, String>(2)?,
                    "cwd": r.get::<_, String>(3)?,
                    "status": r.get::<_, String>(4)?,
                    "started_at": r.get::<_, String>(5)?,
                    "ended_at": r.get::<_, Option<String>>(6)?,
                    "exit_code": r.get::<_, Option<i32>>(7)?,
                }))
            })
            .map_err(sql)?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(sql)
    }
}

fn sql(e: rusqlite::Error) -> AppError {
    AppError::msg(format!("database: {e}"))
}

fn map_conv(r: &rusqlite::Row) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: r.get(0)?,
        title: r.get(1)?,
        created_at: r.get(2)?,
        updated_at: r.get(3)?,
        pinned: r.get::<_, i64>(4)? != 0,
        project: r.get(5)?,
        archived: r.get::<_, i64>(6)? != 0,
        message_count: r.get::<_, i64>(7)? as u32,
    })
}

fn map_message(r: &rusqlite::Row) -> rusqlite::Result<Message> {
    let meta_s: String = r.get(4)?;
    Ok(Message {
        id: r.get(0)?,
        conversation_id: r.get(1)?,
        role: r.get(2)?,
        content: r.get(3)?,
        meta: serde_json::from_str(&meta_s).unwrap_or(serde_json::Value::Object(Default::default())),
        created_at: r.get(5)?,
    })
}

fn map_memory(r: &rusqlite::Row) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: r.get(0)?,
        category: r.get(1)?,
        key: r.get(2)?,
        value: r.get(3)?,
        source: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("astra-test-{}", uuid::Uuid::new_v4()));
        Db::open(&dir.join("t.db")).unwrap()
    }

    #[test]
    fn conversation_lifecycle() {
        let db = mem_db();
        let c = db.create_conversation("First chat", None).unwrap();
        db.insert_message(&c.id, "user", "hello", &serde_json::json!({})).unwrap();
        db.insert_message(&c.id, "assistant", "hi", &serde_json::json!({})).unwrap();
        let list = db.list_conversations(None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].message_count, 2);
        let msgs = db.messages_for(&c.id, 10).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        db.delete_conversation(&c.id).unwrap();
        assert!(db.list_conversations(None).unwrap().is_empty());
    }

    #[test]
    fn memory_upsert_conflicts() {
        let db = mem_db();
        db.upsert_memory("preference", "editor", "neovim", Some("chat")).unwrap();
        db.upsert_memory("preference", "editor", "helix", Some("chat")).unwrap();
        let all = db.list_memories(Some("preference"), 10).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].value, "helix");
    }

    #[test]
    fn settings_roundtrip() {
        let db = mem_db();
        assert!(db.get_setting("settings").unwrap().is_none());
        db.set_setting("settings", "{\"version\":1}").unwrap();
        assert_eq!(db.get_setting("settings").unwrap().unwrap(), "{\"version\":1}");
    }
}

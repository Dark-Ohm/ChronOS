//! Thread persistence: SQLite-backed store for agent chat threads.
//!
//! The agent (Hermes) owns the transcript truth; we cache a copy so the
//! thread list opens instantly and works even when the agent isn't running.
//! On conflict the agent wins — the cache is only a cache.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// A single row in the `threads` table.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadRecord {
    pub id: String,
    pub agent_id: String,
    pub acp_session_id: Option<String>,
    pub title: String,
    pub title_override: Option<String>,
    pub cwd: String,
    pub last_model: Option<String>,
    pub pinned: bool,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub transcript_json: Option<String>,
}

pub struct ThreadStore {
    conn: Mutex<Connection>,
}

impl ThreadStore {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn open_default() -> Result<Self, anyhow::Error> {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("chronos/threads");
        std::fs::create_dir_all(&dir)?;
        Self::open(&dir.join("threads.db"))
    }

    pub fn open(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let store = Self { conn: Mutex::new(conn) };
        store.migrate()?;
        Ok(store)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, anyhow::Error> {
        self.conn.lock().map_err(|e| anyhow::anyhow!("mutex poisoned: {e}"))
    }

    // ── migrations ──────────────────────────────────────────────────

    fn current_version(&self) -> Result<u32, anyhow::Error> {
        Ok(self.lock()?.pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    fn set_version(&self, v: u32) -> Result<(), anyhow::Error> {
        self.lock()?.pragma_update(None, "user_version", v)?;
        Ok(())
    }

    fn migrate(&self) -> Result<(), anyhow::Error> {
        if self.current_version()? < 1 {
            self.migrate_v1()?;
        }
        self.set_version(Self::SCHEMA_VERSION)
    }

    fn migrate_v1(&self) -> Result<(), anyhow::Error> {
        self.lock()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS threads (
                id              TEXT PRIMARY KEY,
                agent_id        TEXT NOT NULL,
                acp_session_id  TEXT,
                title           TEXT NOT NULL DEFAULT '',
                title_override  TEXT,
                cwd             TEXT NOT NULL DEFAULT '',
                last_model      TEXT,
                pinned          INTEGER NOT NULL DEFAULT 0,
                archived        INTEGER NOT NULL DEFAULT 0,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL,
                transcript_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_threads_agent ON threads(agent_id);
            CREATE INDEX IF NOT EXISTS idx_threads_pinned ON threads(pinned) WHERE pinned = 1;
            CREATE INDEX IF NOT EXISTS idx_threads_updated ON threads(updated_at DESC);",
        )?;
        Ok(())
    }

    // ── CRUD ────────────────────────────────────────────────────────

    fn now_utc() -> String { chrono::Utc::now().to_rfc3339() }

    pub fn insert(&self, id: &str, agent_id: &str, cwd: &str) -> Result<ThreadRecord, anyhow::Error> {
        let now = Self::now_utc();
        self.lock()?.execute(
            "INSERT INTO threads (id, agent_id, cwd, created_at, updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![id, agent_id, cwd, now, now],
        )?;
        // Defensive: an INSERT can succeed but the follow-up SELECT miss the
        // row in pathological cases (WAL pragma switch under contention). The
        // store's mutex makes that nearly impossible, so this isn't expected —
        // but the prior version used `.expect("just inserted")` which is a
        // hard panic; `anyhow!(…)` lets the caller handle the missing row
        // instead of crashing the shell.
        self.get(id)?
            .ok_or_else(|| anyhow::anyhow!("insert succeeded but row missing: {id}"))

    }

    pub fn get(&self, id: &str) -> Result<Option<ThreadRecord>, anyhow::Error> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(SELECT_COLS)?;
        Ok(stmt.query_map(params![id], row_to_record)?.next().transpose()?)
    }

    pub fn list(
        &self, agent_id: Option<&str>, pinned_only: bool, include_archived: bool,
    ) -> Result<Vec<ThreadRecord>, anyhow::Error> {
        let mut sql = String::from("SELECT id, agent_id, acp_session_id, title, title_override, cwd, last_model, pinned, archived, created_at, updated_at, transcript_json FROM threads WHERE 1=1");
        let mut binds: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(a) = agent_id { sql.push_str(" AND agent_id = ?"); binds.push(Box::new(a.to_string())); }
        if pinned_only { sql.push_str(" AND pinned = 1"); }
        if !include_archived { sql.push_str(" AND archived = 0"); }
        sql.push_str(" ORDER BY updated_at DESC");
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let mut out = Vec::new();
        for row in stmt.query_map(refs.as_slice(), row_to_record)? { out.push(row?); }
        Ok(out)
    }

    pub fn update(
        &self, id: &str, acp_session_id: Option<&str>, title: Option<&str>,
        title_override: Option<&str>, last_model: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        self.lock()?.execute(
            "UPDATE threads SET acp_session_id = COALESCE(?2,acp_session_id), title = COALESCE(?3,title), title_override = COALESCE(?4,title_override), last_model = COALESCE(?5,last_model), updated_at = ?6 WHERE id = ?1",
            params![id, acp_session_id, title, title_override, last_model, Self::now_utc()],
        )?;
        Ok(())
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), anyhow::Error> {
        self.lock()?.execute("UPDATE threads SET pinned=?2, updated_at=?3 WHERE id=?1", params![id, pinned as i32, Self::now_utc()])?;
        Ok(())
    }

    pub fn set_archived(&self, id: &str, archived: bool) -> Result<(), anyhow::Error> {
        self.lock()?.execute("UPDATE threads SET archived=?2, updated_at=?3 WHERE id=?1", params![id, archived as i32, Self::now_utc()])?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), anyhow::Error> {
        self.lock()?.execute("DELETE FROM threads WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn cache_transcript(&self, id: &str, json: &str) -> Result<(), anyhow::Error> {
        self.lock()?.execute("UPDATE threads SET transcript_json=?2, updated_at=?3 WHERE id=?1", params![id, json, Self::now_utc()])?;
        Ok(())
    }

    pub fn transcript(&self, id: &str) -> Result<Option<String>, anyhow::Error> {
        Ok(self.lock()?.query_row("SELECT transcript_json FROM threads WHERE id=?1", params![id], |r| r.get(0)).optional()?)
    }

    pub fn search(&self, query: &str) -> Result<Vec<ThreadRecord>, anyhow::Error> {
        let pattern = format!("%{}%", query);
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, acp_session_id, title, title_override, cwd,
                    last_model, pinned, archived, created_at, updated_at, transcript_json
             FROM threads
             WHERE title LIKE ?1 OR title_override LIKE ?1 OR transcript_json LIKE ?1
             ORDER BY updated_at DESC",
        )?;
        let mut out = Vec::new();
        for row in stmt.query_map(params![pattern], row_to_record)? { out.push(row?); }
        Ok(out)
    }
}

const SELECT_COLS: &str = "SELECT id, agent_id, acp_session_id, title, title_override, cwd, last_model, pinned, archived, created_at, updated_at, transcript_json FROM threads WHERE id = ?1";

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadRecord> {
    Ok(ThreadRecord {
        id: row.get(0)?, agent_id: row.get(1)?, acp_session_id: row.get(2)?, title: row.get(3)?,
        title_override: row.get(4)?, cwd: row.get(5)?, last_model: row.get(6)?,
        pinned: row.get::<_, i32>(7)? != 0, archived: row.get::<_, i32>(8)? != 0,
        created_at: row.get(9)?, updated_at: row.get(10)?, transcript_json: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store_path() -> (ThreadStore, std::path::PathBuf) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("chronos-ts-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.db");
        (ThreadStore::open(&path).unwrap(), path)
    }

    #[test]
    fn schema_version_is_one_on_fresh_db() {
        let (store, _path) = temp_store_path();
        assert_eq!(store.current_version().unwrap(), 1);
    }

    #[test]
    fn insert_get_roundtrip() {
        let (store, _path) = temp_store_path();
        let r = store.insert("t1", "hermes", "/tmp").unwrap();
        assert_eq!(r.id, "t1");
        assert_eq!(r.agent_id, "hermes");
        assert!(r.acp_session_id.is_none());
        assert!(!r.pinned);
    }

    #[test]
    fn update_fields() {
        let (store, _path) = temp_store_path();
        store.insert("t2", "hermes", "/tmp").unwrap();
        store.update("t2", Some("acp-1"), Some("Hello"), None, Some("gpt-4")).unwrap();
        let r = store.get("t2").unwrap().unwrap();
        assert_eq!(r.acp_session_id.as_deref(), Some("acp-1"));
        assert_eq!(r.title, "Hello");
    }

    #[test]
    fn pin_archive_toggle() {
        let (store, _path) = temp_store_path();
        store.insert("t3", "hermes", "/tmp").unwrap();
        store.set_pinned("t3", true).unwrap();
        assert!(store.get("t3").unwrap().unwrap().pinned);
        store.set_archived("t3", true).unwrap();
        assert!(store.get("t3").unwrap().unwrap().archived);
    }

    #[test]
    fn list_filters() {
        let (store, _path) = temp_store_path();
        store.insert("a1", "agent-a", "/tmp").unwrap();
        store.insert("a2", "agent-b", "/tmp").unwrap();
        store.set_archived("a2", true).unwrap();
        assert_eq!(store.list(None, false, false).unwrap().len(), 1);
        assert_eq!(store.list(None, false, true).unwrap().len(), 2);
        assert_eq!(store.list(Some("agent-b"), false, true).unwrap().len(), 1);
    }

    #[test]
    fn delete_and_get_none() {
        let (store, _path) = temp_store_path();
        store.insert("d1", "hermes", "/tmp").unwrap();
        store.delete("d1").unwrap();
        assert!(store.get("d1").unwrap().is_none());
    }

    #[test]
    fn cache_and_read_transcript() {
        let (store, _path) = temp_store_path();
        store.insert("c1", "hermes", "/tmp").unwrap();
        store.cache_transcript("c1", r#"["hello"]"#).unwrap();
        assert_eq!(store.transcript("c1").unwrap().as_deref(), Some(r#"["hello"]"#));
    }

    #[test]
    fn search_by_title() {
        let (store, _path) = temp_store_path();
        store.insert("s1", "hermes", "/tmp").unwrap();
        store.update("s1", None, Some("Rust refactoring"), None, None).unwrap();
        let found = store.search("refactor").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "s1");
    }

    #[test]
    fn migration_is_idempotent() {
        let (store, _path) = temp_store_path();
        store.migrate_v1().unwrap();
        assert_eq!(store.current_version().unwrap(), 1);
    }
}

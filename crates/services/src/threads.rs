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
    /// T280 / schema v2: the canonical project this thread belongs to.
    /// Backfilled from `cwd` by the v1→v2 migration; new rows set it
    /// explicitly via `insert_for_project` (`insert` keeps `= cwd`).
    pub project_path: String,
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
    pub const SCHEMA_VERSION: u32 = 2;

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

    /// T280 / schema v2 — a REAL transactional migration. One mutable
    /// connection, ONE transaction, explicit version branches. `user_version`
    /// is stamped `2` only after `migrate_v1_to_v2` actually ran — never
    /// merely because `SCHEMA_VERSION` changed. Schema, backfill, index and
    /// the state table commit atomically.
    fn migrate(&self) -> Result<(), anyhow::Error> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let version: u32 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version < 1 {
            Self::migrate_v1(&tx)?;
        }
        if version < 2 {
            Self::migrate_v1_to_v2(&tx)?;
        }
        tx.pragma_update(None, "user_version", Self::SCHEMA_VERSION)?;
        tx.commit()?;
        Ok(())
    }

    fn migrate_v1(tx: &rusqlite::Transaction<'_>) -> Result<(), anyhow::Error> {
        tx.execute_batch(
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

    /// T280 / schema v2: add `project_path` (backfilled from `cwd`), the
    /// project-scoped updated index, and the per-project active-thread
    /// state table. Runs inside the single migration transaction.
    fn migrate_v1_to_v2(tx: &rusqlite::Transaction<'_>) -> Result<(), anyhow::Error> {
        tx.execute_batch(
            "ALTER TABLE threads ADD COLUMN project_path TEXT;
            UPDATE threads SET project_path = cwd WHERE project_path IS NULL;
            CREATE INDEX IF NOT EXISTS idx_threads_project_updated
                ON threads(project_path, archived, updated_at DESC);
            CREATE TABLE IF NOT EXISTS workspace_project_state (
                project_path TEXT PRIMARY KEY NOT NULL,
                active_thread_id TEXT,
                FOREIGN KEY(active_thread_id) REFERENCES threads(id) ON DELETE SET NULL
            );",
        )?;
        Ok(())
    }

    // ── CRUD ────────────────────────────────────────────────────────

    fn now_utc() -> String { chrono::Utc::now().to_rfc3339() }

    /// Compatibility wrapper over `insert_for_project` — project_path = cwd
    /// (the pre-v2 behaviour). Returns the inserted record.
    pub fn insert(&self, id: &str, agent_id: &str, cwd: &str) -> Result<ThreadRecord, anyhow::Error> {
        self.insert_for_project(id, agent_id, cwd, cwd)
    }

    /// T280: insert a thread bound to a canonical project path. Returns the
    /// inserted record from the store (not built in memory), so the caller
    /// sees exactly what got persisted. `project_path` is the identity that
    /// Sessions and active-thread restoration scope on.
    pub fn insert_for_project(
        &self,
        id: &str,
        agent_id: &str,
        cwd: &str,
        project_path: &str,
    ) -> Result<ThreadRecord, anyhow::Error> {
        let now = Self::now_utc();
        self.lock()?.execute(
            "INSERT INTO threads (id, agent_id, cwd, project_path, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, agent_id, cwd, project_path, now, now],
        )?;
        // Defensive: an INSERT can succeed but the follow-up SELECT miss the
        // row in pathological cases (WAL pragma switch under contention). The
        // store's mutex makes that nearly impossible, so this isn't expected —
        // but returning anyhow lets the caller handle it instead of panicking.
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
        let mut sql = String::from("SELECT id, agent_id, acp_session_id, title, title_override, cwd, project_path, last_model, pinned, archived, created_at, updated_at, transcript_json FROM threads WHERE 1=1");
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

    /// T280: list thread records scoped to one canonical project path.
    /// `include_archived` mirrors `list`'s semantics.
    pub fn list_for_project(
        &self,
        project_path: &str,
        include_archived: bool,
    ) -> Result<Vec<ThreadRecord>, anyhow::Error> {
        let sql = if include_archived {
            "SELECT id, agent_id, acp_session_id, title, title_override, cwd, project_path, last_model, pinned, archived, created_at, updated_at, transcript_json FROM threads WHERE project_path = ?1 ORDER BY updated_at DESC".to_string()
        } else {
            "SELECT id, agent_id, acp_session_id, title, title_override, cwd, project_path, last_model, pinned, archived, created_at, updated_at, transcript_json FROM threads WHERE project_path = ?1 AND archived = 0 ORDER BY updated_at DESC".to_string()
        };
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut out = Vec::new();
        for row in stmt.query_map(params![project_path], row_to_record)? { out.push(row?); }
        Ok(out)
    }

    /// T280: persist which thread is active for a project. `None` clears.
    /// One row per project (`project_path` is the primary key).
    pub fn set_active_thread(
        &self,
        project_path: &str,
        thread_id: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        match thread_id {
            Some(id) => self.lock()?.execute(
                "INSERT INTO workspace_project_state (project_path, active_thread_id)
                 VALUES (?1, ?2)
                 ON CONFLICT(project_path)
                 DO UPDATE SET active_thread_id = excluded.active_thread_id",
                params![project_path, id],
            )?,
            None => self.lock()?.execute(
                "DELETE FROM workspace_project_state WHERE project_path = ?1",
                params![project_path],
            )?,
        };
        Ok(())
    }

    /// T280: the active thread for a project, validated by BOTH id and
    /// project_path. Returns `None` when the persisted id is missing,
    /// archived, deleted, or belongs to another project — the workspace
    /// must show empty Chat, never another project's leak.
    pub fn active_thread(
        &self,
        project_path: &str,
    ) -> Result<Option<ThreadRecord>, anyhow::Error> {
        let conn = self.lock()?;
        let Some(id) = conn
            .query_row(
                "SELECT active_thread_id FROM workspace_project_state WHERE project_path = ?1",
                params![project_path],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
        else {
            return Ok(None);
        };
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, acp_session_id, title, title_override, cwd, project_path, last_model, pinned, archived, created_at, updated_at, transcript_json
             FROM threads WHERE id = ?1 AND project_path = ?2 AND archived = 0",
        )?;
        Ok(stmt.query_map(params![id, project_path], row_to_record)?
            .next()
            .transpose()?)
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
                    project_path, last_model, pinned, archived, created_at, updated_at, transcript_json
             FROM threads
             WHERE title LIKE ?1 OR title_override LIKE ?1 OR transcript_json LIKE ?1
             ORDER BY updated_at DESC",
        )?;
        let mut out = Vec::new();
        for row in stmt.query_map(params![pattern], row_to_record)? { out.push(row?); }
        Ok(out)
    }
}

const SELECT_COLS: &str = "SELECT id, agent_id, acp_session_id, title, title_override, cwd, project_path, last_model, pinned, archived, created_at, updated_at, transcript_json FROM threads WHERE id = ?1";

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadRecord> {
    Ok(ThreadRecord {
        id: row.get(0)?, agent_id: row.get(1)?, acp_session_id: row.get(2)?, title: row.get(3)?,
        title_override: row.get(4)?, cwd: row.get(5)?, project_path: row.get(6)?, last_model: row.get(7)?,
        pinned: row.get::<_, i32>(8)? != 0, archived: row.get::<_, i32>(9)? != 0,
        created_at: row.get(10)?, updated_at: row.get(11)?, transcript_json: row.get(12)?,
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
    fn schema_version_is_two_on_fresh_db() {
        let (store, _path) = temp_store_path();
        assert_eq!(store.current_version().unwrap(), 2);
    }

    #[test]
    fn insert_get_roundtrip() {
        let (store, _path) = temp_store_path();
        let r = store.insert("t1", "hermes", "/tmp").unwrap();
        assert_eq!(r.id, "t1");
        assert_eq!(r.agent_id, "hermes");
        assert!(r.acp_session_id.is_none());
        assert!(!r.pinned);
        // Compatibility wrapper: project_path defaults to cwd.
        assert_eq!(r.project_path, "/tmp");
    }

    // ── T280 / schema v2 — project scope + active-thread state ──

    /// Build a REAL v1 database with plain rusqlite — v1 schema, v1 rows,
    /// `PRAGMA user_version = 1` — then close the connection. Only after
    /// that may the test call `ThreadStore::open` on the same file. A fresh
    /// DB is NOT a v1 fixture (the ticket forbids fixture-by-open).
    fn make_v1_fixture(path: &std::path::Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE threads (
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
            CREATE INDEX idx_threads_agent ON threads(agent_id);
            CREATE INDEX idx_threads_pinned ON threads(pinned) WHERE pinned = 1;
            CREATE INDEX idx_threads_updated ON threads(updated_at DESC);
            INSERT INTO threads (id, agent_id, cwd, title, pinned, created_at, updated_at, transcript_json)
                VALUES ('v1-a', 'hermes', '/home/neo/alpha', 'Alpha thread', 1,
                        '2026-01-01T00:00:00Z', '2026-01-02T00:00:00Z', '[\"hello alpha\"]');
            INSERT INTO threads (id, agent_id, cwd, title, archived, created_at, updated_at)
                VALUES ('v1-b', 'hermes', '/home/neo/beta', 'Beta thread', 1,
                        '2026-01-03T00:00:00Z', '2026-01-04T00:00:00Z');
            PRAGMA user_version = 1;",
        )
        .unwrap();
        // Sanity: this really is a v1 fixture — no project_path column yet.
        assert!(conn.prepare("SELECT project_path FROM threads").is_err());
        drop(conn); // close BEFORE ThreadStore::open touches the file
    }

    #[test]
    fn migration_v1_to_v2_real_fixture() {
        let dir =
            std::env::temp_dir().join(format!("chronos-ts-v1fix-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("v1.db");
        let _ = std::fs::remove_file(&path);
        make_v1_fixture(&path);

        let store = ThreadStore::open(&path).unwrap();

        // Version stamped 2 only AFTER the migration ran.
        assert_eq!(store.current_version().unwrap(), 2);

        // Backfill: project_path = cwd for both v1 rows.
        let a = store.get("v1-a").unwrap().unwrap();
        assert_eq!(a.project_path, "/home/neo/alpha");
        let b = store.get("v1-b").unwrap().unwrap();
        assert_eq!(b.project_path, "/home/neo/beta");

        // v1 data survives untouched: pinned, archived, title, transcript.
        assert!(a.pinned);
        assert!(b.archived);
        assert_eq!(a.title, "Alpha thread");
        assert_eq!(
            store.transcript("v1-a").unwrap().as_deref(),
            Some("[\"hello alpha\"]"),
            "transcript must survive the migration byte-identical"
        );

        // v2 schema objects exist.
        let conn = store.lock().unwrap();
        let index_exists: bool = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_threads_project_updated'",
            )
            .unwrap()
            .exists([])
            .unwrap();
        assert!(index_exists, "v2 index must exist");
        let table_exists: bool = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='workspace_project_state'",
            )
            .unwrap()
            .exists([])
            .unwrap();
        assert!(table_exists, "workspace_project_state must exist");
    }

    #[test]
    fn insert_for_project_stores_project_path() {
        let (store, _path) = temp_store_path();
        let r = store
            .insert_for_project("p1", "hermes", "/tmp/work", "/home/neo/alpha")
            .unwrap();
        assert_eq!(r.project_path, "/home/neo/alpha");
        assert_eq!(r.cwd, "/tmp/work");
        assert_eq!(store.get("p1").unwrap().unwrap().project_path, "/home/neo/alpha");
    }

    #[test]
    fn list_for_project_scopes_and_filters_archived() {
        let (store, _path) = temp_store_path();
        store.insert_for_project("a1", "hermes", "/x", "/proj/a").unwrap();
        store.insert_for_project("a2", "hermes", "/x", "/proj/a").unwrap();
        store.insert_for_project("b1", "hermes", "/y", "/proj/b").unwrap();
        store.set_archived("a2", true).unwrap();

        let a = store.list_for_project("/proj/a", false).unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].id, "a1");
        let a_all = store.list_for_project("/proj/a", true).unwrap();
        assert_eq!(a_all.len(), 2);
        let b = store.list_for_project("/proj/b", false).unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].id, "b1");
        // Never leaks the other project.
        assert!(a.iter().all(|t| t.project_path == "/proj/a"));
    }

    #[test]
    fn active_thread_roundtrips_independently_per_project() {
        let (store, _path) = temp_store_path();
        store.insert_for_project("a1", "hermes", "/x", "/proj/a").unwrap();
        store.insert_for_project("a2", "hermes", "/x", "/proj/a").unwrap();
        store.insert_for_project("b1", "hermes", "/y", "/proj/b").unwrap();

        store.set_active_thread("/proj/a", Some("a1")).unwrap();
        store.set_active_thread("/proj/b", Some("b1")).unwrap();
        assert_eq!(store.active_thread("/proj/a").unwrap().unwrap().id, "a1");
        assert_eq!(store.active_thread("/proj/b").unwrap().unwrap().id, "b1");

        // Switching A's active must not touch B's.
        store.set_active_thread("/proj/a", Some("a2")).unwrap();
        assert_eq!(store.active_thread("/proj/a").unwrap().unwrap().id, "a2");
        assert_eq!(store.active_thread("/proj/b").unwrap().unwrap().id, "b1");

        // Clearing A leaves B intact.
        store.set_active_thread("/proj/a", None).unwrap();
        assert!(store.active_thread("/proj/a").unwrap().is_none());
        assert_eq!(store.active_thread("/proj/b").unwrap().unwrap().id, "b1");
    }

    #[test]
    fn active_thread_rejects_stale_archived_deleted_and_cross_project() {
        let (store, path) = temp_store_path();
        store.insert_for_project("a1", "hermes", "/x", "/proj/a").unwrap();
        store.insert_for_project("b1", "hermes", "/y", "/proj/b").unwrap();

        // Stale: a persisted id that no longer exists in threads. The API
        // cannot write one (FK blocks it — good enforcement), but the READ
        // must still defend if such a row ever appears (e.g. manual edit,
        // downgrade, partial write). Insert it with FK disabled.
        {
            let raw = Connection::open(&path).unwrap();
            raw.execute_batch(
                // Force foreign_keys off on THIS connection so the ghost row
                // can physically exist — proving the read guard rejects it.
                "PRAGMA foreign_keys=OFF;
                 INSERT INTO workspace_project_state (project_path, active_thread_id)
                 VALUES ('/proj/a', 'ghost');",
            )
            .unwrap();
            drop(raw);
        }
        assert!(
            store.active_thread("/proj/a").unwrap().is_none(),
            "stale id must yield empty Chat, not a leak"
        );

        // Archived active id → None (read validates archived = 0).
        store.set_active_thread("/proj/a", Some("a1")).unwrap();
        store.set_archived("a1", true).unwrap();
        assert!(store.active_thread("/proj/a").unwrap().is_none());

        // Deleted active id → None (FK ON DELETE SET NULL + read guard).
        store.set_archived("a1", false).unwrap();
        store.set_active_thread("/proj/a", Some("a1")).unwrap();
        store.delete("a1").unwrap();
        assert!(store.active_thread("/proj/a").unwrap().is_none());

        // Cross-project: b1's id stored under /proj/a must NOT resolve —
        // active_thread validates id AND project_path together. (FK allows
        // the write: b1 is a real thread; the READ rejects the scope.)
        store.set_active_thread("/proj/a", Some("b1")).unwrap();
        assert!(
            store.active_thread("/proj/a").unwrap().is_none(),
            "cross-project id must not leak another project's chat"
        );
        // …while b1 remains perfectly valid under its own project.
        store.set_active_thread("/proj/b", Some("b1")).unwrap();
        assert_eq!(store.active_thread("/proj/b").unwrap().unwrap().id, "b1");
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

    /// Re-opening an already-migrated DB is a no-op: version stays 2, no
    /// branch re-runs (ALTER TABLE would fail on a duplicate column if the
    /// version guard were broken).
    #[test]
    fn migration_is_idempotent() {
        let (store, path) = temp_store_path();
        assert_eq!(store.current_version().unwrap(), 2);
        drop(store);
        let store = ThreadStore::open(&path).unwrap();
        assert_eq!(store.current_version().unwrap(), 2);
        // Data still readable after the no-op re-open.
        store.insert("idem-1", "hermes", "/tmp").unwrap();
        assert_eq!(store.get("idem-1").unwrap().unwrap().project_path, "/tmp");
    }
}

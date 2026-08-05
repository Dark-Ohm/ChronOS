//! PTY session registry — the service layer that decouples the PTY from any
//! single widget/view.
//!
//! A `DesktopTerminalView` (GPUI, in `crates/app`) is owned by a layer-shell
//! *window* that the user can close. The shell session behind it, however,
//! must survive window close so the widget can be re-opened / dragged
//! (T259) without losing its state. This registry owns every spawned
//! [`Terminal`] keyed by a stable widget id; the view only ever borrows a
//! shared `Arc<Mutex<Terminal>>` through [`TerminalRegistry::get_or_spawn`].
//!
//! `Terminal` contains an `mpsc::Receiver`, which is `Send` but **not**
//! `Sync`, so it cannot sit in a `Global` directly. We wrap each session in
//! an `Arc<Mutex<...>>`; the mutex is `Send + Sync`, giving the registry
//! `Sync` for free. Per-widget interior mutability is fine: only the owning
//! view drives a given session's poll loop, and short `write`/`drain` calls
//! are contention-free.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;

use super::{TermSize, Terminal};

/// Shared handle to a spawned terminal session.
///
/// Cheap to clone (it's an `Arc`); cloning does **not** spawn a new shell.
/// The view holds this and locks the inner `Mutex` for I/O. Interior
/// mutability keeps the registry value-type-friendly (no `&mut` needed to
/// drive a session).
pub type TerminalHandle = Arc<Mutex<Terminal>>;

/// Keeps every live PTY session alive and addressable by widget id.
///
/// Holds no GPUI types — pure service-layer state. The GPUI side stores an
/// `Arc<TerminalRegistry>` as a `Global` (see `crates/app/src/main.rs`).
#[derive(Default)]
pub struct TerminalRegistry {
    sessions: HashMap<String, TerminalHandle>,
}

impl TerminalRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Return the shared handle for `id`, spawning a fresh shell if none
    /// exists yet. **Idempotent**: a second call with the same `id` returns
    /// the *same* handle (same `Arc`) — it never double-spawns.
    ///
    /// `size`/`cell_w`/`cell_h` are only consulted on first spawn; a
    /// re-acquired session keeps its original geometry.
    pub fn get_or_spawn(
        &mut self,
        id: &str,
        size: TermSize,
        cell_w: f32,
        cell_h: f32,
    ) -> Result<TerminalHandle> {
        if let Some(handle) = self.sessions.get(id) {
            return Ok(Arc::clone(handle));
        }
        let term = Terminal::launch(size, cell_w, cell_h)?;
        let handle: TerminalHandle = Arc::new(Mutex::new(term));
        self.sessions.insert(id.to_string(), Arc::clone(&handle));
        Ok(handle)
    }

    /// Drop the session for `id`, killing its shell. After this,
    /// [`TerminalRegistry::contains`] returns `false` and a subsequent
    /// [`TerminalRegistry::get_or_spawn`] with the same id spawns a *new*
    /// session (not the old one).
    pub fn kill(&mut self, id: &str) {
        self.sessions.remove(id);
    }

    /// Whether a session currently exists for `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.sessions.contains_key(id)
    }

    /// Number of live sessions (diagnostics / debugging).
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// True when no sessions are live.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size() -> TermSize {
        TermSize::new(80, 24)
    }

    #[test]
    fn get_or_spawn_returns_same_handle_on_repeat() {
        let mut reg = TerminalRegistry::new();
        let a = reg
            .get_or_spawn("w1", size(), 8.0, 16.0)
            .expect("spawn");
        let b = reg
            .get_or_spawn("w1", size(), 8.0, 16.0)
            .expect("spawn");
        // Same Arc — pointer identity proves no second shell was spawned.
        assert!(Arc::ptr_eq(&a, &b), "repeat get_or_spawn must reuse session");
        assert_eq!(reg.len(), 1);
        assert!(reg.contains("w1"));
    }

    #[test]
    fn distinct_ids_get_distinct_sessions() {
        let mut reg = TerminalRegistry::new();
        let a = reg
            .get_or_spawn("w1", size(), 8.0, 16.0)
            .expect("spawn");
        let b = reg
            .get_or_spawn("w2", size(), 8.0, 16.0)
            .expect("spawn");
        assert!(!Arc::ptr_eq(&a, &b), "distinct ids must not share a session");
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn kill_removes_session_and_allows_fresh_respawn() {
        let mut reg = TerminalRegistry::new();
        let first = reg
            .get_or_spawn("w1", size(), 8.0, 16.0)
            .expect("spawn");
        assert!(reg.contains("w1"));

        reg.kill("w1");
        assert!(!reg.contains("w1"));
        assert_eq!(reg.len(), 0);

        // Same id, brand-new session (different Arc).
        let second = reg
            .get_or_spawn("w1", size(), 8.0, 16.0)
            .expect("respawn");
        assert!(!Arc::ptr_eq(&first, &second), "respawn must be a new session");
        assert!(reg.contains("w1"));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn kill_unknown_id_is_noop() {
        let mut reg = TerminalRegistry::new();
        reg.kill("ghost");
        assert!(reg.is_empty());
    }

    #[test]
    fn handle_drives_terminal_through_mutex() {
        let mut reg = TerminalRegistry::new();
        let h = reg.get_or_spawn("w1", size(), 8.0, 16.0).expect("spawn");
        // Lock, poke, drop — exercises the Mutex wrapper path.
        {
            let term = h.lock().expect("lock");
            assert!(term.is_alive());
        }
        assert!(reg.contains("w1"));

        // Cloning the handle yields the same Arc (cheap, no spawn).
        let h2 = Arc::clone(&h);
        assert!(Arc::ptr_eq(&h, &h2));
    }
}

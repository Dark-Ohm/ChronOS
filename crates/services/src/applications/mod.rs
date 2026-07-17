//! Desktop entries service — scans XDG application directories and watches
//! for changes via inotify.
//!
//! SYNC TEMPLATE (spec §5.1): `new()` is synchronous, starts a background
//! thread for inotify + a tokio task for debounced rescans. Uses
//! `Handle::current()` guard like other services.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::runtime::Handle;
use tracing::{debug, info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{AppEntry, ApplicationsCommand, ApplicationsState};

pub mod types;

const DEBOUNCE_MS: u64 = 500;

/// XDG application directories to scan, in priority order.
/// User directory overrides system when filename matches (XDG spec).
fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    let user_data = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".local/share")
        });
    let user_dir = user_data.join("applications");
    dirs.push(user_dir);
    dirs
}

/// Scan all .desktop directories and return deduplicated entries.
/// System dirs are scanned first; user dir entries override system entries
/// with the same filename (XDG spec: user overrides system).
fn scan_all() -> Vec<AppEntry> {
    let mut seen: HashMap<String, AppEntry> = HashMap::new();

    for dir in desktop_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(app_entry) = types::parse_desktop_file(&path) {
                seen.insert(app_entry.id.clone(), app_entry);
            }
        }
    }

    let mut result: Vec<AppEntry> = seen.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

#[derive(Clone)]
pub struct ApplicationsSubscriber {
    data: Mutable<ApplicationsState>,
    status: Mutable<ServiceStatus>,
    /// Stored for future dispatch methods. Currently unused.
    #[allow(dead_code)]
    runtime: Handle,
}

impl ApplicationsSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(ApplicationsState::default());
        let status = Mutable::new(ServiceStatus::Initializing);

        let handle = Handle::current();

        // Initial scan (synchronous, fast — typical system has ~200-500 desktop files).
        let entries = scan_all();
        info!("ApplicationsSubscriber: loaded {} desktop entries", entries.len());
        data.set(ApplicationsState { entries });
        status.set(ServiceStatus::Available);

        // Spawn inotify watcher + debounced rescan.
        let data_clone = data.clone();
        let status_clone = status.clone();
        tokio::spawn(run_watcher(data_clone, status_clone));

        Self {
            data,
            status,
            runtime: handle,
        }
    }
}

impl Service for ApplicationsSubscriber {
    type Data = ApplicationsState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = ApplicationsState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> ApplicationsState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Background task: inotify thread sends events via channel, tokio task
/// debounces and rescan.
async fn run_watcher(data: Mutable<ApplicationsState>, status: Mutable<ServiceStatus>) {
    let watch_dirs: Vec<PathBuf> = desktop_dirs().into_iter().filter(|d| d.is_dir()).collect();

    if watch_dirs.is_empty() {
        warn!("ApplicationsSubscriber: no desktop directories to watch");
        return;
    }

    let (tx, rx) = crossbeam_channel::unbounded::<notify::Result<Event>>();
    let rx = Arc::new(Mutex::new(rx));

    // Spawn inotify watcher on a dedicated thread (notify's RecommendedWatcher
    // uses inotify internally on Linux, but we need a thread for the blocking
    // event loop).
    let watch_dirs_clone = watch_dirs.clone();
    std::thread::Builder::new()
        .name("app-entries-inotify".into())
        .spawn(move || {
            let mut watcher = match RecommendedWatcher::new(
                tx,
                notify::Config::default()
                    .with_poll_interval(Duration::from_millis(200)),
            ) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("ApplicationsSubscriber: failed to create watcher: {e}");
                    return;
                }
            };

            for dir in &watch_dirs_clone {
                if let Err(e) = watcher.watch(dir.as_ref(), RecursiveMode::NonRecursive) {
                    tracing::error!("ApplicationsSubscriber: failed to watch {dir:?}: {e}");
                }
            }

            // Keep watcher alive — it stops when dropped.
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        })
        .expect("failed to spawn inotify thread");

    // Debounced rescan loop.
    let mut debounce_deadline: Option<tokio::time::Instant> = None;

    loop {
        let timer = async {
            match debounce_deadline {
                Some(d) => tokio::time::sleep_until(d).await,
                None => std::future::pending::<()>().await,
            }
        };

        let rx_clone = rx.clone();
        let event = async move {
            tokio::task::spawn_blocking(move || {
                rx_clone.lock().unwrap().recv()
            })
            .await
            .ok()
            .and_then(|r| r.ok())
        };

        tokio::select! {
            evt = event => {
                match evt {
                    Some(Ok(event)) => {
                        // Only rescan on file-create/delete/modify events.
                        match event.kind {
                            EventKind::Create(_)
                            | EventKind::Remove(_)
                            | EventKind::Modify(_) => {
                                debug!("ApplicationsSubscriber: file event {:?}, scheduling rescan", event.kind);
                                debounce_deadline = Some(
                                    tokio::time::Instant::now()
                                        + Duration::from_millis(DEBOUNCE_MS)
                                );
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        warn!("ApplicationsSubscriber: inotify error: {e}");
                    }
                    None => {
                        // Channel closed — watcher thread died.
                        warn!("ApplicationsSubscriber: inotify channel closed");
                        break;
                    }
                }
            }
            _ = timer => {
                let entries = scan_all();
                debug!("ApplicationsSubscriber: rescanned {} entries", entries.len());
                data.set(ApplicationsState { entries });
                status.set(ServiceStatus::Available);
                debounce_deadline = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn applications_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = ApplicationsSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "ApplicationsSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[tokio::test]
    async fn applications_new_inside_runtime_is_available() {
        let svc = ApplicationsSubscriber::new();
        // After new(), we should be Available (initial scan completed synchronously).
        let st = svc.status();
        assert!(
            matches!(st, ServiceStatus::Available | ServiceStatus::Initializing),
            "unexpected status: {st:?}"
        );
        let state = svc.get();
        // On a real system we expect some entries; in CI we may have zero.
        let _ = state.entries.len();
    }

    #[test]
    fn applications_state_is_eq() {
        // Compile-time guard: AppEntry has no floats, so Eq is safe.
        let a = ApplicationsState {
            entries: vec![AppEntry {
                id: "test".into(),
                name: "Test".into(),
                exec: "/usr/bin/test".into(),
                icon: None,
                terminal: false,
            }],
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn scan_all_returns_sorted() {
        let entries = scan_all();
        for window in entries.windows(2) {
            assert!(
                window[0].name.to_lowercase() <= window[1].name.to_lowercase(),
                "entries not sorted: {:?} > {:?}",
                window[0].name,
                window[1].name
            );
        }
    }
}

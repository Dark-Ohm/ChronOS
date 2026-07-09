// crates/luau/src/watcher.rs
//! inotify-based hot-reload watcher for LuaU plugins.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};

use crate::manager::PluginManager;
use gpui::BorrowAppContext;

const DEBOUNCE_MS: u64 = 300;
const WATCH_MASK: WatchMask = WatchMask::CLOSE_WRITE
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::CREATE)
    .union(WatchMask::DELETE);

/// Start the inotify watcher loop.
///
/// The blocking inotify read runs on its own `std::thread::spawn` OS thread —
/// `Inotify::init()` always sets `IN_NONBLOCK` on the underlying fd (not
/// optional; see the `inotify` crate docs), so the non-blocking `read_events`
/// returns `WouldBlock` almost immediately after the watch is registered.
/// `App::spawn` runs its future on GPUI's *foreground* executor (the main
/// thread), so a genuinely blocking call like `read_events_blocking` must
/// never run there — it would freeze the whole UI between file events. The OS
/// thread does the blocking read + event interpretation (it owns `Inotify`)
/// and forwards each read's affected-dir batch, undebounced, through a
/// channel. A separate `cx.spawn` task on the GPUI foreground executor runs
/// the trailing debounce timer (reset on every new batch, flushed 300ms after
/// the last one) and applies reloads via `cx.update_global` — the same
/// channel-bridge shape used by `ipc/service.rs` + `ipc/mod.rs`.
pub fn start_watcher_loop(cx: &mut gpui::App, plugin_dirs: Vec<PathBuf>) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<PathBuf>>();

    std::thread::spawn(move || {
        let mut inotify = match Inotify::init() {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("Failed to init inotify: {e}");
                return;
            }
        };

        // Keyed by WatchDescriptor, not path: an inotify event only carries
        // the wd it fired on plus the changed entry's bare basename. Every
        // plugin has files literally named manifest.toml/init.luau, so
        // resolving "which dir changed" by joining the basename against every
        // watched path (instead of looking up the wd) matches every other
        // loaded plugin's same-named file too — see DECISIONS.log entry on
        // this bug for the reproduction.
        let mut watched: HashMap<WatchDescriptor, PathBuf> = HashMap::new();
        for dir in &plugin_dirs {
            if dir.exists() {
                if let Err(e) = add_watch_recursive(&inotify, dir, &mut watched) {
                    tracing::error!("Failed to watch {dir:?}: {e}");
                }
            }
        }

        let mut buf = [0u8; 4096];

        loop {
            let events = match inotify.read_events_blocking(&mut buf) {
                Ok(events) => events,
                Err(e) => {
                    tracing::error!("inotify read error: {e}");
                    break; // Non-recoverable — read_events_blocking never returns WouldBlock
                }
            };

            let mut affected: HashSet<PathBuf> = HashSet::new();
            for event in events {
                let Some(watched_dir) = watched.get(&event.wd).cloned() else {
                    continue; // event on a watch we no longer track (e.g. removed dir)
                };

                // CREATE on new subdirectory → add watch + immediate poll
                if event.mask.contains(EventMask::CREATE) {
                    if let Some(name) = event.name {
                        let new_path = watched_dir.join(name);
                        if new_path.is_dir() {
                            if let Err(e) = add_watch_recursive(&inotify, &new_path, &mut watched) {
                                tracing::warn!("Failed to watch new dir {new_path:?}: {e}");
                            }
                            // Immediate poll — close race window where files
                            // appear between mkdir and watch registration
                            let manifest = new_path.join("manifest.toml");
                            let init = new_path.join("init.luau");
                            if manifest.exists() && init.exists() {
                                affected.insert(new_path.clone());
                            }
                        }
                    }
                }

                // Determine affected plugin dir and queue reload — matched via
                // the watch descriptor the event actually fired on, not by
                // joining the basename against every watched directory.
                if let Some(name) = event.name {
                    let changed = watched_dir.join(name);
                    if changed.is_file() {
                        if let Some(parent) = changed.parent() {
                            affected.insert(parent.to_path_buf());
                        }
                    } else if changed.is_dir() {
                        affected.insert(changed);
                    }
                }
            }

            if !affected.is_empty()
                && tx.send(affected.into_iter().collect::<Vec<_>>()).is_err()
            {
                break; // receiver dropped — GPUI app is shutting down
            }
        }
    });

    cx.spawn(async move |cx| {
        // Trailing debounce: the deadline resets on every new batch and only
        // fires DEBOUNCE_MS after the *last* one, coalescing bursts (e.g. an
        // editor's temp-write-then-rename save) into a single reload instead
        // of firing on the first read that happens to complete.
        let mut pending: HashSet<PathBuf> = HashSet::new();
        let mut deadline: Option<tokio::time::Instant> = None;

        loop {
            let timer = async {
                match deadline {
                    Some(d) => tokio::time::sleep_until(d).await,
                    None => std::future::pending::<()>().await,
                }
            };

            tokio::select! {
                batch = rx.recv() => {
                    match batch {
                        Some(paths) => {
                            pending.extend(paths);
                            deadline = Some(tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                        }
                        None => break, // watcher thread ended
                    }
                }
                _ = timer => {
                    if !pending.is_empty() {
                        let reloads: Vec<PathBuf> = pending.drain().collect();
                        let _ = cx.update(|cx| {
                            for dir in &reloads {
                                // update_global gives both &mut PluginManager and &mut App
                                cx.update_global::<PluginManager, _>(|mgr, cx| {
                                    mgr.reload(dir, cx);
                                });
                            }
                        });
                    }
                    deadline = None;
                }
            }
        }
    })
    .detach();
}

/// Recursively add inotify watches on a directory and its subdirectories.
fn add_watch_recursive(
    inotify: &Inotify,
    dir: &Path,
    watched: &mut HashMap<WatchDescriptor, PathBuf>,
) -> anyhow::Result<()> {
    if watched.values().any(|p| p == dir) {
        return Ok(());
    }
    let wd = inotify.watches().add(dir, WATCH_MASK)?;
    watched.insert(wd, dir.to_path_buf());

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let sub = entry.path();
                if !watched.values().any(|p| p == &sub) {
                    if let Err(e) = add_watch_recursive(inotify, &sub, watched) {
                        tracing::warn!("Failed to watch subdirectory {sub:?}: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

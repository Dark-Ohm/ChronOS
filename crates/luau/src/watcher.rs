// crates/luau/src/watcher.rs
//! inotify-based hot-reload watcher for LuaU plugins.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use inotify::{EventMask, Inotify, WatchMask};

use crate::manager::PluginManager;
use gpui::BorrowAppContext;

const DEBOUNCE_MS: u64 = 300;
const WATCH_MASK: WatchMask = WatchMask::CLOSE_WRITE
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::CREATE)
    .union(WatchMask::DELETE);

/// Start the inotify watcher loop. Spawns a detached GPUI async task.
pub fn start_watcher_loop(cx: &mut gpui::App, plugin_dirs: Vec<PathBuf>) {
    cx.spawn(async move |cx| {
        let mut inotify = match Inotify::init() {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("Failed to init inotify: {e}");
                return;
            }
        };

        let mut watched_dirs: HashSet<PathBuf> = HashSet::new();
        for dir in &plugin_dirs {
            if dir.exists() {
                if let Err(e) = add_watch_recursive(&inotify, dir, &mut watched_dirs) {
                    tracing::error!("Failed to watch {dir:?}: {e}");
                }
            }
        }

        let mut buf = [0u8; 4096];
        let mut pending: HashSet<PathBuf> = HashSet::new();

        loop {
            let events = match inotify.read_events(&mut buf) {
                Ok(events) => events,
                Err(e) => {
                    tracing::error!("inotify read error: {e}");
                    break; // Non-recoverable
                }
            };

            // Snapshot watched dirs to avoid borrow conflict when mutating inside loop
            let watched_snapshot: Vec<PathBuf> = watched_dirs.iter().cloned().collect();
            for event in events {
                // CREATE on new subdirectory → add watch + immediate poll
                if event.mask.contains(EventMask::CREATE) {
                    if let Some(name) = event.name {
                        for watched in &watched_snapshot {
                            let new_path = watched.join(name);
                            if new_path.is_dir() {
                                if let Err(e) = add_watch_recursive(&inotify, &new_path, &mut watched_dirs) {
                                    tracing::warn!("Failed to watch new dir {new_path:?}: {e}");
                                }
                                // Immediate poll — close race window where files
                                // appear between mkdir and watch registration
                                let manifest = new_path.join("manifest.toml");
                                let init = new_path.join("init.luau");
                                if manifest.exists() && init.exists() {
                                    pending.insert(new_path.clone());
                                }
                            }
                        }
                    }
                }

                // Determine affected plugin dir and queue reload
                if let Some(name) = event.name {
                    for watched in &watched_snapshot {
                        let affected = watched.join(name);
                        if affected.is_file() {
                            if let Some(parent) = affected.parent() {
                                pending.insert(parent.to_path_buf());
                            }
                        } else if affected.is_dir() {
                            pending.insert(affected);
                        }
                    }
                }
            }

            // Debounce: process collected reloads, then wait DEBOUNCE_MS.
            // inotify.read_events() above blocks until events arrive; during the
            // wait, further events accumulate in the kernel buffer and are
            // coalesced into the next batch.
            let reloads: Vec<PathBuf> = pending.drain().collect();
            if !reloads.is_empty() {
                let _ = cx.update(|cx| {
                    for dir in &reloads {
                        // update_global gives both &mut PluginManager and &mut App
                        cx.update_global::<PluginManager, _>(|mgr, cx| {
                            mgr.reload(dir, cx);
                        });
                    }
                });
            }
            cx.background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
        }
    })
    .detach();
}

/// Recursively add inotify watches on a directory and its subdirectories.
fn add_watch_recursive(
    inotify: &Inotify,
    dir: &Path,
    watched: &mut HashSet<PathBuf>,
) -> anyhow::Result<()> {
    if watched.contains(dir) {
        return Ok(());
    }
    inotify.watches().add(dir, WATCH_MASK)?;
    watched.insert(dir.to_path_buf());

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let sub = entry.path();
                if !watched.contains(&sub) {
                    if let Err(e) = add_watch_recursive(inotify, &sub, watched) {
                        tracing::warn!("Failed to watch subdirectory {sub:?}: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

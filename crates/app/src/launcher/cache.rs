use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use gpui::{App, BorrowAppContext, Global};
use inotify::{Inotify, WatchMask};

use super::entry::{DesktopEntry, parse_desktop_file};

/// Global cache of parsed desktop entries. Populated at startup, updated
/// live via inotify.
///
/// Design note: this is a plain `Global`, not `Mutable<Vec<DesktopEntry>>`.
/// The cache is mutated only from the GPUI foreground thread — either in
/// `init()` or via `cx.update_global` from the inotify debounce task. A
/// full-replace swap does not benefit from signal subscription; the view
/// re-reads `cx.global()` every render. This differs from Compositor/
/// Network/UPower services which use `Mutable`+`watch()` because those
/// push incremental updates to widgets that selectively re-render via `watch()`.
/// The desktop entry cache does a full replace of the entire `Vec` on every
/// inotify event — there are no incremental deltas, and the launcher view
/// re-reads the entire cache on every render anyway. `Mutable` + signal adds
/// overhead with zero benefit here. The pattern divergence is **deliberate**,
/// not an oversight.
#[derive(Clone)]
pub struct DesktopEntryCache {
    pub entries: Vec<DesktopEntry>,
}

impl Global for DesktopEntryCache {}

/// XDG application directories to scan, in priority order.
/// User directory overrides system when filename matches (XDG spec).
fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Some(data) = dirs::data_local_dir() {
        let user_dir = data.join("applications");
        dirs.push(user_dir);
    }
    dirs
}

/// Scan all .desktop directories and return deduplicated entries.
/// System dirs are scanned first; user dir entries override system entries
/// with the same filename (XDG spec: user overrides system).
fn scan_all() -> Vec<DesktopEntry> {
    let mut seen: HashMap<String, DesktopEntry> = HashMap::new();

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
            if let Some(desktop_entry) = parse_desktop_file(&path) {
                // User dir entries overwrite system entries with same filename
                seen.insert(desktop_entry.id.clone(), desktop_entry);
            }
        }
    }

    let mut result: Vec<DesktopEntry> = seen.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

/// Initialize the desktop entry cache global. Called once at startup.
pub fn init(cx: &mut App) {
    let entries = scan_all();
    tracing::info!("Loaded {} desktop entries", entries.len());
    cx.set_global(DesktopEntryCache { entries });
}

const DEBOUNCE_MS: u64 = 300;
const WATCH_MASK: WatchMask = WatchMask::CLOSE_WRITE
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::CREATE)
    .union(WatchMask::DELETE);

/// Start the inotify watcher for desktop entry directories.
/// Follows the same pattern as `crates/luau/src/watcher.rs`:
/// OS thread does blocking read -> channel -> GPUI foreground debounce -> replace cache.
pub fn start_watcher(cx: &mut App) {
    let watch_dirs: Vec<PathBuf> = desktop_dirs().into_iter().filter(|d| d.is_dir()).collect();

    if watch_dirs.is_empty() {
        tracing::warn!("No desktop directories to watch");
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // OS thread: blocking inotify read
    std::thread::spawn(move || {
        let mut inotify = match Inotify::init() {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("Failed to init inotify for desktop entries: {e}");
                return;
            }
        };

        for dir in &watch_dirs {
            if let Err(e) = inotify.watches().add(dir, WATCH_MASK) {
                tracing::error!("Failed to watch {dir:?}: {e}");
            }
        }

        let mut buf = [0u8; 4096];
        loop {
            match inotify.read_events_blocking(&mut buf) {
                Ok(_) => {
                    let _ = tx.send(());
                }
                Err(e) => {
                    tracing::error!("inotify read error (desktop entries): {e}");
                    break;
                }
            }
        }
    });

    // GPUI foreground: trailing debounce -> replace cache
    cx.spawn(async move |cx| {
        let mut deadline: Option<tokio::time::Instant> = None;

        loop {
            let timer = async {
                match deadline {
                    Some(d) => tokio::time::sleep_until(d).await,
                    None => std::future::pending::<()>().await,
                }
            };

            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Some(()) => {
                            deadline = Some(
                                tokio::time::Instant::now()
                                    + Duration::from_millis(DEBOUNCE_MS)
                            );
                        }
                        None => break,
                    }
                }
                _ = timer => {
                    let entries = scan_all();
                    tracing::debug!("Desktop entry cache refreshed: {} entries", entries.len());
                    let _ = cx.update(|cx| {
                        cx.update_global::<DesktopEntryCache, _>(|cache, _cx| {
                            *cache = DesktopEntryCache { entries };
                        });
                    });
                    deadline = None;
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn dedup_user_overrides_system() {
        let tmp = std::env::temp_dir().join("desktop-cache-test-dedup");
        let sys_dir = tmp.join("system");
        let user_dir = tmp.join("user");
        std::fs::create_dir_all(&sys_dir).unwrap();
        std::fs::create_dir_all(&user_dir).unwrap();

        // System version
        let sys_path = sys_dir.join("test-app.desktop");
        let mut f = std::fs::File::create(&sys_path).unwrap();
        f.write_all(b"[Desktop Entry]\nType=Application\nName=System App\nExec=/usr/bin/system\n")
            .unwrap();

        // User override
        let user_path = user_dir.join("test-app.desktop");
        let mut f = std::fs::File::create(&user_path).unwrap();
        f.write_all(b"[Desktop Entry]\nType=Application\nName=User App\nExec=/usr/bin/user\n")
            .unwrap();

        // Manually scan (not using desktop_dirs() since these are temp dirs)
        let mut seen: HashMap<String, DesktopEntry> = HashMap::new();
        for dir in [&sys_dir, &user_dir] {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                    if let Some(e) = parse_desktop_file(&path) {
                        seen.insert(e.id.clone(), e);
                    }
                }
            }
        }

        let entries: Vec<_> = seen.into_values().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "User App"); // user wins
        assert_eq!(entries[0].exec, "/usr/bin/user");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

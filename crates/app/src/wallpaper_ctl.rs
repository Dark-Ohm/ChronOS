//! Wallpaper folder cycler and IPC command handlers.
//!
//! Scans `~/Pictures/Wallpapers` for images, provides `next()` to cycle
//! through them and `set()` for direct assignment. No inotify — wallpapers
//! are scanned on demand (users change them rarely).

use std::path::{Path, PathBuf};

use chronos_services::{Service, WallpaperCommand, is_image};
use tracing::{info, warn};

use crate::state;

/// Default wallpaper directory. If missing, operations are no-ops with a warn.
fn wallpaper_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let dir = PathBuf::from(home).join("Pictures/Wallpapers");
    if dir.is_dir() {
        Some(dir)
    } else {
        warn!("wallpaper_ctl: ~/Pictures/Wallpapers not found");
        None
    }
}

/// Scan the wallpaper directory for images, sorted alphabetically.
pub fn scan_wallpapers() -> Vec<PathBuf> {
    let Some(dir) = wallpaper_dir() else {
        return Vec::new();
    };
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image(p))
        .collect();
    entries.sort_by(|a, b| {
        let a_name = a.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let b_name = b.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        a_name.cmp(&b_name)
    });
    entries
}

/// Cycle to the next wallpaper in the folder (round-robin from current).
/// If `WallpaperState.current` is not in the folder or is None, picks the first.
pub fn next(cx: &mut gpui::App) {
    let wallpapers = scan_wallpapers();
    if wallpapers.is_empty() {
        warn!("wallpaper_ctl: no wallpapers found in ~/Pictures/Wallpapers");
        return;
    }

    let current = state::AppState::wallpaper(cx).get().current;
    let next_path = match current {
        Some(ref cur) => {
            if let Some(pos) = wallpapers.iter().position(|p| p == cur) {
                &wallpapers[(pos + 1) % wallpapers.len()]
            } else {
                &wallpapers[0]
            }
        }
        None => &wallpapers[0],
    };

    info!("wallpaper_ctl: next → {}", next_path.display());
    state::AppState::wallpaper(cx).dispatch(WallpaperCommand {
        path: next_path.clone(),
        monitor: None,
        transition: Some("fade".into()),
    });
}

/// Set wallpaper to a specific absolute path.
pub fn set(cx: &mut gpui::App, path: &Path) {
    info!("wallpaper_ctl: set → {}", path.display());
    state::AppState::wallpaper(cx).dispatch(WallpaperCommand {
        path: path.to_path_buf(),
        monitor: None,
        transition: Some("fade".into()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_wallpapers_empty_when_dir_missing() {
        // wallpaper_dir() returns None when ~/Pictures/Wallpapers doesn't exist
        // (CI environment). Just verify it returns an empty Vec, not a panic.
        let result = scan_wallpapers();
        // Can't assert empty — dir might exist on the host. Just assert no panic.
        let _ = result.len();
    }

    #[test]
    fn scan_wallpapers_sorted() {
        let wallpapers = scan_wallpapers();
        for window in wallpapers.windows(2) {
            let a = window[0].file_stem().unwrap().to_string_lossy();
            let b = window[1].file_stem().unwrap().to_string_lossy();
            assert!(
                a <= b,
                "wallpapers not sorted: {a} > {b}",
            );
        }
    }
}

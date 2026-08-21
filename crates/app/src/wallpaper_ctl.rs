//! Wallpaper folder cycler and IPC command handlers.
//!
//! Scans `~/Pictures/Wallpapers` for images, provides `next()` to cycle
//! through them and `set()` for direct assignment. No inotify — wallpapers
//! are scanned on demand (users change them rarely).

use std::path::{Path, PathBuf};
use std::process::Command;

use chronos_services::{Service, WallpaperCommand, is_image};
use tracing::{info, warn};

use crate::state;

// ---------------------------------------------------------------------------
// Waytrogen companion detection + gallery launch
// ---------------------------------------------------------------------------

/// Binary name for the waytrogen gallery app. Can be overridden via
/// `CHRONOS_WAYTROGEN` env var (useful for dev / non-PATH installs).
const WAYTROGEN_BIN: &str = "waytrogen";

/// Check if the waytrogen binary is available on `PATH` (or via env override).
pub fn waytrogen_available() -> bool {
    let bin = std::env::var("CHRONOS_WAYTROGEN").unwrap_or_else(|_| WAYTROGEN_BIN.to_string());
    Command::new("which")
        .arg(&bin)
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Errors from gallery open attempts.
#[derive(Debug)]
pub enum GalleryError {
    /// waytrogen binary not found on PATH.
    Missing,
    /// Failed to spawn the process.
    SpawnFailed(std::io::Error),
}

impl std::fmt::Display for GalleryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GalleryError::Missing => write!(f, "waytrogen not found — install the companion"),
            GalleryError::SpawnFailed(e) => write!(f, "failed to launch waytrogen: {e}"),
        }
    }
}

/// Open the waytrogen gallery GUI (their full app, no args = full UI).
///
/// Returns `Ok(())` on successful spawn, `Err(GalleryError)` otherwise.
/// The caller should call `refresh_after_gallery()` after the gallery closes
/// to resync shell state.
pub fn open_waytrogen_gallery() -> Result<(), GalleryError> {
    let bin = waytrogen_bin()?;

    // Spawn waytrogen with no args — full GUI per their CLI contract.
    // Detach stdio so the shell doesn't block on their stdout/stderr.
    Command::new(&bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(GalleryError::SpawnFailed)?;

    info!("wallpaper_ctl: opened waytrogen gallery");
    Ok(())
}

/// Async variant: spawns waytrogen and returns the child handle so the
/// caller can await exit and then resync wallpaper state.
pub fn open_waytrogen_gallery_async() -> Result<tokio::process::Child, GalleryError> {
    let bin = waytrogen_bin()?;

    let child = tokio::process::Command::new(&bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(GalleryError::SpawnFailed)?;

    info!("wallpaper_ctl: opened waytrogen gallery (async)");
    Ok(child)
}

/// Resolve the waytrogen binary path, checking env override then `which`.
fn waytrogen_bin() -> Result<String, GalleryError> {
    let bin = std::env::var("CHRONOS_WAYTROGEN").unwrap_or_else(|_| WAYTROGEN_BIN.to_string());
    let which = Command::new("which")
        .arg(&bin)
        .output()
        .map_err(GalleryError::SpawnFailed)?;
    if !which.status.success() {
        return Err(GalleryError::Missing);
    }
    Ok(bin)
}

/// Re-query awww to sync shell state after gallery use.
///
/// Must be called from a gpui context so we can reach `AppState`.
pub fn refresh_after_gallery(cx: &mut gpui::App) {
    info!("wallpaper_ctl: refreshing wallpaper state after gallery use");
    state::AppState::wallpaper(cx).refresh();
}

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

/// Scan the wallpaper directory, sorted alphabetically. `include_video` adds
/// video files for video-capable backends (mpvpaper/gslapper; T349) — awww /
/// hyprpaper / swaybg cannot play them, so they keep the image-only scan.
fn scan_media(include_video: bool) -> Vec<PathBuf> {
    let Some(dir) = wallpaper_dir() else {
        return Vec::new();
    };
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_media(p, include_video))
        .collect();
    entries.sort_by(|a, b| {
        let a_name = a
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let b_name = b
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        a_name.cmp(&b_name)
    });
    entries
}

/// Whether `path` is scannable media: always images; videos only when the
/// active backend can play them.
fn is_media(path: &Path, include_video: bool) -> bool {
    is_image(path) || (include_video && is_video(path))
}

/// Video extensions awww cannot display (and which `IMAGE_EXTENSIONS` does not
/// cover). Used only to explain why `next()` found nothing — never to set them
/// (awww plays images, not video; T339).
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "m4v", "flv", "wmv", "ogv", "mpg",
    "mpeg", "m2v", "ts", "mts", "m2ts", "vob", "3gp", "3g2",
];

/// Whether `path` looks like a video file by extension.
fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

/// Count video files in the wallpaper directory. `None` when the directory
/// itself is missing.
fn count_videos() -> Option<usize> {
    let dir = wallpaper_dir()?;
    Some(
        std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_video(p))
            .count(),
    )
}

/// Human refusal for an empty `next()` scan (T339).
fn refusal_message(video_count: Option<usize>) -> String {
    match video_count {
        Some(0) => "wallpaper folder is empty".to_string(),
        Some(n) => format!("no images, {n} videos skipped"),
        None => "wallpaper folder not found".to_string(),
    }
}

/// Cycle to the next wallpaper in the folder (round-robin from current).
/// If `WallpaperState.current` is not in the folder or is None, picks the first.
///
/// T349: when the active backend plays video (mpvpaper/gslapper), the cycle
/// includes video files — real rotation, not the T339 "videos skipped" refusal
/// (that message now only fires for engines that genuinely cannot play them).
pub fn next(cx: &mut gpui::App) {
    let backend = state::AppState::wallpaper(cx).get().backend;
    let wallpapers = scan_media(backend.supports_video());
    if wallpapers.is_empty() {
        warn!("wallpaper_ctl: no wallpapers found in ~/Pictures/Wallpapers");
        // T339: an empty scan is a visible refusal, not a dead button — tell
        // the user WHY (folder of videos / empty / missing) instead of silently
        // keeping the previous wallpaper.
        crate::notifications::push_internal(cx, "Wallpapers", &refusal_message(count_videos()));
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
    fn scan_media_empty_when_dir_missing() {
        // wallpaper_dir() returns None when ~/Pictures/Wallpapers doesn't exist
        // (CI environment). Just verify it returns an empty Vec, not a panic.
        let result = scan_media(false);
        // Can't assert empty — dir might exist on the host. Just assert no panic.
        let _ = result.len();
    }

    #[test]
    fn scan_media_sorted() {
        let wallpapers = scan_media(false);
        for window in wallpapers.windows(2) {
            let a = window[0].file_stem().unwrap().to_string_lossy();
            let b = window[1].file_stem().unwrap().to_string_lossy();
            assert!(a <= b, "wallpapers not sorted: {a} > {b}",);
        }
    }

    #[test]
    fn refusal_message_reports_videos_skipped() {
        assert_eq!(refusal_message(Some(34)), "no images, 34 videos skipped");
    }

    #[test]
    fn refusal_message_covers_empty_and_missing_folder() {
        assert_eq!(refusal_message(Some(0)), "wallpaper folder is empty");
        assert_eq!(refusal_message(None), "wallpaper folder not found");
    }

    #[test]
    fn is_video_matches_common_extensions_only() {
        assert!(is_video(Path::new("a.mp4")));
        assert!(is_video(Path::new("a.MKV")));
        assert!(!is_video(Path::new("a.png")));
        assert!(!is_video(Path::new("a.txt")));
    }

    #[test]
    fn is_media_includes_video_only_when_asked() {
        assert!(is_media(Path::new("a.png"), false));
        assert!(is_media(Path::new("a.png"), true));
        assert!(!is_media(Path::new("a.mp4"), false));
        assert!(is_media(Path::new("a.mp4"), true));
        assert!(!is_media(Path::new("a.txt"), true));
    }
}

//! Wallpaper data types and the multi-backend framework.

use std::collections::HashMap;
use std::path::PathBuf;

/// Wallpaper backend engines. All five are real, driveable backends since
/// T349 (dispatcher); command builders live in
/// [`super::backends`]. Mirrors waytrogen's `WallpaperChangers` enum without
/// the iced/GUI bits.
///
/// Knowledge of the per-engine CLI/IPC surface is taken from the `waytrogen`
/// project (Unlicense / public domain — see `Source/NOTICE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    Awww,
    Hyprpaper,
    Swaybg,
    Mpvpaper,
    Gslapper,
}

impl Backend {
    /// Human-readable backend name (also the daemon/binary stem where relevant).
    pub fn as_str(&self) -> &'static str {
        match self {
            Backend::Awww => "awww",
            Backend::Hyprpaper => "hyprpaper",
            Backend::Swaybg => "swaybg",
            Backend::Mpvpaper => "mpvpaper",
            Backend::Gslapper => "gslapper",
        }
    }

    /// Parse a backend name (case-insensitive), e.g. from `wallpaper.toml`.
    /// `None` for unknown names — callers decide whether to warn + fall back.
    pub fn parse(s: &str) -> Option<Backend> {
        match s.trim().to_ascii_lowercase().as_str() {
            "awww" => Some(Backend::Awww),
            "hyprpaper" => Some(Backend::Hyprpaper),
            "swaybg" => Some(Backend::Swaybg),
            "mpvpaper" => Some(Backend::Mpvpaper),
            "gslapper" => Some(Backend::Gslapper),
            _ => None,
        }
    }

    /// Whether this backend can play video wallpapers (mpv / GStreamer).
    pub fn supports_video(&self) -> bool {
        matches!(self, Backend::Mpvpaper | Backend::Gslapper)
    }

    /// `pidof`-able process name used to detect whether this backend is
    /// currently alive. awww is special: the daemon, not the `awww` CLI.
    pub fn process_bin(&self) -> &'static str {
        match self {
            Backend::Awww => "awww-daemon",
            other => other.as_str(),
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Awww
    }
}

/// Reactive snapshot of the wallpaper state.
///
/// `Eq` is derivable: there are no floats, only `PathBuf` (Eq) and the
/// `Backend` enum (Eq).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WallpaperState {
    /// Image currently set across outputs, if known.
    pub current: Option<PathBuf>,
    /// Per-output image path (`"eDP-1" -> "/pics/a.png"`), from `awww query`.
    pub per_monitor: HashMap<String, PathBuf>,
    /// Active backend the service talks to.
    pub backend: Backend,
}

/// Commands issued by the UI to change the wallpaper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WallpaperCommand {
    /// Image to apply.
    pub path: PathBuf,
    /// Target monitor (e.g. `"DP-1"` from `CompositorSubscriber`). If `None`,
    /// the backend applies to all outputs.
    pub monitor: Option<String>,
    /// Transition name for `awww --transition-type` (e.g. `"fade"`). If
    /// `None`, the backend uses its default.
    pub transition: Option<String>,
}

/// Image extensions awww (and the other engines) can display.
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "pnm", "tga", "ff", "hdr", "qoi",
];

/// Whether `path` looks like a displayable image by extension.
pub fn is_image(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

//! Wallpaper data types and the multi-backend framework.

use std::collections::HashMap;
use std::path::PathBuf;

/// Wallpaper backend engines. Only `Awww` is implemented in this MVP; the
/// rest are placeholders so the framework is extensible (mirrors waytrogen's
/// `WallpaperChangers` enum without the iced/GUI bits).
///
/// Knowledge of the awww CLI is taken from the `waytrogen` project
/// (Unlicense / public domain — see `Source/NOTICE`).
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

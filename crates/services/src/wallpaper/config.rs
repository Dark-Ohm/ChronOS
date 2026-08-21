//! Wallpaper config — `~/.config/chronos/wallpaper.toml` (T349).
//!
//! Carries the optional backend override. Resolution order (per the architect,
//! 2026-08-22): explicit `backend` here → autodetect a live engine at startup
//! → `awww` default. Missing/bad file falls through to autodetect with a warn,
//! never a silent default write (theme_config pattern).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Backend;

pub const CONFIG_BASENAME: &str = "wallpaper.toml";

/// Root of `wallpaper.toml`. The `[wallpaper]` table matches the brief's
/// `[wallpaper] backend = "..."` spelling exactly.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct WallpaperConfig {
    #[serde(default)]
    pub wallpaper: WallpaperSection,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct WallpaperSection {
    /// Explicit backend override. Absent → autodetect. Unknown → warn + ignore.
    #[serde(default)]
    pub backend: Option<String>,
}

impl WallpaperConfig {
    /// The configured backend override, if any. Unknown names warn and return
    /// `None` so resolution falls through to autodetect rather than erroring
    /// the whole shell over a typo.
    pub fn backend_override(&self) -> Option<Backend> {
        let raw = self.wallpaper.backend.as_deref()?;
        match Backend::parse(raw) {
            Some(backend) => Some(backend),
            None => {
                tracing::warn!("wallpaper: unknown backend {raw:?} in wallpaper.toml — ignoring");
                None
            }
        }
    }
}

/// Path to `~/.config/chronos/wallpaper.toml`.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

/// Load `wallpaper.toml`. Missing/bad → default + warn. Never silently writes
/// the file back.
pub fn load_config() -> WallpaperConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<WallpaperConfig>(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(
                    "wallpaper: failed to parse {}: {e}, using defaults",
                    path.display()
                );
                WallpaperConfig::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("wallpaper: {} not found, using autodetect", path.display());
            WallpaperConfig::default()
        }
        Err(e) => {
            tracing::warn!(
                "wallpaper: read {} failed: {e}, using defaults",
                path.display()
            );
            WallpaperConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_toml_with_backend_table() {
        let cfg: WallpaperConfig = toml::from_str("[wallpaper]\nbackend = \"mpvpaper\"\n").unwrap();
        assert_eq!(cfg.backend_override(), Some(Backend::Mpvpaper));
    }

    #[test]
    fn backend_override_is_case_insensitive() {
        let cfg: WallpaperConfig = toml::from_str("[wallpaper]\nbackend = \"GSlapper\"\n").unwrap();
        assert_eq!(cfg.backend_override(), Some(Backend::Gslapper));
    }

    #[test]
    fn empty_file_has_no_override() {
        let cfg: WallpaperConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.backend_override(), None);
    }

    #[test]
    fn unknown_backend_falls_through_to_none() {
        let cfg: WallpaperConfig = toml::from_str("[wallpaper]\nbackend = \"bogus\"\n").unwrap();
        assert_eq!(cfg.backend_override(), None);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let cfg: WallpaperConfig =
            toml::from_str("[wallpaper]\nbackend = \"awww\"\nfuture = 42\n").unwrap();
        assert_eq!(cfg.backend_override(), Some(Backend::Awww));
    }
}

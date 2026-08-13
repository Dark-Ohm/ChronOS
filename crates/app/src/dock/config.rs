//! Dock persistent configuration — pinned application list + mode composition.
//!
//! Config file: `~/.config/chronos/dock.toml`
//! Format:
//! ```toml
//! pinned = ["kitty", "thunar", "firefox", "code", "vivaldi"]
//! ```
//!
//! Display composition (what the bar actually shows) is resolved at read time:
//! scene override → mode default → stored `dock.toml` / `DEFAULT_PINNED`.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use gpui::App;
use serde::{Deserialize, Serialize};

use crate::scene;
use crate::workspace_mode::{self, WorkspaceMode};

const DEFAULT_PINNED: &[&str] = &["kitty", "thunar", "firefox", "code", "vivaldi"];

/// Gamer-mode dock default — launchers and companions, not the IDE suite.
/// Ids match `.desktop` basenames where possible (`steam.desktop` → `steam`).
const DEFAULT_PINNED_GAMER: &[&str] = &["steam", "discord", "firefox", "kitty"];

/// Global config cache — loaded once, invalidated by unpin.
static CONFIG_CACHE: OnceLock<Mutex<DockConfig>> = OnceLock::new();

fn config_cache() -> &'static Mutex<DockConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(DockConfig::default()))
}

/// Get the cached config (nanosecond read, no disk I/O).
/// This is the **stored** user list from `dock.toml`, not the mode-composed
/// display list — use [`resolve_pinned`] for what the bar should paint.
pub fn cached() -> DockConfig {
    config_cache().lock().unwrap().clone()
}

/// Mode-default pin list (no scene, no user file).
pub fn default_pinned_for_mode(mode: WorkspaceMode) -> Vec<String> {
    match mode {
        WorkspaceMode::Developer => DEFAULT_PINNED.iter().map(|s| (*s).to_string()).collect(),
        WorkspaceMode::Gamer => DEFAULT_PINNED_GAMER
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

/// Resolve pins for display: scene override → mode default → stored config,
/// then apply explicit user exclusions. Exclusions are needed because Gamer
/// intentionally ignores `dock.toml`'s `pinned` list; without them, Unpin
/// would save successfully and the mode default would immediately paint the
/// icon back.
pub fn resolve_pinned(cx: &App) -> Vec<String> {
    let config = cached();
    resolve_pinned_with_exclusions(
        workspace_mode::current(cx),
        scene::dock_override(cx),
        &config.pinned,
        &config.excluded,
    )
}

/// Testable core of [`resolve_pinned`].
pub fn resolve_pinned_with(
    mode: WorkspaceMode,
    scene_override: Option<Vec<String>>,
    stored: &[String],
) -> Vec<String> {
    resolve_pinned_with_exclusions(mode, scene_override, stored, &[])
}

/// Resolve the mode composition while honoring explicit user exclusions.
pub fn resolve_pinned_with_exclusions(
    mode: WorkspaceMode,
    scene_override: Option<Vec<String>>,
    stored: &[String],
    excluded: &[String],
) -> Vec<String> {
    let candidates = if let Some(over) = scene_override.filter(|over| !over.is_empty()) {
        over
    } else {
        match mode {
            // Developer: prefer the user's saved pins when present; else mode default.
            WorkspaceMode::Developer => {
                if stored.is_empty() {
                    default_pinned_for_mode(mode)
                } else {
                    stored.to_vec()
                }
            }
            // Gamer: mode default wins over the developer-oriented user file so
            // the dock actually changes composition on mode switch.
            WorkspaceMode::Gamer => default_pinned_for_mode(mode),
        }
    };

    candidates
        .into_iter()
        .filter(|id| !excluded.iter().any(|excluded_id| excluded_id == id))
        .collect()
}

/// Reload the cache from disk.
pub fn reload_cache() {
    *config_cache().lock().unwrap() = DockConfig::load();
}

/// Update the cache with a new config (after unpin).
pub fn update_cache(config: DockConfig) {
    *config_cache().lock().unwrap() = config;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockConfig {
    pub pinned: Vec<String>,
    /// User-explicit removals from mode/scene-provided dock composition.
    #[serde(default)]
    pub excluded: Vec<String>,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            pinned: DEFAULT_PINNED.iter().map(|s| s.to_string()).collect(),
            excluded: Vec::new(),
        }
    }
}

impl DockConfig {
    /// Load config from `~/.config/chronos/dock.toml`.
    /// If the file doesn't exist, write the default and return it.
    /// If parsing fails, return default with a warning.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<DockConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("dock: failed to parse dock.toml: {e}, using defaults");
                    Self::default()
                }
            },
            Err(_) => {
                let config = Self::default();
                if let Err(e) = config.save() {
                    tracing::warn!("dock: failed to write default dock.toml: {e}");
                }
                config
            }
        }
    }

    /// Save config to `~/.config/chronos/dock.toml`.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).expect("DockConfig is always serializable");
        std::fs::write(path, content)
    }

    /// Remove an entry by id from the stored pinned list.
    pub fn unpin(&mut self, id: &str) {
        self.pinned.retain(|p| p != id);
    }

    /// Persist an explicit removal that also applies to mode/scene defaults.
    pub fn exclude(&mut self, id: &str) {
        if !id.is_empty() && !self.excluded.iter().any(|excluded| excluded == id) {
            self.excluded.push(id.to_string());
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/dock.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_entries() {
        let config = DockConfig::default();
        assert!(config.pinned.contains(&"kitty".to_string()));
        assert!(config.pinned.contains(&"firefox".to_string()));
    }

    #[test]
    fn unpin_removes_entry() {
        let mut config = DockConfig::default();
        config.unpin("kitty");
        assert!(!config.pinned.contains(&"kitty".to_string()));
        assert!(config.pinned.contains(&"firefox".to_string()));
    }

    #[test]
    fn unpin_nonexistent_is_noop() {
        let mut config = DockConfig::default();
        let before = config.pinned.clone();
        config.unpin("nonexistent");
        assert_eq!(config.pinned, before);
    }

    #[test]
    fn roundtrip_serialization() {
        let config = DockConfig {
            pinned: vec!["a".into(), "b".into()],
            excluded: vec!["steam".into()],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: DockConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.pinned, deserialized.pinned);
        assert_eq!(config.excluded, deserialized.excluded);
    }

    #[test]
    fn parse_default_toml_format() {
        let toml_str = r#"pinned = ["kitty", "thunar", "firefox", "code", "vivaldi"]"#;
        let config: DockConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.pinned.len(), 5);
        assert!(config.excluded.is_empty());
        assert_eq!(config.pinned[0], "kitty");
    }

    #[test]
    fn parse_empty_pinned_list() {
        let toml_str = r#"pinned = []"#;
        let config: DockConfig = toml::from_str(toml_str).unwrap();
        assert!(config.pinned.is_empty());
    }

    #[test]
    fn parse_invalid_toml_returns_none() {
        let result = toml::from_str::<DockConfig>("not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn mode_defaults_are_nonempty_and_differ() {
        let dev = default_pinned_for_mode(WorkspaceMode::Developer);
        let gamer = default_pinned_for_mode(WorkspaceMode::Gamer);
        assert!(!dev.is_empty());
        assert!(!gamer.is_empty());
        assert_ne!(dev, gamer);
    }

    #[test]
    fn scene_override_beats_mode_and_stored() {
        let resolved = resolve_pinned_with(
            WorkspaceMode::Developer,
            Some(vec!["only-this".into()]),
            &["kitty".into(), "code".into()],
        );
        assert_eq!(resolved, vec!["only-this".to_string()]);
    }

    #[test]
    fn unpin_hides_a_gamer_default_without_destroying_mode_composition() {
        let mut config = DockConfig::default();
        config.unpin("steam");
        config.exclude("steam");
        let resolved = resolve_pinned_with_exclusions(
            WorkspaceMode::Gamer,
            None,
            &config.pinned,
            &config.excluded,
        );
        assert!(!resolved.contains(&"steam".to_string()));
        assert!(resolved.contains(&"discord".to_string()));
    }

    #[test]
    fn exclusion_is_idempotent_and_does_not_accept_empty_ids() {
        let mut config = DockConfig::default();
        config.exclude("steam");
        config.exclude("steam");
        config.exclude("");
        assert_eq!(config.excluded, vec!["steam".to_string()]);
    }

    #[test]
    fn gamer_uses_mode_default_not_developer_stored() {
        let resolved = resolve_pinned_with(
            WorkspaceMode::Gamer,
            None,
            &["code".into(), "thunar".into()],
        );
        assert_eq!(resolved, default_pinned_for_mode(WorkspaceMode::Gamer));
        assert!(!resolved.contains(&"code".to_string()));
    }

    #[test]
    fn developer_prefers_stored_pins() {
        let stored = vec!["a".into(), "b".into()];
        let resolved = resolve_pinned_with(WorkspaceMode::Developer, None, &stored);
        assert_eq!(resolved, stored);
    }

    #[test]
    fn empty_scene_override_falls_through() {
        let resolved = resolve_pinned_with(
            WorkspaceMode::Gamer,
            Some(vec![]),
            &["code".into()],
        );
        assert_eq!(resolved, default_pinned_for_mode(WorkspaceMode::Gamer));
    }
}

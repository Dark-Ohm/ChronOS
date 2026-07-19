//! Dock persistent configuration — pinned application list.
//!
//! Config file: `~/.config/chronos/dock.toml`
//! Format:
//! ```toml
//! pinned = ["kitty", "thunar", "firefox", "code", "vivaldi"]
//! ```

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const DEFAULT_PINNED: &[&str] = &["kitty", "thunar", "firefox", "code", "vivaldi"];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockConfig {
    pub pinned: Vec<String>,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            pinned: DEFAULT_PINNED.iter().map(|s| s.to_string()).collect(),
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

    /// Remove an entry by id from the pinned list.
    pub fn unpin(&mut self, id: &str) {
        self.pinned.retain(|p| p != id);
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
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: DockConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.pinned, deserialized.pinned);
    }

    #[test]
    fn parse_default_toml_format() {
        let toml_str = r#"pinned = ["kitty", "thunar", "firefox", "code", "vivaldi"]"#;
        let config: DockConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.pinned.len(), 5);
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
}

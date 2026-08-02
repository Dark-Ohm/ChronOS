//! Games persistent configuration — pinned game list + recent-launch bookkeeping.
//!
//! Config file: `~/.config/chronos/games.toml`
//! Format:
//! ```toml
//! version = 1
//! pinned = ["Counter-Strike 2", "SCUM"]
//!
//! [[recent]]
//! id = "Counter-Strike 2"
//! ts = 1730000000
//! ```
//!
//! Errors are logged at warn level, not panicked. Corrupt file → default loaded
//! into memory, **not** silently overwritten on disk (only `save()` writes).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const MAX_RECENT: usize = 20;

/// A single recent-launch record.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecentEntry {
    /// Desktop entry id (filename stem, e.g. "Counter-Strike 2").
    pub id: String,
    /// Unix timestamp of last launch.
    pub ts: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GamesConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Pinned game ids — the user's curated list, kept in insertion order.
    #[serde(default)]
    pub pinned: Vec<String>,
    /// Recent launches, newest first (capped at MAX_RECENT).
    #[serde(default)]
    pub recent: Vec<RecentEntry>,
}

fn default_version() -> u32 {
    1
}

impl Default for GamesConfig {
    fn default() -> Self {
        Self {
            version: 1,
            pinned: Vec::new(),
            recent: Vec::new(),
        }
    }
}

impl GamesConfig {
    /// Load config from `~/.config/chronos/games.toml`.
    /// Missing file → default (no implicit write). Parse error → default + warn.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<GamesConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("games_config: failed to parse games.toml: {e}, using defaults");
                    Self::default()
                }
            },
            Err(_) => {
                // File doesn't exist — not an error, just no config yet.
                Self::default()
            }
        }
    }

    /// Save config to `~/.config/chronos/games.toml`.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).expect("GamesConfig is always serializable");
        std::fs::write(path, content)
    }

    /// Pin a game id. Idempotent — does nothing if already pinned.
    pub fn pin(&mut self, id: &str) {
        if !self.is_pinned(id) {
            self.pinned.push(id.to_string());
        }
    }

    /// Remove a game id from the pinned list.
    pub fn unpin(&mut self, id: &str) {
        self.pinned.retain(|p| p != id);
    }

    /// Check whether a game is pinned.
    pub fn is_pinned(&self, id: &str) -> bool {
        self.pinned.iter().any(|p| p == id)
    }

    /// Record a recent launch. Moves the id to the front (or inserts it),
    /// updates the timestamp, and caps the list at MAX_RECENT.
    pub fn touch_recent(&mut self, id: &str) {
        let now = chrono::Utc::now().timestamp();
        // Remove existing entry for this id if present
        self.recent.retain(|r| r.id != id);
        // Insert at front
        self.recent.insert(
            0,
            RecentEntry {
                id: id.to_string(),
                ts: now,
            },
        );
        // Cap
        self.recent.truncate(MAX_RECENT);
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/games.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let config = GamesConfig::default();
        assert_eq!(config.version, 1);
        assert!(config.pinned.is_empty());
        assert!(config.recent.is_empty());
    }

    #[test]
    fn pin_and_unpin() {
        let mut config = GamesConfig::default();
        config.pin("CS2");
        assert!(config.is_pinned("CS2"));
        assert_eq!(config.pinned.len(), 1);

        // Idempotent
        config.pin("CS2");
        assert_eq!(config.pinned.len(), 1);

        config.unpin("CS2");
        assert!(!config.is_pinned("CS2"));
    }

    #[test]
    fn unpin_nonexistent_is_noop() {
        let mut config = GamesConfig::default();
        config.pin("CS2");
        let before = config.pinned.clone();
        config.unpin("nonexistent");
        assert_eq!(config.pinned, before);
    }

    #[test]
    fn touch_recent_moves_to_front_and_caps() {
        let mut config = GamesConfig::default();
        config.touch_recent("CS2");
        config.touch_recent("PUBG");
        config.touch_recent("SCUM");
        config.touch_recent("CS2"); // should move to front, dedup

        assert_eq!(config.recent.len(), 3);
        assert_eq!(config.recent[0].id, "CS2");
        assert_eq!(config.recent[1].id, "SCUM");
        assert_eq!(config.recent[2].id, "PUBG");
        // Timestamps are monotonically non-increasing (newest first).
        for window in config.recent.windows(2) {
            assert!(window[0].ts >= window[1].ts);
        }
    }

    #[test]
    fn recent_capped_at_max() {
        let mut config = GamesConfig::default();
        for i in 0..(MAX_RECENT + 5) {
            config.touch_recent(&format!("game-{i}"));
        }
        assert_eq!(config.recent.len(), MAX_RECENT);
        // Newest is at front
        assert_eq!(config.recent[0].id, format!("game-{}", MAX_RECENT + 4));
    }

    #[test]
    fn roundtrip_serialization() {
        let mut config = GamesConfig::default();
        config.pin("CS2");
        config.pin("PUBG");
        config.touch_recent("CS2");

        let serialized = toml::to_string(&config).unwrap();
        let deserialized: GamesConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.pinned, deserialized.pinned);
        assert_eq!(config.recent.len(), deserialized.recent.len());
        assert_eq!(config.recent[0].id, deserialized.recent[0].id);
    }

    #[test]
    fn parse_sample_toml() {
        let toml_str = r#"
version = 1
pinned = ["Counter-Strike 2", "SCUM"]

[[recent]]
id = "Counter-Strike 2"
ts = 1730000000
"#;
        let config: GamesConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.pinned, vec!["Counter-Strike 2", "SCUM"]);
        assert_eq!(config.recent.len(), 1);
        assert_eq!(config.recent[0].id, "Counter-Strike 2");
        assert_eq!(config.recent[0].ts, 1730000000);
    }

    #[test]
    fn parse_minimal_toml() {
        let toml_str = r#"version = 1"#;
        let config: GamesConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.version, 1);
        assert!(config.pinned.is_empty());
        assert!(config.recent.is_empty());
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let result = toml::from_str::<GamesConfig>("not valid toml [[[");
        assert!(result.is_err());
    }
}

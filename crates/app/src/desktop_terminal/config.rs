//! Persistence for desktop-terminal widget layout.
//!
//! Each widget is a `[[widget]]` table in
//! `~/.config/chronos/desktop_terminal.toml`. No file (or an empty
//! `[[widget]]` list) means **zero** widgets at startup — the spike's old
//! behaviour (always open exactly one fixed window) is gone. Widgets are
//! created explicitly (T259 Add button) or by hand-editing this file.
//!
//! Path resolution mirrors `crate::monitor` (`monitor.toml`): always
//! `dirs::config_dir().join("chronos/desktop_terminal.toml")`. Don't invent
//! a second way to find the config dir.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One desktop-terminal widget instance.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TerminalWidgetSpec {
    /// Stable identity for this widget; key into the PTY registry. Random on
    /// creation (caller generates via [`new_id`]); never reused after a kill.
    pub id: String,
    /// Anchor offset from the top-left of the screen (logical px).
    pub anchor_x: f32,
    /// Anchor offset from the top-left of the screen (logical px).
    pub anchor_y: f32,
    /// Window width (logical px).
    pub width: f32,
    /// Window height (logical px).
    pub height: f32,
}

/// Canonical config file name inside `~/.config/chronos/`.
const CONFIG_FILE: &str = "desktop_terminal.toml";

/// On-disk TOML shape: a named `[[widget]]` array (per the spec §1).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct FileFormat {
    #[serde(default)]
    widget: Vec<TerminalWidgetSpec>,
}

/// Resolve the config path: `~/.config/chronos/desktop_terminal.toml`.
///
/// Falls back to `~/.config` (not `~/`) if `dirs::config_dir()` is unset,
/// matching `crate::monitor::config_path`.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_FILE)
}

/// Load all widgets from `path`. Missing file → empty `Vec` (zero widgets at
/// startup). A parse error logs and returns empty rather than crashing the
/// shell — a corrupt config must not prevent boot.
fn load_from(path: &Path) -> Vec<TerminalWidgetSpec> {
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<FileFormat>(&content) {
            Ok(file) => file.widget,
            Err(err) => {
                tracing::warn!(
                    "desktop_terminal: failed to parse {}: {err}; starting with no widgets",
                    path.display()
                );
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    }
}

/// Persist the widget list to `path`. Creates the parent dir if needed.
/// Best-effort: logs on failure, never panics.
fn save_to(specs: &[TerminalWidgetSpec], path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(&FileFormat {
        widget: specs.to_vec(),
    })
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, content)
}

/// Load widgets from the canonical config path.
pub fn load() -> Vec<TerminalWidgetSpec> {
    load_from(&config_path())
}

/// Save widgets to the canonical config path.
pub fn save(specs: &[TerminalWidgetSpec]) -> std::io::Result<()> {
    save_to(specs, &config_path())
}

/// Generate a fresh widget id. Uses `uuid` (already a dependency of this
/// crate) — no new dep. v4 is fine; uniqueness is all we need.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Build a default-positioned widget spec with a fresh id. Used by the T259
/// Add button; here so callers have one obvious constructor.
pub fn make_spec(anchor_x: f32, anchor_y: f32, width: f32, height: f32) -> TerminalWidgetSpec {
    TerminalWidgetSpec {
        id: new_id(),
        anchor_x,
        anchor_y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temp config path inside an isolated dir (never `~/.config`).
    fn temp_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(CONFIG_FILE);
        (dir, path)
    }

    #[test]
    fn spec_roundtrips_through_toml() {
        let specs = vec![
            TerminalWidgetSpec {
                id: "abc".into(),
                anchor_x: 48.0,
                anchor_y: 80.0,
                width: 600.0,
                height: 400.0,
            },
            TerminalWidgetSpec {
                id: "def".into(),
                anchor_x: 120.0,
                anchor_y: 240.0,
                width: 720.0,
                height: 320.0,
            },
        ];
        let (_dir, path) = temp_path();
        save_to(&specs, &path).expect("save");
        let back = load_from(&path);
        assert_eq!(back, specs, "roundtrip must preserve all fields");
    }

    #[test]
    fn load_empty_when_file_missing() {
        let (_dir, path) = temp_path(); // exists as a dir entry? no — path is new, not written
        // Ensure the file does NOT exist.
        let _ = std::fs::remove_file(&path);
        assert!(load_from(&path).is_empty(), "missing file → zero widgets");
    }

    #[test]
    fn load_empty_on_empty_file() {
        let (_dir, path) = temp_path();
        std::fs::write(&path, "").expect("write empty");
        assert!(
            load_from(&path).is_empty(),
            "empty doc → no widgets (falls back to empty, not panic)"
        );
    }

    #[test]
    fn new_id_is_unique_each_call() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b, "two fresh ids must differ");
        assert!(!a.is_empty());
    }

    #[test]
    fn make_spec_has_fresh_id_and_fields() {
        let s = make_spec(10.0, 20.0, 640.0, 480.0);
        assert_eq!(
            (s.anchor_x, s.anchor_y, s.width, s.height),
            (10.0, 20.0, 640.0, 480.0)
        );
        assert!(!s.id.is_empty());
        let s2 = make_spec(10.0, 20.0, 640.0, 480.0);
        assert_ne!(s.id, s2.id, "two specs get distinct ids");
    }

    #[test]
    fn parse_realistic_two_widget_doc() {
        let doc = r#"
[[widget]]
id = "w1"
anchor_x = 48.0
anchor_y = 80.0
width = 600.0
height = 400.0

[[widget]]
id = "w2"
anchor_x = 700.0
anchor_y = 80.0
width = 520.0
height = 360.0
"#;
        let (_dir, path) = temp_path();
        std::fs::write(&path, doc).expect("write doc");
        let specs = load_from(&path);
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].id, "w1");
        assert_eq!(specs[1].id, "w2");
        assert_eq!(specs[1].anchor_x, 700.0);
        assert_eq!(specs[1].height, 360.0);
    }

    #[test]
    fn config_path_ends_with_expected_file() {
        let p = config_path();
        assert!(p.ends_with(Path::new("chronos").join(CONFIG_FILE)));
    }
}

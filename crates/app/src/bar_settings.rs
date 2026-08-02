//! Bar appearance presets + System settings bridge (T202).
//!
//! Lives in the **lib** tree (`side_panel_right` is lib-visible; `bar` is
//! bin-only because it pulls dock/plugin_bridge/edit_mode). The page in
//! `side_panel_right/tab/bar_settings.rs` writes `~/.config/chronos/bar.toml`
//! through this module; the existing inotify watcher (T134 →
//! `bar::layout_config::apply` → `bar::apply_appearance`, T200) re-reads the
//! file on its 300 ms debounce and applies live. **No apply logic is
//! duplicated here** — this module only persists the `[appearance]` section
//! (plus optional widget removal for presets) and lets the watcher do the rest.
//!
//! The file is edited as a raw `toml::Value` so untouched keys (widget
//! sections, `known`, unknown future fields) survive byte-for-byte — an
//! appearance-only control must never wipe the widget list.

use std::path::PathBuf;

use toml::Value;

const CONFIG_BASENAME: &str = "bar.toml";

/// Same path as `bar::layout_config::config_path` — kept here because that
/// module is bin-only and this page runs in the lib tree.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

// ── Control enums (mirror `bar::appearance` keys) ──────────────────────────

/// Bar edge — v1 UI limits to top/bottom (vertical bar is a later wave).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeChoice {
    #[default]
    Top,
    Bottom,
}

impl EdgeChoice {
    fn as_toml(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }

    fn from_toml(s: &str) -> Self {
        match s {
            "bottom" => Self::Bottom,
            _ => Self::Top,
        }
    }
}

/// Width — v1 UI offers full + two fractions (schema accepts hug; the page
/// does not expose it yet).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WidthChoice {
    #[default]
    Full,
    /// 70% of display width.
    Fraction70,
    /// 50% of display width.
    Fraction50,
}

impl WidthChoice {
    fn as_toml(self) -> String {
        match self {
            Self::Full => "full".to_string(),
            Self::Fraction70 => "fraction:0.7".to_string(),
            Self::Fraction50 => "fraction:0.5".to_string(),
        }
    }

    fn from_toml(s: &str) -> Self {
        match s {
            "fraction:0.7" => Self::Fraction70,
            "fraction:0.5" => Self::Fraction50,
            _ => Self::Full,
        }
    }
}

/// Elevation tier — maps to `BarElevation` in bar.toml.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElevationChoice {
    #[default]
    None,
    Soft,
    Strong,
}

impl ElevationChoice {
    fn as_toml(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Soft => "soft",
            Self::Strong => "strong",
        }
    }

    fn from_toml(s: &str) -> Self {
        match s {
            "soft" => Self::Soft,
            "strong" => Self::Strong,
            _ => Self::None,
        }
    }
}

/// Full appearance the page edits. Defaults mirror the code hardcoded chrome
/// (T198 table) — same values `BarAppearance::default()` produces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarSettingsPatch {
    pub edge: EdgeChoice,
    /// Bar thickness px (slider 20..=48 in the page).
    pub height: f32,
    pub width: WidthChoice,
    pub floating: bool,
    /// Corner radius px (slider 0..=16 in the page).
    pub radius: f32,
    pub elevation: ElevationChoice,
    /// Reserve exclusive zone. Forced off by `sanitized()` when floating.
    pub exclusive: bool,
}

impl BarSettingsPatch {
    /// Const default for `PRESETS` statics (`Default::default()` is not const).
    pub const DEFAULT: Self = Self {
        edge: EdgeChoice::Top,
        height: chronos_luau::bar::BAR_HEIGHT,
        width: WidthChoice::Full,
        floating: false,
        radius: 0.0,
        elevation: ElevationChoice::None,
        exclusive: true,
    };
}

impl Default for BarSettingsPatch {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A named preset: a full appearance plus optional widget removals
/// (e.g. `gaming-quiet` hides cava/mpris when present).
pub struct BarPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub appearance: BarSettingsPatch,
    /// Widget names to drop from any section when the preset is applied.
    pub remove_widgets: &'static [&'static str],
}

/// Builtin presets (v1, code defaults — no preset file on disk).
///
/// `top-full` and `bottom-full` mirror the current hardcoded chrome; the
/// pill/minimal/gaming variants are the plan §5 demo presets. `bottom-*`
/// rely on edge apply which is cold-path in T200 — the preset still writes
/// the config and the bar warns "restart to flip edge" on next apply.
pub const PRESETS: &[BarPreset] = &[
    BarPreset {
        id: "top-full",
        name: "Top full",
        description: "Full-width bar on top (default)",
        appearance: BarSettingsPatch {
            edge: EdgeChoice::Top,
            ..BarSettingsPatch::DEFAULT
        },
        remove_widgets: &[],
    },
    BarPreset {
        id: "bottom-full",
        name: "Bottom full",
        description: "Full-width bar on bottom",
        appearance: BarSettingsPatch {
            edge: EdgeChoice::Bottom,
            ..BarSettingsPatch::DEFAULT
        },
        remove_widgets: &[],
    },
    BarPreset {
        id: "bottom-pill",
        name: "Bottom pill",
        description: "Floating 70% pill, rounded, soft elevation",
        appearance: BarSettingsPatch {
            edge: EdgeChoice::Bottom,
            height: 30.0,
            width: WidthChoice::Fraction70,
            floating: true,
            radius: 12.0,
            elevation: ElevationChoice::Soft,
            exclusive: false,
        },
        remove_widgets: &[],
    },
    BarPreset {
        id: "minimal",
        name: "Minimal",
        description: "Slim bar, no elevation",
        appearance: BarSettingsPatch {
            edge: EdgeChoice::Top,
            height: 26.0,
            elevation: ElevationChoice::None,
            ..BarSettingsPatch::DEFAULT
        },
        remove_widgets: &[],
    },
    BarPreset {
        id: "gaming-quiet",
        name: "Gaming quiet",
        description: "Hide music/visualizer widgets",
        appearance: BarSettingsPatch {
            edge: EdgeChoice::Top,
            ..BarSettingsPatch::DEFAULT
        },
        remove_widgets: &["cava", "mpris"],
    },
];

pub fn preset_by_id(id: &str) -> Option<&'static BarPreset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// Drop widget names from `left`/`center`/`right` arrays in the doc.
/// Pure — no I/O.
pub fn remove_widgets_from(doc: &mut Value, names: &[&str]) {
    let table = match doc.as_table_mut() {
        Some(t) => t,
        None => return,
    };
    for key in ["left", "center", "right"] {
        let Some(arr) = table.get_mut(key).and_then(Value::as_array_mut) else {
            continue;
        };
        arr.retain(|v| {
            let name = v.as_str().unwrap_or("");
            !names.contains(&name)
        });
    }
}

/// Merge a patch into the doc's `[appearance]` table (created if absent).
/// Pure — no I/O. `version` is **not** touched here: `apply_patch` sets it.
pub fn merge_appearance_into(doc: &mut Value, patch: &BarSettingsPatch) {
    let table = doc.as_table_mut().expect("doc must be a table");
    let appearance = table
        .entry("appearance")
        .or_insert_with(|| Value::Table(toml::map::Map::new()));
    let app_table = appearance.as_table_mut().expect("appearance must be a table");

    app_table.insert("edge".into(), Value::String(patch.edge.as_toml().into()));
    app_table.insert("height".into(), Value::Float(f64::from(patch.height)));
    app_table.insert("width".into(), Value::String(patch.width.as_toml().into()));
    app_table.insert("floating".into(), Value::Boolean(patch.floating));
    app_table.insert("radius".into(), Value::Float(f64::from(patch.radius)));
    app_table.insert(
        "elevation".into(),
        Value::String(patch.elevation.as_toml().into()),
    );
    app_table.insert("exclusive".into(), Value::Boolean(patch.exclusive));
}

/// Extract the current appearance from a doc (missing file/table → defaults).
/// Pure — no I/O.
pub fn extract_appearance(doc: &Value) -> BarSettingsPatch {
    let mut out = BarSettingsPatch::default();
    let Some(app_table) = doc.get("appearance").and_then(Value::as_table) else {
        return out;
    };
    if let Some(v) = app_table.get("edge").and_then(Value::as_str) {
        out.edge = EdgeChoice::from_toml(v);
    }
    if let Some(v) = app_table.get("height").and_then(Value::as_float) {
        out.height = v as f32;
    }
    if let Some(v) = app_table.get("width").and_then(Value::as_str) {
        out.width = WidthChoice::from_toml(v);
    }
    if let Some(v) = app_table.get("floating").and_then(Value::as_bool) {
        out.floating = v;
    }
    if let Some(v) = app_table.get("radius").and_then(Value::as_float) {
        out.radius = v as f32;
    }
    if let Some(v) = app_table.get("elevation").and_then(Value::as_str) {
        out.elevation = ElevationChoice::from_toml(v);
    }
    if let Some(v) = app_table.get("exclusive").and_then(Value::as_bool) {
        out.exclusive = v;
    }
    out
}

// ── Disk-backed ────────────────────────────────────────────────────────────

/// Load the current bar.toml as a `Value` (missing/parse-fail → default
/// appearance; **no silent write**). Private — disk path is not injectable
/// (same contract as `layout_config::load`), tests exercise the pure
/// functions above.
fn load_doc() -> Value {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<Value>(&content) {
            Ok(doc) => doc,
            Err(e) => {
                tracing::warn!("bar_settings: failed to parse {}: {e}, using defaults", path.display());
                Value::Table(toml::map::Map::new())
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("bar_settings: {} not found, using defaults", path.display());
            Value::Table(toml::map::Map::new())
        }
        Err(e) => {
            tracing::warn!("bar_settings: read {} failed: {e}, using defaults", path.display());
            Value::Table(toml::map::Map::new())
        }
    }
}

/// Current appearance as the page sees it (what the file will apply).
pub fn read_current() -> BarSettingsPatch {
    extract_appearance(&load_doc())
}

/// Write a full appearance patch to `bar.toml`: merge into the existing
/// document (widgets and unknown keys survive), force `version = 2` so the
/// v1/v2 gate (T199) honors the section, persist. On parse/write failure the
/// file is left untouched and the error is returned — the page shows it
/// (§13, no panic).
pub fn apply_patch(patch: &BarSettingsPatch) -> Result<(), String> {
    let mut doc = load_doc();
    merge_appearance_into(&mut doc, patch);
    // v1 files (no `version`) must become v2 or `gated_appearance` would
    // silently drop the section on the next load (T199 compat).
    if let Some(t) = doc.as_table_mut() {
        t.insert("version".into(), Value::Integer(2));
    }
    write_doc(&doc)
}

/// Apply a named preset (appearance + optional widget removals).
pub fn apply_preset(id: &str) -> Result<&'static BarPreset, String> {
    let preset = preset_by_id(id)
        .ok_or_else(|| format!("unknown preset '{id}'"))?;
    let mut doc = load_doc();
    merge_appearance_into(&mut doc, &preset.appearance);
    remove_widgets_from(&mut doc, preset.remove_widgets);
    if let Some(t) = doc.as_table_mut() {
        t.insert("version".into(), Value::Integer(2));
    }
    write_doc(&doc)?;
    Ok(preset)
}

fn write_doc(doc: &Value) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!("bar_settings: mkdir {} failed: {e}", parent.display());
            return Err(format!("cannot create config dir: {e}"));
        }
    }
    let body = toml::to_string_pretty(doc)
        .map_err(|e| format!("cannot serialize bar.toml: {e}"))?;
    std::fs::write(&path, body).map_err(|e| {
        tracing::warn!("bar_settings: write {} failed: {e}", path.display());
        format!("cannot write bar.toml: {e}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preset `top-full` must equal the code defaults (T198 table).
    #[test]
    fn top_full_preset_equals_defaults() {
        let p = preset_by_id("top-full").unwrap();
        assert_eq!(p.appearance, BarSettingsPatch::default());
        assert!(p.remove_widgets.is_empty());
    }

    #[test]
    fn default_patch_mirrors_t198_hardcoded_chrome() {
        let d = BarSettingsPatch::default();
        assert_eq!(d.edge, EdgeChoice::Top);
        assert_eq!(d.height, 30.0);
        assert_eq!(d.width, WidthChoice::Full);
        assert!(!d.floating);
        assert_eq!(d.radius, 0.0);
        assert_eq!(d.elevation, ElevationChoice::None);
        assert!(d.exclusive);
    }

    #[test]
    fn merge_appearance_writes_all_keys_and_sets_version() {
        let mut doc = Value::Table(toml::map::Map::new());
        let patch = BarSettingsPatch {
            edge: EdgeChoice::Bottom,
            height: 40.0,
            width: WidthChoice::Fraction70,
            floating: true,
            radius: 12.0,
            elevation: ElevationChoice::Soft,
            exclusive: false,
        };
        merge_appearance_into(&mut doc, &patch);
        if let Some(t) = doc.as_table_mut() {
            t.insert("version".into(), Value::Integer(2));
        }
        let out: Value = toml::from_str(&toml::to_string(&doc).unwrap()).unwrap();
        let back = extract_appearance(&out);
        assert_eq!(back, patch);
        assert_eq!(out.get("version").and_then(Value::as_integer), Some(2));
    }

    /// An appearance-only patch must not touch widget sections.
    #[test]
    fn appearance_only_patch_preserves_widgets() {
        let src = "left = [\"dock\", \"workspaces\"]\nright = [\"clock\"]";
        let mut doc: Value = toml::from_str(src).unwrap();
        merge_appearance_into(&mut doc, &BarSettingsPatch { height: 36.0, ..Default::default() });
        let out = toml::to_string(&doc).unwrap();
        assert!(out.contains("left"), "widgets wiped:\n{out}");
        assert!(out.contains("\"dock\""), "widgets wiped:\n{out}");
        assert!(out.contains("\"clock\""), "widgets wiped:\n{out}");
        // Round-trip: parse back and confirm lists unchanged.
        let back: Value = toml::from_str(&out).unwrap();
        assert_eq!(
            back.get("left").and_then(Value::as_array).unwrap().len(),
            2
        );
        assert_eq!(
            back.get("right").and_then(Value::as_array).unwrap().len(),
            1
        );
    }

    /// `gaming-quiet` removes cava/mpris from any section; unknown keys and
    /// other widgets survive.
    #[test]
    fn gaming_quiet_removes_cava_and_mpris_only() {
        let src = "left = [\"dock\"]\ncenter = [\"mpris\", \"cava\"]\nright = [\"clock\", \"mpris\"]";
        let mut doc: Value = toml::from_str(src).unwrap();
        remove_widgets_from(&mut doc, preset_by_id("gaming-quiet").unwrap().remove_widgets);
        let out = toml::to_string(&doc).unwrap();
        assert!(!out.contains("mpris"), "mpris survived:\n{out}");
        assert!(!out.contains("cava"), "cava survived:\n{out}");
        assert!(out.contains("dock"));
        assert!(out.contains("clock"));
    }

    /// A v1 file (no `version`, no `[appearance]`) plus an appearance-only
    /// patch becomes v2 with appearance merged and widgets intact.
    #[test]
    fn v1_file_plus_patch_becomes_v2() {
        let src = "left = [\"dock\"]\ncenter = [\"cava\"]\nright = [\"clock\"]";
        let mut doc: Value = toml::from_str(src).unwrap();
        merge_appearance_into(&mut doc, &BarSettingsPatch { height: 40.0, ..Default::default() });
        if let Some(t) = doc.as_table_mut() {
            t.insert("version".into(), Value::Integer(2));
        }
        let body = toml::to_string(&doc).unwrap();
        let parsed: Value = toml::from_str(&body).unwrap();
        assert_eq!(parsed.get("version").and_then(Value::as_integer), Some(2));
        let app = extract_appearance(&parsed);
        assert_eq!(app.height, 40.0);
        assert_eq!(app.edge, EdgeChoice::Top, "unset keys fall back to defaults");
    }

    /// `extract_appearance` on a doc without `[appearance]` returns defaults.
    #[test]
    fn extract_missing_appearance_is_defaults() {
        let doc: Value = toml::from_str("left = [\"dock\"]").unwrap();
        assert_eq!(extract_appearance(&doc), BarSettingsPatch::default());
    }

    /// Unknown values degrade per-field to defaults (same lenience as the
    /// TOML deserializer in `bar::appearance`).
    #[test]
    fn extract_unknown_values_degrade_to_defaults() {
        let doc: Value = toml::from_str(
            "[appearance]\nedge = \"diagonal\"\nheight = 0.0\nwidth = \"half\"\nelevation = \"hyper\"",
        )
        .unwrap();
        let a = extract_appearance(&doc);
        assert_eq!(a.edge, EdgeChoice::Top);
        assert_eq!(a.width, WidthChoice::Full);
        assert_eq!(a.elevation, ElevationChoice::None);
        // height = 0 is parseable but out of slider range — the page clamps;
        // sanitize (T199) clamps on load. Here we report the raw value.
        assert_eq!(a.height, 0.0);
    }

    #[test]
    fn unknown_preset_id_errors() {
        assert!(preset_by_id("nope").is_none());
    }

    #[test]
    fn all_presets_have_unique_ids_and_names() {
        let mut ids: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
        let mut names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
        let n = ids.len();
        ids.sort_unstable();
        names.sort_unstable();
        ids.dedup();
        names.dedup();
        assert_eq!(ids.len(), n, "duplicate preset id");
        assert_eq!(names.len(), n, "duplicate preset name");
    }
}

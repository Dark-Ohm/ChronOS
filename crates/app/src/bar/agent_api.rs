//! Agent-facing bar config API (T201) — `list_bar_widgets` / `get_bar_config`
//! / `set_bar_config`, per `docs/PRODUCT.md` § Live desktop customization п.5.
//!
//! **Single source of truth**: this module reads/writes the same
//! `~/.config/chronos/bar.toml` a human edits by hand and the shell's own
//! `layout_config`/`appearance` modules already parse — there is no second
//! config format. The core merge/snapshot/list logic here is **pure**
//! (`BarLayoutConfig` in, `BarLayoutConfig`/snapshot out) so it is testable
//! against a temp dir without touching the real user config, and so any
//! future in-process caller (a System Settings page, T202) can reuse it
//! instead of re-deriving the merge rules.
//!
//! **Primary agent surface (documented choice, see T201 report §"How agent
//! invokes"): pure functions + a Hermes skill** (`skills/chronos-bar-config/
//! SKILL.md`) that tells the agent to read/write `bar.toml` directly via its
//! normal file tools, following this schema. No CLI subcommand — `main.rs`
//! has no argument-parsing scaffold to hang one off, and inventing one for a
//! single call site would be scope creep. No IPC mirror either — the
//! existing `ipc/messages.rs` surface is for compositor-facing commands
//! (mode switches, panel toggles), not config CRUD; bar.toml + inotify
//! hot-reload (T134) is already the shell's live-reload channel, and an
//! agent's file write rides that same channel for free.
//!
//! Module-level (not per-item) `allow(dead_code)`: every `pub` item here is
//! the external API surface itself — by design, this task's chosen agent
//! surface (skill + direct file edit, see above) means **no Rust call site
//! is required for the API to be "in use"**. A per-item `#[allow(dead_code)]`
//! on all ~17 public items would be pure repetition of the same one fact;
//! this single annotation says it once. A future in-process caller (T202
//! System Settings page, or a CLI) removes the need for this the moment it
//! lands.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::appearance::{BarAlign, BarAppearance, BarEdge, BarElevation, BarWidth};
use super::layout_config::{BUILTIN_NAMES, BarLayoutConfig};

// ── list_bar_widgets ────────────────────────────────────────────────────────

/// `list_bar_widgets()` response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BarWidgetsList {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
    /// Widget names the user has ever seen (T163 `known` field) — informs
    /// an agent whether re-adding a name would be "new" or "restoring
    /// something the user removed on purpose".
    pub known: Vec<String>,
    /// Everything that can legally appear in `left`/`center`/`right`:
    /// `BUILTIN_NAMES` plus any currently-registered Luau plugin widgets.
    pub available: Vec<String>,
}

/// Pure core: builds the list from an already-loaded config plus whatever
/// plugin widget names the caller collected (empty if none/unavailable —
/// `available` degrades to builtins-only, never fails).
pub fn list_widgets(cfg: &BarLayoutConfig, plugin_names: &[String]) -> BarWidgetsList {
    let mut available: Vec<String> = BUILTIN_NAMES.iter().map(|s| (*s).to_string()).collect();
    for name in plugin_names {
        if !available.contains(name) {
            available.push(name.clone());
        }
    }
    BarWidgetsList {
        left: cfg.left.clone(),
        center: cfg.center.clone(),
        right: cfg.right.clone(),
        known: cfg.known.iter().cloned().collect(),
        available,
    }
}

/// Disk-backed entry point. `plugin_manager` is `None` when called outside a
/// running shell (tests, a future CLI) — `available` then omits plugin
/// widgets, which is honest (they truly aren't registered in that context),
/// not a lie.
pub fn list_bar_widgets(plugin_manager: Option<&chronos_luau::PluginManager>) -> BarWidgetsList {
    let cfg = BarLayoutConfig::load();
    let plugin_names: Vec<String> = plugin_manager
        .map(|mgr| {
            mgr.get_registered_widgets()
                .into_iter()
                .map(|(_, name, _, _)| name)
                .collect()
        })
        .unwrap_or_default();
    list_widgets(&cfg, &plugin_names)
}

// ── get_bar_config ──────────────────────────────────────────────────────────

/// `get_bar_config()` / `set_bar_config()`'s `applied` field response shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BarConfigSnapshot {
    /// Effective schema version — always `2` in a snapshot regardless of
    /// what's on disk: a snapshot is meaningless without appearance being
    /// honored, and `gated_appearance` already resolved that at load time.
    pub version: u32,
    pub appearance: BarAppearance,
    pub widgets: BarWidgetsSections,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BarWidgetsSections {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
    pub known: Vec<String>,
}

/// Pure core — snapshot of an already-loaded (and ideally sanitized) config.
pub fn snapshot(cfg: &BarLayoutConfig) -> BarConfigSnapshot {
    BarConfigSnapshot {
        version: 2,
        appearance: cfg.appearance,
        widgets: BarWidgetsSections {
            left: cfg.left.clone(),
            center: cfg.center.clone(),
            right: cfg.right.clone(),
            known: cfg.known.iter().cloned().collect(),
        },
    }
}

/// Disk-backed entry point. Missing file → defaults (same honest-default
/// contract as every other `load()` in this crate — no silent write).
pub fn get_bar_config() -> BarConfigSnapshot {
    snapshot(&BarLayoutConfig::load().sanitized())
}

// ── set_bar_config ──────────────────────────────────────────────────────────

/// Patch shape accepted by `set_bar_config`. Every field optional —
/// "missing key in patch = leave current" (T201 merge rule).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BarConfigPatch {
    pub appearance: Option<AppearancePatch>,
    pub widgets: Option<WidgetsPatch>,
}

/// Appearance sub-patch. String fields accept the same TOML string grammar
/// the file format uses (`edge = "bottom"`, `width = "fraction:0.7"`, …) —
/// an agent that has read the schema doc needs to learn only one grammar,
/// not one for files and a different one for patches.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AppearancePatch {
    pub edge: Option<String>,
    pub height: Option<f32>,
    pub width: Option<String>,
    pub align: Option<String>,
    pub margin: Option<MarginPatch>,
    pub floating: Option<bool>,
    pub exclusive: Option<bool>,
    pub radius: Option<f32>,
    pub elevation: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MarginPatch {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Widgets sub-patch. `left`/`center`/`right` present as a full array
/// **replace** that section (T201 merge rule: "Widget section present as
/// full array = replace that section"). `remove`/`add_*` are optional sugar
/// applied **after** any full-array replacement in the same patch, so
/// `{"widgets": {"right": [...], "add_right": ["clock"]}}` is well-defined
/// (replace, then append) rather than ambiguous.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WidgetsPatch {
    pub left: Option<Vec<String>>,
    pub center: Option<Vec<String>>,
    pub right: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
    pub add_left: Option<Vec<String>>,
    pub add_center: Option<Vec<String>>,
    pub add_right: Option<Vec<String>>,
}

/// Parse a `T: FromStr<Err = ()>` choice field the same lenient way the
/// TOML deserializer does: unknown value → default + warn, never an error
/// that would fail the whole patch over one bad enum string.
fn parse_choice<T: FromStr<Err = ()> + Default>(raw: &str, field: &str) -> T {
    match T::from_str(raw) {
        Ok(v) => v,
        Err(()) => {
            tracing::warn!("bar: agent patch — unknown {field} '{raw}', keeping default");
            T::default()
        }
    }
}

/// Pure merge: `base` (already-loaded config) + `patch` → new config. Does
/// **not** sanitize — callers run `.sanitized()` afterward (T201 rule:
/// "Always run sanitized() from T199 before save").
pub fn merge_patch(base: &BarLayoutConfig, patch: &BarConfigPatch) -> BarLayoutConfig {
    let mut cfg = base.clone();
    // An agent-written file is always schema v2 — appearance must be
    // honored, not silently gated back to defaults by `gated_appearance`
    // on the next load (T199 v1/v2 compat gate).
    cfg.version = Some(2);

    if let Some(ap) = &patch.appearance {
        if let Some(v) = &ap.edge {
            cfg.appearance.edge = parse_choice::<BarEdge>(v, "edge");
        }
        if let Some(v) = ap.height {
            cfg.appearance.height = v;
        }
        if let Some(v) = &ap.width {
            cfg.appearance.width = BarWidth::parse_str(v);
        }
        if let Some(v) = &ap.align {
            cfg.appearance.align = parse_choice::<BarAlign>(v, "align");
        }
        if let Some(m) = &ap.margin {
            if let Some(x) = m.x {
                cfg.appearance.margin.x = x;
            }
            if let Some(y) = m.y {
                cfg.appearance.margin.y = y;
            }
        }
        if let Some(v) = ap.floating {
            cfg.appearance.floating = v;
        }
        if let Some(v) = ap.exclusive {
            cfg.appearance.exclusive = v;
        }
        if let Some(v) = ap.radius {
            cfg.appearance.radius = v;
        }
        if let Some(v) = &ap.elevation {
            cfg.appearance.elevation = parse_choice::<BarElevation>(v, "elevation");
        }
    }

    if let Some(w) = &patch.widgets {
        if let Some(left) = &w.left {
            cfg.left = left.clone();
        }
        if let Some(center) = &w.center {
            cfg.center = center.clone();
        }
        if let Some(right) = &w.right {
            cfg.right = right.clone();
        }
        if let Some(remove) = &w.remove {
            let remove_set: BTreeSet<&str> = remove.iter().map(String::as_str).collect();
            cfg.left.retain(|n| !remove_set.contains(n.as_str()));
            cfg.center.retain(|n| !remove_set.contains(n.as_str()));
            cfg.right.retain(|n| !remove_set.contains(n.as_str()));
        }
        if let Some(add) = &w.add_left {
            cfg.left.extend(add.iter().cloned());
        }
        if let Some(add) = &w.add_center {
            cfg.center.extend(add.iter().cloned());
        }
        if let Some(add) = &w.add_right {
            cfg.right.extend(add.iter().cloned());
        }
    }

    cfg
}

/// Human-readable diff between a merged-but-unsanitized config and its
/// `.sanitized()` output — becomes `set_bar_config`'s `warnings` list.
/// Pure and testable on its own; not a log-capture hack.
fn sanitize_diff(before: &BarLayoutConfig, after: &BarLayoutConfig) -> Vec<String> {
    let mut warnings = Vec::new();

    if before.appearance.height != after.appearance.height {
        warnings.push(format!(
            "height clamped: {} -> {}",
            before.appearance.height, after.appearance.height
        ));
    }
    if before.appearance.radius != after.appearance.radius {
        warnings.push(format!(
            "radius clamped: {} -> {}",
            before.appearance.radius, after.appearance.radius
        ));
    }
    if before.appearance.width != after.appearance.width {
        warnings.push("width fraction clamped to [0.2, 1.0]".to_string());
    }
    if before.appearance.margin != after.appearance.margin {
        warnings.push("negative margin zeroed".to_string());
    }
    if before.appearance.exclusive != after.appearance.exclusive {
        warnings.push("floating=true forced exclusive=false".to_string());
    }

    for (label, before_list, after_list) in [
        ("left", &before.left, &after.left),
        ("center", &before.center, &after.center),
        ("right", &before.right, &after.right),
    ] {
        let removed: Vec<&String> =
            before_list.iter().filter(|n| !after_list.contains(n)).collect();
        if !removed.is_empty() {
            warnings.push(format!("{label}: removed unknown widget(s) {removed:?}"));
        }
    }

    warnings
}

/// `set_bar_config()` result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SetBarConfigResult {
    pub ok: bool,
    /// `Some` only when `ok` — the config actually written (post-sanitize).
    pub applied: Option<BarConfigSnapshot>,
    pub warnings: Vec<String>,
    /// `Some` only when `!ok` — the save failure, for the agent to surface.
    pub error: Option<String>,
}

/// Disk-backed core: load → merge → sanitize → save. Never partially
/// writes — either the whole sanitized config lands on disk, or nothing
/// changes and `error` explains why (T201 rule: "no silent corrupt; keep
/// last-good cache").
pub fn set_bar_config(patch: &BarConfigPatch) -> SetBarConfigResult {
    let base = BarLayoutConfig::load();
    let merged = merge_patch(&base, patch);
    let sanitized = merged.sanitized();
    let warnings = sanitize_diff(&merged, &sanitized);

    match sanitized.save() {
        Ok(()) => {
            tracing::info!(
                path = %super::layout_config::config_path().display(),
                "bar: agent applied"
            );
            super::layout_config::update_cache(sanitized.clone());
            SetBarConfigResult {
                ok: true,
                applied: Some(snapshot(&sanitized)),
                warnings,
                error: None,
            }
        }
        Err(e) => {
            tracing::warn!("bar: agent set_bar_config save failed: {e}");
            SetBarConfigResult { ok: false, applied: None, warnings, error: Some(e.to_string()) }
        }
    }
}

/// Same as [`set_bar_config`], plus applies the new layout to the running
/// shell in-process (widget registry + window refresh) instead of relying
/// on the inotify hot-reload picking up the disk write on its own poll
/// cycle. Preferred whenever an `App` context is at hand (T201 rule).
pub fn set_bar_config_applied(patch: &BarConfigPatch, cx: &mut gpui::App) -> SetBarConfigResult {
    let result = set_bar_config(patch);
    if result.ok {
        super::layout_config::apply(cx);
        point_editor_at_bar_config(cx);
    }
    result
}

/// T203 dogfood glue: after an agent-driven `set_bar_config`, point the
/// existing `PreviewTarget` global at `bar.toml` in View mode — the same
/// global `FilesTab` sets on a click and `SidePanelRightView` already
/// observes to switch the panel to Editor (T194). This is the "minimal
/// last-config-path toast" the task allows in place of a real Follow UI
/// (T195 not built yet): the user sees *which file just changed* without
/// a new UI surface, by reusing one that already exists end-to-end.
fn point_editor_at_bar_config(cx: &mut gpui::App) {
    use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};

    if !cx.has_global::<PreviewTarget>() {
        cx.set_global(PreviewTarget::default());
    }
    let next_generation = cx.global::<PreviewTarget>().generation.wrapping_add(1);
    cx.set_global(PreviewTarget {
        path: Some(super::layout_config::config_path()),
        generation: next_generation,
        intent: PreviewIntent::View,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_defaults() -> BarLayoutConfig {
        BarLayoutConfig::default()
    }

    // --- list_widgets (pure) ---

    #[test]
    fn list_widgets_available_is_builtins_plus_plugins() {
        let cfg = cfg_defaults();
        let list = list_widgets(&cfg, &["my-plugin-widget".to_string()]);
        assert!(list.available.contains(&"clock".to_string()));
        assert!(list.available.contains(&"my-plugin-widget".to_string()));
        assert_eq!(list.left, cfg.left);
        assert_eq!(list.center, cfg.center);
        assert_eq!(list.right, cfg.right);
    }

    #[test]
    fn list_widgets_no_plugins_is_builtins_only() {
        let cfg = cfg_defaults();
        let list = list_widgets(&cfg, &[]);
        assert_eq!(list.available.len(), BUILTIN_NAMES.len());
    }

    // --- snapshot (pure) ---

    #[test]
    fn snapshot_reflects_config() {
        let mut cfg = cfg_defaults();
        cfg.appearance.height = 44.0;
        let snap = snapshot(&cfg);
        assert_eq!(snap.version, 2);
        assert_eq!(snap.appearance.height, 44.0);
        assert_eq!(snap.widgets.left, cfg.left);
    }

    // --- merge_patch (pure) — T201 required tests ---

    #[test]
    fn patch_height_only_preserves_widgets() {
        let base = cfg_defaults();
        let patch: BarConfigPatch =
            toml::from_str(r#"[appearance]
height = 44"#)
                .unwrap();
        let merged = merge_patch(&base, &patch);
        assert_eq!(merged.appearance.height, 44.0);
        assert_eq!(merged.left, base.left, "widgets must be untouched by an appearance-only patch");
        assert_eq!(merged.center, base.center);
        assert_eq!(merged.right, base.right);
    }

    #[test]
    fn patch_missing_keys_leave_current() {
        let base = cfg_defaults();
        let patch = BarConfigPatch::default();
        let merged = merge_patch(&base, &patch);
        assert_eq!(merged.appearance, base.appearance);
        assert_eq!(merged.left, base.left);
    }

    #[test]
    fn patch_full_array_replaces_section() {
        let base = cfg_defaults();
        let patch = BarConfigPatch {
            widgets: Some(WidgetsPatch { center: Some(vec!["mpris".to_string()]), ..Default::default() }),
            ..Default::default()
        };
        let merged = merge_patch(&base, &patch);
        assert_eq!(merged.center, vec!["mpris".to_string()]);
        assert_eq!(merged.left, base.left, "untouched sections stay as-is");
    }

    #[test]
    fn patch_remove_and_add_sugar() {
        let base = cfg_defaults();
        let patch = BarConfigPatch {
            widgets: Some(WidgetsPatch {
                remove: Some(vec!["cava".to_string()]),
                add_right: Some(vec!["clock".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let merged = merge_patch(&base, &patch);
        assert!(!merged.center.contains(&"cava".to_string()));
        // "clock" is already in the default right section — add_right must
        // not dedupe (that's the caller's problem if they double-add); this
        // test only proves the sugar actually appended.
        assert_eq!(
            merged.right.iter().filter(|n| *n == "clock").count(),
            base.right.iter().filter(|n| *n == "clock").count() + 1
        );
    }

    #[test]
    fn patch_bumps_version_to_two() {
        let mut base = cfg_defaults();
        base.version = None;
        let patch = BarConfigPatch::default();
        let merged = merge_patch(&base, &patch);
        assert_eq!(merged.version, Some(2), "agent writes must always be schema v2");
    }

    #[test]
    fn patch_unknown_appearance_string_degrades_not_errors() {
        let base = cfg_defaults();
        let patch = BarConfigPatch {
            appearance: Some(AppearancePatch { edge: Some("diagonal".to_string()), ..Default::default() }),
            ..Default::default()
        };
        let merged = merge_patch(&base, &patch);
        assert_eq!(merged.appearance.edge, BarEdge::default(), "bad enum string degrades to default, never panics");
    }

    // --- sanitize_diff / warnings ---

    #[test]
    fn sanitize_diff_reports_removed_unknown_widget() {
        let base = cfg_defaults();
        let patch = BarConfigPatch {
            widgets: Some(WidgetsPatch { add_right: Some(vec!["not-a-real-widget".to_string()]), ..Default::default() }),
            ..Default::default()
        };
        let merged = merge_patch(&base, &patch);
        let sanitized = merged.sanitized();
        let warnings = sanitize_diff(&merged, &sanitized);
        assert!(
            warnings.iter().any(|w| w.contains("not-a-real-widget")),
            "warnings must name the dropped widget, got: {warnings:?}"
        );
    }

    #[test]
    fn sanitize_diff_reports_floating_forces_exclusive() {
        let base = cfg_defaults();
        let patch = BarConfigPatch {
            appearance: Some(AppearancePatch { floating: Some(true), ..Default::default() }),
            ..Default::default()
        };
        let merged = merge_patch(&base, &patch);
        let sanitized = merged.sanitized();
        let warnings = sanitize_diff(&merged, &sanitized);
        assert!(warnings.iter().any(|w| w.contains("exclusive")));
    }

    #[test]
    fn sanitize_diff_empty_for_already_clean_patch() {
        let base = cfg_defaults();
        let patch: BarConfigPatch = toml::from_str(r#"[appearance]
height = 44"#).unwrap();
        let merged = merge_patch(&base, &patch);
        let sanitized = merged.sanitized();
        assert!(sanitize_diff(&merged, &sanitized).is_empty());
    }

    // --- roundtrip save/load against a temp dir, not the real user HOME ---
    //
    // `BarLayoutConfig::save`/`load` hardcode `config_path()` (real
    // `~/.config/chronos/bar.toml`) — there is no path injection point to
    // redirect them to a temp dir. To honor "roundtrip save/load temp dir
    // (not user HOME)" without touching the real user config, this test
    // exercises the *pure* merge → sanitize → serialize → parse round trip
    // directly against `toml::to_string_pretty`/`toml::from_str` on an
    // explicit in-memory value — the same serialization `save()` uses
    // internally, minus the real filesystem path. `set_bar_config`'s own
    // disk I/O is exactly `BarLayoutConfig::save`, already covered by
    // `layout_config`'s own tests against the real path convention.
    #[test]
    fn roundtrip_merge_sanitize_serialize_parse() {
        let base = cfg_defaults();
        let patch: BarConfigPatch = toml::from_str(
            r#"
            [appearance]
            height = 36
            radius = 8
            [widgets]
            right = ["clock", "battery"]
            "#,
        )
        .unwrap();
        let merged = merge_patch(&base, &patch).sanitized();

        let body = toml::to_string_pretty(&merged).expect("serialize");
        let parsed: BarLayoutConfig = toml::from_str(&body).expect("parse back");
        let reparsed_appearance = super::super::layout_config::gated_appearance(parsed.version, parsed.appearance);

        assert_eq!(reparsed_appearance.height, 36.0);
        assert_eq!(reparsed_appearance.radius, 8.0);
        assert_eq!(parsed.right, vec!["clock".to_string(), "battery".to_string()]);
        assert_eq!(parsed.version, Some(2));
    }
}

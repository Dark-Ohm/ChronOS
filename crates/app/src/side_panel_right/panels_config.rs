//! Right-panel rail tab layout — `~/.config/chronos/panels.toml` + hot-reload
//! (T219).
//!
//! Two groups per mode: `top` (above the spacer) and `bottom` (between the
//! spacer and the dock toggle). `System settings` (`editor_settings`) lives
//! in the bottom group by default — right above the dock button.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::App;
use inotify::{EventMask, Inotify, WatchMask};
use serde::{Deserialize, Serialize};

use crate::workspace_mode::WorkspaceMode;

use super::tabs::PanelTab;

const CONFIG_BASENAME: &str = "panels.toml";
const DEBOUNCE_MS: u64 = 300;

/// Per-mode rail groups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RailGroups {
    /// Tabs above the spacer (top of the rail).
    #[serde(default = "default_dev_top")]
    pub top: Vec<String>,
    /// Tabs between the spacer and the dock toggle (bottom of the rail).
    #[serde(default = "default_dev_bottom")]
    pub bottom: Vec<String>,
}

/// Per-mode rail config with a nested `[right.rail.<mode>]` structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RailConfig {
    #[serde(default)]
    pub developer: RailGroups,
    #[serde(default)]
    pub gamer: RailGroups,
}

/// Top-level config — wraps `version` + `[right.rail]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PanelLayoutConfig {
    pub version: u32,
    pub right: RightConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RightConfig {
    pub rail: RailConfig,
}

// --- Defaults ---------------------------------------------------------------

fn default_dev_top() -> Vec<String> {
    vec![
        "system".into(),
        // T294: Updates right after System (frequent entry point).
        "updates".into(),
        // T293: Notifications — same slot as for_mode (after Updates).
        "notifications".into(),
        "files".into(),
        "preview".into(),
        "hyprland_binds".into(),
        "acp_settings".into(),
    ]
}

fn default_dev_bottom() -> Vec<String> {
    // T296: Display (brightness + wallpaper) is the first button of the
    // bottom group, immediately above shell settings.
    vec!["display".into(), "editor_settings".into()]
}

fn default_gamer_top() -> Vec<String> {
    vec![
        "system".into(),
        // T294: Updates right after System (frequent entry point).
        "updates".into(),
        // T293: Notifications — same slot as for_mode (after Updates).
        "notifications".into(),
        "library".into(),
        "captures".into(),
        "acp_settings".into(),
        "hyprland_binds".into(),
    ]
}

fn default_gamer_bottom() -> Vec<String> {
    // T296: Display (brightness + wallpaper) leads the bottom group in Gamer too.
    vec!["display".into(), "editor_settings".into()]
}

impl Default for RailGroups {
    fn default() -> Self {
        Self {
            top: default_dev_top(),
            bottom: default_dev_bottom(),
        }
    }
}

impl RailConfig {
    fn default_developer() -> RailGroups {
        RailGroups {
            top: default_dev_top(),
            bottom: default_dev_bottom(),
        }
    }

    fn default_gamer() -> RailGroups {
        RailGroups {
            top: default_gamer_top(),
            bottom: default_gamer_bottom(),
        }
    }
}

impl Default for RightConfig {
    fn default() -> Self {
        Self {
            rail: RailConfig {
                developer: RailConfig::default_developer(),
                gamer: RailConfig::default_gamer(),
            },
        }
    }
}

impl Default for PanelLayoutConfig {
    fn default() -> Self {
        Self {
            version: 1,
            right: RightConfig::default(),
        }
    }
}

// --- Config path / cache / load / save --------------------------------------

static CONFIG_CACHE: OnceLock<Mutex<PanelLayoutConfig>> = OnceLock::new();

fn config_cache() -> &'static Mutex<PanelLayoutConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(PanelLayoutConfig::default()))
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

fn parent_dir() -> PathBuf {
    let p = config_path();
    p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
}

/// Cached layout (no disk I/O).
pub fn cached() -> PanelLayoutConfig {
    config_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Replace the cache. **Invariant: the stored config must be sanitized.**
pub fn update_cache(cfg: PanelLayoutConfig) {
    *config_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = cfg;
}

impl PanelLayoutConfig {
    /// Load from disk. Missing → default (no silent write). Bad parse → warn + default.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<PanelLayoutConfig>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(
                        "panels: failed to parse {}: {e}, using defaults",
                        path.display()
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!("panels: {} not found, using defaults", path.display());
                Self::default()
            }
            Err(e) => {
                tracing::warn!("panels: read {} failed: {e}, using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Persist.
    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        std::fs::write(path, body)
    }

    /// Drop unknown names (warn), deduplicate, and ensure the result is
    /// never empty (fall back to mode defaults).
    ///
    /// Also appends any tab that exists in the mode set but is missing from
    /// both groups — prevents a manual config edit from silently losing a tab.
    pub fn sanitized(&self) -> Self {
        let mode_dev = PanelTab::for_mode(WorkspaceMode::Developer);
        let mode_gamer = PanelTab::for_mode(WorkspaceMode::Gamer);

        let sanitize_groups =
            |groups: &RailGroups, mode_tabs: &[PanelTab]| -> RailGroups {
                let (top, bottom) = sanitize_pair(&groups.top, &groups.bottom, mode_tabs);
                RailGroups { top, bottom }
            };

        Self {
            version: self.version,
            right: RightConfig {
                rail: RailConfig {
                    developer: sanitize_groups(
                        &self.right.rail.developer,
                        &mode_dev,
                    ),
                    gamer: sanitize_groups(
                        &self.right.rail.gamer,
                        &mode_gamer,
                    ),
                },
            },
        }
    }

    /// Move a tab within its group by `delta` (-1 up, +1 down in rail
    /// coordinates). When the tab hits a group boundary, it moves to the
    /// adjacent group. Returns `true` if the order changed.
    pub fn move_tab(
        &mut self,
        mode: WorkspaceMode,
        tab: PanelTab,
        delta: isize,
    ) -> bool {
        let groups = match mode {
            WorkspaceMode::Developer => &mut self.right.rail.developer,
            WorkspaceMode::Gamer => &mut self.right.rail.gamer,
        };

        // Find which group the tab is in.
        let top_idx = groups.top.iter().position(|n| {
            PanelTab::parse_id(n) == Some(tab)
        });
        let bottom_idx = groups.bottom.iter().position(|n| {
            PanelTab::parse_id(n) == Some(tab)
        });

        match (top_idx, bottom_idx) {
            (Some(idx), None) => {
                // In top group — move within top, or cross to bottom.
                let new_idx = idx as isize + delta;
                if new_idx < 0 {
                    // Move to end of bottom group.
                    let tab_id = tab.id().to_string();
                    groups.top.remove(idx);
                    groups.bottom.push(tab_id);
                    return true;
                } else if new_idx as usize >= groups.top.len() {
                    // Move to start of bottom group.
                    let tab_id = tab.id().to_string();
                    groups.top.remove(idx);
                    groups.bottom.insert(0, tab_id);
                    return true;
                }
                groups.top.swap(idx, new_idx as usize);
                true
            }
            (None, Some(idx)) => {
                // In bottom group — move within bottom, or cross to top.
                let new_idx = idx as isize + delta;
                if new_idx < 0 {
                    // Move to end of top group.
                    let tab_id = tab.id().to_string();
                    groups.bottom.remove(idx);
                    groups.top.push(tab_id);
                    return true;
                } else if new_idx as usize >= groups.bottom.len() {
                    // Move to start of top group.
                    let tab_id = tab.id().to_string();
                    groups.bottom.remove(idx);
                    groups.top.insert(0, tab_id);
                    return true;
                }
                groups.bottom.swap(idx, new_idx as usize);
                true
            }
            _ => false, // Tab not found — no-op.
        }
    }
}

/// Resolve a `PanelLayoutConfig` into per-mode `(top, bottom)` `Vec<PanelTab>`.
/// Falls back to `PanelTab::for_mode` if the config is empty after sanitization.
pub fn resolve_grouped(
    mode: WorkspaceMode,
    config: &PanelLayoutConfig,
) -> (Vec<PanelTab>, Vec<PanelTab>) {
    let groups = match mode {
        WorkspaceMode::Developer => &config.right.rail.developer,
        WorkspaceMode::Gamer => &config.right.rail.gamer,
    };        let parse = |names: &[String]| -> Vec<PanelTab> {
        let mut seen = std::collections::HashSet::new();
        names
            .iter()
            .filter_map(|n| PanelTab::parse_id(n))
            .filter(|tab| seen.insert(*tab))
            .collect()
    };

    let top = parse(&groups.top);
    let bottom = parse(&groups.bottom);

    // If both groups are empty after parsing, fall back to mode defaults.
    if top.is_empty() && bottom.is_empty() {
        let mode_tabs = PanelTab::for_mode(mode);
        // Split: everything except EditorSettings goes top, EditorSettings goes bottom.
        let mut fallback_top = Vec::new();
        let mut fallback_bottom = Vec::new();
        for tab in mode_tabs {
            if tab == PanelTab::EditorSettings {
                fallback_bottom.push(tab);
            } else {
                fallback_top.push(tab);
            }
        }
        (fallback_top, fallback_bottom)
    } else {
        (top, bottom)
    }
}

// --- Sanitization helpers ---------------------------------------------------

/// Sanitize top + bottom group pair: drop unknown, deduplicate, ensure
/// non-empty, append missing mode tabs to top.
fn sanitize_pair(
    top: &[String],
    bottom: &[String],
    mode_tabs: &[PanelTab],
) -> (Vec<String>, Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    let mut out_top = Vec::new();
    let mut out_bottom = Vec::new();

    // Parse and deduplicate top.
    for name in top {
        match PanelTab::parse_id(name) {
            Some(tab) => {
                if seen.insert(tab) {
                    out_top.push(tab.id().to_string());
                }
            }
            None => {
                tracing::warn!(name, "panels: unknown tab id in top group, skipping");
            }
        }
    }

    // Parse and deduplicate bottom.
    for name in bottom {
        match PanelTab::parse_id(name) {
            Some(tab) => {
                if seen.insert(tab) {
                    out_bottom.push(tab.id().to_string());
                }
            }
            None => {
                tracing::warn!(name, "panels: unknown tab id in bottom group, skipping");
            }
        }
    }

    // If both groups are empty after dedup (all unknown/garbage input),
    // fall back to mode defaults before appending missing tabs.
    if out_top.is_empty() && out_bottom.is_empty() {
        for tab in mode_tabs {
            if *tab == PanelTab::EditorSettings {
                out_bottom.push(tab.id().to_string());
            } else {
                out_top.push(tab.id().to_string());
            }
        }
        // Rebuild seen from the fallback so the append step below is a no-op.
        for name in out_top.iter().chain(out_bottom.iter()) {
            if let Some(tab) = PanelTab::parse_id(name) {
                seen.insert(tab);
            }
        }
    }

    // Append any mode tab that was lost (neither top nor bottom).
    for tab in mode_tabs {
        if !seen.contains(tab) {
            tracing::warn!(
                tab = tab.id(),
                "panels: tab in mode set but missing from config — appending to top"
            );
            out_top.push(tab.id().to_string());
        }
    }

    (out_top, out_bottom)
}

// --- Hot-reload -------------------------------------------------------------

/// inotify hot-reload for `panels.toml`.
pub fn spawn_watcher(cx: &mut App) {
    let parent = parent_dir();
    if !parent.is_dir() {
        tracing::debug!(
            "panels: parent dir {} missing, hot-reload disabled",
            parent.display()
        );
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let watch_target = parent.clone();

    std::thread::Builder::new()
        .name("panels-layout-inotify".into())
        .spawn(move || {
            let mut inotify = match Inotify::init() {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("panels: inotify init failed: {e}");
                    return;
                }
            };
            let mask = WatchMask::CLOSE_WRITE
                .union(WatchMask::MOVED_TO)
                .union(WatchMask::CREATE)
                .union(WatchMask::DELETE)
                .union(WatchMask::MODIFY);
            if let Err(e) = inotify.watches().add(&watch_target, mask) {
                tracing::error!("panels: failed to watch {}: {e}", watch_target.display());
                return;
            }
            let target = std::ffi::OsStr::new(CONFIG_BASENAME);
            let mut buf = [0u8; 4096];
            loop {
                let events = match inotify.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("panels: inotify read error: {e}");
                        break;
                    }
                };
                let mut changed = false;
                for ev in events {
                    if ev.mask.contains(EventMask::ISDIR) {
                        continue;
                    }
                    if let Some(name) = ev.name {
                        if name == target {
                            changed = true;
                        }
                    }
                }
                if changed && tx.send(()).is_err() {
                    break;
                }
            }
        })
        .expect("panels: failed to spawn inotify thread");

    cx.spawn(async move |cx| {
        let mut deadline: Option<tokio::time::Instant> = None;
        loop {
            let timer = async {
                match deadline {
                    Some(d) => tokio::time::sleep_until(d).await,
                    None => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                _ = rx.recv() => {
                    deadline = Some(tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                }
                _ = timer => {
                    deadline = None;
                    let _ = cx.update(|cx| {
                        apply(cx);
                        tracing::info!(
                            "panels: hot-reloaded from {}",
                            config_path().display()
                        );
                    });
                }
            }
        }
    })
    .detach();
}

/// Load → sanitize → cache → notify views to repaint.
pub fn apply(cx: &mut App) {
    let cfg = PanelLayoutConfig::load().sanitized();
    update_cache(cfg);
    cx.refresh_windows();
}

/// Public rail-move entry point — called from `side_panel_right::view`'s
/// on_move closure and from any future surface that proxies a reorder
/// (e.g. a non-edit-mode drag surface if T219 gets one later).
///
/// Mirrors `bar::layout_config::move_widget`: read cache, attempt the move
/// inside that snapshot, persist on success, refresh cache, repaint. The
/// closure at the call site is meant to be a single `move(cx, mode,
/// tab, delta)` invocation — no cache IO in the click handler, no risk of
/// the closure drifting from the persistence path the tests cover.
///
/// **Returns `true` when the cache AND disk were both updated.** Cache
/// updates and disk updates are deliberately committed together (or both
/// skipped): a `save()` error logs and returns `false` without touching
/// the cache, so a panel restart would re-read the previous on-disk
/// order instead of showing a phantom reorder that lives only in memory.
/// The user reads the `warn!` and can react; a stale UI is the price of
/// not lying about persistence.
pub fn move_tab(cx: &mut App, mode: WorkspaceMode, tab: PanelTab, delta: isize) -> bool {
    let mut cfg = cached();
    if !cfg.move_tab(mode, tab, delta) {
        return false;
    }
    if let Err(e) = cfg.save() {
        tracing::warn!("panels: failed to save panels.toml on move ({tab:?}, delta={delta}): {e}");
        return false;
    }
    update_cache(cfg);
    cx.refresh_windows();
    tracing::info!(?mode, tab = tab.id(), delta, "panels: rail tab moved and persisted");
    true
}

// --- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_editor_settings_in_bottom_for_both_modes() {
        let cfg = PanelLayoutConfig::default();
        let dev = &cfg.right.rail.developer;
        let gamer = &cfg.right.rail.gamer;

        assert!(
            dev.bottom.contains(&"editor_settings".into()),
            "Developer default must have editor_settings in bottom: {:?}",
            dev.bottom
        );
        assert!(
            gamer.bottom.contains(&"editor_settings".into()),
            "Gamer default must have editor_settings in bottom: {:?}",
            gamer.bottom
        );
    }

    #[test]
    fn sanitize_drops_unknown_and_deduplicates() {
        let cfg = PanelLayoutConfig {
            right: RightConfig {
                rail: RailConfig {
                    developer: RailGroups {
                        top: vec![
                            "system".into(),
                            "nope".into(),
                            "files".into(),
                            "system".into(), // duplicate
                        ],
                        bottom: vec!["editor_settings".into()],
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        let s = cfg.sanitized();
        let dev = &s.right.rail.developer;
        // "system" deduped, "nope" dropped — then missing mode tabs appended
        // in mode order. T294: `updates` is now a mode tab, so it is
        // appended too (right after the already-present system/files).
        assert_eq!(
            dev.top,
            vec![
                "system",
                "files",
                "updates",
                "notifications",
                "preview",
                "hyprland_binds",
                "acp_settings",
                "display"
            ]
        );
        assert_eq!(dev.bottom, vec!["editor_settings"]);
    }

    #[test]
    fn sanitize_appends_missing_mode_tab() {
        // Remove system from top — it should be appended back.
        let cfg = PanelLayoutConfig {
            right: RightConfig {
                rail: RailConfig {
                    developer: RailGroups {
                        top: vec!["files".into()],
                        bottom: vec!["editor_settings".into()],
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        let s = cfg.sanitized();
        let dev = &s.right.rail.developer;
        assert!(
            dev.top.contains(&"system".into()),
            "system must be appended: {:?}",
            dev.top
        );
    }

    #[test]
    fn sanitize_empty_falls_back_to_mode_defaults() {
        let cfg = PanelLayoutConfig {
            right: RightConfig {
                rail: RailConfig {
                    developer: RailGroups {
                        top: vec!["garbage".into()],
                        bottom: vec!["also-bad".into()],
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        let s = cfg.sanitized();
        let dev = &s.right.rail.developer;
        // Fallback: system, files, preview, hyprland_binds, acp_settings in top;
        // editor_settings in bottom.
        assert!(!dev.top.is_empty(), "fallback top must not be empty");
        assert!(!dev.bottom.is_empty(), "fallback bottom must not be empty");
        assert_eq!(dev.bottom, vec!["editor_settings"]);
    }

    #[test]
    fn move_within_top_group_swaps() {
        let mut cfg = PanelLayoutConfig::default();
        // Developer top: [system, updates, notifications, files, preview,
        // hyprland_binds, acp_settings]. Move files (index 3) up (delta -1)
        // → swaps with notifications: [system, updates, files, ...].
        assert!(cfg.move_tab(WorkspaceMode::Developer, PanelTab::Files, -1));
        let top: Vec<&str> = cfg
            .right
            .rail
            .developer
            .top
            .iter()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(top[0], "system");
        assert_eq!(top[1], "updates");
        assert_eq!(top[2], "files");
        assert_eq!(top[3], "notifications");
    }

    #[test]
    fn move_first_in_top_crosses_to_bottom() {
        let mut cfg = PanelLayoutConfig::default();
        // Move system (first in top, delta -1) → goes to end of bottom.
        assert!(cfg.move_tab(WorkspaceMode::Developer, PanelTab::System, -1));
        let dev = &cfg.right.rail.developer;
        assert!(
            !dev.top.contains(&"system".into()),
            "system must leave top: {:?}",
            dev.top
        );
        assert_eq!(dev.bottom.last().unwrap(), "system");
    }

    #[test]
    fn move_last_in_bottom_crosses_to_top() {
        let mut cfg = PanelLayoutConfig::default();
        // editor_settings is the only item in bottom (index 0, len 1).
        // delta +1 → new_idx = 1 >= len → cross to start of top.
        assert!(cfg.move_tab(
            WorkspaceMode::Developer,
            PanelTab::EditorSettings,
            1
        ));
        let dev = &cfg.right.rail.developer;
        assert!(
            !dev.bottom.contains(&"editor_settings".into()),
            "editor_settings must leave bottom: {:?}",
            dev.bottom
        );
        assert_eq!(dev.top[0], "editor_settings");
    }

    #[test]
    fn move_tab_not_in_config_is_noop() {
        let mut cfg = PanelLayoutConfig::default();
        // Terminal is not in the default config at all.
        assert!(!cfg.move_tab(WorkspaceMode::Developer, PanelTab::Terminal, 1));
    }

    #[test]
    fn resolve_grouped_uses_config_values() {
        let cfg = PanelLayoutConfig::default();
        let (top, bottom) = resolve_grouped(WorkspaceMode::Developer, &cfg);
        // system, updates, notifications, files, preview, hyprland_binds, acp_settings
        assert_eq!(top.len(), 7);
        assert_eq!(top[0], PanelTab::System);
        assert_eq!(top[1], PanelTab::Updates); // T294
        assert_eq!(top[2], PanelTab::Notifications); // T293
        assert_eq!(bottom.len(), 2); // display, editor_settings
        assert_eq!(bottom[0], PanelTab::Display);
        assert_eq!(bottom[1], PanelTab::EditorSettings);
    }

    #[test]
    fn resolve_grouped_empty_falls_back() {
        let cfg = PanelLayoutConfig {
            right: RightConfig {
                rail: RailConfig {
                    developer: RailGroups {
                        top: vec![],
                        bottom: vec![],
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        let (top, bottom) = resolve_grouped(WorkspaceMode::Developer, &cfg);
        assert!(!top.is_empty());
        assert_eq!(bottom, vec![PanelTab::EditorSettings]);
    }

    #[test]
    fn resolve_grouped_deduplicates() {
        let cfg = PanelLayoutConfig {
            right: RightConfig {
                rail: RailConfig {
                    developer: RailGroups {
                        top: vec!["system".into(), "system".into(), "files".into()],
                        bottom: vec!["editor_settings".into()],
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        let (top, _) = resolve_grouped(WorkspaceMode::Developer, &cfg);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0], PanelTab::System);
        assert_eq!(top[1], PanelTab::Files);
    }

    #[test]
    fn move_within_bottom_group_swaps() {
        let mut cfg = PanelLayoutConfig {
            right: RightConfig {
                rail: RailConfig {
                    gamer: RailGroups {
                        top: vec!["system".into()],
                        bottom: vec![
                            "editor_settings".into(),
                            "hyprland_binds".into(),
                            "acp_settings".into(),
                        ],
                    },
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        // Move editor_settings (index 0) down (delta +1) → swaps with hyprland_binds.
        assert!(cfg.move_tab(
            WorkspaceMode::Gamer,
            PanelTab::EditorSettings,
            1
        ));
        let bottom = &cfg.right.rail.gamer.bottom;
        assert_eq!(bottom[0], "hyprland_binds");
        assert_eq!(bottom[1], "editor_settings");
    }
}

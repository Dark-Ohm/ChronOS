//! Bar widget layout — `~/.config/chronos/bar.toml` + hot-reload (T134).
//!
//! Order of names in each section is registration order. Default matches
//! historical `register_builtin` (byte-identical lists).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::{App, BorrowAppContext};
use inotify::{EventMask, Inotify, WatchMask};
use serde::{Deserialize, Serialize};

const CONFIG_BASENAME: &str = "bar.toml";
const DEBOUNCE_MS: u64 = 300;

/// Known builtin widget names (for validation / edit UI).
pub const BUILTIN_NAMES: &[&str] = &[
    "dock",
    "separator",
    "workspaces",
    "mpris",
    "cava",
    "project",
    "volume",
    "network",
    "tray",
    "updates",
    "system",
    "notification_bell",
    "battery",
    "clock",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BarLayoutConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
}

impl Default for BarLayoutConfig {
    fn default() -> Self {
        // Must match pre-T134 `register_builtin` order exactly.
        Self {
            left: vec![
                "dock".into(),
                "separator".into(),
                "workspaces".into(),
            ],
            center: vec!["mpris".into(), "cava".into()],
            right: vec![
                "project".into(),
                "separator".into(),
                "volume".into(),
                "network".into(),
                "tray".into(),
                "updates".into(),
                "system".into(),
                "notification_bell".into(),
                "separator".into(),
                "battery".into(),
                "clock".into(),
            ],
        }
    }
}

static CONFIG_CACHE: OnceLock<Mutex<BarLayoutConfig>> = OnceLock::new();

fn config_cache() -> &'static Mutex<BarLayoutConfig> {
    CONFIG_CACHE.get_or_init(|| Mutex::new(BarLayoutConfig::default()))
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
pub fn cached() -> BarLayoutConfig {
    config_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

pub fn update_cache(cfg: BarLayoutConfig) {
    *config_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = cfg;
}

impl BarLayoutConfig {
    /// Load from disk. Missing → default (no silent write). Bad parse → warn + default.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<BarLayoutConfig>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(
                        "bar: failed to parse {}: {e}, using defaults",
                        path.display()
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!("bar: {} not found, using defaults", path.display());
                Self::default()
            }
            Err(e) => {
                tracing::warn!("bar: read {} failed: {e}, using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Persist (user/edit-mode save only — not on mere load).
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

    /// Drop unknown names (warn). Keeps separators and duplicates.
    pub fn sanitized(&self) -> Self {
        let filter = |names: &[String]| -> Vec<String> {
            names
                .iter()
                .filter(|n| {
                    let ok = BUILTIN_NAMES.contains(&n.as_str());
                    if !ok {
                        tracing::warn!("bar: unknown widget name '{n}', skipping");
                    }
                    ok
                })
                .cloned()
                .collect()
        };
        Self {
            left: filter(&self.left),
            center: filter(&self.center),
            right: filter(&self.right),
        }
    }

    /// Flat list of (name, section) in paint order within each section.
    pub fn slots(&self) -> Vec<(String, chronos_luau::bar::BarSection)> {
        use chronos_luau::bar::BarSection;
        let mut out = Vec::new();
        for n in &self.left {
            out.push((n.clone(), BarSection::Left));
        }
        for n in &self.center {
            out.push((n.clone(), BarSection::Center));
        }
        for n in &self.right {
            out.push((n.clone(), BarSection::Right));
        }
        out
    }

    /// Move widget at `index` within `section` by `delta` (-1 left, +1 right).
    /// Returns true if order changed.
    pub fn move_in_section(
        &mut self,
        section: chronos_luau::bar::BarSection,
        index: usize,
        delta: isize,
    ) -> bool {
        use chronos_luau::bar::BarSection;
        let list = match section {
            BarSection::Left => &mut self.left,
            BarSection::Center => &mut self.center,
            BarSection::Right => &mut self.right,
        };
        if list.is_empty() || index >= list.len() {
            return false;
        }
        let new_i = index as isize + delta;
        if new_i < 0 || new_i as usize >= list.len() {
            return false;
        }
        list.swap(index, new_i as usize);
        true
    }
}

/// Load → sanitize → cache → apply to registry → refresh windows.
pub fn apply(cx: &mut App) {
    let cfg = BarLayoutConfig::load().sanitized();
    update_cache(cfg.clone());
    super::widgets::apply_layout(cx, &cfg);
    reregister_plugin_widgets(cx);
    cx.refresh_windows();
    tracing::info!(
        left = cfg.left.len(),
        center = cfg.center.len(),
        right = cfg.right.len(),
        "bar: layout applied"
    );
}

/// Mutate cached layout, save, re-apply.
pub fn move_widget(
    cx: &mut App,
    section: chronos_luau::bar::BarSection,
    index: usize,
    delta: isize,
) {
    let mut cfg = cached();
    if !cfg.move_in_section(section, index, delta) {
        return;
    }
    if let Err(e) = cfg.save() {
        tracing::warn!("bar: failed to save bar.toml: {e}");
        return;
    }
    update_cache(cfg.clone());
    super::widgets::apply_layout(cx, &cfg);
    reregister_plugin_widgets(cx);
    cx.refresh_windows();
    tracing::info!(?section, index, delta, "bar: moved widget");
}

/// `apply_layout` clears the registry — re-attach Luau bar widgets if plugins
/// are already loaded (after `main` sets `PluginManager` global).
fn reregister_plugin_widgets(cx: &mut App) {
    if !cx.has_global::<chronos_luau::PluginManager>() {
        return;
    }
    cx.update_global::<chronos_luau::PluginManager, _>(|mgr, cx| {
        crate::plugin_bridge::register_plugin_widgets(mgr, cx);
    });
}

/// inotify hot-reload for `bar.toml` (theme_config pattern).
pub fn spawn_watcher(cx: &mut App) {
    let parent = parent_dir();
    if !parent.is_dir() {
        tracing::debug!(
            "bar: parent dir {} missing, layout hot-reload disabled",
            parent.display()
        );
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let watch_target = parent.clone();

    std::thread::Builder::new()
        .name("bar-layout-inotify".into())
        .spawn(move || {
            let mut inotify = match Inotify::init() {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("bar: inotify init failed: {e}");
                    return;
                }
            };
            let mask = WatchMask::CLOSE_WRITE
                .union(WatchMask::MOVED_TO)
                .union(WatchMask::CREATE)
                .union(WatchMask::DELETE)
                .union(WatchMask::MODIFY);
            if let Err(e) = inotify.watches().add(&watch_target, mask) {
                tracing::error!("bar: failed to watch {}: {e}", watch_target.display());
                return;
            }
            let target = std::ffi::OsStr::new(CONFIG_BASENAME);
            let mut buf = [0u8; 4096];
            loop {
                let events = match inotify.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("bar: inotify read error: {e}");
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
        .expect("bar: failed to spawn inotify thread");

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
                            "bar: hot-reloaded layout from {}",
                            config_path().display()
                        );
                    });
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_luau::bar::BarSection;

    #[test]
    fn default_matches_historical_builtin_order() {
        let d = BarLayoutConfig::default();
        assert_eq!(d.left, vec!["dock", "separator", "workspaces"]);
        assert_eq!(d.center, vec!["mpris", "cava"]);
        assert_eq!(
            d.right,
            vec![
                "project",
                "separator",
                "volume",
                "network",
                "tray",
                "updates",
                "system",
                "notification_bell",
                "separator",
                "battery",
                "clock",
            ]
        );
    }

    #[test]
    fn sanitize_drops_unknown() {
        let cfg = BarLayoutConfig {
            left: vec!["dock".into(), "nope".into()],
            center: vec![],
            right: vec!["clock".into()],
        };
        let s = cfg.sanitized();
        assert_eq!(s.left, vec!["dock"]);
        assert_eq!(s.right, vec!["clock"]);
    }

    #[test]
    fn move_in_section_swaps() {
        let mut cfg = BarLayoutConfig::default();
        assert!(cfg.move_in_section(BarSection::Center, 0, 1));
        assert_eq!(cfg.center, vec!["cava", "mpris"]);
        assert!(!cfg.move_in_section(BarSection::Center, 0, -1));
    }

    #[test]
    fn empty_section_ok() {
        let cfg = BarLayoutConfig {
            left: vec![],
            center: vec!["cava".into()],
            right: vec![],
        };
        assert!(cfg.sanitized().left.is_empty());
    }
}

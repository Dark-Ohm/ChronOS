//! Persistence for launcher favorites / recents / folders (T265-C).
//!
//! Single config file `~/.config/chronos/launcher.toml` — deliberately NOT
//! `frecency.toml`. Writes are debounced (at most one per `SAVE_DEBOUNCE`, and
//! a final `flush()` on close), and go through read-modify-write over
//! `toml::Value`: we read whatever is on disk, replace only the three keys we
//! own (`favorites`, `recents`, `folders`), and write the merged table back.
//! Unknown top-level sections written by other code therefore survive — we do
//! NOT serde-dump a struct blindly over the file (T284 / frame.toml lesson).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_signals::signal::{Mutable, Signal};
use gpui::App;
use serde::{Deserialize, Serialize};

/// Default number of recents surfaced when the config has no `[recents]`.
pub const DEFAULT_RECENTS_LIMIT: usize = 8;
/// Max one disk write per window — batches a DnD reorder burst.
pub const SAVE_DEBOUNCE: Duration = Duration::from_millis(600);
/// File-watcher debounce (frame/bar pattern) — T265-G hot-reload.
pub const WATCH_DEBOUNCE_MS: u64 = 300;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoritesConfig {
    /// Manual ordering of favorite app ids; first = leftmost.
    #[serde(default)]
    pub order: Vec<String>,
    /// When true, favorites render alphabetically instead of manual order.
    #[serde(default)]
    pub sort_alpha: bool,
    /// When true, favorite cells show icons only (labels hidden).
    #[serde(default)]
    pub hide_labels: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentsConfig {
    #[serde(default = "default_recents_limit")]
    pub limit: usize,
}

impl Default for RecentsConfig {
    fn default() -> Self {
        Self {
            limit: DEFAULT_RECENTS_LIMIT,
        }
    }
}

fn default_recents_limit() -> usize {
    DEFAULT_RECENTS_LIMIT
}

/// Header system-action order (T265-F): `[system_actions] order = [...]`.
/// Empty list = the built-in default order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemActionsConfig {
    #[serde(default)]
    pub order: Vec<String>,
}

/// Launcher appearance (T265-G): `[appearance]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// Open the launcher with the grid collapsed (compact mode).
    #[serde(default)]
    pub compact_default: bool,
    /// Hide labels under grid cells (icons only).
    #[serde(default)]
    pub hide_labels: bool,
}

/// Grid geometry (T265-G): `[grid] columns/rows/icon_size`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridConfig {
    #[serde(default = "default_columns")]
    pub columns: usize,
    #[serde(default = "default_rows")]
    pub rows: usize,
    #[serde(default = "default_icon_size")]
    pub icon_size: u32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: default_columns(),
            rows: default_rows(),
            icon_size: default_icon_size(),
        }
    }
}

impl GridConfig {
    /// Clamp raw toml values into sane ranges (T265-G sanitize): columns
    /// 1..=12, rows 1..=10, icon 16..=64px. Garbage in the file must not
    /// divide-by-zero (`move_2d` with columns=0) or render a zero-size cell.
    pub fn sanitized(&self) -> Self {
        Self {
            columns: self.columns.clamp(1, 12),
            rows: self.rows.clamp(1, 10),
            icon_size: self.icon_size.clamp(16, 64),
        }
    }
}

fn default_columns() -> usize {
    7
}

fn default_rows() -> usize {
    4
}

fn default_icon_size() -> u32 {
    36
}

/// Search behavior (T265-G): `[search]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Include user-hidden apps in results.
    #[serde(default)]
    pub include_hidden: bool,
    /// Show the inline-completion tail in the field.
    #[serde(default = "default_true")]
    pub inline_completion: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            include_hidden: false,
            inline_completion: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Category visibility (T265-G): `[categories] hide = [...]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoriesConfig {
    /// Category names to hide from the bar (empty = show all).
    #[serde(default)]
    pub hide: Vec<String>,
}

/// A user folder: a named, manual grouping of app ids.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub apps: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default)]
    pub favorites: FavoritesConfig,
    #[serde(default)]
    pub recents: RecentsConfig,
    #[serde(default)]
    pub folders: Vec<Folder>,
    /// User-hidden app ids (T265-D "Hide from list") — launcher-level
    /// NoDisplay, NOT a `.desktop` edit on disk. Hidden entries stay in the
    /// applications service and can be surfaced again later (T265-G).
    #[serde(default)]
    pub hidden: Vec<String>,
    /// Header system-action order (T265-F). Empty → default order.
    #[serde(default)]
    pub system_actions: SystemActionsConfig,
    /// Appearance (T265-G): compact default + grid labels.
    #[serde(default)]
    pub appearance: AppearanceConfig,
    /// Grid geometry (T265-G).
    #[serde(default)]
    pub grid: GridConfig,
    /// Search behavior (T265-G).
    #[serde(default)]
    pub search: SearchConfig,
    /// Category visibility (T265-G).
    #[serde(default)]
    pub categories: CategoriesConfig,
}

/// Process-wide change signal: fires whenever the config mutates (favorites /
/// hidden / folders), so the launcher view re-filters and re-renders sections.
static CHANGED: OnceLock<Mutable<()>> = OnceLock::new();

fn changed() -> &'static Mutable<()> {
    CHANGED.get_or_init(|| Mutable::new(()))
}

/// Subscribe to config mutations (T265-D). The launcher view uses this to drop
/// newly-hidden ids from the grid and to refresh Favorites/Folders immediately.
pub fn subscribe() -> impl Signal<Item = ()> {
    changed().signal()
}

fn bump_changed() {
    *changed().lock_mut() = ();
}

struct Store {
    config: LauncherConfig,
    dirty: bool,
    last_save: Instant,
}

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();

fn store() -> &'static Mutex<Store> {
    STORE.get_or_init(|| {
        Mutex::new(Store {
            config: load(),
            dirty: false,
            last_save: Instant::now(),
        })
    })
}

/// Path of the launcher config file (exposed for tests/diagnostics).
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/launcher.toml")
}

fn load() -> LauncherConfig {
    read_config(&config_path())
}

/// Parse a `LauncherConfig` from `path`; missing/corrupt file → defaults.
pub fn read_config(path: &Path) -> LauncherConfig {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => LauncherConfig::default(),
    }
}

/// Read-modify-write: merge `config` over the existing file, preserving keys we
/// do not own. Only `favorites` / `recents` / `folders` are replaced.
pub fn write_config(path: &Path, config: &LauncherConfig) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut root: toml::Table = match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => toml::Table::new(),
    };
    let ours = toml::Value::try_from(config).expect("LauncherConfig is always serializable");
    if let toml::Value::Table(table) = ours {
        for key in [
            "favorites",
            "recents",
            "folders",
            "hidden",
            "system_actions",
            "appearance",
            "grid",
            "search",
            "categories",
        ] {
            if let Some(value) = table.get(key) {
                root.insert(key.to_string(), value.clone());
            }
        }
    }
    let content = toml::to_string_pretty(&toml::Value::Table(root)).expect("toml table serializes");
    std::fs::write(path, content)
}

fn save_locked(config: &LauncherConfig) {
    if let Err(err) = write_config(&config_path(), config) {
        tracing::warn!("launcher_config: failed to save: {err}");
    }
}

/// Snapshot of the current config.
pub fn get() -> LauncherConfig {
    store().lock().unwrap().config.clone()
}

/// Mutate the config, then persist (debounced). The mutation always applies to
/// the in-memory store immediately; disk is written at most once per
/// `SAVE_DEBOUNCE`, with a final `flush()` covering the tail.
pub fn update(f: impl FnOnce(&mut LauncherConfig)) {
    let mut s = store().lock().unwrap();
    f(&mut s.config);
    bump_changed();
    s.dirty = true;
    if s.last_save.elapsed() >= SAVE_DEBOUNCE {
        save_locked(&s.config);
        s.dirty = false;
        s.last_save = Instant::now();
    }
}

/// Force a persist — call on launcher close / shell shutdown.
pub fn flush() {
    let mut s = store().lock().unwrap();
    if s.dirty {
        save_locked(&s.config);
        s.dirty = false;
        s.last_save = Instant::now();
    }
}

/// Reload config from disk and notify subscribers (file watcher hot-reload,
/// T265-G). The OSD and the settings page both subscribe to `subscribe()`; a
/// file change re-reads config and re-renders them without a restart.
pub fn reload() {
    let mut s = store().lock().unwrap();
    s.config = load();
    bump_changed();
}

/// inotify hot-reload for `launcher.toml` (frame/bar pattern, 300 ms debounce).
pub fn spawn_watcher(cx: &mut App) {
    let path = config_path();
    let Some(parent) = path.parent().map(|p| p.to_path_buf()) else {
        return;
    };
    if !parent.is_dir() {
        tracing::debug!(
            "launcher_config: parent dir {} missing, hot-reload disabled",
            parent.display()
        );
        return;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let watch_target = parent.clone();
    let basename = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();

    std::thread::Builder::new()
        .name("launcher-config-inotify".into())
        .spawn(move || {
            let mut inotify = match inotify::Inotify::init() {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("launcher_config: inotify init failed: {e}");
                    return;
                }
            };
            let mask = inotify::WatchMask::CLOSE_WRITE
                .union(inotify::WatchMask::MOVED_TO)
                .union(inotify::WatchMask::CREATE)
                .union(inotify::WatchMask::DELETE)
                .union(inotify::WatchMask::MODIFY);
            if let Err(e) = inotify.watches().add(&watch_target, mask) {
                tracing::error!(
                    "launcher_config: failed to watch {}: {e}",
                    watch_target.display()
                );
                return;
            }
            let mut buf = [0u8; 4096];
            loop {
                let events = match inotify.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("launcher_config: inotify read error: {e}");
                        break;
                    }
                };
                let mut changed = false;
                for ev in events {
                    if ev.mask.contains(inotify::EventMask::ISDIR) {
                        continue;
                    }
                    if ev.name.as_deref() == Some(basename.as_os_str()) {
                        changed = true;
                    }
                }
                if changed && tx.send(()).is_err() {
                    break;
                }
            }
        })
        .expect("launcher_config: failed to spawn inotify thread");

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
                    deadline = Some(tokio::time::Instant::now() + Duration::from_millis(WATCH_DEBOUNCE_MS));
                }
                _ = timer => {
                    deadline = None;
                    let _ = cx.update(|_cx| {
                        reload();
                        tracing::info!(
                            "launcher_config: hot-reloaded {}",
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

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("launcher-config-test-{tag}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_are_sane() {
        let cfg = LauncherConfig::default();
        assert_eq!(cfg.recents.limit, DEFAULT_RECENTS_LIMIT);
        assert!(!cfg.favorites.sort_alpha);
        assert!(!cfg.favorites.hide_labels);
        assert!(cfg.favorites.order.is_empty());
        assert!(cfg.folders.is_empty());
        assert_eq!(cfg.grid.columns, 7);
        assert_eq!(cfg.grid.rows, 4);
        assert_eq!(cfg.grid.icon_size, 36);
        assert!(!cfg.appearance.compact_default);
        assert!(!cfg.appearance.hide_labels);
        assert!(!cfg.search.include_hidden);
        assert!(cfg.search.inline_completion, "inline completion is on by default (T265-A)");
        assert!(cfg.categories.hide.is_empty());
    }

    #[test]
    fn grid_and_search_and_categories_round_trip() {
        let dir = temp_dir("gsc-roundtrip");
        let path = dir.join("launcher.toml");
        let cfg = LauncherConfig {
            grid: GridConfig {
                columns: 5,
                rows: 3,
                icon_size: 28,
            },
            search: SearchConfig {
                include_hidden: true,
                inline_completion: false,
            },
            categories: CategoriesConfig {
                hide: vec!["Dev".into()],
            },
            ..LauncherConfig::default()
        };
        write_config(&path, &cfg).unwrap();
        let back = read_config(&path);
        assert_eq!(back.grid, cfg.grid);
        assert_eq!(back.search, cfg.search);
        assert_eq!(back.categories, cfg.categories);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn garbage_grid_values_sanitize_to_defaults() {
        let dir = temp_dir("grid-sanitize");
        let path = dir.join("launcher.toml");
        std::fs::write(&path, "[grid]\ncolumns = 0\nrows = 999\nicon_size = 3\n").unwrap();
        let cfg = read_config(&path);
        let s = cfg.grid.sanitized();
        assert_eq!(s.columns, 1, "columns=0 clamps to 1");
        assert_eq!(s.rows, 10, "rows=999 clamps to 10");
        assert_eq!(s.icon_size, 16, "icon_size=3 clamps to 16");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_serializes_and_reloads() {
        let dir = temp_dir("folder-roundtrip");
        let path = dir.join("launcher.toml");
        let cfg = LauncherConfig {
            favorites: FavoritesConfig {
                order: vec!["firefox".into(), "kitty".into()],
                sort_alpha: true,
                hide_labels: false,
            },
            recents: RecentsConfig { limit: 4 },
            folders: vec![Folder {
                id: "folder-1".into(),
                name: "Work".into(),
                apps: vec!["code".into(), "slack".into()],
            }],
            hidden: vec!["org.gnome.eog".into()],
            ..LauncherConfig::default()
        };
        write_config(&path, &cfg).unwrap();
        let back = read_config(&path);
        assert_eq!(back, cfg);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_preserves_unknown_top_level_keys() {
        let dir = temp_dir("rmw-preserve");
        let path = dir.join("launcher.toml");
        std::fs::write(&path, "[unrelated]\nfoo = \"bar\"\n").unwrap();

        let cfg = LauncherConfig {
            folders: vec![Folder {
                id: "folder-1".into(),
                name: "Work".into(),
                apps: vec!["code".into()],
            }],
            ..LauncherConfig::default()
        };
        write_config(&path, &cfg).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("foo = \"bar\""),
            "unknown top-level keys must survive the RMW write:\n{text}"
        );
        let back = read_config(&path);
        assert_eq!(back.folders, cfg.folders);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = temp_dir("corrupt");
        let path = dir.join("launcher.toml");
        std::fs::write(&path, "not [[ valid toml").unwrap();
        assert_eq!(read_config(&path), LauncherConfig::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hidden_round_trips() {
        let dir = temp_dir("hidden-roundtrip");
        let path = dir.join("launcher.toml");
        let cfg = LauncherConfig {
            hidden: vec!["firefox".into(), "org.gnome.eog".into()],
            ..LauncherConfig::default()
        };
        write_config(&path, &cfg).unwrap();
        assert_eq!(read_config(&path).hidden, vec!["firefox", "org.gnome.eog"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn system_actions_round_trips() {
        let dir = temp_dir("system-actions-roundtrip");
        let path = dir.join("launcher.toml");
        let cfg = LauncherConfig {
            system_actions: SystemActionsConfig {
                order: vec!["lock".into(), "shutdown".into(), "logout".into()],
            },
            ..LauncherConfig::default()
        };
        write_config(&path, &cfg).unwrap();
        assert_eq!(
            read_config(&path).system_actions.order,
            vec!["lock", "shutdown", "logout"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recents_limit_defaults_when_missing() {
        let dir = temp_dir("recents-default");
        let path = dir.join("launcher.toml");
        std::fs::write(&path, "[favorites]\norder = [\"firefox\"]\n").unwrap();
        let cfg = read_config(&path);
        assert_eq!(cfg.recents.limit, DEFAULT_RECENTS_LIMIT);
        assert_eq!(cfg.favorites.order, vec!["firefox"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

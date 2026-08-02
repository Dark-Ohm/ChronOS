//! Bar widget layout — `~/.config/chronos/bar.toml` + hot-reload (T134).
//!
//! Order of names in each section is registration order. Default matches
//! historical `register_builtin` (byte-identical lists).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use gpui::{App, BorrowAppContext};
use inotify::{EventMask, Inotify, WatchMask};
use serde::{Deserialize, Serialize};

use super::appearance::BarAppearance;

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
    "workspace_mode",
    "volume",
    "network",
    "tray",
    "updates",
    "system",
    "notification_bell",
    "battery",
    "clock",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BarLayoutConfig {
    /// Schema version of `bar.toml`. Absent or `1` → v1 (appearance falls
    /// back to code defaults, even if an `[appearance]` section is present).
    /// `2` → `appearance` honored (T199). Omitted on save when `None` so v1
    /// files stay byte-stable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Bar appearance (geometry/chrome). Defaults mirror the hardcoded chrome
    /// (T198 table). Omitted on save while default so v1 files stay
    /// byte-stable (`is_default`).
    #[serde(default, skip_serializing_if = "BarAppearance::is_default")]
    pub appearance: BarAppearance,
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
    /// Widget names the user has ever seen (option B, T163). Used to
    /// distinguish "never seen" from "intentionally removed" when new
    /// builtins are added to `BUILTIN_NAMES`.
    pub known: BTreeSet<String>,
}

impl Default for BarLayoutConfig {
    fn default() -> Self {
        // Must match pre-T134 `register_builtin` order exactly.
        // `known` stays empty — the default path never needs migration.
        Self {
            version: None,
            appearance: BarAppearance::default(),
            left: vec![
                "dock".into(),
                "separator".into(),
                "workspaces".into(),
            ],
            center: vec!["mpris".into(), "cava".into()],
            right: vec![
                "project".into(),
                "workspace_mode".into(),
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
            known: BTreeSet::new(),
        }
    }
}

/// v1 gate: files without `version` or with `version < 2` get default
/// appearance regardless of any `[appearance]` section (T199 compat).
///
/// **Load-time only contract:** `BarLayoutConfig::sanitized()` does NOT
/// re-apply this gate — programmatic construction must call it explicitly
/// (or go through `load()`). Real paths (`apply`) always go through `load()`.
pub fn gated_appearance(version: Option<u32>, appearance: BarAppearance) -> BarAppearance {
    if version.unwrap_or(1) < 2 {
        BarAppearance::default()
    } else {
        appearance
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

/// Cached appearance, sanitized. Refreshed whenever `apply()`/`move_widget`
/// update the layout cache (same cache object). T200 reads this on hot-reload.
///
/// `allow(dead_code)`: schema-only task (T199) — the consumer is T200
/// (window apply), which lands next and calls this from the hot-reload path.
#[allow(dead_code)]
pub fn cached_appearance() -> BarAppearance {
    cached().appearance.sanitized()
}

/// Replace the cache. **Invariant: the stored config must be sanitized**
/// (`apply()` does `load().sanitized()`); `cached_appearance()` re-sanitizes
/// defensively on read, so a raw store would only cost repeated warn calls.
pub fn update_cache(cfg: BarLayoutConfig) {
    *config_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = cfg;
}

impl BarLayoutConfig {
    /// Load from disk. Missing → default (no silent write). Bad parse → warn + default.
    /// Runs builtin migration (T163) for parsed configs only.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<BarLayoutConfig>(&content) {
                Ok(mut cfg) => {
                    // v1 files (no `version` / version < 2): appearance stays
                    // code defaults even if an `[appearance]` section exists.
                    cfg.appearance = gated_appearance(cfg.version, cfg.appearance);
                    if cfg.migrate_new_builtins() {
                        // Persist so the migration doesn't loop on every
                        // restart. This is the one exception to the
                        // "no silent write on load" rule — the
                        // alternative (migration re-inserting on every
                        // boot) is worse.
                        if let Err(e) = cfg.save() {
                            tracing::warn!(
                                "bar: failed to persist migration: {e}"
                            );
                        } else {
                            tracing::info!(
                                "bar: migrated layout persisted to {}",
                                path.display()
                            );
                        }
                    }
                    cfg
                }
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

    /// Persist. Also updates `known` to reflect all current widget names
    /// (option B: the user has now "seen" everything in the layout).
    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut cfg = self.clone();
        cfg.known.extend(cfg.left.iter().cloned());
        cfg.known.extend(cfg.center.iter().cloned());
        cfg.known.extend(cfg.right.iter().cloned());
        let body = toml::to_string_pretty(&cfg).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        std::fs::write(path, body)
    }

    /// T163: Insert new builtin widgets that the user has never seen.
    ///
    /// Two-phase logic:
    /// 1. If `known` is empty (old config without T163 field, or first
    ///    boot after code change): bootstrap `known` from the config's
    ///    current contents. This is a no-op migration — the user keeps
    ///    their existing layout unchanged.
    /// 2. If `known` is non-empty: any name in `BUILTIN_NAMES` that is
    ///    NOT in `known` is truly new → insert it at the position defined
    ///    by `Default`. Names the user intentionally removed stay in
    ///    `known` and are never resurrected.
    ///
    /// Returns `true` when the config changed and the caller (`load`) must
    /// persist it. That covers BOTH the bootstrap pass and real insertions.
    ///
    /// Why bootstrap must persist too (эррата T163, 2026-07-31): if it does
    /// not, `known` lives only in memory, the next start finds the field
    /// empty again, and the migration bootstraps forever without ever
    /// reaching phase 2. Live-проверено: два полных рестарта, конфиг не
    /// тронут, виджет не приехал.
    ///
    /// Consequence of the two-phase design: an existing user needs two
    /// restarts to receive a widget added in the same release as this
    /// migration — the first records what they already have, the second
    /// adds what is genuinely new. This is deliberate: on first sight there
    /// is no way to tell "never existed" from "deliberately removed", so we
    /// record the current set and only add what appears later.
    pub fn migrate_new_builtins(&mut self) -> bool {
        if self.known.is_empty() {
            // Bootstrap: first time seeing T163 field. Record what's
            // already in the config so removed items aren't resurrected.
            self.known.extend(self.left.iter().cloned());
            self.known.extend(self.center.iter().cloned());
            self.known.extend(self.right.iter().cloned());
            tracing::info!(
                count = self.known.len(),
                "bar: bootstrapped known widget set, persisting"
            );
            return true;
        }

        let default_cfg = Self::default();
        let new_names: Vec<&str> = BUILTIN_NAMES
            .iter()
            .copied()
            .filter(|n| !self.known.contains(*n))
            .collect();

        if new_names.is_empty() {
            return false;
        }

        for name in &new_names {
            self.insert_at_default_pos(name, &default_cfg);
            tracing::info!(widget = *name, "bar: migrated new builtin widget");
        }

        // Record the new names so they aren't re-inserted next time.
        self.known.extend(new_names.iter().map(|n| n.to_string()));
        true
    }

    /// Insert `name` into the section and position defined by `default_cfg`.
    /// Strategy: insert before the first successor that already exists in
    /// the current config's section. If no successor found, append.
    fn insert_at_default_pos(&mut self, name: &str, default_cfg: &Self) {
        let (section, index_in_default) =
            match default_cfg.find_in_default(name) {
                Some(s) => s,
                None => return,
            };

        let target = match section {
            chronos_luau::bar::BarSection::Left => &mut self.left,
            chronos_luau::bar::BarSection::Center => &mut self.center,
            chronos_luau::bar::BarSection::Right => &mut self.right,
        };
        let default_list = match section {
            chronos_luau::bar::BarSection::Left => &default_cfg.left,
            chronos_luau::bar::BarSection::Center => &default_cfg.center,
            chronos_luau::bar::BarSection::Right => &default_cfg.right,
        };

        // Anchor on the NEAREST PREDECESSOR first, successor only as
        // fallback (эррата T163, 2026-07-31).
        //
        // Successor-first ставит виджет не туда, когда пользователь
        // переставил кластер. Живой пример: `workspace_mode` стоит в
        // `Default` сразу после `project`, но у пользователя правый кластер
        // начинался с `separator` — а `separator` тоже успешник, только
        // дальше по списку. Виджет уехал в позицию 0, к левому краю
        // кластера, вместо места рядом с project-пилюлей.
        //
        // `separator` вдобавок повторяется в секции, поэтому якорем не
        // годится вовсе: `position()` найдёт первый попавшийся.
        let is_anchor = |n: &str| n != "separator";

        let mut insert_at = None;

        // 1. Последний предшественник, который есть у пользователя → сразу
        //    за ним. Идём от ближайшего к дальнему.
        for pred in default_list.iter().take(index_in_default).rev() {
            if !is_anchor(pred) {
                continue;
            }
            if let Some(pos) = target.iter().position(|n| n == pred) {
                insert_at = Some(pos + 1);
                break;
            }
        }

        // 2. Иначе — перед первым успешником.
        if insert_at.is_none() {
            for succ in default_list.iter().skip(index_in_default + 1) {
                if !is_anchor(succ) {
                    continue;
                }
                if let Some(pos) = target.iter().position(|n| n == succ) {
                    insert_at = Some(pos);
                    break;
                }
            }
        }

        // 3. Иначе — в конец секции.
        target.insert(insert_at.unwrap_or(target.len()), name.to_string());
    }

    /// Find a name in the default layout. Returns (section, index).
    fn find_in_default(&self, name: &str) -> Option<(chronos_luau::bar::BarSection, usize)> {
        use chronos_luau::bar::BarSection;
        if let Some(i) = self.left.iter().position(|n| n == name) {
            return Some((BarSection::Left, i));
        }
        if let Some(i) = self.center.iter().position(|n| n == name) {
            return Some((BarSection::Center, i));
        }
        if let Some(i) = self.right.iter().position(|n| n == name) {
            return Some((BarSection::Right, i));
        }
        None
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
            known: self.known.clone(),
            version: self.version,
            appearance: self.appearance.sanitized(),
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
                "workspace_mode",
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
            known: BTreeSet::new(),
            ..Default::default()
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
            known: BTreeSet::new(),
            ..Default::default()
        };
        assert!(cfg.sanitized().left.is_empty());
    }

    // -- T163 migration tests ------------------------------------------------

    /// Old config without `known` field → bootstrap, no widget added.
    #[test]
    fn migration_old_config_bootstraps_no_add() {
        // Simulate a pre-T163 config: known is empty (serde default).
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into(), "separator".into(), "workspaces".into()],
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
            known: BTreeSet::new(),
            ..Default::default()
        };
        let changed = cfg.migrate_new_builtins();
        // Bootstrap: known populated from current contents, no widget added.
        assert!(!cfg.known.is_empty());
        assert!(!cfg.right.contains(&"workspace_mode".to_string()));
        // Эррата T163: bootstrap ОБЯЗАН просить персист. Без этого `known`
        // живёт только в памяти, следующий старт снова видит пустое поле,
        // и фаза 2 не наступает никогда — виджет не доезжает.
        assert!(changed, "bootstrap must ask the caller to persist");
    }

    /// Виджет обязан встать рядом со СВОИМ соседом из `Default`, даже если
    /// пользователь переставил кластер. Регрессия эрраты T163: раньше якорь
    /// искался только среди успешников, и `workspace_mode` уезжал в позицию 0.
    #[test]
    fn migration_anchors_on_predecessor_not_successor() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        for name in BUILTIN_NAMES.iter().filter(|&&n| n != "workspace_mode") {
            known.insert(name.to_string());
        }
        // Пользовательский порядок: кластер НАЧИНАЕТСЯ с separator, а
        // project стоит в середине. В `Default` порядок другой.
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into()],
            center: vec!["cava".into()],
            right: vec![
                "separator".into(),
                "system".into(),
                "tray".into(),
                "project".into(),
                "battery".into(),
                "clock".into(),
            ],
            known,
            ..Default::default()
        };

        assert!(cfg.migrate_new_builtins());

        let pos = cfg
            .right
            .iter()
            .position(|n| n == "workspace_mode")
            .expect("виджет вставлен");
        let project_pos = cfg
            .right
            .iter()
            .position(|n| n == "project")
            .expect("project на месте");

        assert_eq!(
            pos,
            project_pos + 1,
            "workspace_mode обязан встать сразу за project, а не в начало кластера: {:?}",
            cfg.right
        );
    }

    /// Два прохода подряд (= два рестарта с персистом между ними) обязаны
    /// довести новый виджет до конфига. Это тест на связку фаз, а не на
    /// каждую по отдельности — именно он ловит эрратуT163.
    #[test]
    fn migration_reaches_phase_two_on_second_pass() {
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into()],
            center: vec!["cava".into()],
            right: vec!["project".into(), "clock".into()],
            known: BTreeSet::new(),
            ..Default::default()
        };

        // Рестарт 1: bootstrap, виджета ещё нет, но персист запрошен.
        assert!(cfg.migrate_new_builtins());
        assert!(!cfg.right.contains(&"workspace_mode".to_string()));

        // Рестарт 2: `known` пришёл с диска, виджет распознан как новый.
        assert!(cfg.migrate_new_builtins());
        assert!(cfg.right.contains(&"workspace_mode".to_string()));

        // Рестарт 3: идемпотентность, дубликата нет.
        assert!(!cfg.migrate_new_builtins());
        assert_eq!(
            cfg.right.iter().filter(|n| *n == "workspace_mode").count(),
            1
        );
    }

    /// Config with known set but missing new widget → widget added at
    /// correct position (after "project" in right cluster).
    #[test]
    fn migration_adds_new_widget_at_default_pos() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        // Simulate: user has seen everything EXCEPT workspace_mode.
        for name in BUILTIN_NAMES.iter().filter(|&&n| n != "workspace_mode") {
            known.insert(name.to_string());
        }
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into(), "separator".into(), "workspaces".into()],
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
            known,
            ..Default::default()
        };
        cfg.migrate_new_builtins();
        // workspace_mode inserted at index 1 (after "project", before "separator").
        assert_eq!(cfg.right[1], "workspace_mode");
        assert!(cfg.known.contains("workspace_mode"));
    }

    /// User removed an existing widget → it does NOT reappear.
    #[test]
    fn migration_does_not_resurrect_removed_widget() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        // User has seen all widgets including the one they later removed.
        for name in BUILTIN_NAMES {
            known.insert(name.to_string());
        }
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into(), "separator".into(), "workspaces".into()],
            center: vec!["mpris".into(), "cava".into()],
            right: vec![
                "project".into(),
                "workspace_mode".into(),
                "separator".into(),
                // user removed "volume" here
                "network".into(),
                "tray".into(),
                "updates".into(),
                "system".into(),
                "notification_bell".into(),
                "separator".into(),
                "battery".into(),
                "clock".into(),
            ],
            known,
            ..Default::default()
        };
        cfg.migrate_new_builtins();
        // volume is known → not re-inserted.
        assert!(!cfg.right.contains(&"volume".to_string()));
    }

    /// Config with garbage name + migration → garbage doesn't break migration.
    #[test]
    fn migration_ignores_garbage() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        for name in BUILTIN_NAMES {
            known.insert(name.to_string());
        }
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into(), "nope".into()],
            center: vec![],
            right: vec!["clock".into()],
            known,
            ..Default::default()
        };
        // Should not panic, no new builtins to add.
        cfg.migrate_new_builtins();
        assert_eq!(cfg.left.len(), 2); // "nope" still there, sanitized() is separate.
    }

    /// Migration is idempotent: second run doesn't add duplicates.
    #[test]
    fn migration_idempotent() {
        let mut known: BTreeSet<String> = BTreeSet::new();
        for name in BUILTIN_NAMES.iter().filter(|&&n| n != "workspace_mode") {
            known.insert(name.to_string());
        }
        let mut cfg = BarLayoutConfig {
            left: vec!["dock".into(), "separator".into(), "workspaces".into()],
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
            known,
            ..Default::default()
        };
        cfg.migrate_new_builtins();
        let after_first = cfg.right.clone();
        cfg.migrate_new_builtins();
        assert_eq!(cfg.right, after_first);
        assert_eq!(cfg.right.iter().filter(|n| *n == "workspace_mode").count(), 1);
    }

    // -- T199 appearance schema ---------------------------------------------

    #[test]
    fn v1_file_loads_with_default_appearance() {
        // Real user shape: flat widgets only, no version, no [appearance].
        let toml_str = r#"
            left = ["dock", "separator", "workspaces"]
            center = ["cava", "mpris"]
            right = ["clock"]
        "#;
        let cfg: BarLayoutConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.version, None);
        assert_eq!(cfg.appearance, BarAppearance::default());
        assert_eq!(cfg.left, vec!["dock", "separator", "workspaces"]);
    }

    #[test]
    fn version_absent_or_one_gates_appearance_to_defaults() {
        let explicit = BarAppearance {
            height: 64.0,
            ..Default::default()
        };
        assert_eq!(gated_appearance(None, explicit), BarAppearance::default());
        assert_eq!(gated_appearance(Some(1), explicit), BarAppearance::default());
        // v2 honors the section.
        assert_eq!(gated_appearance(Some(2), explicit), explicit);
    }

    #[test]
    fn v2_file_with_appearance_parses_and_sanitizes() {
        let toml_str = r#"
            version = 2
            left = ["dock"]
            [appearance]
            height = 40
            floating = true
            exclusive = true
        "#;
        let cfg: BarLayoutConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.version, Some(2));
        assert_eq!(cfg.appearance.height, 40.0);
        let s = cfg.sanitized();
        assert!(s.appearance.floating);
        assert!(!s.appearance.exclusive, "floating must force exclusive off");
    }

    #[test]
    fn v2_missing_appearance_section_defaults() {
        let toml_str = r#"
            version = 2
            left = ["dock"]
        "#;
        let cfg: BarLayoutConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.version, Some(2));
        assert_eq!(cfg.appearance, BarAppearance::default());
    }

    #[test]
    fn sanitized_passes_appearance_through() {
        let cfg = BarLayoutConfig {
            version: Some(2),
            appearance: BarAppearance {
                height: 200.0,
                floating: true,
                exclusive: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let s = cfg.sanitized();
        assert_eq!(s.appearance.height, 80.0); // clamped
        assert!(!s.appearance.exclusive); // floating forces off
    }

    #[test]
    fn serialize_roundtrip_v2_with_appearance() {
        let cfg = BarLayoutConfig {
            version: Some(2),
            appearance: BarAppearance {
                height: 40.0,
                radius: 12.0,
                ..Default::default()
            },
            left: vec!["dock".into()],
            right: vec!["clock".into()],
            ..Default::default()
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        assert!(s.contains("[appearance]"), "appearance section must be written:\n{s}");
        let back: BarLayoutConfig = toml::from_str(&s).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn v1_serialize_omits_version_and_appearance() {
        let cfg = BarLayoutConfig::default(); // v1 shape
        let s = toml::to_string_pretty(&cfg).unwrap();
        assert!(!s.contains("version"), "v1 save must not write version:\n{s}");
        assert!(!s.contains("appearance"), "v1 save must not write appearance:\n{s}");
    }
}

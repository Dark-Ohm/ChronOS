//! Scene state — shared composition state outside individual panel views
//! (слайс 2 спеки `docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`,
//! §11 «add shared workspace-mode and scene state outside individual panel views»).
//!
//! Единственный источник правды о том, «какая сцена активна». Композиция
//! (набор вкладок рейла, состав дока) читается из сцены, а не из вью.
//!
//! Формат `~/.config/chronos/scenes.toml`:
//!
//! ```toml
//! version = 1
//!
//! [last]
//! developer = "chronos"
//! gamer = "hub"
//!
//! [[scene]]
//! id = "chronos"
//! name = "ChronOS"
//! mode = "developer"
//! display = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
//! rail_tabs = ["system", "files", "editor", "terminal"]
//! active_tab = "files"
//! dock = ["kitty", "code", "vivaldi"]
//!
//! # ЗАРЕЗЕРВИРОВАНО под вариант C (внешние окна). Не пишется и не читается;
//! # парсер обязан переживать его наличие.
//! # [scene.windows]
//! ```
//!
//! Все три поля-оверрайда (rail_tabs, active_tab, dock) опциональны:
//! отсутствие означает «бери дефолт режима».
//!
//! `display` — UUID строкой, никогда не индекс и не DisplayId (§3.6).
//! В слайсе 2 поле только парсится и сериализуется; резолвить его в
//! реальный вывод НЕ надо.

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::workspace_mode::{self, WorkspaceMode};

const CONFIG_BASENAME: &str = "scenes.toml";

/// Активная сцена. Если `None` — композиция = чистый дефолт режима.
#[derive(Debug, Clone, Default)]
pub struct SceneState {
    pub active: Option<Scene>,
    pub config: ScenesConfig,
}

impl Global for SceneState {}

// ── Model ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScenesConfig {
    /// Версия формата. Отсутствие = 1.
    #[serde(default)]
    pub version: u32,
    /// Последняя активная сцена по каждому режиму.
    #[serde(default)]
    pub last: HashMap<String, String>,
    /// Сцены.
    #[serde(default)]
    pub scene: Vec<Scene>,
    /// Захват неизвестных верхнеуровневых секций (forward-compat).
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub mode: String,
    /// UUID вывода, не индекс и не DisplayId.
    #[serde(default)]
    pub display: String,
    /// Оверрайд набора вкладок рейла. Пусто = дефолт режима.
    #[serde(default)]
    pub rail_tabs: Vec<String>,
    /// Оверрайд активной вкладки. Пусто = дефолт режима.
    #[serde(default)]
    pub active_tab: String,
    /// Оверрайд состава дока. Пусто = дефолт режима.
    #[serde(default)]
    pub dock: Vec<String>,
    /// Захват неизвестных полей сцены (forward-compat, например [scene.windows]).
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

// ── Config I/O (чистые относительно файловой системы) ─────────────────────

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

/// Чистая функция парсинга — вынесена из `load_config`, чтобы тестировать
/// разбор без файловой системы.
pub fn parse_config(content: &str) -> Result<ScenesConfig, toml::de::Error> {
    let mut cfg: ScenesConfig = toml::from_str(content)?;
    // Отсутствие version трактуется как 1.
    if cfg.version == 0 {
        cfg.version = 1;
    }
    Ok(cfg)
}

pub fn load_config() -> ScenesConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match parse_config(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(
                    "scene: failed to parse {}: {e}, using defaults",
                    path.display()
                );
                ScenesConfig::default()
            }
        },
        Err(_) => ScenesConfig::default(),
    }
}

/// Запись конфига на диск. Вызывается ТОЛЬКО когда пользователь реально
/// сменил активную сцену (будущий SceneManager), а не при каждом `set`.
// Currently unused — scene manager (slice 3/4) will call this.
#[allow(dead_code)]
fn save_config(cfg: &ScenesConfig) {
    let path = config_path();
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!("scene: mkdir {} failed: {e}", parent.display());
        return;
    }
    match toml::to_string_pretty(cfg) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                tracing::warn!("scene: write {} failed: {e}", path.display());
            }
        }
        Err(e) => tracing::warn!("scene: serialize failed: {e}"),
    }
}

// ── Чистые функции ────────────────────────────────────────────────────────

/// Сцена по id. `None` если id нет в списке.
pub fn find_by_id<'a>(cfg: &'a ScenesConfig, id: &str) -> Option<&'a Scene> {
    cfg.scene.iter().find(|s| s.id == id)
}

/// Последняя сцена для данного режима, если id существует и mode совпадает.
/// Ссылка на несуществующий id → `None`, не паника.
pub fn resolve_last(cfg: &ScenesConfig, mode: WorkspaceMode) -> Option<Scene> {
    let key = mode_label(mode);
    let id = cfg.last.get(key)?;
    let scene = find_by_id(cfg, id)?;
    // Сцена с невалидным mode уже отфильтрована в validate, но проверяем
    // дополнительно — если вдруг validate не вызывался.
    if WorkspaceMode::parse(&scene.mode) == Some(mode) {
        Some(scene.clone())
    } else {
        tracing::warn!(
            "scene: last[{key}] = {id:?} exists but its mode {:?} != {key}, ignoring",
            scene.mode
        );
        None
    }
}

/// Фильтрует сцены с невалидным mode, логируя warn. Возвращает валидные.
pub fn filter_valid(scenes: Vec<Scene>) -> Vec<Scene> {
    scenes
        .into_iter()
        .filter(|s| {
            if WorkspaceMode::parse(&s.mode).is_some() {
                true
            } else {
                tracing::warn!(
                    "scene: ignoring scene {:?} with unknown mode {:?}",
                    s.id,
                    s.mode
                );
                false
            }
        })
        .collect()
}

fn mode_label(mode: WorkspaceMode) -> &'static str {
    match mode {
        WorkspaceMode::Developer => "developer",
        WorkspaceMode::Gamer => "gamer",
    }
}

// ── Public API (для T165 и остальных) ─────────────────────────────────────

/// Активная сцена, если есть. `None` = чистый дефолт режима.
// T165 consumer — suppress until wired.
#[allow(dead_code)]
pub fn current(cx: &App) -> Option<Scene> {
    cx.try_global::<SceneState>().and_then(|s| s.active.clone())
}

/// Оверрайд набора вкладок рейла из активной сцены. `None` = дефолт режима.
// T165 consumer — suppress until wired.
#[allow(dead_code)]
pub fn rail_tabs_override(cx: &App) -> Option<Vec<String>> {
    let tabs = current(cx)?.rail_tabs;
    if tabs.is_empty() {
        None
    } else {
        Some(tabs)
    }
}

/// Оверрайд состава дока из активной сцены. `None` = дефолт режима.
// T165 consumer — suppress until wired.
#[allow(dead_code)]
pub fn dock_override(cx: &App) -> Option<Vec<String>> {
    let items = current(cx)?.dock;
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Активная вкладка из сцены. `None` = дефолт режима.
// T165 consumer — suppress until wired.
#[allow(dead_code)]
pub fn active_tab_override(cx: &App) -> Option<String> {
    let tab = current(cx)?.active_tab;
    if tab.is_empty() {
        None
    } else {
        Some(tab)
    }
}

/// Восстановить последнюю сцену для данного режима. Вызывается из
/// `workspace_mode::set` при каждой смене (и не-смене) режима.
///
/// **Read-only на диске**: фильтрация невалидных сцен — только для выбора
/// активной сцены в памяти. Конфиг на диске не перезаписывается, невалидные
/// сцены не стираются. `[last]` персистится только когда пользователь
/// реально сменил активную сцену (будущий SceneManager).
///
/// Сцена не найдена → `None` (композиция = дефолт режима, не ошибка).
pub fn restore_for_mode(cx: &mut App, mode: WorkspaceMode) {
    // Читаем конфиг как есть, фильтруем только для резолвинга.
    let cfg = &cx.global::<SceneState>().config;
    let mut for_resolve = cfg.clone();
    for_resolve.scene = filter_valid(cfg.scene.clone());

    let restored = resolve_last(&for_resolve, mode);

    if let Some(ref scene) = restored {
        tracing::info!(
            scene = %scene.id,
            mode = mode_label(mode),
            "scene: restored"
        );
    } else {
        tracing::info!(
            mode = mode_label(mode),
            "scene: no last scene, using mode defaults"
        );
    }

    // Меняем ТОЛЬКО active — конфиг на диске не трогаем.
    cx.global_mut::<SceneState>().active = restored;
}

// ── Init ──────────────────────────────────────────────────────────────────

pub fn init(cx: &mut App) {
    let cfg = load_config();
    let version = cfg.version;
    let scene_count = cfg.scene.len();

    // Резолвим активную сцену, фильтруя невалидные только для выбора.
    // Сохранённый конфиг содержит все сцены, включая невалидные.
    let mut for_resolve = cfg.clone();
    for_resolve.scene = filter_valid(cfg.scene.clone());

    let mode = workspace_mode::current(cx);
    let initial = resolve_last(&for_resolve, mode);

    if let Some(ref scene) = initial {
        tracing::info!(
            version,
            scene_count,
            scene = %scene.id,
            mode = mode_label(mode),
            "scene: initial"
        );
    } else {
        tracing::info!(
            version,
            scene_count,
            mode = mode_label(mode),
            "scene: initial (no last scene, mode defaults)"
        );
    }

    cx.set_global(SceneState {
        active: initial,
        config: cfg,
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_scenes(scenes: Vec<Scene>) -> ScenesConfig {
        ScenesConfig {
            version: 1,
            last: HashMap::new(),
            scene: scenes,
            extra: HashMap::new(),
        }
    }

    // 1. Пустая строка → дефолт, без паники.
    #[test]
    fn empty_content_returns_default() {
        let result = parse_config("");
        // Пустая строка — валидный пустой TOML: всё дефолтное.
        let cfg = result.expect("пустой TOML парсится");
        assert_eq!(cfg.version, 1); // 0 → нормализуется в 1
        assert!(cfg.scene.is_empty());
        assert!(cfg.last.is_empty());
    }

    // 2. Мусор вместо TOML → parse_config возвращает ошибку, не панику.
    #[test]
    fn garbage_toml_returns_error() {
        let result = parse_config("this is not { valid toml");
        assert!(result.is_err(), "мусорный TOML должен вернуть ошибку, не панику");
    }

    // 3. Неизвестная секция [scene.windows] и незнакомое поле → парсится, не теряет остальное.
    #[test]
    fn unknown_section_and_field_preserved() {
        let input = r#"
            version = 1

            [last]
            developer = "chronos"

            [[scene]]
            id = "chronos"
            name = "ChronOS"
            mode = "developer"
            future_field = true

            [scene.windows]
            capture = "reserved"
        "#;
        let cfg: ScenesConfig = toml::from_str(input).unwrap();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.last.get("developer").unwrap(), "chronos");
        assert_eq!(cfg.scene.len(), 1);
        assert_eq!(cfg.scene[0].id, "chronos");
        // future_field попал в extra.
        assert_eq!(
            cfg.scene[0].extra.get("future_field"),
            Some(&toml::Value::Boolean(true))
        );
        // [scene.windows] попал в extra как вложенная таблица.
        let windows = cfg.scene[0].extra.get("windows").unwrap();
        assert!(windows.is_table());
        assert_eq!(
            windows.as_table().unwrap().get("capture").unwrap(),
            &toml::Value::String("reserved".into())
        );
    }

    // 4. [last] резолвится в существующую сцену; ссылка на несуществующий id → None.
    #[test]
    fn resolve_last_existing_and_missing() {
        let mut cfg = cfg_with_scenes(vec![Scene {
            id: "chronos".into(),
            name: "ChronOS".into(),
            mode: "developer".into(),
            ..Default::default()
        }]);
        cfg.last.insert("developer".into(), "chronos".into());

        // Существующая сцена.
        let resolved = resolve_last(&cfg, WorkspaceMode::Developer);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, "chronos");

        // Несуществующий id.
        cfg.last.insert("gamer".into(), "nonexistent".into());
        let resolved = resolve_last(&cfg, WorkspaceMode::Gamer);
        assert!(resolved.is_none());
    }

    // 5. Неизвестный mode → сцена игнорируется с warn, остальные живы.
    #[test]
    fn unknown_mode_scene_filtered() {
        let scenes = vec![
            Scene {
                id: "good".into(),
                name: "Good".into(),
                mode: "developer".into(),
                ..Default::default()
            },
            Scene {
                id: "bad".into(),
                name: "Bad".into(),
                mode: "invalid_mode".into(),
                ..Default::default()
            },
        ];
        let filtered = filter_valid(scenes);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "good");
    }

    // 6. Round-trip: сериализация → парс → те же данные, ВКЛЮЧАЯ непустой extra.
    #[test]
    fn roundtrip_with_extra() {
        let mut scene_extra = HashMap::new();
        scene_extra.insert(
            "windows".into(),
            toml::Value::Table({
                let mut t = toml::map::Map::new();
                t.insert("capture".into(), toml::Value::String("reserved".into()));
                t
            }),
        );
        scene_extra.insert("future_flag".into(), toml::Value::Integer(42));

        let original = ScenesConfig {
            version: 1,
            last: {
                let mut m = HashMap::new();
                m.insert("developer".into(), "chronos".into());
                m
            },
            scene: vec![Scene {
                id: "chronos".into(),
                name: "ChronOS".into(),
                mode: "developer".into(),
                display: "09e7b298-aad0-546d-a4de-adcb9106fd7d".into(),
                rail_tabs: vec!["system".into(), "files".into()],
                active_tab: "files".into(),
                dock: vec!["kitty".into()],
                extra: scene_extra,
            }],
            extra: HashMap::new(),
        };

        let text = toml::to_string_pretty(&original).expect("сериализация с непустым extra");
        let parsed: ScenesConfig =
            toml::from_str(&text).expect("десериализация round-trip с extra");
        assert_eq!(parsed, original, "round-trip потерял данные из extra");
    }

    // 7. version отсутствует → parse_config трактует как 1.
    #[test]
    fn missing_version_normalized_to_one() {
        let input = r#"
            [[scene]]
            id = "test"
            name = "Test"
            mode = "developer"
        "#;
        let cfg = parse_config(input).expect("разбор без version");
        assert_eq!(cfg.version, 1, "отсутствие version должно трактоваться как 1");
    }

    // 8. Неизвестный mode в [last] ссылается на сцену с другим mode → None.
    #[test]
    fn resolve_last_mode_mismatch() {
        let mut cfg = cfg_with_scenes(vec![Scene {
            id: "chronos".into(),
            name: "ChronOS".into(),
            mode: "developer".into(),
            ..Default::default()
        }]);
        // Ссылка на chronos из gamer, но chronos.mode = "developer".
        cfg.last.insert("gamer".into(), "chronos".into());
        let resolved = resolve_last(&cfg, WorkspaceMode::Gamer);
        assert!(resolved.is_none());
    }

    // 9. Пустые override-поля в resolve_last → Some, но поля пустые.
    #[test]
    fn empty_overrides_resolve_but_fields_empty() {
        let cfg = cfg_with_scenes(vec![Scene {
            id: "minimal".into(),
            name: "Minimal".into(),
            mode: "developer".into(),
            ..Default::default()
        }]);
        // resolve_last находит сцену.
        let mut cfg = cfg;
        cfg.last.insert("developer".into(), "minimal".into());
        let resolved = resolve_last(&cfg, WorkspaceMode::Developer);
        let scene = resolved.expect("сцена должна резолвиться");
        assert!(scene.rail_tabs.is_empty(), "rail_tabs пуст → override = None");
        assert!(scene.active_tab.is_empty(), "active_tab пуст → override = None");
        assert!(scene.dock.is_empty(), "dock пуст → override = None");
    }
}

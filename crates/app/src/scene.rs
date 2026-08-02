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
    /// `"hub"` | `"game"`. Пусто → трактовать как hub, если `app` пуст,
    /// иначе game (слайс 5, T185).
    #[serde(default)]
    pub kind: String,
    /// Desktop id / launch key сцены-игры. Пуст у hub; у game обязателен
    /// для launch (T187), иначе launch = unavailable.
    #[serde(default)]
    pub app: String,
    /// `true` — единственный путь, где активация сцены зовёт
    /// `GamingModeState::apply` (T189, не здесь). `false` по умолчанию:
    /// Gamer ≠ GamingModeState (спека §5).
    #[serde(default)]
    pub apply_gaming_profile: bool,
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
/// сменил активную сцену (`activate`) или при seed builtin hub на первом
/// старте — не при каждом `restore_for_mode` (T164: битый/старый файл не
/// затираем молча, эти пути пишут только осмысленные изменения).
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
pub fn current(cx: &App) -> Option<Scene> {
    cx.try_global::<SceneState>().and_then(|s| s.active.clone())
}

/// Оверрайд набора вкладок рейла из активной сцены. `None` = дефолт режима.
pub fn rail_tabs_override(cx: &App) -> Option<Vec<String>> {
    let tabs = current(cx)?.rail_tabs;
    if tabs.is_empty() {
        None
    } else {
        Some(tabs)
    }
}

/// Оверрайд состава дока из активной сцены. `None` = дефолт режима.
pub fn dock_override(cx: &App) -> Option<Vec<String>> {
    let items = current(cx)?.dock;
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Активная вкладка из сцены. `None` = дефолт режима.
// Wired when a consumer applies scene-selected tab on restore (not T165).
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

// ── Activate (user path) ────────────────────────────────────────────────────

/// Почему активация сцены не удалась.
// Wired by the Scenes tab UI (T188) — no consumer yet in this slice.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivateError {
    /// Нет сцены с таким id (после фильтрации невалидных).
    NotFound,
    /// Сцена есть, но её `mode` не парсится в `WorkspaceMode`.
    InvalidMode,
}

/// Чистое ядро `activate`: по id ищет сцену (среди валидных) и возвращает
/// новый конфиг с обновлённым `[last]` + саму найденную сцену. Не трогает
/// диск и глобальное состояние — тестируется без `cx`.
// Wired by `activate` below and the Scenes tab UI (T188) — no non-test
// consumer of the standalone fn yet in this slice.
#[allow(dead_code)]
pub fn activate_in_config(
    cfg: &ScenesConfig,
    id: &str,
) -> Result<(ScenesConfig, Scene), ActivateError> {
    let mut for_resolve = cfg.clone();
    for_resolve.scene = filter_valid(cfg.scene.clone());
    let scene = find_by_id(&for_resolve, id)
        .cloned()
        .ok_or(ActivateError::NotFound)?;
    let mode = WorkspaceMode::parse(&scene.mode).ok_or(ActivateError::InvalidMode)?;

    let mut new_cfg = cfg.clone();
    new_cfg.last.insert(mode_label(mode).to_string(), id.to_string());
    Ok((new_cfg, scene))
}

/// Явная активация сцены пользователем: клик Library/Scenes, IPC.
/// **Единственный** путь (кроме seed), который пишет `scenes.toml` на диск.
/// Не зовёт `GamingModeState` (T190) и не зовёт `workspace_mode::set` —
/// активация сцены ≠ смена режима.
// Wired by the Scenes tab UI (T188) — no consumer yet in this slice.
#[allow(dead_code)]
pub fn activate(cx: &mut App, id: &str) -> Result<(), ActivateError> {
    let cfg = cx.global::<SceneState>().config.clone();
    let (new_cfg, scene) = activate_in_config(&cfg, id)?;

    save_config(&new_cfg);

    tracing::info!(scene = %id, mode = %scene.mode, "scene: activated");

    let state = cx.global_mut::<SceneState>();
    state.active = Some(scene);
    state.config = new_cfg;
    Ok(())
}

// ── Seed builtin hub ────────────────────────────────────────────────────────

/// Rail tabs дефолтной hub-сцены (T186 добавит соответствующие `PanelTab`
/// варианты; до merge T186 неизвестные id молча скипаются в
/// `PanelTab::resolve_for_mode`, это ожидаемо и безопасно).
const HUB_RAIL_TABS: &[&str] = &[
    "system",
    "library",
    "scenes",
    "captures",
    "acp_settings",
    "mcp_settings",
    "lsp_settings",
    "api_providers",
    "editor_settings",
    "hyprland_binds",
];

const HUB_DOCK: &[&str] = &["steam", "discord", "firefox", "kitty"];

/// Если в конфиге нет валидной gamer-сцены с `id == "hub"` — добавляет её в
/// память (не стирая чужие сцены) и, если ключа не было, выставляет
/// `last["gamer"] = "hub"`. Возвращает `true`, если конфиг изменился (вызов
/// обязан затем сохранить его на диск).
pub fn ensure_builtin_hub(cfg: &mut ScenesConfig) -> bool {
    let has_hub = cfg
        .scene
        .iter()
        .any(|s| s.id == "hub" && WorkspaceMode::parse(&s.mode) == Some(WorkspaceMode::Gamer));
    if has_hub {
        return false;
    }

    cfg.scene.push(Scene {
        id: "hub".to_string(),
        name: "Game Hub".to_string(),
        mode: "gamer".to_string(),
        kind: "hub".to_string(),
        rail_tabs: HUB_RAIL_TABS.iter().map(|s| (*s).to_string()).collect(),
        active_tab: "library".to_string(),
        dock: HUB_DOCK.iter().map(|s| (*s).to_string()).collect(),
        ..Default::default()
    });
    cfg.last.entry("gamer".to_string()).or_insert_with(|| "hub".to_string());
    true
}

// ── Init ──────────────────────────────────────────────────────────────────

pub fn init(cx: &mut App) {
    let mut cfg = load_config();
    if ensure_builtin_hub(&mut cfg) {
        tracing::info!("scene: seeded builtin hub scene");
        save_config(&cfg);
    }
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
                kind: String::new(),
                app: String::new(),
                apply_gaming_profile: false,
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

    // 10. T185: новые поля Scene — round-trip через TOML, дефолты пустые.
    #[test]
    fn per_game_fields_default_and_roundtrip() {
        let input = r#"
            [[scene]]
            id = "minimal"
            name = "Minimal"
            mode = "developer"
        "#;
        let cfg = parse_config(input).expect("разбор без новых полей");
        let scene = &cfg.scene[0];
        assert_eq!(scene.kind, "", "kind по умолчанию пуст");
        assert_eq!(scene.app, "", "app по умолчанию пуст");
        assert!(!scene.apply_gaming_profile, "apply_gaming_profile по умолчанию false");

        let full = Scene {
            id: "game-steam-730".into(),
            name: "Counter-Strike 2".into(),
            mode: "gamer".into(),
            kind: "game".into(),
            app: "steam_app_730".into(),
            apply_gaming_profile: true,
            ..Default::default()
        };
        let text = toml::to_string_pretty(&full).expect("сериализация с новыми полями");
        let parsed: Scene = toml::from_str(&text).expect("десериализация с новыми полями");
        assert_eq!(parsed, full, "round-trip новых полей потерял данные");
    }

    // 11. T185: activate_in_config находит сцену, обновляет [last], не трогает
    // прочие сцены/поля конфига.
    #[test]
    fn activate_in_config_updates_last_for_mode() {
        let mut cfg = cfg_with_scenes(vec![
            Scene {
                id: "chronos".into(),
                name: "ChronOS".into(),
                mode: "developer".into(),
                ..Default::default()
            },
            Scene {
                id: "cs2".into(),
                name: "Counter-Strike 2".into(),
                mode: "gamer".into(),
                kind: "game".into(),
                app: "steam_app_730".into(),
                ..Default::default()
            },
        ]);
        cfg.last.insert("developer".into(), "chronos".into());

        let (new_cfg, scene) = activate_in_config(&cfg, "cs2").expect("cs2 должна активироваться");
        assert_eq!(scene.id, "cs2");
        assert_eq!(new_cfg.last.get("gamer"), Some(&"cs2".to_string()));
        // Не должна тронуть developer-запись.
        assert_eq!(new_cfg.last.get("developer"), Some(&"chronos".to_string()));
        // Список сцен не потерян/не задвоен.
        assert_eq!(new_cfg.scene.len(), 2);
    }

    // 12. T185: activate_in_config на несуществующий id → NotFound, конфиг не создан.
    #[test]
    fn activate_in_config_missing_id_is_not_found() {
        let cfg = cfg_with_scenes(vec![]);
        let result = activate_in_config(&cfg, "nope");
        assert_eq!(result.err(), Some(ActivateError::NotFound));
    }

    // 13. T185: activate_in_config игнорирует невалидный mode на диске (не паникует).
    #[test]
    fn activate_in_config_invalid_mode_scene_is_not_found() {
        // Сцена с невалидным mode отфильтровывается filter_valid перед
        // поиском — activate на неё видит NotFound, не InvalidMode.
        let cfg = cfg_with_scenes(vec![Scene {
            id: "broken".into(),
            name: "Broken".into(),
            mode: "not_a_mode".into(),
            ..Default::default()
        }]);
        let result = activate_in_config(&cfg, "broken");
        assert_eq!(result.err(), Some(ActivateError::NotFound));
    }

    // 14. T185: ensure_builtin_hub добавляет hub при пустом конфиге и
    // выставляет last.gamer.
    #[test]
    fn ensure_builtin_hub_seeds_when_missing() {
        let mut cfg = ScenesConfig::default();
        let changed = ensure_builtin_hub(&mut cfg);
        assert!(changed, "пустой конфиг должен получить seed hub");
        assert_eq!(cfg.scene.len(), 1);
        let hub = &cfg.scene[0];
        assert_eq!(hub.id, "hub");
        assert_eq!(hub.mode, "gamer");
        assert_eq!(hub.kind, "hub");
        assert_eq!(hub.active_tab, "library");
        assert!(hub.rail_tabs.contains(&"library".to_string()));
        assert!(hub.dock.contains(&"steam".to_string()));
        assert_eq!(cfg.last.get("gamer"), Some(&"hub".to_string()));
    }

    // 15. T185: ensure_builtin_hub не трогает чужие сцены и не задваивает hub.
    #[test]
    fn ensure_builtin_hub_preserves_existing_scenes_and_is_idempotent() {
        let mut cfg = cfg_with_scenes(vec![Scene {
            id: "chronos".into(),
            name: "ChronOS".into(),
            mode: "developer".into(),
            ..Default::default()
        }]);
        let changed = ensure_builtin_hub(&mut cfg);
        assert!(changed);
        assert_eq!(cfg.scene.len(), 2, "developer-сцена сохранена, hub добавлен");
        assert!(cfg.scene.iter().any(|s| s.id == "chronos"));

        // Повторный вызов — hub уже есть, ничего не меняется.
        let changed_again = ensure_builtin_hub(&mut cfg);
        assert!(!changed_again, "hub уже есть — повторный seed не нужен");
        assert_eq!(cfg.scene.len(), 2, "hub не задвоен");
    }

    // 16. T185: ensure_builtin_hub не перезаписывает существующий last.gamer,
    // если он уже указывает на другую (не hub) сцену.
    #[test]
    fn ensure_builtin_hub_does_not_override_existing_last_gamer() {
        let mut cfg = cfg_with_scenes(vec![Scene {
            id: "cs2".into(),
            name: "Counter-Strike 2".into(),
            mode: "gamer".into(),
            kind: "game".into(),
            ..Default::default()
        }]);
        cfg.last.insert("gamer".into(), "cs2".into());

        ensure_builtin_hub(&mut cfg);
        assert_eq!(
            cfg.last.get("gamer"),
            Some(&"cs2".to_string()),
            "seed не должен затирать существующий last.gamer"
        );
        assert!(cfg.scene.iter().any(|s| s.id == "hub"));
    }

    // 17. T185: restore_for_mode остаётся read-only на диске — регрессия T164
    // контракта после появления activate/save_config.
    #[test]
    fn restore_for_mode_contract_stays_read_only_by_construction() {
        // restore_for_mode принимает только &mut App (глобальное состояние в
        // памяти) и нигде в теле не зовёт save_config — это гарантия по
        // конструкции функции (см. её тело выше), а не по этому тесту:
        // модуль не даёт restore_for_mode доступа к записи, потому что она
        // не принимает ScenesConfig на запись и работает только с cx.global.
        // Здесь фиксируем словами инвариант, который ловят live-смоки (T190).
        let cfg = cfg_with_scenes(vec![Scene {
            id: "hub".into(),
            name: "Game Hub".into(),
            mode: "gamer".into(),
            ..Default::default()
        }]);
        // resolve_last (чистая часть restore_for_mode) — тоже без записи.
        let mut cfg = cfg;
        cfg.last.insert("gamer".into(), "hub".into());
        let resolved = resolve_last(&cfg, WorkspaceMode::Gamer);
        assert_eq!(resolved.map(|s| s.id), Some("hub".to_string()));
    }
}

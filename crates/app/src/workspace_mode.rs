//! Workspace mode — Developer/Gamer shell composition flag (слайс 1 спеки
//! `docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`).
//!
//! Режим меняет состав инструментов и дефолты, но НИКОГДА не переключается
//! сам: §1 спеки требует явного действия пользователя. Любой автоматический
//! переход — нарушение контракта.
//!
//! Порядок разрешения стартового режима (по образцу `theme_config.rs`):
//!   1. env `CHRONOS_WORKSPACE_MODE` — удобно для смоков; мусор/пусто → дальше
//!   2. `~/.config/chronos/workspace.toml`, поле `mode = "developer" | "gamer"`
//!   3. `WorkspaceMode::Developer`
//!
//! Файл не перезаписывается молча при отсутствии/битом — только warn и дефолт.
//! Запись происходит лишь при явной смене режима пользователем.

use std::collections::BTreeMap;
use std::path::PathBuf;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

const CONFIG_BASENAME: &str = "workspace.toml";
const ENV_OVERRIDE: &str = "CHRONOS_WORKSPACE_MODE";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceMode {
    #[default]
    Developer,
    Gamer,
}

impl WorkspaceMode {
    pub fn label(self) -> &'static str {
        match self {
            WorkspaceMode::Developer => "Developer",
            WorkspaceMode::Gamer => "Gamer",
        }
    }

    // Разведка T159: `icons/code.svg` и `icons/gamepad.svg` в дереве НЕ
    // существуют (36 файлов в `crates/app/assets/icons/`, ни одного
    // подходящего по имени). GPUI при несуществующем пути молча рисует
    // пустоту — поэтому берём существующие и помечаем TODO.
    pub fn icon_path(self) -> &'static str {
        match self {
            // TODO: заменить на code.svg / gamepad.svg, когда добавим свои SVG.
            WorkspaceMode::Developer => "icons/rail-editor.svg",
            WorkspaceMode::Gamer => "icons/bolt.svg",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "developer" => Some(WorkspaceMode::Developer),
            "gamer" => Some(WorkspaceMode::Gamer),
            _ => None,
        }
    }

    pub fn other(self) -> Self {
        match self {
            WorkspaceMode::Developer => WorkspaceMode::Gamer,
            WorkspaceMode::Gamer => WorkspaceMode::Developer,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptPref {
    /// Спрашивать при следующем сигнале от этого приложения.
    #[default]
    Ask,
    /// Больше не спрашивать для этого приложения. Режим при этом НЕ
    /// переключается автоматически — вариант «всегда переключать» намеренно
    /// отсутствует, он нарушал бы §1 спеки.
    Never,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceConfig {
    pub mode: Option<WorkspaceMode>,
    #[serde(default)]
    pub prompt_prefs: BTreeMap<String, PromptPref>,
}

/// Ожидающее ответа предложение сменить режим. Ненавязчивое: живёт плашкой в
/// баре, не крадёт фокус клавиатуры, не блокирует ввод.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPrompt {
    pub target: WorkspaceMode,
    pub app_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceModeState {
    pub mode: WorkspaceMode,
    pub pending: Option<PendingPrompt>,
}

impl Global for WorkspaceModeState {}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

pub fn load_config() -> WorkspaceConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<WorkspaceConfig>(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(
                    "workspace_mode: failed to parse {}: {e}, using defaults",
                    path.display()
                );
                WorkspaceConfig::default()
            }
        },
        Err(_) => WorkspaceConfig::default(),
    }
}

fn save_config(cfg: &WorkspaceConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!("workspace_mode: mkdir {} failed: {e}", parent.display());
            return;
        }
    }
    match toml::to_string_pretty(cfg) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                tracing::warn!("workspace_mode: write {} failed: {e}", path.display());
            }
        }
        Err(e) => tracing::warn!("workspace_mode: serialize failed: {e}"),
    }
}

/// Чистая функция разрешения стартового режима — вынесена из `init`, чтобы
/// её можно было тестировать без реального окружения и файловой системы.
pub fn resolve_initial(cfg: &WorkspaceConfig, env: Option<&str>) -> WorkspaceMode {
    if let Some(raw) = env {
        if let Some(mode) = WorkspaceMode::parse(raw) {
            return mode;
        }
        if !raw.trim().is_empty() {
            tracing::warn!("workspace_mode: ignoring bad {ENV_OVERRIDE}={raw:?}");
        }
    }
    cfg.mode.unwrap_or_default()
}

pub fn init(cx: &mut App) {
    let env = std::env::var(ENV_OVERRIDE).ok();
    let mode = resolve_initial(&load_config(), env.as_deref());
    tracing::info!(mode = mode.label(), "workspace_mode: initial");
    cx.set_global(WorkspaceModeState { mode, pending: None });
}

pub fn current(cx: &App) -> WorkspaceMode {
    cx.try_global::<WorkspaceModeState>()
        .map(|s| s.mode)
        .unwrap_or_default()
}

/// Явная смена режима. Вызывается ТОЛЬКО из пользовательских путей: клик по
/// виджету бара, IPC-команда, подтверждение предложения. Никаких вызовов из
/// детекторов и таймеров.
pub fn set(cx: &mut App, mode: WorkspaceMode) {
    if current(cx) == mode {
        cx.global_mut::<WorkspaceModeState>().pending = None;
        cx.refresh_windows();
        return;
    }
    cx.global_mut::<WorkspaceModeState>().mode = mode;
    cx.global_mut::<WorkspaceModeState>().pending = None;
    let mut cfg = load_config();
    cfg.mode = Some(mode);
    save_config(&cfg);
    tracing::info!(mode = mode.label(), "workspace_mode: switched");
    cx.refresh_windows();
}

pub fn toggle(cx: &mut App) {
    set(cx, current(cx).other());
}

/// Чистая функция решения — тестируется без GPUI и файловой системы.
pub fn should_prompt(
    cfg: &WorkspaceConfig,
    current: WorkspaceMode,
    target: WorkspaceMode,
    app_id: &str,
) -> bool {
    if current == target || app_id.trim().is_empty() {
        return false;
    }
    cfg.prompt_prefs.get(app_id.trim()) != Some(&PromptPref::Never)
}

/// Точка входа для детекторов (игра вышла в фуллскрин, открылся проект).
/// НИКОГДА не переключает режим сама — только ставит предложение в очередь.
/// Детектор в этом слайсе не реализуется; функция — согласованный контракт.
pub fn request_switch(cx: &mut App, target: WorkspaceMode, app_id: &str) {
    if !should_prompt(&load_config(), current(cx), target, app_id) {
        return;
    }
    let prompt = PendingPrompt {
        target,
        app_id: app_id.trim().to_string(),
    };
    tracing::info!(
        target = target.label(),
        app_id = %prompt.app_id,
        "workspace_mode: prompting"
    );
    cx.global_mut::<WorkspaceModeState>().pending = Some(prompt);
    cx.refresh_windows();
}

pub fn pending(cx: &App) -> Option<PendingPrompt> {
    cx.try_global::<WorkspaceModeState>()
        .and_then(|s| s.pending.clone())
}

/// Пользователь согласился — единственный путь, которым предложение может
/// привести к смене режима.
pub fn accept_prompt(cx: &mut App) {
    let Some(prompt) = pending(cx) else {
        return;
    };
    cx.global_mut::<WorkspaceModeState>().pending = None;
    set(cx, prompt.target);
}

/// Пользователь отказался. `silence = true` — «больше не спрашивать для этого
/// приложения»: пишет `PromptPref::Never` в конфиг.
pub fn dismiss_prompt(cx: &mut App, silence: bool) {
    let Some(prompt) = pending(cx) else {
        return;
    };
    cx.global_mut::<WorkspaceModeState>().pending = None;
    if silence {
        let mut cfg = load_config();
        cfg.prompt_prefs.insert(prompt.app_id.clone(), PromptPref::Never);
        save_config(&cfg);
        tracing::info!(app_id = %prompt.app_id, "workspace_mode: prompt silenced");
    }
    cx.refresh_windows();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_developer() {
        assert_eq!(WorkspaceMode::default(), WorkspaceMode::Developer);
    }

    #[test]
    fn parse_is_case_insensitive_and_rejects_garbage() {
        assert_eq!(WorkspaceMode::parse("developer"), Some(WorkspaceMode::Developer));
        assert_eq!(WorkspaceMode::parse("Developer"), Some(WorkspaceMode::Developer));
        assert_eq!(WorkspaceMode::parse("  GAMER  "), Some(WorkspaceMode::Gamer));
        assert_eq!(WorkspaceMode::parse("gamer!"), None);
        assert_eq!(WorkspaceMode::parse(""), None);
    }

    #[test]
    fn other_flips_the_mode() {
        assert_eq!(WorkspaceMode::Developer.other(), WorkspaceMode::Gamer);
        assert_eq!(WorkspaceMode::Gamer.other(), WorkspaceMode::Developer);
    }

    #[test]
    fn env_override_wins_over_config() {
        let cfg = WorkspaceConfig { mode: Some(WorkspaceMode::Developer), prompt_prefs: BTreeMap::new() };
        assert_eq!(resolve_initial(&cfg, Some("gamer")), WorkspaceMode::Gamer);
    }

    #[test]
    fn bad_env_falls_through_to_config() {
        let cfg = WorkspaceConfig { mode: Some(WorkspaceMode::Gamer), prompt_prefs: BTreeMap::new() };
        assert_eq!(resolve_initial(&cfg, Some("nonsense")), WorkspaceMode::Gamer);
        assert_eq!(resolve_initial(&cfg, Some("   ")), WorkspaceMode::Gamer);
    }

    #[test]
    fn empty_config_falls_back_to_default() {
        let cfg = WorkspaceConfig { mode: None, prompt_prefs: BTreeMap::new() };
        assert_eq!(resolve_initial(&cfg, None), WorkspaceMode::Developer);
    }

    #[test]
    fn labels_are_distinct_and_non_empty() {
        assert_ne!(WorkspaceMode::Developer.label(), WorkspaceMode::Gamer.label());
        assert!(!WorkspaceMode::Developer.label().is_empty());
        assert!(!WorkspaceMode::Gamer.label().is_empty());
    }

    fn cfg_with_pref(app: &str, pref: PromptPref) -> WorkspaceConfig {
        let mut cfg = WorkspaceConfig::default();
        cfg.prompt_prefs.insert(app.to_string(), pref);
        cfg
    }

    #[test]
    fn does_not_prompt_when_already_in_target_mode() {
        let cfg = WorkspaceConfig::default();
        assert!(!should_prompt(
            &cfg,
            WorkspaceMode::Gamer,
            WorkspaceMode::Gamer,
            "steam_app_570"
        ));
    }

    #[test]
    fn prompts_by_default_for_unknown_app() {
        let cfg = WorkspaceConfig::default();
        assert!(should_prompt(
            &cfg,
            WorkspaceMode::Developer,
            WorkspaceMode::Gamer,
            "steam_app_570"
        ));
    }

    #[test]
    fn never_pref_silences_that_app_only() {
        let cfg = cfg_with_pref("steam_app_570", PromptPref::Never);
        assert!(!should_prompt(
            &cfg,
            WorkspaceMode::Developer,
            WorkspaceMode::Gamer,
            "steam_app_570"
        ));
        assert!(should_prompt(
            &cfg,
            WorkspaceMode::Developer,
            WorkspaceMode::Gamer,
            "steam_app_620"
        ));
    }

    #[test]
    fn empty_app_id_never_prompts() {
        let cfg = WorkspaceConfig::default();
        assert!(!should_prompt(
            &cfg,
            WorkspaceMode::Developer,
            WorkspaceMode::Gamer,
            "  "
        ));
    }

    #[test]
    fn prompt_pref_roundtrips_through_toml() {
        let cfg = cfg_with_pref("steam_app_570", PromptPref::Never);
        let text = toml::to_string_pretty(&cfg).expect("сериализация конфига");
        let back: WorkspaceConfig = toml::from_str(&text).expect("разбор конфига");
        assert_eq!(back, cfg);
    }
}

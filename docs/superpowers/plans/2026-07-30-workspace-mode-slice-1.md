# Workspace Mode (слайс 1) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ввести режим рабочего пространства (Developer/Gamer) как персистентное глобальное состояние с ручным переключателем в баре, IPC-командами и контрактом ненавязчивого предложения смены режима — без единого автоматического переключения.

**Architecture:** Один новый модуль `crates/app/src/workspace_mode.rs` по образцу `edit_mode.rs` (GPUI `Global` + `init`/`current`/`set`/`toggle`), с персистентностью в `~/.config/chronos/workspace.toml` по образцу `theme_config.rs`. Переключатель — новый виджет бара в правом кластере (`BarSection::Right`), регистрируется через существующий механизм `build_widget`/`BUILTIN_NAMES`. Smart-prompt в этом слайсе — контракт, а не детектор: решающая логика, хранилище пер-аппных предпочтений и ненавязчивая плашка в баре; сам детектор игр/проектов приезжает в слайсах 5-6.

**Tech Stack:** Rust 2024, gpui-ce ChronOS fork (`../Source`), serde + toml, tracing.

## Global Constraints

- Спека: `docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`. При расхождении плана и спеки побеждает спека.
- **Режим НИКОГДА не переключается автоматически** (спека §1). Любой код, меняющий режим без явного действия пользователя, — провал задачи.
- Композиция бара из `STYLE.md`: CAVA строго по центру, часы — крайние справа. Новый виджет встаёт **только** в правый кластер и **только** левее `clock`; трогать `center` запрещено.
- Линты воркспейса включены: `unsafe_code = deny`, `clippy::unwrap_used`/`expect_used = warn`. Новый код не добавляет `unwrap`/`expect` вне тестов.
- **Никогда не глушить ошибку через `let _ = fallible_call()`** — только `?`, `.log_err()` или явный `match`.
- Коммиты без AI-трейлеров, формат `область : что сделано`, поимённый `git add`, перед коммитом глазами `git diff --staged`.
- Тики и UI — только GPUI executor; tokio допустим лишь в IPC/D-Bus периметре (DECISIONS «Runtime split»).
- Тесты: `cargo test -p chronos --bins`. Сборка: `cargo build --release -p chronos`.
- Работа ведётся в свежем воркетри от актуального ствола (см. «Подготовка» ниже), не в главном дереве.

## Подготовка (выполнить один раз до Task 1)

- [ ] **Создать изолированный воркетри**

```bash
cd /home/neo/projects/chronos-ecosystem/ChronOS
git worktree add -b feat/workspace-mode ../ChronOS-wt-workspace-mode
cd ../ChronOS-wt-workspace-mode
cargo check -p chronos --bin chronos
```

Ожидается: `Finished` без ошибок. Если чек падает — не начинать работу, доложить архитектору: ствол сломан, это не твоя регрессия.

---

### Task 1: Состояние режима + персистентность

**Files:**
- Create: `crates/app/src/workspace_mode.rs`
- Modify: `crates/app/src/main.rs:5` (объявление модуля), `crates/app/src/main.rs:77` (вызов init)

**Interfaces:**
- Consumes: ничего (первая задача).
- Produces:
  - `pub enum WorkspaceMode { Developer, Gamer }` — `Copy`, `Default` = `Developer`
  - `WorkspaceMode::label(self) -> &'static str`
  - `WorkspaceMode::icon_path(self) -> &'static str`
  - `WorkspaceMode::parse(s: &str) -> Option<WorkspaceMode>`
  - `WorkspaceMode::other(self) -> WorkspaceMode`
  - `pub struct WorkspaceModeState { pub mode: WorkspaceMode }` — GPUI `Global`
  - `pub struct WorkspaceConfig { pub mode: Option<WorkspaceMode> }`
  - `pub fn resolve_initial(cfg: &WorkspaceConfig, env: Option<&str>) -> WorkspaceMode`
  - `pub fn init(cx: &mut App)`
  - `pub fn current(cx: &App) -> WorkspaceMode`
  - `pub fn set(cx: &mut App, mode: WorkspaceMode)`
  - `pub fn toggle(cx: &mut App)`

- [ ] **Step 1: Написать падающий тест**

Создать `crates/app/src/workspace_mode.rs` с одним только тестовым модулем и заглушками типов:

```rust
//! Workspace mode — Developer/Gamer shell composition flag.

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
        let cfg = WorkspaceConfig { mode: Some(WorkspaceMode::Developer) };
        assert_eq!(resolve_initial(&cfg, Some("gamer")), WorkspaceMode::Gamer);
    }

    #[test]
    fn bad_env_falls_through_to_config() {
        let cfg = WorkspaceConfig { mode: Some(WorkspaceMode::Gamer) };
        assert_eq!(resolve_initial(&cfg, Some("nonsense")), WorkspaceMode::Gamer);
        assert_eq!(resolve_initial(&cfg, Some("   ")), WorkspaceMode::Gamer);
    }

    #[test]
    fn empty_config_falls_back_to_default() {
        let cfg = WorkspaceConfig { mode: None };
        assert_eq!(resolve_initial(&cfg, None), WorkspaceMode::Developer);
    }

    #[test]
    fn labels_are_distinct_and_non_empty() {
        assert_ne!(WorkspaceMode::Developer.label(), WorkspaceMode::Gamer.label());
        assert!(!WorkspaceMode::Developer.label().is_empty());
        assert!(!WorkspaceMode::Gamer.label().is_empty());
    }
}
```

Добавить объявление модуля в `crates/app/src/main.rs` рядом с `mod edit_mode;` (строка 5):

```rust
mod workspace_mode;
```

- [ ] **Step 2: Запустить тест и убедиться, что он падает**

```bash
cargo test -p chronos --bins workspace_mode
```

Ожидается: FAIL — `cannot find type WorkspaceMode in this scope` (E0412).

- [ ] **Step 3: Написать минимальную реализацию**

Заменить содержимое `crates/app/src/workspace_mode.rs` (тестовый модуль из Step 1 оставить в конце файла без изменений):

```rust
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

    pub fn icon_path(self) -> &'static str {
        match self {
            WorkspaceMode::Developer => "icons/code.svg",
            WorkspaceMode::Gamer => "icons/gamepad.svg",
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

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceConfig {
    pub mode: Option<WorkspaceMode>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceModeState {
    pub mode: WorkspaceMode,
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
    cx.set_global(WorkspaceModeState { mode });
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
        return;
    }
    cx.global_mut::<WorkspaceModeState>().mode = mode;
    save_config(&WorkspaceConfig { mode: Some(mode) });
    tracing::info!(mode = mode.label(), "workspace_mode: switched");
    cx.refresh_windows();
}

pub fn toggle(cx: &mut App) {
    set(cx, current(cx).other());
}
```

- [ ] **Step 4: Запустить тесты и убедиться, что они проходят**

```bash
cargo test -p chronos --bins workspace_mode
```

Ожидается: `test result: ok. 7 passed; 0 failed`.

- [ ] **Step 5: Подключить init в main**

В `crates/app/src/main.rs` добавить вызов сразу после `edit_mode::init(cx);` (строка 77):

```rust
        workspace_mode::init(cx);
```

- [ ] **Step 6: Проверить сборку**

```bash
cargo check -p chronos --bin chronos
```

Ожидается: `Finished` без ошибок и без новых предупреждений.

- [ ] **Step 7: Коммит**

```bash
git add crates/app/src/workspace_mode.rs crates/app/src/main.rs
git diff --staged
git commit -m "workspace : режим Developer/Gamer — глобал, персистентность, env-оверрайд"
```

---

### Task 2: IPC-команды переключения

**Files:**
- Modify: `crates/app/src/ipc/messages.rs` (константы, `encode_*`/`is_*`, `classify_set_workspace_mode`, тесты)
- Modify: `crates/app/src/ipc/service.rs:73` (канал), `:88` (передача sender), `:104` (возврат receiver), `:182` (поле структуры), `:195` (клон), `:224` (диспетч)
- Modify: `crates/app/src/ipc/mod.rs:20` (приём receiver), `:34` (дебаунс-таймстамп), `:116` (арм `tokio::select!`)

**Interfaces:**
- Consumes: `workspace_mode::{WorkspaceMode, set, toggle}` из Task 1.
- Produces:
  - `pub const TOGGLE_WORKSPACE_MODE_PAYLOAD: &str = "toggle-workspace-mode"`
  - `pub fn encode_toggle_workspace_mode() -> String`
  - `pub fn is_toggle_workspace_mode(payload: &str) -> bool`
  - `pub fn encode_set_workspace_mode(mode: WorkspaceMode) -> String`
  - `pub fn classify_set_workspace_mode(payload: &str) -> Option<WorkspaceMode>`

- [ ] **Step 1: Написать падающие тесты**

В `crates/app/src/ipc/messages.rs`, в существующий `#[cfg(test)] mod tests` (рядом с `encodes_and_recognizes_toggle_edit_mode`, строка 286), добавить:

```rust
    #[test]
    fn encodes_and_recognizes_toggle_workspace_mode() {
        let payload = encode_toggle_workspace_mode();
        assert!(is_toggle_workspace_mode(&payload));
        assert!(!is_toggle_workspace_mode("toggle-edit-mode"));
    }

    #[test]
    fn classifies_set_workspace_mode() {
        use crate::workspace_mode::WorkspaceMode;
        assert_eq!(
            classify_set_workspace_mode(&encode_set_workspace_mode(WorkspaceMode::Gamer)),
            Some(WorkspaceMode::Gamer)
        );
        assert_eq!(
            classify_set_workspace_mode("set-workspace-mode:developer"),
            Some(WorkspaceMode::Developer)
        );
        assert_eq!(classify_set_workspace_mode("set-workspace-mode:nonsense"), None);
        assert_eq!(classify_set_workspace_mode("set-workspace-mode:"), None);
        assert_eq!(classify_set_workspace_mode("toggle-workspace-mode"), None);
    }
```

- [ ] **Step 2: Запустить тесты и убедиться, что они падают**

```bash
cargo test -p chronos --bins workspace_mode
```

Ожидается: FAIL — `cannot find function encode_toggle_workspace_mode` (E0425).

- [ ] **Step 3: Реализовать протокол**

В `crates/app/src/ipc/messages.rs` рядом с блоком `TOGGLE_EDIT_MODE_PAYLOAD` (строка 42) добавить:

```rust
pub const TOGGLE_WORKSPACE_MODE_PAYLOAD: &str = "toggle-workspace-mode";
const SET_WORKSPACE_MODE_PREFIX: &str = "set-workspace-mode:";

// Тот же контракт, что и `encode_toggle_launcher` выше — внешние keybind-демоны
// переключают режим рабочего пространства.
#[allow(dead_code)]
pub fn encode_toggle_workspace_mode() -> String {
    TOGGLE_WORKSPACE_MODE_PAYLOAD.to_string()
}

pub fn is_toggle_workspace_mode(payload: &str) -> bool {
    payload.trim() == TOGGLE_WORKSPACE_MODE_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_set_workspace_mode(mode: crate::workspace_mode::WorkspaceMode) -> String {
    format!("{SET_WORKSPACE_MODE_PREFIX}{}", mode.label().to_ascii_lowercase())
}

/// Разбирает `set-workspace-mode:<mode>`. Неизвестный режим → `None`
/// (команда игнорируется, режим не меняется).
pub fn classify_set_workspace_mode(
    payload: &str,
) -> Option<crate::workspace_mode::WorkspaceMode> {
    let rest = payload.trim().strip_prefix(SET_WORKSPACE_MODE_PREFIX)?;
    crate::workspace_mode::WorkspaceMode::parse(rest)
}
```

- [ ] **Step 4: Запустить тесты и убедиться, что они проходят**

```bash
cargo test -p chronos --bins workspace_mode
```

Ожидается: `test result: ok. 9 passed; 0 failed`.

- [ ] **Step 5: Прокинуть канал в сервисе**

В `crates/app/src/ipc/service.rs`:

Рядом со строкой 73 (`let (edit_mode_toggle_sender, edit_mode_toggle_receiver) = mpsc::unbounded_channel();`) добавить:

```rust
        let (workspace_mode_sender, workspace_mode_receiver) = mpsc::unbounded_channel();
```

В вызов, передающий sender'ы (строка ~88), добавить `workspace_mode_sender,` следом за `edit_mode_toggle_sender,`. В структуру возврата (строка ~104) добавить `workspace_mode_receiver,` следом за `edit_mode_toggle_receiver,`. В сигнатуру функции (строка ~182) добавить поле:

```rust
    workspace_mode_sender: mpsc::UnboundedSender<WorkspaceModeIpcCmd>,
```

Рядом со строкой 195 добавить клон:

```rust
                let workspace_mode_sender = workspace_mode_sender.clone();
```

В цепочку диспетча (строка ~224, после ветки `is_toggle_edit_mode`) добавить:

```rust
                        } else if is_toggle_workspace_mode(&payload) {
                            if let Err(e) =
                                workspace_mode_sender.send(WorkspaceModeIpcCmd::Toggle)
                            {
                                tracing::warn!("IPC workspace-mode toggle dropped: {e}");
                            }
                            tracing::info!("IPC toggle-workspace-mode received");
                        } else if let Some(mode) = classify_set_workspace_mode(&payload) {
                            if let Err(e) =
                                workspace_mode_sender.send(WorkspaceModeIpcCmd::Set(mode))
                            {
                                tracing::warn!("IPC workspace-mode set dropped: {e}");
                            }
                            tracing::info!(mode = mode.label(), "IPC set-workspace-mode received");
```

Импорты в шапке `service.rs` (строка 9) дополнить: `is_toggle_workspace_mode, classify_set_workspace_mode, WorkspaceModeIpcCmd`. В `crates/app/src/ipc/mod.rs` в шапку импортов добавить `WorkspaceModeIpcCmd` из `crate::ipc::messages` — арм `tokio::select!` из Step 6 сопоставляет его варианты.

Тип команды объявить в `crates/app/src/ipc/messages.rs` рядом с `WallpaperIpcCmd`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceModeIpcCmd {
    Toggle,
    Set(crate::workspace_mode::WorkspaceMode),
}
```

- [ ] **Step 6: Обработать команду в цикле**

В `crates/app/src/ipc/mod.rs`: в деструктуризацию (строка ~20) добавить `mut workspace_mode_receiver,`; рядом со строкой 34 добавить таймстамп:

```rust
            let mut last_workspace_mode_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
```

После арма `edit_mode_toggle` (строка ~116) добавить:

```rust
                    workspace_mode_cmd = workspace_mode_receiver.recv() => {
                        if let Some(cmd) = workspace_mode_cmd {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_workspace_mode_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_workspace_mode_at = now;
                                if let Err(e) = cx.update(|cx| match cmd {
                                    WorkspaceModeIpcCmd::Toggle => {
                                        crate::workspace_mode::toggle(cx)
                                    }
                                    WorkspaceModeIpcCmd::Set(mode) => {
                                        crate::workspace_mode::set(cx, mode)
                                    }
                                }) {
                                    tracing::warn!("workspace-mode IPC update failed: {e}");
                                }
                            }
                        } else {
                            break;
                        }
                    }
```

- [ ] **Step 7: Проверить сборку и весь тест-набор**

```bash
cargo check -p chronos --bin chronos && cargo test -p chronos --bins
```

Ожидается: `Finished` + `test result: ok`, ноль failed.

- [ ] **Step 8: Коммит**

```bash
git add crates/app/src/ipc/messages.rs crates/app/src/ipc/service.rs crates/app/src/ipc/mod.rs
git diff --staged
git commit -m "ipc : toggle-workspace-mode и set-workspace-mode:<mode>"
```

---

### Task 3: Переключатель режима в баре

**Files:**
- Create: `crates/app/src/bar/widgets/workspace_mode.rs`
- Modify: `crates/app/src/bar/widgets/mod.rs:46-59` (ветка `build_widget`), объявление `mod`
- Modify: `crates/app/src/bar/layout_config.rs:18-33` (`BUILTIN_NAMES`), `:53-64` (дефолтный правый кластер)

**Interfaces:**
- Consumes: `workspace_mode::{current, toggle, WorkspaceMode}` из Task 1.
- Produces: `pub struct WorkspaceModeWidget` реализующий `chronos_luau::bar::BarWidget` с `name() == "workspace_mode"` и `section() == BarSection::Right`.

- [ ] **Step 1: Написать падающий тест**

Создать `crates/app/src/bar/widgets/workspace_mode.rs`:

```rust
//! Переключатель режима рабочего пространства для бара — иконка + подпись,
//! клик переключает Developer ⇄ Gamer.
//!
//! Живёт СТРОГО в правом кластере левее часов: `STYLE.md` фиксирует CAVA по
//! центру и часы крайними справа, и этот виджет их не двигает.

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_luau::bar::{BarSection, BarWidget};

    #[test]
    fn widget_name_and_section_are_stable() {
        let w = WorkspaceModeWidget;
        assert_eq!(w.name(), "workspace_mode");
        assert!(matches!(w.section(), BarSection::Right));
    }
}
```

Добавить в `crates/app/src/bar/widgets/mod.rs` рядом с прочими `mod`-объявлениями:

```rust
mod workspace_mode;
```

- [ ] **Step 2: Запустить тест и убедиться, что он падает**

```bash
cargo test -p chronos --bins widget_name_and_section
```

Ожидается: FAIL — `cannot find type WorkspaceModeWidget` (E0412).

- [ ] **Step 3: Реализовать виджет**

Заменить содержимое `crates/app/src/bar/widgets/workspace_mode.rs` (тестовый модуль оставить в конце файла без изменений):

```rust
//! Переключатель режима рабочего пространства для бара — иконка + подпись,
//! клик переключает Developer ⇄ Gamer.
//!
//! Живёт СТРОГО в правом кластере левее часов: `STYLE.md` фиксирует CAVA по
//! центру и часы крайними справа, и этот виджет их не двигает.

use gpui::{AnyElement, App, Window, div, prelude::*, px, svg};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

use crate::workspace_mode;

pub struct WorkspaceModeWidget;

impl BarWidget for WorkspaceModeWidget {
    fn name(&self) -> &str {
        "workspace_mode"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let mode = workspace_mode::current(cx);

        div()
            .id("bar-workspace-mode")
            .flex()
            .items_center()
            .gap(px(5.))
            .cursor_pointer()
            .px(px(7.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(
                svg()
                    .path(mode.icon_path())
                    .size(px(12.))
                    .text_color(theme.text.secondary),
            )
            .child(
                div()
                    .child(mode.label())
                    .text_color(theme.text.secondary)
                    .text_size(px(12.)),
            )
            .on_click(|_event, _window, cx| {
                workspace_mode::toggle(cx);
            })
            .into_any_element()
    }
}
```

- [ ] **Step 4: Зарегистрировать виджет**

В `crates/app/src/bar/widgets/mod.rs`, в `build_widget` рядом со строкой 51 (`"project" => …`) добавить:

```rust
        "workspace_mode" => Box::new(workspace_mode::WorkspaceModeWidget),
```

В `crates/app/src/bar/layout_config.rs` в `BUILTIN_NAMES` (строка 18) добавить `"workspace_mode",` сразу после `"project",`. В `Default for BarLayoutConfig` в вектор `right` (строка 53) добавить `"workspace_mode".into(),` сразу после `"project".into(),`.

- [ ] **Step 5: Запустить тесты и убедиться, что они проходят**

```bash
cargo test -p chronos --bins
```

Ожидается: `test result: ok`, ноль failed. Тест `default_layout` в `layout_config.rs` (строка ~353) сверяет дефолтный правый кластер поимённо — если он падает, обнови его ожидаемый вектор, добавив `"workspace_mode"` после `"project"`, и только так; порядок остальных не трогай.

- [ ] **Step 6: Проверить наличие иконок**

```bash
ls crates/app/assets/icons/code.svg crates/app/assets/icons/gamepad.svg
```

Если какого-то файла нет — найди ближайший существующий в `crates/app/assets/icons/` (`ls crates/app/assets/icons/`) и подставь его путь в `WorkspaceMode::icon_path` в `crates/app/src/workspace_mode.rs`. **Не создавай SVG сам** и не оставляй путь к несуществующему файлу: иконка молча не отрисуется, и это всплывёт только на живом прогоне.

- [ ] **Step 7: Живая проверка**

```bash
cargo build --release -p chronos
```

Дальше — доложить архитектору, что готово к живому смоку, и **остановиться**. Живой прогон шелла, `grim`-скриншот бара и клик по переключателю делает архитектор: «компилируется и тесты зелёные» для оконного кода в этом проекте не считается за проверку.

- [ ] **Step 8: Коммит**

```bash
git add crates/app/src/bar/widgets/workspace_mode.rs crates/app/src/bar/widgets/mod.rs crates/app/src/bar/layout_config.rs
git diff --staged
git commit -m "bar : виджет переключения режима рабочего пространства в правом кластере"
```

---

### Task 4: Контракт ненавязчивого предложения смены режима

**Files:**
- Modify: `crates/app/src/workspace_mode.rs` (тип `PromptPref`, `PendingPrompt`, `should_prompt`, `request_switch`, `accept_prompt`, `dismiss_prompt`, `silence_app`, расширение `WorkspaceConfig`)
- Modify: `crates/app/src/bar/widgets/workspace_mode.rs` (рендер плашки предложения)

**Interfaces:**
- Consumes: `WorkspaceMode`, `WorkspaceModeState`, `set`, `current`, `save_config`, `load_config` из Task 1.
- Produces:
  - `pub enum PromptPref { Ask, Never }` — `Default` = `Ask`
  - `pub struct PendingPrompt { pub target: WorkspaceMode, pub app_id: String }`
  - `pub fn should_prompt(cfg: &WorkspaceConfig, current: WorkspaceMode, target: WorkspaceMode, app_id: &str) -> bool`
  - `pub fn request_switch(cx: &mut App, target: WorkspaceMode, app_id: &str)`
  - `pub fn pending(cx: &App) -> Option<PendingPrompt>`
  - `pub fn accept_prompt(cx: &mut App)`
  - `pub fn dismiss_prompt(cx: &mut App, silence: bool)`

**Важно про семантику предпочтений.** В спеке §5 сказано «remember a per-application preference», но вариант «всегда переключать» противоречил бы §1 («never switch automatically»). Поэтому предпочтений ровно два: `Ask` (спрашивать) и `Never` (больше не спрашивать для этого приложения). Автоматического переключения нет ни при каком значении.

**Что в этот слайс НЕ входит.** Детектор — то есть код, который наблюдает за композитором и решает, что запустилась игра или открылся проект. Его пишут слайсы 5-6. Здесь реализуется только точка входа `request_switch`, которую детектор потом дёрнет.

- [ ] **Step 1: Написать падающие тесты**

В `crates/app/src/workspace_mode.rs`, в существующий `#[cfg(test)] mod tests`, добавить:

```rust
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
```

- [ ] **Step 2: Запустить тесты и убедиться, что они падают**

```bash
cargo test -p chronos --bins workspace_mode
```

Ожидается: FAIL — `cannot find type PromptPref` (E0412).

- [ ] **Step 3: Реализовать хранилище предпочтений и решающую логику**

В `crates/app/src/workspace_mode.rs` добавить импорт `use std::collections::BTreeMap;` в шапку, заменить `WorkspaceConfig` и дописать типы:

```rust
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
```

`WorkspaceModeState` перестал быть `Copy` — поправить `current`:

```rust
pub fn current(cx: &App) -> WorkspaceMode {
    cx.try_global::<WorkspaceModeState>()
        .map(|s| s.mode)
        .unwrap_or_default()
}
```

И `set` — сохранение должно беречь `prompt_prefs`, а не затирать их:

```rust
pub fn set(cx: &mut App, mode: WorkspaceMode) {
    if current(cx) == mode {
        return;
    }
    cx.global_mut::<WorkspaceModeState>().mode = mode;
    let mut cfg = load_config();
    cfg.mode = Some(mode);
    save_config(&cfg);
    tracing::info!(mode = mode.label(), "workspace_mode: switched");
    cx.refresh_windows();
}
```

Дописать точку входа для будущего детектора и обработчики ответа:

```rust
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
```

- [ ] **Step 4: Запустить тесты и убедиться, что они проходят**

```bash
cargo test -p chronos --bins workspace_mode
```

Ожидается: `test result: ok. 14 passed; 0 failed`.

- [ ] **Step 5: Отрисовать плашку предложения**

В `crates/app/src/bar/widgets/workspace_mode.rs` заменить тело `render`, обернув существующую пилюлю в строку с необязательной плашкой слева:

```rust
    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let mode = workspace_mode::current(cx);

        let pill = div()
            .id("bar-workspace-mode")
            .flex()
            .items_center()
            .gap(px(5.))
            .cursor_pointer()
            .px(px(7.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(
                svg()
                    .path(mode.icon_path())
                    .size(px(12.))
                    .text_color(theme.text.secondary),
            )
            .child(
                div()
                    .child(mode.label())
                    .text_color(theme.text.secondary)
                    .text_size(px(12.)),
            )
            .on_click(|_event, _window, cx| {
                workspace_mode::toggle(cx);
            });

        let mut row = div().flex().items_center().gap(px(6.));

        if let Some(prompt) = workspace_mode::pending(cx) {
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(theme.radius)
                    .bg(theme.bg.elevated)
                    .child(
                        div()
                            .child(format!("Перейти в {}?", prompt.target.label()))
                            .text_color(theme.text.primary)
                            .text_size(px(12.)),
                    )
                    .child(
                        div()
                            .id("workspace-mode-prompt-yes")
                            .cursor_pointer()
                            .child("Да")
                            .text_color(theme.accent.primary)
                            .text_size(px(12.))
                            .on_click(|_event, _window, cx| {
                                workspace_mode::accept_prompt(cx);
                            }),
                    )
                    .child(
                        div()
                            .id("workspace-mode-prompt-no")
                            .cursor_pointer()
                            .child("Нет")
                            .text_color(theme.text.muted)
                            .text_size(px(12.))
                            .on_click(|_event, _window, cx| {
                                workspace_mode::dismiss_prompt(cx, false);
                            }),
                    )
                    .child(
                        div()
                            .id("workspace-mode-prompt-never")
                            .cursor_pointer()
                            .child("Не спрашивать")
                            .text_color(theme.text.muted)
                            .text_size(px(12.))
                            .on_click(|_event, _window, cx| {
                                workspace_mode::dismiss_prompt(cx, true);
                            }),
                    ),
            );
        }

        row.child(pill).into_any_element()
    }
```

- [ ] **Step 6: Проверить сборку и весь набор**

```bash
cargo check -p chronos --bin chronos && cargo test -p chronos --bins
```

Ожидается: `Finished` + `test result: ok`, ноль failed. Если компилятор ругается на имена токенов темы (`theme.bg.elevated`, `theme.accent.primary`, `theme.text.muted`) — сверься с реальными полями в `crates/ui/src/theme/mod.rs` и возьми существующие; **не хардкодь hex** (спека §11 «do not hard-code palette values in runtime components»).

- [ ] **Step 7: Живая проверка предложения**

Собрать релиз и подготовить смок-команду, которой архитектор проверит плашку живьём:

```bash
cargo build --release -p chronos
```

В отчёте укажи: детектора нет, поэтому предложение в живом прогоне вызывается только из кода; предложи архитектору временный способ дёрнуть `request_switch` (например, разово из `init` под env-флагом) — **но не коммить этот вызов**.

- [ ] **Step 8: Коммит**

```bash
git add crates/app/src/workspace_mode.rs crates/app/src/bar/widgets/workspace_mode.rs
git diff --staged
git commit -m "workspace : контракт предложения смены режима — пер-аппные предпочтения + плашка в баре"
```

---

## Осознанно отложено (не считать пропуском)

Спека §5 перечисляет четыре ручных входа в переключение режима. Этот слайс закрывает два:

| Вход из §5 | Статус |
|---|---|
| Мода-контрол в баре | Task 3 |
| Настраиваемый глобальный шорткат | Task 2 — через IPC, биндится в конфиге Hyprland |
| Лаунчер | Отложено: требует правок в `crates/app/src/launcher/`, это чужая зона и отдельный набор файлов |
| Командная палитра | Отложено: командной палитры в дереве пока нет вообще, её вводит отдельная задача |

Оба отложенных входа добавляются поверх готового `workspace_mode::set` одним вызовом каждый — состояние и протокол этого слайса их уже обслуживают. Исполнителю их **не делать**: расширение зоны файлов без задания — повод отклонить отчёт.

## Итоговая верификация слайса

- [ ] **Полный набор тестов**

```bash
cargo test --workspace --lib --bins
```

Ожидается: ноль failed.

- [ ] **Релизная сборка**

```bash
cargo build --release -p chronos
```

Ожидается: `Finished`, ноль новых предупреждений.

- [ ] **Проверка отсутствия автопереключения**

```bash
grep -rn "workspace_mode::set\|workspace_mode::toggle" --include='*.rs' crates/
```

Каждое найденное место обязано быть пользовательским путём: клик по виджету, IPC-команда, `accept_prompt`. Любой вызов из таймера, детектора или подписки на сервис — нарушение §1 спеки, задача не сдана.

- [ ] **Отчёт**

Написать отчёт в `orchestration/tasks/report/<T-ID>-workspace-mode-slice-1-report.md`, где `<T-ID>` — номер, выданный архитектором в задании (он же в заголовке твоего `orchestration/tasks/active/`-файла). Отчёт содержит точный перечень изменённых файлов, выводом тестов и честным списком того, что НЕ проверено живьём. Не заявлять «работает», если шелл не запускался.

## Что живьём проверяет архитектор (не исполнитель)

1. Переключатель виден в правом кластере левее часов; CAVA осталась по центру, часы крайние справа.
2. Клик меняет подпись и иконку; `~/.config/chronos/workspace.toml` получает `mode = "gamer"`.
3. Перезапуск шелла поднимает сохранённый режим.
4. `CHRONOS_WORKSPACE_MODE=developer` перебивает конфиг.
5. IPC: `echo -n "toggle-workspace-mode" | nc -U <сокет>` переключает; `set-workspace-mode:gamer` ставит конкретный; `set-workspace-mode:мусор` игнорируется без паники.
6. Плашка предложения не крадёт фокус клавиатуры (проверить `hyprctl activewindow` до и после появления).
7. «Не спрашивать» пишет `prompt_prefs` в конфиг и не теряет `mode`.
8. `grim`-скриншот бара в тёмной и светлой теме.

# T196 report

**Зона:** `crates/app/src/side_panel_right/tab/bar_settings.rs` (extend),
`crates/app/src/side_panel_right/tab/acp_settings.rs` (новый, 154 строки),
`crates/app/src/side_panel_right/tab/mod.rs` (module + TabContent arm),
`crates/app/src/side_panel_right/view.rs` (match arm), `crates/app/src/lib.rs`
(theme_config export), `crates/app/src/theme_config.rs` (одна строка
видимости). Ничего вне зоны.

## System settings — bar_settings.rs extend (T196 §MVP)

Существующий `BarSettingsTab` (T202, bar presets) расширен тремя новыми
секциями внизу скролла:

### Theme toggle

- **Показывает** текущую тему: `🌙 Dark` / `☀ Light` + имя схемы
  (`Default` / `Light`).
- **Кнопка «Toggle»** — через `cx.update_global::<Theme, _>(|theme, cx| { ... })`
  получает `&mut App` внутри Render-контекста таба. Вызывает
  `theme_config::persist_scheme(next)` (пишет `theme.toml`), замещает глобал
  `Theme` новым значением, зовёт `theme_config::sync_gpui_component_theme(cx)`
  (синхронизирует gpui-component theme mode с shell theme), затем
  `cx.refresh_windows()`.
- **Инфраструктура:** `theme_config::persist_scheme` был `fn` (private) —
  поднят до `pub(crate) fn`. Модуль `theme_config` был bin-only (`mod
  theme_config` в `main.rs`) — добавлен в `lib.rs` как `pub mod theme_config`
  для доступа из lib-tree табов.

### Hypr modules

- **Lazy-load** из `~/.config/hypr/modules/` на первом рендере. Фильтрует
  `.lua` файлы, сортирует по имени. Кешируется в полях `hypr_modules` +
  `hypr_modules_loaded` на entity.
- **Каждый модуль** — кликабельный ряд (имя + путь), открывает файл в Editor
  через `PreviewTarget { intent: View }`.
- **Empty state:** «No modules found in ~/.config/hypr/modules/».

### About

- Статическая секция: «ChronOS shell», `env!("CARGO_PKG_VERSION")`,
  «Apache-2.0», «Rust + GPUI + mlua/LuauJIT», «2026».

## ACP agents — acp_settings.rs (новый, T196 §ACP)

Новый таб, монтируется на `PanelTab::AcpSettings` (ранее был `EmptyTab`).

### Data source

- **Использует `chronos_services::hermes_acp::registry::known_agents()`** —
  не дублирует TOML-схему. Функция читает `~/.config/chronos/agents.toml` и
  мёржит с built-in Hermes. Таб получает готовый `Vec<AgentDescriptor>`,
  конвертирует во внутренний `AgentRow` (id, display_name, command, args,
  builtin flag).
- `is_builtin("hermes")` — жёсткая проверка; при добавлении новых built-in
  агентов в registry потребует обновления здесь (residual).

### UI

- **Список агентов:** display_name + «built-in» badge для Hermes, id
  (mono), command + args.
- **«Open agents.toml»** → Editor (`PreviewTarget { intent: Edit }`).
- **«Reload»** → перечитывает `known_agents()`.
- **Empty / error:** честное сообщение (файл не найден, невалидный TOML,
  нет агентов).
- **Example TOML** внизу — подсказка формата `[[agents]]`.

### Add/Remove

- **Inline Add/Remove не сделано** — pragmatic MVP. `cx.listener` внутри
  циклов рендера создаёт per-agent замыкания с уникальными типами, которые
  невозможно сохранить в гомогенную коллекцию (каждый `cx.listener` →
  уникальный opaque-тип). Паттерн `bar_settings.rs` (все listeners на верхнем
  уровне `render()`, до `div()`/`.child()`) не масштабируется на динамическое
  число агентов.
- **Путь пользователя:** открыть `agents.toml` в Editor → добавить/удалить
  `[[agents]]` entry → нажать Reload в табе.
- **Residual:** inline Remove возможен через единый listener +
  `self.confirm_remove: Option<String>` + element id-based dispatch, но
  требует рефакторинга рендера (вынос данных из move-замыканий).

## Wiring

- **`tab/mod.rs`:** `pub(crate) mod acp_settings;`, `use
  acp_settings::AcpSettingsTab;`, `AcpSettings(gpui::Entity<AcpSettingsTab>)`
  в `TabContent`, `PanelTab::AcpSettings => TabContent::AcpSettings(...)` в
  `create()`.
- **`view.rs`:** `TabContent::AcpSettings(entity)` arm в render + `entity_id()`.

## Верификация

```
$ cargo check -p chronos
exit 0 (0 errors)

$ cargo test -p chronos --lib
test result: ok. 233 passed; 0 failed

$ cargo build --release -p chronos
exit 0
```

Новых юнит-тестов не писал — UI-логика чисто render (списки агентов,
клики открытия файла, reload). Тестирование через `#[gpui::test]` с mock
`App` для entity-таба — избыточно для данного объёма.

**Живой смок:** LIVE NOT VERIFIED. Smoke checklist:

```bash
# System settings (EditorSettings rail tab):
# 1. Theme section: shows current scheme, Toggle меняет тему без рестарта
# 2. Hypr modules: список .lua файлов, клик → открывает в Editor
# 3. About: версия из Cargo.toml

# ACP agents (AcpSettings rail tab):
# 4. Список: Hermes с badge «built-in»
# 5. Open agents.toml → Editor
# 6. Добавить [[agents]] entry в Editor, Save, Reload → агент в списке
```

## Что НЕ сделано

- **Inline ACP Add/Remove** — deferred (listener complexity, см. выше).
  Текущий путь: Edit agents.toml → Reload.
- **Hypr modules reload** — кеш на entity, не инвалидируется без
  пересоздания таба. Кнопка «Reload» не добавлена (секция Hypr modules —
  read-only snapshot).
- **theme_config двойная компиляция** — `main.rs` сохраняет `mod
  theme_config;` параллельно с `pub mod theme_config` в `lib.rs`. Модуль
  компилируется дважды. Убрать `mod theme_config` из `main.rs` — отдельная
  чистка.
- **Живой прогон** — LIVE NOT VERIFIED.

---

## Приёмка

| claim | check |
|---|---|
| Theme toggle в System settings | ✅ bar_settings.rs |
| Theme persist через persist_scheme | ✅ pub(crate) |
| Theme sync gpui_component_theme | ✅ update_global |
| Hypr modules список + open in Editor | ✅ lazy-load + PreviewTarget |
| About с CARGO_PKG_VERSION | ✅ |
| ACP agents список из known_agents() | ✅ не дублирует схему |
| ACP Open agents.toml + Reload | ✅ |
| Empty/error честные | ✅ |
| theme_config в lib.rs | ✅ |
| Wiring TabContent + view.rs | ✅ |
| 233/233 тестов | ✅ |
| LIVE | **NOT VERIFIED** |

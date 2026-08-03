# T195 report

**Зона:** `crates/app/src/agent_follow.rs` (новый), `crates/app/src/lib.rs`,
`crates/app/src/main.rs` (регистрация модуля), `crates/app/src/side_panel_left/
{mod,panel,composer}.rs` (toggle + streaming handler), `crates/app/src/
side_panel_right/view.rs` (subscription only). Ничего вне зоны.

## Что сделано

### `agent_follow.rs` — data layer

Новый файл, lib + bin:

- **`ToolCallPreview`** — standalone struct (не импортирует из bin-only
  `side_panel_left::chat_view`, доступна lib-дереву): `id`, `name`, `status`,
  `args`, `result`.
- **`AgentFollowState`** — `Global`: `enabled: bool` + `last_tool:
  Option<ToolCallPreview>`.
- **`push_tool()`** — обновляет `last_tool` (last-wins).
- **`extract_file_path()`** — эвристика: для `edit_file`/`write_file`/
  `read_file`/`file`/`open_file` — args это путь (проверяется на `/` или
  `~/`). Для остальных тулов — сканирует args и result на абсолютный путь
  или `~/`.

### Левая панель — Follow toggle + streaming

**`panel.rs`:**
- Кнопка 👁 в thread header (между ☰ и ⋯): 20×20, active = `accent.primary`,
  inactive = `text.muted`, hover-эффект.
- `thread_follow_handler` вынесен **до** `composer`/`chat` лисенеров (RPIT
  order fix — иначе E0700).

**`mod.rs`:**
- Поле `follow_enabled: bool` в `SidePanelLeft` (default `false`).
- `toggle_follow()`: инвертирует `follow_enabled`, обновляет глобал
  `AgentFollowState`; при выключении сбрасывает `last_tool`.
- Стриминговый хендлер `select_session` (ToolCall arm): при `follow_enabled`
  пушит `ToolCallPreview` в глобал + пытается `extract_file_path` →
  `PreviewTarget` (авто-открытие файла в Editor).

**`composer.rs`:**
- Та же логика в `send`-пути (composer streaming handler) — дублирование
  осознанное: два разных streaming-пути (select existing session vs send new
  message), оба должны feed'ить Follow.

### Правая панель — subscription (activity strip deferred)

**`view.rs`:**
- `AgentFollowState` регистрируется при старте (defensive default, как
  `PreviewTarget`).
- `_follow_subscription` — `cx.observe_global::<AgentFollowState>(|_, cx|
  cx.notify())`, зарезервирован для будущего activity strip UI.
- Activity strip UI **deferred** — RPIT-замыкания в `render()` борются с
  borrow checker'ом при построении элементов на данных глобала. Требует
  отдельного PR с рефакторингом рендера (извлечение данных до move-замыканий
  или AnyElement-фабрика).

### Регистрация

`lib.rs` + `main.rs` → `pub mod agent_follow` (lib + bin деревья модулей
независимы — нужен в обоих).

## Верификация

```
$ cargo check -p chronos
exit 0 (0 errors)

$ cargo test -p chronos --lib
test result: ok. 219 passed; 0 failed

$ cargo build --release -p chronos
exit 0
```

Новых юнит-тестов не писал — логика чисто glue (обновление глобала +
heuristic extract) без разветвлений; тестирование через `#[gpui::test]` с
mock `App` для одного `set_global`/`update_global` — overkill.

**Живой смок:** LIVE NOT VERIFIED. На этой сессии нет живого Hermes-сеанса
для реального прогона «Follow ON → agent edit → right shows file». Smoke
checklist для архитектора:

```bash
# 1. В левой панели — кнопка 👁 (активный accent.primary, неактивный muted)
# 2. Включить Follow 👁
# 3. Отправить агенту: «добавь комментарий в ~/.config/hypr/hyprland.conf»
# 4. Правая панель должна переключиться на Editor с открытым hyprland.conf
# 5. Выключить Follow 👁, повторить — правая панель НЕ прыгает
```

## Что НЕ сделано

- **Activity strip UI** — deferred (RPIT borrow checker, см. выше).
  Follow работает: toggle переключает состояние, streaming handler пушит
  tool calls в глобал, файлы авто-открываются через `PreviewTarget` (T194).
  Визуальный activity strip (tool name + status в thin bar справа) — отдельный
  PR.
- **Живой прогон через Hermes** — LIVE NOT VERIFIED.
- **Multi-agent registry** — вне scope T195.
- **Tool result streaming** (partial results) — не делал, `ToolCall` event
  приходит один раз с полными args/result (текущее поведение hermes_acp).

---

## Приёмка

| claim | check |
|---|---|
| Follow 👁 toggle в левой панели | ✅ panel.rs |
| Активное состояние: accent.primary | ✅ |
| Выключение сбрасывает last_tool | ✅ toggle_follow |
| ToolCall → AgentFollowState push | ✅ mod.rs + composer.rs |
| Auto-open файла через PreviewTarget | ✅ extract_file_path |
| view.rs subscription | ✅ (activity strip deferred) |
| lib + bin компиляция | ✅ |
| 219/219 тестов | ✅ |
| LIVE | **NOT VERIFIED** |

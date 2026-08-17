# T226 — Infrastructure for localization attempt #4 (2026-08-04)

> **ОТКЛОНЁН архитектором, 2026-08-04.** Отчёт честный (сам признаёт
> "живой прогон не сделан"), но "инфраструктура готова" преждевременно —
> живьём проверил (`chronos-ipc`, grim, лог `chronos.log`) и нашёл, что
> 2 из 3 фаз собственного скрипта локализации (`/tmp/t226-localize-4.sh`)
> гарантированно провалятся как написаны:
>
> 1. **Фаза 2 (`select-tab:terminal`) никогда не сработает.**
>    `WorkspaceMode::Developer` (`crates/app/src/side_panel_right/
>    tabs.rs:565-572`) не включает `PanelTab::Terminal` в rail вообще —
>    только `[System, Files, Preview, HyprlandBinds, AcpSettings,
>    EditorSettings]`. IPC парсит команду и на кадр создаёт вкладку, но
>    `resolve_active_tab` (`view.rs:160-171`) тут же откатывает на
>    System, потому что Terminal не входит в tab-сет текущего режима.
>    Живой лог: `side_panel_right: active tab not in mode set → System
>    was="Terminal"`. Это не баг IPC-механизма — неверная посылка о
>    существующих вкладках правого панеля. Если T226 нужен «терминал
>    в записи», это отдельный сюрфейс (`desktop_terminal`, background
>    layer), не вкладка правой панели.
> 2. **Фаза 3 сама себя ломает.** `preview-target:$FILE` корректно
>    открывает Editor (живой лог: `preview: loaded kind=Markdown
>    bytes=14`, `switched tab → opened at per-tab width tab="Editor"
>    width=560.0`) — это реально работает. Но следующий по скрипту
>    `select-tab:preview` попадает на уже-открытую вкладку и триггерит
>    toggle-в-rail (тот же механизм, что T221 «single affordance»):
>    живой лог `side_panel_right: same tab → collapsed to rail (memory
>    preserved)`. Скрипт схлопывает панель прямо перед записью клипа.
>
> **Что оставить как есть (не трогать):** IPC-плюмбинг (`expand-left`,
> `preview-target`, сам механизм `select-tab` для вкладок, которые
> реально есть в режиме) — работает, живьём подтверждено. Пункты
> 7-9 отчёта (rail/wallpaper/session-dot фиксы) — уже приняты и
> закоммичены архитектором отдельно, тут просто корректно
> задокументированы как «попутно».
>
> **Правки перед attempt #5:** убрать фазу 2 из скрипта (заменить целью
> на реальную вкладку типа `preview`/`files`, либо явно нацелиться на
> `desktop_terminal`) и убрать лишний `select-tab:preview` после
> `preview-target` в фазе 3 (preview-target уже переключает и
> открывает — второй вызов не нужен, только вредит).

**Status:** инфраструктура готова, бинарь собран, скрипт написан, локализация не проведена.

## Что сделано

### 1. IPC-команды для программного управления панелями (T230 task B + T226 tooling)

Три новых IPC-команды в `crates/app/src/ipc/messages.rs` + `service.rs` + `mod.rs`:

| Команда | Файлы | Что делает |
|---------|-------|------------|
| `expand-left` | `messages.rs:54`, `mod.rs:160-176` | Открывает левую панель, докает чат, фокусирует композер |
| `select-tab:<id>` | `messages.rs:48`, `mod.rs:163-173` | Переключает правую панель на вкладку по id (terminal/preview/…) |
| `preview-target:<path>` | `messages.rs:51`, `mod.rs:179-189` | Устанавливает `PreviewTarget` global, переключает на Editor |

Диспатч в `accept_loop` (`service.rs:207-272`), приём в `ipc/mod.rs` с дебаунсом (100ms для select-tab, 200ms для expand-left).

### 2. `expand_with_composer()` — левая панель

`crates/app/src/side_panel_left/mod.rs:1251-1270` — открывает панель (`open_pinned`), докает чат (`dock_chat = true`), фокусирует композер (`window.focus(&this.composer_focus, cx)`), запускает blink-курсор.

### 3. `select_tab()` + `preview_target()` — правая панель

`crates/app/src/side_panel_right/mod.rs:350-417`:
- `select_tab(tab, cx)` — открывает панель если закрыта, вызывает `on_tab_select`, деферит фокус на 50ms через `active_tab_focus`
- `preview_target(path, cx)` — открывает панель, устанавливает `PreviewTarget` global, `PreviewIntent::Edit` (файл открывается в Edit mode → создаётся `InputState` → фокус работает)

### 4. `active_tab_focus()` — фокус для Editor

`crates/app/src/side_panel_right/view.rs:453-465`:
- `TabContent::Terminal` → `TerminalTab::focus_handle` (было)
- `TabContent::Preview` → `PreviewTab::editor_focus_handle` (добавлено)

`crates/app/src/side_panel_right/tab/preview.rs:614-623`:
- Новый метод `editor_focus_handle(&self, cx: &App) -> Option<FocusHandle>` — возвращает фокус от `InputState` (gpui-component), если editor создан (Edit mode)
- Добавлен импорт `gpui::Focusable`

### 5. `PreviewIntent::Edit` вместо `View`

`crates/app/src/side_panel_right/mod.rs:410` — `preview_target()` теперь ставит `PreviewIntent::Edit`. Это значит файл открывается в Edit mode сразу, `InputState` материализуется при первом рендере, `active_tab_focus` возвращает валидный хендл. Для нередактируемых файлов (картинки, truncated) `resolve_view_mode` принудительно возвращает `View`.

### 6. Скрипт локализации #4

`/tmp/t226-localize-4.sh` — три фазы:
1. **Composer** (левая панель): `expand-left` → `wf-recorder` + `wtype` печатает `123abc;abc123;1a2b3c` на EN и RU
2. **Terminal** (правая панель): `select-tab:terminal` → фокус через 50ms → запись
3. **Editor** (правая панель): `preview-target:/tmp/t226-test-file.md` + `select-tab:preview` → файл в Edit mode → фокус → запись с видимым гуттером

Тестовый файл: `/tmp/t226-test-file.md` (42 строки, markdown с code-блоком).

### 7. Попутно: wallpaper restore после ребута

`crates/services/src/wallpaper/mod.rs:75-91` — если демон awww запущен но без загруженного изображения (свежий процесс после ребута), вызывается `awww restore` для восстановления из кэша `~/.cache/awww/`.

### 8. Попутно: фикс header/thread_column в левой панели (T230-errata)

`crates/app/src/side_panel_left/panel.rs` — header перенесён внутрь `thread_column_with_header`, больше не sibling `clipped_content`. Устраняет reflow рейла при ресайзе через порог `chat_open`.

### 9. Попутно: кликабельные session-dot в рейле

`crates/app/src/side_panel_left/panel.rs:565-570` — неактивные dot'ы сессий теперь имеют `on_click` → `select_session`.

## Чем доказано

- `cargo check` — зелёный
- `cargo test -p chronos --lib side_panel_right` — 161/161 passed
- `cargo build --release` — собран (26 MB, stripped)

## Что НЕ сделано

- **Локализация бага T226 не проведена.** Инфраструктура готова, бинарь собран, скрипт написан. Нужен рестарт ChronOS с новым бинарём → запуск скрипта → анализ клипов.
- **Живой прогон (grim/wf-recorder) не сделан.** Все три фазы проверены только статически (компиляция + тесты).

## Файлы

| Файл | Что менялось |
|------|-------------|
| `crates/app/src/ipc/messages.rs` | +80 строк: SELECT_TAB_PREFIX, PREVIEW_TARGET_PREFIX, EXPAND_LEFT_PAYLOAD, classify/parse/encode |
| `crates/app/src/ipc/service.rs` | +12 строк: три новых канала (select_tab, preview_target, expand_left), диспатч в accept_loop |
| `crates/app/src/ipc/mod.rs` | +50 строк: приём трёх новых каналов с дебаунсом |
| `crates/app/src/side_panel_left/mod.rs` | +22 строки: `expand_with_composer()` |
| `crates/app/src/side_panel_left/panel.rs` | ~30 строк: header внутри thread_column, кликабельные dot'ы |
| `crates/app/src/side_panel_right/mod.rs` | +70 строк: `select_tab()`, `preview_target()`, `view` поле в State, `PreviewIntent::Edit` |
| `crates/app/src/side_panel_right/view.rs` | +3 строки: `TabContent::Preview` в `active_tab_focus` |
| `crates/app/src/side_panel_right/tab/preview.rs` | +7 строк: `editor_focus_handle()`, импорт `Focusable` |
| `crates/services/src/wallpaper/mod.rs` | +22 строки: `awww restore` fallback |

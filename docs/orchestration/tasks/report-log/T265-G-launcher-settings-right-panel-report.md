# T265-G — Launcher settings page in the right panel — Report

**Date:** 2026-08-16
**Role:** FRONTEND + persist. Zone: `launcher/**` (watcher + hot-apply) +
`side_panel_right/{tabs,tabs/tab/mod,tabs/tab/launcher_settings,view}.rs`.
**Commit:** `35fbb035` `feat(launcher): settings page in right panel (T265-G)`.

**Приёмка (архитектор, 2026-08-16):** код + юниты **приняты**. Сверил
дерево и прогнал сам: launcher 83/83, side_panel_right 198/198, `--lib`
563/563. Live grim — долг. Hidden-поиск и «скрыть категорию» из UI —
не блокеры.

## Status

**Done (code). В этот раз дерево НЕ было заблокировано чужим WIP —**
параллельная T287-C (left-panel) закоммитилась прямо во время сессии
(`220a05e2`), поэтому полная верификация прогонялась на чистом дереве.

## Что сделано

### `side_panel_right/tab/launcher_settings.rs` — новая страница (7 групп)

Живая вьюха (не `EmptyTab`), карточка `ui::elevated_card` + `section_header` /
`setting_row` / `setting_label` как `bar_settings`/`acp_settings`:

1. **Appearance** — `[appearance] compact_default` / `hide_labels` (Switch).
2. **Grid** — `[grid] columns` (1..12), `rows` (1..10), `icon_size` (16..64px)
   — три слайдера через `bar_settings::slider_control` (свой слайдер не писал).
3. **Search** — `[search] include_hidden` / `inline_completion` (Switch).
4. **Categories** — `[categories] hide` список + «Show» (убрать из hide);
   приписка «Empty categories are hidden automatically» (дефолт B).
5. **Favorites** — `[favorites] sort_alpha` / `hide_labels` (Switch).
6. **System actions** — `[system_actions] order` порядок с ↑/↓ (reorder) +
   «Reset to default» (чистит order → дефолт F).
7. **Hidden apps** — `[hidden]` список id→имя + «Unhide».

Все контролы пишут через `launcher_config::update` (RMW `toml::Value`, не
serde-дамп вслепую) — открытый OSD подхватывает по тому же `subscribe()`.

### `launcher_config.rs` — новые ключи + watcher

- `AppearanceConfig` / `GridConfig` / `SearchConfig` / `CategoriesConfig` +
  поля в `LauncherConfig`. `GridConfig::sanitized()` клампит мусор
  (columns 1..=12, rows 1..=10, icon 16..=64) — `move_2d` с columns=0 не
  делит на ноль. **Каждый ключ имеет читателя** (view.rs / страница) — мёртвых
  ключей нет.
- `reload()` + `spawn_watcher(cx)` — inotify на `~/.config/chronos/` по
  basename `launcher.toml`, 300ms debounce, паттерн frame/bar. Зовётся из
  `launcher::init`.
- `icon_size` — **`u32`**, не `f32` (пиксели целые; `f32` рвал `Eq`-derive
  на `LauncherConfig`).

### `launcher/view.rs` — hot-apply + tune

- `apply_config_derived()`: columns/page_rows/grid_icon/hide_labels/
  include_hidden/inline_completion/hidden_categories перечитываются на
  каждый мутейт (свой edit или file-watcher) — сетка/поиск/секции меняются
  без рестарта.
- `include_hidden` реально включает user-hidden в выдачу; `hide_labels`
  прячет подписи ячеек; `hidden_categories` режет бар; `inline_completion`
  гейтит ghost-хвост.
- **Tune вернулся в футер** (T246 «нет контрола без бэкенда»): kit `Button`
  → `launcher::close` + `select_tab(PanelTab::LauncherSettings)` (тот же
  путь, что IPC `select-tab` и колокольчик T293).

### `tabs.rs` / `tab/mod.rs` / `view.rs` (right panel)

`PanelTab::LauncherSettings` (enum, `ALL` 20→21, `id`/`parse_id`/`label`
«Launcher»/`icon_path` sigil/`preferred_content_width` 410), `TabContent`
variant + `create` arm + `placeholder_description` arm, render/tab_entity_id
arms. **В `for_mode` не добавлял** — вкладка открывается tune-кнопкой/
`select-tab` (спека: «клик открывает эту вкладку через IPC select-tab»).

### `system_actions.rs`

`action_id(PowerAction) -> &'static str` (инверсия `parse_action`) — нужен
reorder'у, чтобы писать канонические id обратно в `order`.

## Verification

| Command | Result |
|---|---|
| `cargo test -p chronos --lib launcher` | **83 passed; 0 failed** |
| `cargo test -p chronos --lib side_panel_right` | **198 passed; 0 failed** |
| `cargo test -p chronos --lib` | **563 passed; 0 failed** |
| `cargo build --release -p chronos` | **чисто, 5m18s** (76 warnings — все чужие/pre-existing) |

Юниты спеки на месте (pure, без GPUI):
- sanitize мусора в toml → `launcher_config::tests::garbage_grid_values_sanitize_to_defaults`
  (columns=0→1, rows=999→10, icon=3→16);
- hidden unhide вычёркивает id → `launcher_settings::tests::unhide_removes_the_id`;
- reorder/кламп → `move_action_reorders_and_clamps`;
- `PanelTab::ALL.len() == 21` + `ALL[20] == LauncherSettings` (обновлён
  coverage-тест), icon/label/placeholder-уникальность зелёные.

## Что НЕ сделано (Архитектор / дальше)

1. **Live grim** (спека): вкладка открывается; ползунок columns сразу меняет
   сетку открытого OSD; Unhide возвращает приложение; tune в футере ведёт
   сюда. Требует живого шелла + кадра.
2. **Перенос в `done/` + статус родителя T265** — самоприём не делаю.

## Отчёт одной строкой (выборы из спеки)

- Слайдеры — **`bar_settings::slider_control`** (сделал `pub(crate)`), не свой.
- Toggle — **`gpui_component::Switch`**. Kit `Select`/`VirtualList` не
  понадобились (страница — слайдеры+свитчи+списки, сетку не виртуализую).
- Watcher — **inotify 300ms**, паттерн frame/bar.
- `icon_size` — **`u32`** (не `f32`; пиксели целые, сохраняет `Eq`).

Одна ловушка по пути (не в спеке, но пригодится следующему): хендлеры
`on_click`/`slider_control` в этом форке требуют higher-ranked замыкания —
обычное `let h = move |_e,_w,_cx| {...}` даёт «implementation of `Fn` is not
general enough» (фиксированная лайфтайм-сигнатура). Лечится инлайном в вызов
или `cx.listener(...)`. Я инлайнил все хендлеры страницы.

## Коммит

```
feat(launcher): settings page in right panel (T265-G)
```

(9 files: `launcher/{launcher_config,mod,system_actions,view}.rs`,
`side_panel_right/{tabs,view}.rs`, `side_panel_right/tab/{mod,bar_settings,
launcher_settings}.rs`. `Cargo.lock`, `Source/gpui/` — не тронуты.)

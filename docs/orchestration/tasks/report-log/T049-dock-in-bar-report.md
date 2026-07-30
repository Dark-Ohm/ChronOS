<!-- T049 — migrated 2026-07-22 from docs/orchestration/report-log/mimo-report-8.md — see docs/orchestration/tasks/MIGRATION.md -->

# MIMO — Задание №8: dock → bar widget + кнопка «Пуск»

**Дата:** 2026-07-19
**Коммит:** (ожидает git add)

## Что сделано

### 1. `bar/widgets/dock.rs` — новый stateless BarWidget
- `DockWidget` implements `BarWidget` (BarSection::Left)
- `render()` — читает `config::cached()` + `AppState::applications(cx).get()` на каждый рендер (без disk I/O)
- Кнопка «Пуск» — первая иконка, hexagon glyph `⏻`, `on_click` → `crate::launcher::toggle(cx)`
- Divider после «Пуск»
- App icons — из `build_dock_icons()`, `on_click` → `launch()`, `on_mouse_down(Right)` → `context_menu::open()`
- Fallback-буква вместо пустоты для нерезолвленных иконок
- Icon resolution: перенесён из `dock/view.rs` целиком (resolve_icon, theme_chain, gtk icon theme walk)
- `register(cx)` — инициализирует глобалы (DockMenuState, DockConfigSignal), загружает кэш конфига, регистрирует виджет

### 2. `dock/config.rs` — глобальный кэш конфига
- `CONFIG_CACHE: OnceLock<Mutex<DockConfig>>` — загружается ОДИН РАЗ при init
- `cached()` — наносекундное чтение из памяти (без disk I/O на каждый тик бара)
- `reload_cache()` — перезагрузка с диска
- `update_cache(config)` — обновление после unpin (вызывается из context_menu)

### 3. `dock/context_menu.rs` — миграция
- `Anchor::BOTTOM` → `Anchor::TOP` (попап выпадает ИЗ бара вниз)
- `MENU_MARGIN_BOTTOM` → `MENU_MARGIN_TOP: 36.` (bar height + gap)
- После unpin: `config::update_cache(config)` вместо прямого обращения к bar widget

### 4. `dock/mod.rs` — убран оконный lifecycle
- Удалены: `init()`, `open_on_display()`, `window_options()`, `DOCK_HEIGHT`, все layer-shell импорты, `mod view`
- Осталось: `pub mod config; pub mod context_menu; pub mod signal;`

### 5. `dock/view.rs` — удалён
- Весь код перенесён в `bar/widgets/dock.rs` (build_dock_icons, resolve_icon, theme_chain, etc.)
- Тесты перенесены в `bar/widgets/dock.rs`

### 6. `main.rs` — удалена строка `dock::init(cx)`
- Дока теперь инициализируется через `bar::init` → `widgets::register_builtin` → `dock::register`

### 7. `bar/widgets/mod.rs` — регистрация
- `mod dock;` + `dock::register(cx);` (prepend перед clock)

## Что подтверждено деревом

- `cargo check -p chronos` — 0 errors
- `cargo build --release -p chronos` — зелёный
- `dock/view.rs` — удалён (git rm)
- `dock/mod.rs` — без оконного lifecycle, без `init()`
- `main.rs` — без `dock::init(cx)`
- Context menu: `Anchor::TOP`, `MENU_MARGIN_TOP: 36.`
- Кэш конфига: `config::cached()` в render(), `config::update_cache()` в unpin handler
- Иконки: `build_dock_icons()` + `resolve_icon()` перенесены в bar widget
- Зоны не нарушены: не тронуты `bar/mod.rs`, `launcher/`, `services/`, другие виджеты

## Дерево файлов

```
crates/app/src/bar/widgets/dock.rs    (новый) — BarWidget, Start button, icon resolution
crates/app/src/bar/widgets/mod.rs     (изменён) — +mod dock, +dock::register
crates/app/src/dock/config.rs         (изменён) — +CONFIG_CACHE, cached/reload_cache/update_cache
crates/app/src/dock/context_menu.rs   (изменён) — Anchor::TOP, config::update_cache
crates/app/src/dock/mod.rs            (изменён) — убран оконный lifecycle
crates/app/src/dock/view.rs           (удалён) — код перенесён в bar widget
crates/app/src/main.rs                (изменён) — удалена строка dock::init(cx)
```

## Живой смок

Не проведён (нет Hyprland). Приёмщик проверяет:
1. `hyprctl layers -j` — namespace `dock` отсутствует (окно дока больше нет)
2. Иконки (включая «Пуск» первой) — в левом кластере бара
3. Клик по «Пуск» → launcher открывается
4. Клик по app-иконке → приложение запускается
5. Правый клик → «Unpin» → меню ПОД иконкой (Anchor::TOP)
6. Unpin работает + персистится через рестарт (`cat dock.toml`, `pkill -x chronos` + рестарт)

## Follow-up
- Внешнее редактирование `dock.toml` не инвалидирует кэш (только unpin) — можно добавить inotify
- `DockView` (old entity) удалён — `DockWidget` полностью замещает

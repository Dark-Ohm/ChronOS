<!-- T032 — migrated 2026-07-22 from docs/orchestration/report-log/mimo-report-6.md — see docs/orchestration/tasks/MIGRATION.md -->

# MIMO — отчёт: задание №6 dock (закреплённая панель запуска)

**Статус: ВЫПОЛНЕНО**
**Коммит: `d646406`** — `dock : закреплённая панель запуска`

## Что сделано

### dock/mod.rs
- Layer-shell surface, `Anchor::BOTTOM`, `Layer::Top`, `KeyboardInteractivity::None`
- Окно по центру屏幕а, ширина 400px, высота 56px
- `exclusive_zone: Some(56)` — резервирует место под док
- `init()` — открывает одно окно на каждый дисплей ( образец bar/mod.rs)
- Задержка 150ms перед открытием для Wayland display enumeration

### dock/view.rs
- `DockView` — GPUI view, horizontal row of pinned icons
- Pinned list: `["kitty", "thunar", "firefox", "code", "vivaldi"]` — заглушка, персистентный конфиг отдельная задача
- `build_dock_icons(&[AppEntry])` — фильтрует по PINNED_IDS, резолвит иконки
- `resolve_icon()` — кэшированный резолвер имя→путь (паттерн tray.rs, собственная копия):
  - GTK icon theme chain (index.theme Inherits=)
  - Поиск в `/usr/share/icons`, `~/.local/share/icons`, `~/.icons`
  - Sizes: 48x48, 64x64, 32x32, 256x256; contexts: apps, categories, devices, mimetypes
  - Fallback: первая буква имени на фоне elevated bg
- `on_click` → `launch(&entry.exec)` + `window.remove_window()`
- Подписка через `state::watch()` — live обновления при установке/удалении пакетов

### main.rs
- `mod dock;` + `dock::init(cx);` (2 строки, после bar::init)

## Верификация

### cargo test --workspace --lib --bins
```
test result: ok. 4 passed
test result: ok. 65 passed
test result: ok. 25 passed
test result: ok. 80 passed  ← (включая dock view tests)
test result: ok. 3 passed
```
**Итого: 177 тестов, все зелёные.**

### Что подтверждено деревом
- `dock/mod.rs:36-60` — window_options: BOTTOM anchor, 56px height, Layer::Top, exclusive_zone
- `dock/mod.rs:62-71` — open_on_display: cx.open_window + cx.new(DockView::new)
- `dock/mod.rs:74-90` — init: spawn + displays loop + open_on_display
- `dock/view.rs:52-67` — DockView::new: get entries → build_dock_icons → subscribe
- `dock/view.rs:70-131` — render: div().id() + flex + icons + on_click → launch + remove_window
- `dock/view.rs:143-157` — build_dock_icons: PINNED_IDS filter + resolve_icon
- `dock/view.rs:162-218` — resolve_icon: cache + theme chain + size/context/ext loops
- `main.rs:56` — `dock::init(cx);` после bar::init

## Замечание
- Pinned-список — заглушка (hardcoded в `PINNED_IDS`). Персистентный конфиг — отдельная задача (follow-up).
- Иконки кэшируются в `OnceLock<Mutex<HashMap>>` — не аллоцируются в render().
- Док НЕ имеет toggle/скрытие по IPC — всегда видим (как в задании).
- Fallback-иконка (первая буква) используется если резолвер не находит иконку — не пустота.

## Зоны (соблюдены)
- Тронуто: `crates/app/src/dock/` (новый), `crates/app/src/main.rs` (+2 строки)
- НЕ трогать: `services/**`, `bar/**`, `tray_menu/`, `osd/`, `launcher/**` (кроме ВЫЗОВА launch()), `ipc/`, `wallpaper_ctl.rs`, `Source/`

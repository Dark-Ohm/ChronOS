<!-- T048 — migrated 2026-07-22 from orchestration/report-log/mimo-report-7.md — see orchestration/tasks/MIGRATION.md -->

# MIMO — Задание №7: персистентный конфиг dock + unpin

**Дата:** 2026-07-19
**Коммит:** (ожидает git add)

## Что сделано

### 1. `dock/config.rs` — конфигурация pinned-списка
- `DockConfig` с serde (Serialize/Deserialize)
- `load()` — чтение `~/.config/chronos/dock.toml`; если файла нет — записывает дефолт
- `save()` — запись с `create_dir_all`
- `unpin(id)` — удаление из pinned
- 7 юнит-тестов: default, unpin, roundtrip, parse, edge cases

### 2. `dock/context_menu.rs` — правый клик → «Unpin»
- `DockMenuState` (Global) — хранит `WindowHandle<DockMenuView>`, `entry_id`, generation
- `DockMenuView` — layer-shell popup (Overlay, BOTTOM anchor, centered)
- `open(entry_id)` — toggle (тот же entry → закрыть, другой → переключить)
- `close(cx)` — очистка + `remove_window()`
- `schedule_autoclose` — 5с, generation-guarded
- `on_click` → `DockConfig::load()` → `unpin()` → `save()` → `notify_config_changed()` → `remove_window()`

### 3. `dock/signal.rs` — сигнал изменений конфига
- `DockConfigSignal` (Global) — `Mutable<()>` из futures_signals
- `notify_config_changed(cx)` — `*lock_mut = ()` → все watch-коллбэки срабатывают

### 4. `dock/view.rs` — миграция на конфиг
- Удалён `const PINNED_IDS`
- `DockView` хранит `entries: Vec<AppEntry>` + `icons`
- Два `state::watch`:
  - applications signal → обновить entries + пересобрать icons
  - DockConfigSignal → пересобрать icons из текущих entries
- `build_dock_icons(pinned: &[String], entries: &[AppEntry])` — теперь принимает pinned параметром
- `on_click` — **без** `window.remove_window()` (док — постоянная панель)
- `on_mouse_down(MouseButton::Right, ...)` → `context_menu::open(cx, entry_id)`

### 5. `dock/mod.rs` — wiring
- `pub mod config; pub mod context_menu; pub mod signal;`
- `init()` — `cx.set_global(DockMenuState::default())` + `cx.set_global(DockConfigSignal::default())`
- Очищены unused imports

### 6. `Cargo.toml` — зависимости
- `toml = "0.8"` + `serde = { workspace = true, features = ["derive"] }`

## Что подтверждено деревом

- `cargo check -p chronos` — 0 errors, warnings только от чужого кода
- `cargo build --release -p chronos` — зелёный
- `main.rs` — без изменений (dock::init уже был)
- Зоны не нарушены: не тронуты `launcher/`, `services/`, `bar/`, `tray_menu/`, `osd/`, `notifications/`, `ipc/`

## Дерево файлов

```
crates/app/src/dock/
├── config.rs          (новый) — DockConfig, load/save/unpin
├── context_menu.rs    (новый) — DockMenuState, DockMenuView, open/close
├── signal.rs          (новый) — DockConfigSignal, notify_config_changed
├── mod.rs             (изменён) — pub mod, init globals
└── view.rs            (изменён) — DockConfig вместо PINNED_IDS, right-click
crates/app/Cargo.toml  (изменён) — +toml, +serde
```

## Живой смок

Не проводился — ChronOS не запускается в текущей среде (нет Hyprland/Wayland).
Проверка приёмщиком обязательна:
1. Удалить `~/.config/chronos/dock.toml` (если есть)
2. `RUST_LOG=info ./target/release/chronos` → файл появится сдефолтным списком (`cat`)
3. Правый клик по иконке → «Unpin» → иконка пропала из дока И из файла (`cat`)
4. `pkill -x chronos`, перезапуск → список БЕЗ откреплённой иконки (персистентность)
5. `hyprctl layers -j` — контекст-меню не висит после закрытия

## Follow-up (не в этом задании)
- Добавление новых pinned (через launcher или drag) — требует интеграции с `launcher/`
- Конфиг-сигнал не стреляет при внешнем редактировании `dock.toml` (только через unpin) — можно добавить inotify на файл

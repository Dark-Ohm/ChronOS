<!-- T058 — migrated 2026-07-22 from orchestration/report-log/mimo-report-10.md — see orchestration/tasks/MIGRATION.md -->

# MIMO — Задание №10: chrome на один пультовый монитор

**Дата:** 2026-07-20
**Коммит:** (ожидает git add)

## Что сделано

### 1. `monitor.rs` — модуль выбора пультового монитора
- `MonitorConfig` (serde) — `~/.config/chronos/monitor.toml` с полем `chrome_monitor: Option<String>` (uuid)
- `pult_display(cx) -> Option<DisplayId>`:
  1. UUID из конфига матчит живой дисплей → используем его
  2. Fallback: самый большой дисплей по площади
  3. Fallback: первый дисплей
  4. None только если дисплеев нет
- Авто-дизейн: при fallback пишет uuid победившего дисплея в конфиг
- `lib.rs` — `pub mod monitor;` (доступен из examples/tests)

### 2. `bar/mod.rs` — бар только на пультовом
- Вместо цикла `for d in displays` → `open_on_display(Some(pult_display(cx)))`
- Один бар на одном мониторе

### 3. Все chrome-попапы → `pult_display`
Обновлены 8 `pick_display` функций (все `primary_display().or_else(first)` → `crate::monitor::pult_display(cx)`):
- `tray_menu/mod.rs`
- `volume_popup/mod.rs`
- `updates_popup/mod.rs`
- `notifications/mod.rs`
- `notifications/history_popup/mod.rs`
- `osd/mod.rs`
- `dock/context_menu.rs`
- `system_popup/mod.rs` (+ удалён `window.display(cx)` из `toggle`)

### 4. `launcher/mod.rs` — тоже на пультовом
- `primary_display().or_else(first)` → `crate::monitor::pult_display(cx)`

### 5. `main.rs` — добавлен `mod monitor;`
### 6. `Cargo.toml` — добавлена зависимость `uuid = "1"`

## Что НЕ тронуто
- `desktop_terminal` — desktop-canvas виджет, не chrome (по брифу)

## Что подтверждено деревом
- `cargo check -p chronos` — 0 errors
- `cargo build --release -p chronos` — зелёный
- `monitor.rs` — `pult_display` с uuid matching + fallback + auto-designate
- Bar init — одиночный `open_on_display(Some(pult_display(cx)))`
- Все 8 pick_display → `crate::monitor::pult_display(cx)`
- `system_popup::toggle` — `window.display(cx)` удалён
- `launcher::open` — `pult_display(cx)` вместо primary_display

## Дерево файлов

```
crates/app/src/monitor.rs          (новый) — pult_display, MonitorConfig
crates/app/src/lib.rs              (изменён) — +pub mod monitor
crates/app/src/main.rs             (изменён) — +mod monitor
crates/app/Cargo.toml              (изменён) — +uuid = "1"
crates/app/src/bar/mod.rs          (изменён) — bar только на pult_display
crates/app/src/tray_menu/mod.rs    (изменён) — pick_display → pult_display
crates/app/src/volume_popup/mod.rs (изменён) — pick_display → pult_display
crates/app/src/updates_popup/mod.rs(изменён) — pick_display → pult_display
crates/app/src/notifications/mod.rs(изменён) — pick_display → pult_display
crates/app/src/notifications/history_popup/mod.rs (изменён) — pick_display → pult_display
crates/app/src/osd/mod.rs          (изменён) — pick_display → pult_display
crates/app/src/dock/context_menu.rs(изменён) — pick_display → pult_display
crates/app/src/system_popup/mod.rs (изменён) — pick_display → pult_display, -window.display
crates/app/src/launcher/mod.rs     (изменён) — display_id → pult_display
```

## Живой смок

Не проведён (нет Hyprland). Приёмщик проверяет:
1. Бар виден ТОЛЬКО на DP-1 (Samsung 2560×1440), НЕТ на HDMI-A-1
2. `hyprctl layers -j` → namespace бара на DP-1, нет на HDMI
3. Любой bar-попап → открывается на DP-1
4. `~/.config/chronos/monitor.toml` создан с uuid DP-1
5. Лог без panic

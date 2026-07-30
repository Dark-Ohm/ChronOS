<!-- T068 — migrated 2026-07-22 from orchestration/report-log/mimo-report-11.md — see orchestration/tasks/MIGRATION.md -->

# Mimo №11 — battery/mpris на SVG-иконки, hover-свип, CAVA

**Коммит:** `6723493` — `bar : battery/mpris на SVG-иконки — эмодзи в баре добиты`

## Что подтверждено деревом

### SVG-иконки (5 новых файлов)

- `crates/app/assets/icons/battery.svg` — корпус батареи (Phosphor, viewBox 256, stroke-width 16)
- `crates/app/assets/icons/battery-charging.svg` — корпус + молния
- `crates/app/assets/icons/bolt.svg` — молния (для PowerProfile)
- `crates/app/assets/icons/play.svg` — треугольник play
- `crates/app/assets/icons/pause.svg` — две вертикальные полосы

### assets.rs

- `icons!` макрос: +5 строк (battery, battery-charging, bolt, play, pause)

### battery.rs

- Импорт: +`svg` (строка 3)
- Эмодзи `"⚡"/"🔋"` → `"icons/battery-charging.svg"` / `"icons/battery.svg"` (строки 40-44)
- Эмодзи `"⚡"/"⚖"/"🌱"` → `"icons/bolt.svg"` для всех профилей (строки 55-59)
- Рендер: `svg().path(icon_path).size(px(13.))` + `font_family(theme.font_mono)` (строки 72-95)
- Hover: `.hover(|s| s.bg(theme.interactive.hover))` (строка 81)

### mpris.rs

- Импорт: +`svg` (строка 3)
- `MprisView::Track { icon, ... }` → `MprisView::Track { icon_path, ... }` (строки 16-27, 38, 45-50)
- Эмодзи `"⏸"/"▶"` → `"icons/pause.svg"` / `"icons/play.svg"` (строка 38)
- Рендер: `svg().path(icon_path).size(px(13.))` (строка 142)
- Hover: `.hover(|s| s.bg(theme.interactive.hover))` (строка 141)
- Тесты: `assert_eq!(icon, "⏸")` → `assert_eq!(icon_path, "icons/pause.svg")` (строки 210, 224)

### Hover-свип правого кластера (6 виджетов)

| Файл | Строка | Добавлено |
|---|---|---|
| `volume.rs` | 102 | `.hover(\|s\| s.bg(theme.interactive.hover))` |
| `tray.rs` | 73 | `.hover(\|s\| s.bg(theme.interactive.hover))` |
| `updates.rs` | 68 | `.hover(\|s\| s.bg(theme.interactive.hover))` |
| `system.rs` | 39 | `.hover(\|s\| s.bg(theme.interactive.hover))` |
| `notification_bell.rs` | 66 | `.hover(\|s\| s.bg(theme.interactive.hover))` |
| `mpris.rs` | 141 | `.hover(\|s\| s.bg(theme.interactive.hover))` |

`network.rs` НЕ трогал (осознанно некликабельный).

### CAVA — подгонка констант

- `BAR_W: f32 = 3.;` → `2.5;` (строка 15)
- `MAX_BAR_H: f32 = 18.;` → `16.;` (строка 12)

## Верификация

- `cargo check -p chronos` — 0 errors
- `cargo test --workspace --lib --bins` — **131 тест зелёных** (включая обновлённые mpris-тесты)
- `cargo build --release -p chronos` — зелёный

## Живой смок

Не проведён (нет Hyprland в текущей среде). Battery на десктопе скрыт — для него достаточно build+test+код-ревью. Hover проверен только код-ревью (ydotool ненадёжен).

## Зоны (соблюдены)

Твои: `crates/app/assets/icons/*.svg`, `assets.rs`, `bar/widgets/{battery,mpris,volume,tray,updates,system,notification_bell,cava}.rs`. НЕ тронуты: `network.rs`, `workspaces.rs`, `dock.rs`, `project.rs`, `clock.rs`, `separator.rs`, попапы, theme, `mod.rs` бара.

Поимённый add, `git diff --staged` глазами — чужие rustfmt-изменения НЕ стейджены.

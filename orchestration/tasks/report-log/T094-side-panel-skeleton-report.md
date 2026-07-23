<!-- T094 — migrated 2026-07-22 from orchestration/report-log/hermes-report-7.md — see orchestration/tasks/MIGRATION.md -->

# Session: side_panel_right Task 7 — оконный скелет — 2026-07-21

## Сделано (факт, не намерение)

- `crates/app/src/side_panel_right/mod.rs` — lifecycle: `init`, `open_pinned`,
  `close` (match+warn на Err, не `let _ =`), `close_this` (direct
  `remove_window`, reentrancy guard), `toggle`. Layer-shell Overlay,
  anchor `TOP|BOTTOM|RIGHT`, width 300, height 0 (компоузитор тянет
  TOP|BOTTOM), `KeyboardInteractivity::None`, namespace `side_panel_right`,
  app_id `chronos-side-panel-right`, display via `monitor::pult_display`.
- `crates/app/src/side_panel_right/view.rs` — пустая оболочка: `bg.secondary`,
  `border_l_1` + `border.default`, `text.primary`, placeholder-текст
  (tasks 9–11 заполнят).
- `crates/app/src/main.rs` — `mod side_panel_right;` + `side_panel_right::init(cx);`
  (НЕ `lib.rs` — правка плана из брифа).
- Scratch smoke-hook (`CHRONOS_SMOKE_SIDE_PANEL`) **вырезан** перед коммитом —
  в product-коде только `set_global`.
- Коммит: `app : side_panel_right — оконный скелет (layer-shell overlay, toggle/close_this)`
  (3 файла, поимённый add).

## Расхождения со спекой/планом

- План Step 1/4: `pub mod` в `lib.rs` → **исправлено брифом**: `mod` в
  `main.rs` (app = bin). Сделано по брифу.
- `LayerShellOptions`: `..Default::default()` (поле `exclusive_edge`), как
  у `system_popup`.
- View: `Theme::global(cx)` + явный `text_color(theme.text.primary)`.
- Live-смок: env-hook на время проверки (не в коммите). Постоянного
  Hyprland-keybind / IPC нет (Task 9).

## Не реализовано из acceptance criteria

- Hover-peek / bar-click / content (Tasks 8–11) — не в scope Task 7.
- Постоянная keybind / bar wiring — не в scope.

## Проверено фактом, не на словах

```
cargo build --release -p chronos
# → Finished `release` profile … EXIT 0

cargo test -p chronos --lib --bins
# → 120 passed; 0 failed

# Live smoke (release, temporary env open/close — NOT in commit):
# hyprctl layers OPEN:
#   xywh: 2260 30 300 1410, namespace: side_panel_right, pid: <chronos>
#   → 300px, y=30 (под баром), height 1410 = 1440−30, правый край pult ✓
# CLOSE +0.5s / +2.5s: residual layers side_panel = none; clients = 0
# log: opened pinned → closed (no ghost/remove_window warn)
# grim: orchestration/reports/hermes-7-smoke/open-crop.png — skeleton text

git show --stat HEAD
# 3 files: main.rs + side_panel_right/{mod,view}.rs
# no theme/**, no services/**, no smoke env
```

## Новые риски / известные баги

- **Severity none observed:** TOP|BOTTOM anchor + height 0 → компоузитор
  отдал 1410px; exclusive_zone 0 не сдвинул бар.
- **Severity process:** toggle пока не привязан к бару/IPC — панель
  «мёртвая» без следующего task (ожидаемо).
- **Severity low (archived):** первый poll после close мог показать
  `pid: -1` на layer; повтор +0.5s — чисто 0.

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлялись (скелет; канон в плане/спеке правой панели).

## Зона

Только: `side_panel_right/**` + 2 строки `main.rs`. Чужой WIP
(`theme/font_ui`, `services/audio/**`) в дереве **не** стейджился.

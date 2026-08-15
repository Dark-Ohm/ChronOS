**Принято архитектором 2026-08-16.** Live `+` владельца.

# T292 — workspace mode toggle moves off the bar — Report

**Date:** 2026-08-16
**Role:** FRONTEND.
**Zone:** `side_panel_right/rail.rs`, `workspace_mode.rs` (`icon_path` only),
`bar/widgets/` (delete), `bar/layout_config.rs`, `bar/mod.rs`, `assets.rs`,
`icons/gamepad.svg`, `icons/mode-daily.svg`.
**IPC** `set-workspace-mode` / `toggle-workspace-mode` — не тронут.

## Status

**Done.** Код не был в git на момент приёмки — сел вместе с этой записью.
Live `+` владельца (бар без пилюли, рельса над dock, клик Developer⇄Gamer).

## Contract

- Кнопка на правой рельсе **над dock-toggle**, не `PanelTab`, не в
  `panels_config` / edit-reorder.
- Клик → `workspace_mode::toggle` (`set` уже зовёт `refresh_windows`).
- Prompt Да/Нет/Не спрашивать — инлайн вертикальный ряд в 36px рельсе,
  без второй поверхности. Пока `pending` — клик по кнопке no-op.
- Бар-виджет `workspace_mode` снят: файл, `instantiate`, `BUILTIN_NAMES`,
  default `right`. Sanitize дропает старое имя как unknown.
- Иконки: Gamer `icons/gamepad.svg`, Developer `icons/mode-daily.svg`
  (`currentColor`, viewBox **256** — как остальные rail-иконки, не 24 из
  брифа).

## Verified (не со слов)

- `bar/widgets/workspace_mode.rs` отсутствует.
- `instantiate` / `BUILTIN_NAMES` без `workspace_mode`.
- `PanelTab::ALL` не содержит `workspace_mode` (тест + греп).
- `WorkspaceMode::icon_path` → `mode-daily` / `gamepad`; оба в `assets.rs`.
- `cargo test -p chronos --lib workspace_mode` → 12 ok
- `cargo test -p chronos --lib side_panel_right` → 197 ok
- `cargo test -p chronos --lib bar` → 17 ok (`layout_config` живёт в **bin**)
- `cargo test -p chronos --bins layout_config` → сначала **fail**
  `migration_idempotent` (assert count `workspace_mode` == 1). Эррата
  архитектора: assert → 0, known = все актуальные builtins. После — 21 ok.

## Caveats (не блокер)

- Тест `mode_button_click_toggles_mode` — тавтология на
  `WorkspaceMode::other()`, клик по рельсе не гоняет. Live закрыл клик.
- Tooltip «Developer»/«Gamer» на кнопке нет (бриф просил). Иконка говорит
  сама; не тикет.
- `--lib` 478 не ловит `layout_config` — только `--bins`. Миньон это проглотил.

## Не тронуто

T285, T291 `GamingModeState`, `Cargo.lock`, launcher dirty tree.

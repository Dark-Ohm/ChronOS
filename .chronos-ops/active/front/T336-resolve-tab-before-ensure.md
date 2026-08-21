---
ticket: T336
role: front
status: active
tags: [chronos-ops, front, active]
---

# T336 — select-tab вне mode set не создаёт скрытый backend

**Роль:** FRONTEND. **P2.** Живая находка T327 B3.
**Зона:** `crates/app/src/side_panel_right/view.rs` —
`on_tab_select` (`:324-350`) и `resolve_active_tab` (`:160-171`).
**Не трогать:** `updates.rs` (T334), `acp_settings.rs` (T335),
`select-tab:` имя IPC.

Параллелен T334/T335.

## Зачем

В Developer `select-tab:terminal` не в 11 видимых вкладок. Код ставит
tab и зовёт `ensure_tab_view` → `/bin/zsh` PID 501243, затем следующий
render сбрасывает в System. Пользователь видит System; zsh остаётся
zombie (`Z<s`). `select-tab:build` так же пишет loading tasks до
fallback.

Улики: `dump/qa-ux/T327/frames/right-terminal.png` (на экране System),
`log/out-of-mode-terminal-process.txt`, лог
`10:29:51.321` spawn → `10:29:51.324` `not in mode set → System`.
Источник: `done/qa/DRAFT-T336-out-of-mode-select-tab-spawns-hidden-terminal.md`.

Видимый fallback в System — канон T327/T323. Side effect — нет.

## Корень (сверено)

- `on_tab_select` не проверяет mode set, сразу `ensure_tab_view`.
- `resolve_active_tab` только в следующем `render`.

## Что сделать

Resolve **до** `ensure_tab_view`. Id вне текущего mode set → System,
без создания Terminal/Build/прочих. Реальный id в set — как сейчас.

Регрессия: клик по видимой вкладке Developer/Gamer не ломается.

## Готово когда

- `select-tab:terminal` в Developer: System на экране, в логе нет
  `terminal: shell spawned` на этот IPC.
- `select-tab:build` вне set: нет loading tasks.
- Юнит: порядок resolve-before-ensure.
- `cargo test -p chronos --lib` не краснеет. Живой лог в отчёт.

**Отчёт:** `.chronos-ops/reports-fresh/T336-resolve-tab-before-ensure-report.md`

# T210 — Right panel: drag vs peek-close + half-rate resize

**Статус:** active **P0**. Источник: T209 FAIL R7 + residual R2/R3.  
**Роль:** FRONTEND. **Модель: Sonnet / GLM.**  
**Спека смока:** `docs/superpowers/specs/2026-08-03-live-smoke-residuals.md`  
**Отчёт T209:** `docs/orchestration/tasks/report-log/T209-live-smoke-residuals-report.md`  
**Артефакты:** `/tmp/t209-smoke/20260803-0250/`

## Симптомы (доказано live)

1. **P0 dead hover after interrupted handle-drag:**  
   peek open → grab 4px handle → drag past panel edge → peek-leave  
   `side_panel_right: closed` mid-drag → hover strip never opens again  
   (cursor on `side_panel_hover_strip`, zero log). IPC `toggle-side-panel-right`  
   still opens → state handle OK; **strip enter handler dead**.
2. **Half-rate tracking (R2/R3):** pointer Δ ~2× panel edge Δ after expand;  
   never converges under cursor. Same local-delta path as T206.

## Зона

`crates/app/src/side_panel_right/{mod,view,hover_strip}.rs`  
(left panel only if shared peek pattern — don't break left).

## Must

- Active **resize drag** suppresses peek-close / hold_peek for the drag lifetime.
- After mid-drag close (if any path remains), hover strip **must** reopen panel.
- Resize delta: cursor and edge stick 1:1 after rail→expand (revisit  
  `start_x + (target-w)` + local coords under TOP|RIGHT).
- Live: R1–R3, R6, R7 repro no longer fails; grim + log.

## Не

- Absolute pointer experiments that caused T204 snap-to-36.
- Activity strip UI (T195 residual).

**Отчёт:** `report/T210-right-panel-drag-peek-dead-report.md`  
Коммит: `panels : drag holds peek + resize stick rate (T210)`.

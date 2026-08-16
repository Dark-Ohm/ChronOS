# T209 — Live smoke: residual tails (customization + Follow + settings)

**Статус:** active. **Роль:** QA / Architect + user eyes.  
**Модель:** manual (не отдавать «PASS» слабой модели без grim на диске).  
**Спека (канон):**  
`docs/superpowers/specs/2026-08-03-live-smoke-residuals.md`

**Бинарник:** `target/release/chronos` (пересобрать если HEAD ≠ smoke commit).

## Цель

Закрыть **LIVE N/V** по хвостам T194c/b, T200–T208, T195, T196 одним  
прогоном. Unit green не считается.

## Зона

Не код (если не FAIL→hotfix). Только: release run, grim, log, отчёт.

## Отчёт

`docs/orchestration/tasks/report/T209-live-smoke-residuals-report.md`  
по §8 спеки (матрица ID × PASS/FAIL/SKIP + evidence paths).

## Коммит

Не нужен, если только артефакты/отчёт. Hotfix → отдельный T-ID или  
`smoke : …` с узким диффом.

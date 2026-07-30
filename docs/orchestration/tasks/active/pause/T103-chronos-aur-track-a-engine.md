<!-- T103 — активный бриф Cline, перенесён 2026-07-22 из docs/orchestration/agents/CLINE.md -->

## АКТИВНОЕ ЗАДАНИЕ — Chronos-AUR порт (Alloy Tauri→GPUI), ТРЕК A — движок aur-core

**ПРОЕКТ ДРУГОЙ РЕПО:** `/home/neo/projects/chronos-ecosystem/Chronos-AUR` (git,
MIT). НЕ ChronOS-шелл. Это отдельное приложение экосистемы (Путь 2 — свой бинарь,
рядом с шеллом, не в нём).

**ПЕРВОЕ ЧТЕНИЕ — ПЛАН ЦЕЛИКОМ:** `Chronos-AUR/docs/port-plan.md`. Там цель,
Global Constraints, целевая структура репо, **load-bearing интерфейсы** (ShellExec,
StreamEvent, core API, Page-slot) и твой трек по шагам. Это порт: **исходный код =
спека**, переводим, не редизайним.

**Твой трек — движок.** Переносишь `src-tauri/src/{models,updater}.rs` +
`services/*` в `crates/aur-core/`, снимаешь `#[tauri::command]`/`tauri::`,
маршрутизируешь exec через `crate::shell::exec_*` (Трек B — сверься с его
сигнатурами в плане §Interfaces до старта). **malware_check/pkg_analyze/pkg_build —
байт-верно, только механический tauri-strip + ShellExec-swap, ноль логики.** Держи
все `#[cfg(test)]`. План §TRACK A — шаги. Coord: Грок (B) даёт ShellExec-контракт;
работай против сигнатур из плана.

**Коммит** в репо Chronos-AUR (MIT, свой), поимённо, малыми коммитами — по шагам
плана. Отчёт в `docs/orchestration/tasks/report/TNNN-<slug>-report.md` (каталог `docs/orchestration/reports/` упразднён 2026-07-31). Приёмку и свод интерфейсов делает
Архитектор.

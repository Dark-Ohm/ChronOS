<!-- T104 — активный бриф Grok, перенесён 2026-07-22 из orchestration/agents/GROK.md -->

## АКТИВНОЕ ЗАДАНИЕ — Chronos-AUR порт (Alloy Tauri→GPUI), ТРЕК B — shell-exec fish/zsh/bash

**ПРОЕКТ ДРУГОЙ РЕПО:** `/home/neo/projects/chronos-ecosystem/Chronos-AUR` (git,
MIT). НЕ ChronOS-шелл. Это отдельное приложение экосистемы (Путь 2 — свой бинарь,
рядом с шеллом, не в нём).

**ПЕРВОЕ ЧТЕНИЕ — ПЛАН ЦЕЛИКОМ:** `Chronos-AUR/docs/port-plan.md`. Там цель,
Global Constraints, целевая структура репо, **load-bearing интерфейсы** (ShellExec,
StreamEvent, core API, Page-slot) и твой трек по шагам. Это порт: **исходный код =
спека**, переводим, не редизайним.

**Твой трек — мульти-шелл executor** (ты делал desktop_terminal PTY). Порт
`src-tauri/src/fish.rs` → `crates/aur-core/src/shell.rs`: StreamEvent-парсер
оставляешь (shell-agnostic), добавляешь `enum Shell{Fish,Zsh,Bash}`+`detect_shell`,
`exec_one`/`exec_streaming` берут `Shell` и ветвят spawn на бинарь+обёртку.
**Аудит скриптов `services/*` на fish-измы** (`set`/`; and`/`$status`) — posix
для zsh/bash. Контракт (сигнатуры) — план §Interfaces#2, **определи shell.rs первым**,
чтобы Cline (A) компилился против них. Фикстуру парсера снять с ЖИВОГО pacman.

**Коммит** в репо Chronos-AUR (MIT, свой), поимённо, малыми коммитами — по шагам
плана. Отчёт в `orchestration/tasks/report/TNNN-<slug>-report.md` (каталог `orchestration/reports/` упразднён 2026-07-31). Приёмку и свод интерфейсов делает
Архитектор.

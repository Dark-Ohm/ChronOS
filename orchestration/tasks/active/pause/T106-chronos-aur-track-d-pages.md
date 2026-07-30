<!-- T106 — активный бриф Zed, перенесён 2026-07-22 из orchestration/agents/ZED.md -->

## АКТИВНОЕ ЗАДАНИЕ — Chronos-AUR порт (Alloy Tauri→GPUI), ТРЕК D — порт страниц React→rsx

**ПРОЕКТ ДРУГОЙ РЕПО:** `/home/neo/projects/chronos-ecosystem/Chronos-AUR` (git,
MIT). НЕ ChronOS-шелл. Это отдельное приложение экосистемы (Путь 2 — свой бинарь,
рядом с шеллом, не в нём).

**ПЕРВОЕ ЧТЕНИЕ — ПЛАН ЦЕЛИКОМ:** `Chronos-AUR/docs/port-plan.md`. Там цель,
Global Constraints, целевая структура репо, **load-bearing интерфейсы** (ShellExec,
StreamEvent, core API, Page-slot) и твой трек по шагам. Это порт: **исходный код =
спека**, переводим, не редизайним.

**Твой трек — первые страницы.** Порт `src/pages/PackagesPage.tsx`(263) +
`SystemUpdatePage.tsx`(471) в `crates/aur-app/src/pages/`: разметку читаешь из TSX
→ `rsx!`, состояние (query/results/selection) → GPUI-view-entity, `safeInvoke("…")`
→ `aur_core::…().await` в `cx.spawn`, стрим-апгрейд (StreamEvent через mpsc) →
живой лог/прогресс, ретем Catppuccin. Вставляешь в `router::render_page` (Трек C).
Зависишь от C (Page-slot) и A (core API) — сверься с планом §Interfaces. Живой смок:
реальный `pacman_search` в UI. План §TRACK D.

**Коммит** в репо Chronos-AUR (MIT, свой), поимённо, малыми коммитами — по шагам
плана. Отчёт в `orchestration/tasks/report/TNNN-<slug>-report.md` (каталог `orchestration/reports/` упразднён 2026-07-31). Приёмку и свод интерфейсов делает
Архитектор.

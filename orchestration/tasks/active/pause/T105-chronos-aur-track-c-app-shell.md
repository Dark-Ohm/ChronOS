<!-- T105 — активный бриф Hermes, перенесён 2026-07-22 из orchestration/agents/HERMES.md -->

## АКТИВНОЕ ЗАДАНИЕ — Chronos-AUR порт (Alloy Tauri→GPUI), ТРЕК C — GPUI-каркас aur-app

**ПРОЕКТ ДРУГОЙ РЕПО:** `/home/neo/projects/chronos-ecosystem/Chronos-AUR` (git,
MIT). НЕ ChronOS-шелл. Это отдельное приложение экосистемы (Путь 2 — свой бинарь,
рядом с шеллом, не в нём).

**ПЕРВОЕ ЧТЕНИЕ — ПЛАН ЦЕЛИКОМ:** `Chronos-AUR/docs/port-plan.md`. Там цель,
Global Constraints, целевая структура репо, **load-bearing интерфейсы** (ShellExec,
StreamEvent, core API, Page-slot) и твой трек по шагам. Это порт: **исходный код =
спека**, переводим, не редизайним.

**Твой трек — оконный каркас приложения** (ты владеешь rsx/GPUI после панели v2).
Новый `crates/aur-app/` GPUI-бинарь: `main.rs` (окно ~1100×720, WindowKind::Normal),
`theme.rs` (Catppuccin токены как в ChronOS), `components/sidebar.rs` (порт
`src/components/Sidebar.tsx` → `rsx!`, 7 пунктов → `Page`), `router.rs`+`app.rs`
(сайдбар слева + контент зовёт `render_page`, страницы-заглушки кроме одной от
Трека D). Cargo: gpui-форк git-деп (@99cab5e) + gpui-rsx + gpui-animation + aur-core
path. План §TRACK C. Отметь rsx-вердикт (где 1:1 vs div).

**Коммит** в репо Chronos-AUR (MIT, свой), поимённо, малыми коммитами — по шагам
плана. Отчёт в `orchestration/tasks/report/TNNN-<slug>-report.md` (каталог `orchestration/reports/` упразднён 2026-07-31). Приёмку и свод интерфейсов делает
Архитектор.

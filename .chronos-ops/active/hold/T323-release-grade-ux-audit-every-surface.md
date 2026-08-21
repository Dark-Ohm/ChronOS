# T323 — HOLD: целый шелл одним заходом

**Статус:** HOLD / SPLIT 2026-08-21. **Не выдавать, не исполнять.**
**Почему:** один live-проход не покрывает шелл. Кадры первого захода
сгорели в `/tmp` после ребута. Два черновика в `reports-fresh/` —
гипотезы без улик, не приёмка.

Нарезка (исполнять по одному, не параллелить — одна живая сессия):

| ID | Сфера | Файл |
|---|---|---|
| **T324** | Бар, dock, виджеты | `active/qa/T324-bar-dock-widgets-ux-audit.md` — **сейчас в поле** |
| T325 | Попапы, start, launcher, OSD, click-catcher | `active/qa/T325-popups-overlays-ux-audit.md` |
| T326 | Левая панель | `active/qa/T326-left-panel-ux-audit.md` |
| T327 | Правая панель + Gamer/Developer | `active/qa/T327-right-panel-modes-ux-audit.md` |
| T328 | Рама, схемы, alpha/blur, обои | `active/qa/T328-frame-theme-wallpaper-ux-audit.md` — **последним** |

Вердикт «продалось бы за $100» сводит архитектор после T324–T328.
Черновики первого прогона: `reports-fresh/T323-*-report.md`.

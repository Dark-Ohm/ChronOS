# T329 — календарь: непрозрачная подложка

**Роль:** FRONTEND. **P1.** Живая находка T324 B1.
**Зона:** `crates/app/src/calendar_popup/` только.
**Не трогать:** `volume_popup/`, бар, `frame.rs`, панели.

## Зачем

Клик по часам открывает календарь без заливки. Обои и содержимое панели
просвечивают сквозь сетку дат: текст на тексте. Кадры T324:
`.chronos-ops/dump/qa-ux/T324/crops/22-calendar-over-panel-1to1.png`,
`frames/23-calendar-over-wallpaper.png`.

## Корень (сверено)

- `calendar_popup/view.rs` `render` возвращает голый `Calendar::new(&self.calendar)` — ни одного `.bg(`.
- `calendar_popup/mod.rs` Root: `.bg(gpui::transparent_black())` на якорном и fallback путях.
- Как надо: `volume_popup/view.rs` — `.bg(theme.surface_color(bg.alpha(0.82)))` плюс бордер/радиус/тень.

## Что сделать

Обернуть `Calendar` в карточку той же семьи, что Sound: `theme.surface_color` на непрозрачной/стеклянной плите, не `transparent_black` на Root. Заголовок и крестик — не обязательны. Escape/click-catcher — **T332**, не этот тикет. T325 живьём подтвердил: заливки нет, клик мимо не закрывает.

`KeyboardInteractivity::Exclusive` запрещён.

## Готово когда

- Живой клик по часам на светлых обоях и поверх открытой правой панели: сетка дат читается, панель/обои не лезут в ячейки. grim в отчёт.
- `cargo test -p chronos --lib` не краснеет.
- Код только в `calendar_popup/`.

**Отчёт:** `.chronos-ops/reports-fresh/T329-calendar-popup-opaque-plate-report.md`

# T342 — surface_alpha красит раму и Start, не только бар

**Роль:** FRONTEND. **P1.** T328 B5 / гипотеза T323.
**Зона:** `crates/app/src/frame.rs` wrap matte (`theme.bg.tertiary` ~741–744),
`crates/app/src/start_menu/view.rs` (`.bg(theme.bg.primary/tertiary)`),
рейлы панелей если сырой `bg.*` на хроме оболочки (не чат-карточки).
**Не трогать:** `calendar_popup/` (T329), `volume_popup/` (T332),
`HEIGHT_MIN` (T337).

Бар уже `theme.surface_color(theme.bg.tertiary)` (`bar/mod.rs:124`, T266).

## Зачем

Mocha 0.7, белые обои: бар `#49464A`, рельсы/низ/Start `#18141A` /
глухой primary. Ползунок Surface opacity выглядит общим, красит одну
поверхность. `frames/13-mocha-alpha07-white.png`, `14-startmenu-alpha07.png`.

## Что сделать

Хром оболочки (wrap matte/кольцо, Start фон/рейл, боковые рельсы рамы)
через `theme.surface_color(...)`, как бар. Не делать прозрачными
вложенные карточки списков.

## Готово когда

`surface_alpha = 0.7`, Mocha, белый стол: бар и рама/Start заметно
просвечивают одинаково (пробы как в отчёте T328). 1.0 — глухие.
grim. `cargo test -p chronos --lib` не краснеет.

**Отчёт:** `.chronos-ops/reports-fresh/T342-surface-alpha-chrome-report.md`

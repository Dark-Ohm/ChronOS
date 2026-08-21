# T340 — выбранный чип читается во всех схемах

**Роль:** FRONTEND. **P1.** T328 B3.
**Зона:** `crates/ui/src/theme/schemes.rs` (+ `contrast_ratio` / тесты
там же). При необходимости стиль выбранного сегмента в
`bar_settings.rs` (заливка чипа), не раскладка ACP (T335).
**Не трогать:** `HEIGHT_MIN` (T337), calendar (T329).

## Зачем

Solarized Dark, выбранный чип «Top»: акцент на 20%-акцентной заливке,
**1.19:1**. Теряются Top/Full/Soft/on/Wrapped и имя карточки.
`crops/11c-solarized-toppill-5x.png`, `11f-solarized-appearance-1to1.png`.

Тот же паттерн < 4.5:1 во всех четырёх схемах (Default 2.44, Light 2.92,
Mocha 3.05). T317 мерил `text.muted` на `bg.primary`, не selected-chip.

## Что сделать

1. Выбранный чип: текст ≥ 4.5:1 к заливке во **всех** `builtin_schemes()`.
   Типично: `text.primary` на `interactive.active` / непрозрачная плита,
   не `accent` на `accent@0.2`.
2. Тест итерирует `builtin_schemes()`, как T317, по паре
   selected-fill / selected-label. Мутация одной схемы должна валить тест.

Не выкидывать Solarized. Light+белый стол растворяет раму (T328 F3) —
не этот тикет, TBD.

## Готово когда

Живой пикер: Solarized Dark, чипы Top/Wrapped читаются. Юнит ≥ 4.5:1
на всех схемах. grim.

**Отчёт:** `.chronos-ops/reports-fresh/T340-scheme-selected-chip-contrast-report.md`

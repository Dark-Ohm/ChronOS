# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

## Очередь

1. **T314** — `T314-wrap-strips-live-exclusive-zone.md`. P1, СВОБОДЕН.
   Живая `set_exclusive_zone` для wrap-полос: рельс спрятан → кольцо
   16px, рельс показан → он сам кромка на x=0. Остаток T311 D3.
   **Читать раздел «как НЕ чинить» — предыдущий заход утонул на
   `close + open` и протокольных ошибках Hyprland.**
2. **T313** — `T313-theme-picker-and-mocha-mousse-scheme.md`. P2,
   СВОБОДЕН. Picker схем со свотчами + схема Mocha Mousse.
3. **T312** — `T312-frame-modes-normal-wrapped.md`. P2,
   **ЗАБЛОКИРОВАН T314**. Два режима `normal`/`wrapped`, алиасы старых
   имён в `deserialize_style`.
4. **T316** — `T316-bar-radius-closes-aperture-top.md`. P2,
   **ЗАБЛОКИРОВАН T315** (дизайн задаёт радиус). Нижние углы бара
   замыкают апертуру сверху. Остаток T311 D4.

5. **T317** — `T317-text-muted-wcag-contrast.md`. P2, СВОБОДЕН.
   `text.muted` не проходит WCAG: 2.91:1 в светлой, 3.36:1 в тёмной.
   Пересечение с T313 по `schemes.rs` — если оба в поле, T317 первым.

T314, T312 и T316 параллелить нельзя между собой — все трогают раму.
Порядок: T314 → приёмка → T312; T316 отдельно после T315.

T313 независим от всех (зоны `crates/ui/src/theme/`,
`side_panel_right/tab/bar_settings.rs`, `theme_config.rs`) — можно
вести параллельно с чем угодно.

**Смежное:** `active/design/T315-rail-as-frame-edge-artboards.md` —
как рельс должен ВЫГЛЯДЕТЬ, когда он стал кромкой кадра. Кодового
тикета по рельсу до приёмки T315 не будет.

**Закрыто 2026-08-18/19 (детали — `MIGRATION.md`):** T301 (composer Select
эллипсис, `96f713a`), T302 (rail-only by-design, бага нет), T303 (wrap
matte-геометрия LEFT|BOTTOM + отрицательный margin, `c6df21a`), T305
(control-center popup, `f326fc7`), T307 (wrap hot-reload thickness/radius,
`601f8f0`), T308 (wrap matte `exclusive_zone: Some(px(-1.))` opt-out,
`f2cacee`), T311 (единая плита — D2 принят, D3→T314, D4→T316).

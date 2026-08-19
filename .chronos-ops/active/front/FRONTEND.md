# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

## Очередь

1. **T311** — `T311-shell-single-plate-geometry.md`. P1, СВОБОДЕН.
   Оболочка как единая плита: толщина краёв по функции, рельсы одним
   материалом, замкнутая апертура. Канон решения — раздел A-J
   дополнения в `.chronos-ops/active/design/T310-*.md`.
2. **T312** — `T312-frame-modes-normal-wrapped.md`. P2,
   **ЗАБЛОКИРОВАН T311**. Два режима оболочки `normal`/`wrapped`,
   алиасы старых имён в `deserialize_style`.

Параллелить нельзя: оба тикета лезут в `crates/app/src/frame.rs`.
Строго T311 → приёмка → T312.

**Закрыто 2026-08-18/19 (детали — `MIGRATION.md`):** T301 (composer Select
эллипсис, `96f713a`), T302 (rail-only by-design, бага нет), T303 (wrap
matte-геометрия LEFT|BOTTOM + отрицательный margin, `c6df21a`), T305
(control-center popup, `f326fc7`), T307 (wrap hot-reload thickness/radius,
`601f8f0`), T308 (wrap matte `exclusive_zone: Some(px(-1.))` opt-out —
съезжала на 56px от резервации соседнего рейла, `f2cacee`).

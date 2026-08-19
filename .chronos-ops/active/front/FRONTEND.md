# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

**Очередь FRONTEND пуста.** Сверяться с `checkpoint/HANDOFF.md` за новыми
находками владельца.

**Закрыто 2026-08-18/19 (детали — `MIGRATION.md`):** T301 (composer Select
эллипсис, `96f713a`), T302 (rail-only by-design, бага нет), T303 (wrap
matte-геометрия LEFT|BOTTOM + отрицательный margin, `c6df21a`), T305
(control-center popup, `f326fc7`), T307 (wrap hot-reload thickness/radius,
`601f8f0`), T308 (wrap matte `exclusive_zone: Some(px(-1.))` opt-out —
съезжала на 56px от резервации соседнего рейла, `f2cacee`).

# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

**Активное:**
- **T308** — `T308-wrap-matte-shifted-by-sibling-panel-reservation.md`.
  **P0** — живая находка владельца прямо на T303-кровном факте: с
  открытой боковой панелью матте съезжает на 56px (40 rail + 16 wrap),
  правое кольцо улетает за экран. Подозреваемый корень: матте
  `exclusive_zone: None` вместо T305-паттерна `Some(px(-1.))`.
  Независимо подтверждено T307-исполнителем (тот же сдвиг, cold=hot).

**Очередь FRONTEND пуста после T308** — сверяться с `checkpoint/HANDOFF.md`
за новыми находками владельца.

**Закрыто 2026-08-18 (детали — `MIGRATION.md`):** T301 (composer Select
эллипсис, `96f713a`), T302 (rail-only by-design, бага нет), T303 (wrap
matte-геометрия LEFT|BOTTOM + отрицательный margin, `c6df21a`), T305
(control-center popup, `f326fc7`), T307 (wrap hot-reload thickness/radius,
`601f8f0` — попутно подтвердил T308).

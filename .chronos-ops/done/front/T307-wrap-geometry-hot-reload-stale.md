# T307 — wrap-геометрия не подхватывается на живом шелле (hot-reload stale)

**Роль:** FRONTEND. **Приоритет:** P3. **Зона:** `crates/app/src/frame.rs`
(`apply_wrap`/`open_wrap_windows`, вокруг frame.rs:596-680 после T303).

## Контекст

Найдено живьём исполнителем T303 (`.chronos-ops/reports-log/front/
T303-frame-wrap-border-geometry-mismatch-report.md`, «Честные оговорки»
п.2), не чинилось — вне зоны T303. Бар и Hide-полоса перечитывают
`frame.toml` на лету; wrap-матте и три exclusive-полосы (L/R/B) — нет:

- `apply_wrap` → `open_wrap_windows` **early-return, если матте уже
  открыта**;
- следствие: правка `wrap.thickness`/`inner_radius` в `frame.toml` на
  живом (уже запущенном в стиле `wrap`) шелле не применяется — окна
  остаются со старой геометрией, пока не `pkill chronos && chronos`.

## Что сделать

1. Найти путь `apply_wrap`/`open_wrap_windows` (frame.rs, после
   T303-геометрии — margin/anchor теперь другие, сверяться с деревом, не
   с этим тикетом).
2. Разобраться, почему стоит early-return при уже открытой матте —
   вероятно защита от повторного `window.open` дубликатом. Починить так,
   чтобы при изменении `wrap.thickness`/`inner_radius` окна
   переоткрывались (закрыть старые + `open_wrap_windows` заново) либо
   применяли новую геометрию через `window.resize`/`set_margin`-эквивалент
   форка (если API это позволяет — сверить с `Source/`, не гадать).
3. Живой проверочный рецепт: запустить `chronos` в стиле `wrap`, поменять
   `thickness` в `frame.toml` на лету, снять `grim` до/после — кольцо
   должно измениться без рестарта шелла. Тот же DP-1 2560×1440 сетап, что
   в T303-отчёте (reserved `[4,30,4,4]` или актуальный).
4. Regression: bar/Hide hot-reload не сломать — сверить живьём отдельно
   (это работало и до T307).

## Верификация

- `cargo check -p chronos`, `cargo test -p chronos --lib frame`.
- Живой grim до/после смены `thickness` без рестарта — обязателен, это
  весь смысл тикета.

## Отчёт

`.chronos-ops/reports-fresh/T307-wrap-geometry-hot-reload-stale-report.md`.

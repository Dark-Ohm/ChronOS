<!-- T036 — migrated 2026-07-22 from orchestration/report-log/grok-report-6.md — see orchestration/tasks/MIGRATION.md -->

# Session: Grok №6 — remove_window / ghost Wayland windows — 2026-07-18

## Сделано (факт, не намерение)
- **Source** (`~/projects/chronos-ecosystem/Source`, отдельный git):
  - `gpui_linux/.../wayland/window.rs` — `Drop for WaylandWindow`:
    protocol destroy → **sync** `client.drop_window()` → **sync**
    `Connection::flush()` → deferred `close()` only (GPUI callback).
  - `gpui_linux/.../wayland/client.rs` — `drop_window` idempotent
    (no `unwrap` panic if already removed).
  - Diagnostic `log::debug!` left in place (module `gpui_linux`); useful
    under `RUST_LOG=gpui_linux=debug`.
- ChronOS code **not** changed (no soft-hide workarounds).

**Source commit:** `3800d3a` (`wayland : sync drop_window+flush on window Drop (ghost fix)`).

## Расхождения со спекой/планом
- Клики «снаружи» мышью (ydotool) **не** гонял — вместо этого 15 циклов
  open/close через IPC `toggle-launcher` (тот же `remove_window` path, что
  и close-from-outside). Повторить ручным кликом Архитектору при желании.
- tray_menu UI в `main.rs` сейчас **выключен** хотфиксом (db7e595) — отдельный
  tray_menu open/close цикл не гонял; launcher + notify-send — да.
- Гипотеза A (late `update_window` → ERROR window not found) **не опровергнута
  как класс**, но за 15 циклов launcher **0** раз `window not found` в логе.
  Основная подтверждённая причина ghost'ов — **B + missing flush**.

## Не реализовано из acceptance criteria
- Синтетический ydotool click-outside (калибровка нестабильна, HANDOFF).
- tray_menu 5× open/close при выключенном init в master.
- Отмена scheduled frames в `remove_window()` (gpui `window.rs`) — не
  понадобилась после sync unregister+flush; остаётся опциональным harden.

## Проверено фактом, не на словах

### Анализ (гипотеза B — подтверждена кодом)
1. `WaylandSource` (calloop-wayland-source) вызывает `queue.flush()` **только
   после итерации event loop**.
2. В `Drop` destroy-запросы уходили в write buffer **без** flush.
3. `drop_window` (снять surface из `client.state.windows`) жил в
   `.detach()` async — между Drop и таском map ещё роутил events.
4. `close()` deferred **оставлен** — reenter App mid-`trail()` опасен.

### Живой смок (release chronos + path-dep Source)
- `pkill -x chronos`; `RUST_LOG=gpui_linux=debug,chronos=info ./target/release/chronos`
- **5 циклов** toggle open→close, poll `hyprctl clients -j` 100ms:
  - all: `appeared=true`, `max_seen=1`, `residual=0`, `gone_at≈0.11–0.12s`
  - log: `drop_window done`×5, `flush after destroy ok`×5, `window not found`=0
- **10 циклов** stress (hold open 400ms, then close):
  - all OK, `gone≈50–70ms`, max=1, residual=0
  - `STRESS PASS`
- notify-send ×2 — без `window not found`
- `cargo test --workspace --lib --bins` — 4+65+25+80+3 all ok

Лог: `/tmp/chronos-rmwin-smoke/chronos.log`

## Новые риски / известные баги
- **low:** `close()` всё ещё async — late callback edge cases possible;
  unregister already done so compositor routing is clean.
- **low:** flush failure only `log::warn` — rare; next event loop tick will
  flush again.
- **medium (process):** ghost bug was systemic — any ChronOS surface that
  still soft-hides (OSD) can keep soft-hide; remove_window is now safer
  for real destroy paths (launcher, dock, tray_menu when re-enabled).

## Статус ARCHITECTURE.md / DECISIONS.log
- Не обновлялись (баг/фикс в Source fork, не канон ChronOS app).
  Рекомендация Архитектору: короткий пункт в HANDOFF «remove_window fixed
  in Source @ <hash>».

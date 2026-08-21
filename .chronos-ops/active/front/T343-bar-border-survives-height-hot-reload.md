---
ticket: T343
role: front
status: active
tags: [chronos-ops, front, active]
---

# T343 — граница бара в normal переживает смену высоты

**Роль:** FRONTEND. **P2.** T328 F1.
**Зона:** `crates/app/src/bar/mod.rs` render бордера (`:137-153`) и
hot-reload высоты (layout_config / тот же модуль). Не `HEIGHT_MIN`
(T337).
**Не трогать:** wrap-раму (там бордер снят каноном T284 §5.3).

Параллелен T337 (разные файлы).

## Зачем

`normal`, свежий старт, height 20: y=19 `#45475A` (`border_b_1` +
`border.subtle`). После hot-reload `20→24→20` граница пропадает навсегда
до рестарта (три колонки x=300/800/2000). Round-trip рамы/схемы/alpha
не ломает — ломает только Height.

`crops/29a-bar-border-3x.png` vs `33a-bar-noborder-3x.png`.

## Что сделать

Смена `appearance.height` в `normal` оставляет нижнюю (для top-edge)
границу. Не возвращать бордер в `wrapped`.

## Готово когда

Живой: `normal`, Default, height round-trip через слайдер или toml:
граница на месте без рестарта. grim до/после. `wrapped` без режущего
шва (T284).

**Отчёт:** `.chronos-ops/reports-fresh/T343-bar-border-survives-height-hot-reload-report.md`

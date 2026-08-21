# T337 — HEIGHT_MIN бара: высота, на которой виджеты читаются

**Роль:** FRONTEND. **P1.** Живой grim 2026-08-21, `bar.toml` height=20.
**Зона:** `crates/app/src/bar/appearance.rs` (`HEIGHT_MIN`, sanitize,
тест `sanitize_clamps_height`), `crates/app/src/side_panel_right/tab/bar_settings.rs`
(свой `HEIGHT_MIN`/`HEIGHT_MAX` слайдера — держать в локе с appearance).
**Не трогать:** `network.rs` (T331 HOLD), calendar/volume, `view.rs`.

Не параллелить с T331: тот тикет запрещал поднимать пол, этот пол меняет.

## Зачем

Продукт разрешает 20 px (`HEIGHT_MIN` в двух местах) и на этой высоте
ломает **весь** бар, не только сеть: cava — пыль, часы нечитаемы, правый
кластер — каша иконок, сеть обрезана. Кадры:
`.chronos-ops/dump/notes/bar-top-24.png`, `bar-right-900.png`.
Слой: `bar 2560×20`. Дефолт кода `chronos_luau::bar::BAR_HEIGHT` = **30**.
Слайдер настроек и sanitize расходятся ещё и по максимуму (48 vs 80) —
не раздувать скоуп, но полы синхронизировать.

20 — не «компактный режим», а дыра в clamp. T331 («втиснуть сеть в 20»)
не лечит класс бага. Пол должен быть высотой, на которой дефолтный набор
виджетов остаётся внутри бара.

## Что сделать

1. Живой промер на release: 28 / 32 / 36 / 40 (и 30 = `BAR_HEIGHT`).
   grim каждого. Выбрать **наименьшее целое**, где целиком читаются:
   цифры часов, бары cava, точки ws, две строки network xs.
2. Это число → `HEIGHT_MIN` в `appearance.rs` **и** `bar_settings.rs`.
   Тест clamp обновить. Существующий `height=20` в toml после sanitize
   поднимается на новый пол (hot-reload, не ручной rewrite остальных ключей).
3. Не рисовать однострочный network в этом тикете. Не менять `BAR_HEIGHT`
   (30), если выбранный пол ≤ 30.

Не делать `yay`. Не ставить 42 «потому что раньше так было».

## Готово когда

- Слайдер не отдаёт 20. Live `bar.toml` height=20 после sanitize ≥ новый пол.
- Grim на новом минимуме: часы/cava/сеть без обрезки глифов.
- `cargo test -p chronos --lib` appearance/bar_settings зелёные.

**Отчёт:** `.chronos-ops/reports-fresh/T337-bar-height-min-readable-report.md`

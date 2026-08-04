# T243 — right panel width desync: root cause found by live tracing, fixed

**Дата:** 2026-08-05
**Статус:** FIXED
**Роль:** FRONTEND (GPUI state) + live tracing

## Вердикт

Залипание правой панели на `w=40` при докнутой вкладке — исправлено.
Корень найден живым трейсингом, а не гипотезой (первый раз за все
попытки T242/T243, тикет прямо это требовал).

## Что было сделано

### 1. Трейсинг (первый за серию)

`view.rs` `render()`: `tracing::debug!` на каждый кадр —
`last_resized_width`, `panel_width`, `actual_width` (из `window.bounds()`),
`dock_content`, `content_open`. Запуск через `systemd-run` юнит с
`RUST_LOG='info,chronos::side_panel_right=trace'`, лог в journald.

Репро-скрипт `/tmp/t243-repro-rand.sh` (toggle×5 + select-tab, случайные
тайминги): **5/12 залипаний** на сломанном бинаре.

### 2. Дымящийся ствол

Залипшая итерация (секундами):

```
last_resized_width=320.0 panel_width=320.0 actual_width=40.0
```

`window.resize(320)` был выдан один раз и **потерян композитором**
(async Wayland configure — та же задокументированная проблема T216
"state runs ahead of the compositor"). Гард `last_resized_width !=
panel_width` сравнивает ДВЕ state-копии: 320 == 320 → "уже заресайзено"
→ resize больше не перевыпускается → окно навсегда на рейле.

Это ровно та ошибка принципа, что T216 зафиксировал в `update_resize`,
но гард в `render()` её нарушал.

### 3. Фикс (`view.rs`)

```rust
let actual_width = window.bounds().size.width.as_f32();
if needs_width_resize(actual_width, panel_width) { ... window.resize(...) }
```

Гейт по **живой геометрии** композитора, не по state-копиям: resize
перевыпускается каждый кадр, пока `actual != target` (самовосстановление
после потерянного configure); после акка пере-выпуск сам останавливается.
`needs_width_resize(actual, target) = (actual - target).abs() > 1.0` —
отдельная юнит-тестируемая функция.

Заодно `content_open` тоже гейтится по `window.bounds()` (это был "заход
2" архитектора — правильный, но недостаточный без гейта самого resize).

Юнит-тест: `width_resize_guard_retries_until_compositor_acks`.

### 4. Зеркальный фикс левой панели (`side_panel_left/mod.rs`)

Тот же гард `last_resized_width != state.width` нашёлся в левой панели.
Живой триггер при T226: `expand-left` открывал композер на 40px при
state=352 (тот же механизм). Применён тот же гейт по `window.bounds()`.
Верифицировано: 352 на t=200мс.

## Верификация

| проверка | до | после |
|---|---|---|
| Репро 15 итераций (тот же скрипт) | 5/12 | **0/15** |
| Поллинг ширины после select-tab | 40 (≥120мс, вечно при потерянном configure) | **320 на t=10мс** |
| Wobble-состояние `content_open=false` при `actual=320` | (исключено bounds-гейтингом) | **0 случаев** за весь трейс |
| `cargo test --release -p chronos --lib -- side_panel_right` | — | **167/167** |

## Кадры

Нет статичного кадра (это state-баг, кадр ничего не показывает) —
доказательство в логе journald: последовательности
`last=320 panel=320 actual=40` до фикса против `320 на t=10мс` после,
плюс 0/15 на репро.

## Коммит

`panels : right panel width desync fix, mirrors T242 (T243)` —
ожидается, коммит сделает владелец/агент по завершении серии.

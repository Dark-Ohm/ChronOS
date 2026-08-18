# T307 — wrap-геометрия не hot-reload'ится на живом шелле — report

**Роль:** FRONTEND. **Статус:** готово к приёмке. **Зона:** `crates/app/src/frame.rs` (один файл).

## Диагноз

`apply_wrap` → `open_wrap_windows` (frame.rs) стоит early-return, если матте
уже открыта:

```rust
fn open_wrap_windows(cx: &mut App) {
    let slots = wrap_windows().lock()...;
    if slots.matte.is_some() {
        return; // ← геометрия в size/exclusive_zone/margin запечена при open
    }
    ...
}
```

Геометрия (толщина матте-бордера, размер и exclusive_zone трёх полос,
отрицательный margin матте) задаётся **в момент `window.open`** через
`wrap_window_options` — живьём её не поменять (`set_margin` публичного в
форке нет, только `set_exclusive_zone` + `resize`; отрицательный margin
матовца — только при open). Поэтому `apply()` на правку `frame.toml`
вызывал `apply_wrap` → `open_wrap_windows` → no-op: окна оставались со
старой геометрией, пока не `pkill chronos`.

## Что сделано

1. **Трек последней применённой геометрии** (`LAST_WRAP_GEOMETRY`,
   `Mutex<Option<WrapConfig>>`) — санитизированная пара
   `(thickness, inner_radius)`.
2. **`apply_wrap` пересоздаёт сетап при изменении** (`frame.rs:819-843`):
   если `wrap_geometry_changed(last, &cfg.wrap)` — `close_wrap_windows` +
   `open_wrap_windows` заново. При том же значении — как раньше, no-op
   (исключён лишний переоткрыт на каждый rail-open/close и на дубль
   `apply()`).
3. **Чистый предикат `wrap_geometry_changed`** вынесен отдельно (без
   `App`) — юнит-тестируется напрямую.
4. **Тест** `wrap_geometry_changed_only_on_actual_edit` — первый apply /
   тот же геометрия / edit толщины / edit радиуса.

`apply_hide` и бар не тронуты (диф только `apply_wrap` + call-site +
статик + тест).

## Верификация

- `cargo check -p chronos` — чисто, 0 новых warnings.
- `cargo test -p chronos --lib` — **599 passed** (было 598, +1 новый).
- `cargo test -p chronos --bins` — **791 passed, 0 failed**.

### Живой смок (release, DP-1 2560×1440, wrap-стиль) — пройден

Запустил `chronos` с `[wrap] thickness=16`, затем на лету поменял на 8
**без рестарта**:

- Лог: `frame: closed wrap surface Matte/ExclLeft/ExclRight/ExclBottom` →
  `frame: hot-reloaded config` (300 мс debounce).
- `hyprctl layers -j` до/после: `frame_wrap_excl_left/right` `w: 16 → 8`,
  `frame_wrap_excl_bottom` `h: 16 → 8` — exclusive-зоны пересозданы.
- **grim до/после**: цвет кольца `srgb(24,24,37)` (=`bg.tertiary`) на левой
  кромке y=720 занимает `x0-15` при 16 → `x0-7` при 8; пиксели `x8-15`
  стали обоями (DIFF в таблице замеров). Кольцо реально сузилось 16→8 на
  живом шелле.
- **Эквивалентность холодному старту**: `pkill chronos && chronos` с
  thickness=8 даёт те же `excl_* w=8/h=8` и то же положение матте для того
  же состояния панелей — hot-reload не разошёлся с cold.

### Regression (bar / Hide)

- **Hide hot-reload не сломан**: переключил `style="hide"`, открыл правый
  рейл → `frame_bottom_strip h=4`, на лету `height=8` → `h=8`
  (`y 1436→1432`). Путь `apply_hide` в диф не входит, подтверждено живьём.
- Бар — отдельный модуль (`bar.toml`, свой watcher), не тронут.

## Находка (не регрессия T307, в cold и hot одинаково)

При **открытой левой панели** матте сдвигается вправо (x=48/56 в
зависимости от ширины панели) — отрицательный left-margin `-inset`
компенсирует только резервацию `ExclLeft`, а не левой панели. Это
поведение T303-кода, воспроизводится **одинаково** в холодном старте и
после hot-reload (сверял оба пути): мой фикс ничего не меняет в
геометрии, только пересоздаёт сетап по тем же `wrap_window_options`.
Выношу как наблюдение — отдельный тикет, если владелец сочтёт нужным
(зона T303-геометрии, не T307).

## Честные оговорки

- Коммит не делал (в брифе раздела «Коммит» нет). В дереве только
  `frame.rs`.
- `LAST_WRAP_GEOMETRY` — process-static; `f32`-равенство по санитизированным
  значениям (оба клампнуты) — сравнение точное, клейких дробей нет (TOML
  `16`/`16.0` дают один и тот же f32).
- Переоткрыт сетапа на edit — разовое мигание кольца/перерасчёт
  exclusive-зон; это сам смысл фикса, частота = частота правок конфига.

## Файлы

- `crates/app/src/frame.rs` — единственный изменённый файл.

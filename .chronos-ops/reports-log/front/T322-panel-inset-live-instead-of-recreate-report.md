# T322 — панели перестают пересоздавать поверхности при смене стиля рамки

**Роль:** FRONTEND. **Вердикт от исполнителя:** зона выполнена, но критерии
приёмки заблокированы отдельным багом в `frame.rs` + форке (вне зоны) —
детальным, с точной строкой. Нужно решение владельца.

## Что сделано (в зоне)

`side_panel_left::apply_frame_inset` и `side_panel_right::apply_frame_inset`
больше не делают `close()` + `open_pinned()`. Вместо этого обе поверхности
(rail + content) ресайзят высоту живьём через `window.resize`:

```rust
let rail_res = rail.update(cx, |_, window: &mut Window, _| {
    window.resize(Size::new(px(tabs::RAIL_WIDTH), px(new_h)));
});
let content_res = content.update(cx, |_, window: &mut Window, _| {
    window.resize(Size::new(px(tabs::CONTENT_CANVAS_WIDTH), px(new_h)));
});
```

`new_h = panel_height(...)` — единственное, что меняется при Hide↔Wrap.
Состояние (pinned/width/dock) и `rail_mapped` больше не трогаются — каскад
`close+open` панелей убран. При неудаче resize пишется `warn`, поверхности
остаются на месте (не полу-состояние).

Тест `window_options_have_no_resize_calls` (T278) уточнён: запрет
`window.resize` остался на drag-пути (`workspace_view.rs`, `rail_view.rs`
полностью, `mod.rs` — всё кроме `apply_frame_inset`). Исключение — одна
функция, сканер пропускает её тело по балансу скобок.

## Ключевой факт №1: margin НЕ меняется, меняется только высота

Бриф писал «при смене стиля меняется margin (инсет края) и высота». Это не
так. Боковая компонента content-margin =

```
RAIL_WIDTH + frame::wrap_inset_left_cached(true)   // левая панель
RAIL_ONLY_WIDTH + frame::wrap_inset_right_cached(true)  // правая
```

а `wrap_inset_left/right(cfg, rail_mapped=true)` сворачивается в `0.0` **в
обоих стилях** — см. `frame.rs:558-585`, закреплено тестом
`wrap_per_edge_insets_follow_rail_mapping` (`wrap_inset_left(&wrap, true) ==
0.0`, `wrap_inset_left(&hide, false) == 0.0`). Rail-окно имеет `margin: None`
(T310 D1). Значит живого сеттера margin не нужно вообще — хватает
`window.resize` по высоте (`wrap_inset_bottom` даёт дельту `wrap.bottom`).

**Факт про margin в форке:** публичного живого сеттера margin нет. `margin`
применяется только при создании поверхности,
`Source/gpui_linux/src/linux/wayland/window.rs:170-180`
(`layer_surface.set_margin(...)`), единственный раз — в
`WaylandSurfaceState::new()`. В `WaylandSurfaceState`/`WaylandWindow`/`Window`
нет `set_margin`. Живые сеттеры есть только `set_exclusive_zone`
(`window.rs:2005`, wayland `window.rs:1848`), `set_exclusive_edge`
(`window.rs:2011`, wayland `window.rs:1860`), `resize` (`window.rs:2318`,
wayland `window.rs:1520`).

## Ключевой факт №2: падение — НЕ гонка, а нулевой размер в wp_viewport

Живое воспроизведение (release, `style = "wrap"`): открытие **одной** левой
панели роняет адаптер без всякого style-перехода. Точная ошибка из лога:

```
wp_viewport#106: error 1: Size was <= 0
```

Дальше флуд `Protocol error 1 on object wp_viewport@106` (замерено
9 248 113 строк), следующий `open_window` даёт `Adapter ... not compatible`,
`hyprctl layers` пустеет.

Цепочка (детерминированная, не гонка):

1. `side_panel_left::open_window` → `frame::set_rail_mapped(Left, true)`.
2. `frame::apply` → `apply_wrap` → `sync_wrap_surfaces` (`frame.rs:1152`).
3. Для `ExclLeft` цель `Size::new(px(inset_left), px(h))`, где
   `inset_left = wrap_inset_left(cfg, true) == 0.0` (рейл замаплен) →
   **resize до ширины 0** (лог: `frame: ExclLeft geometry synced zone=0`).
4. Форк: `WaylandWindow::resize` (`window.rs:1520`) **клампит** размер для
   `set_geometry` — `map_size(|v| if v <= 0 { 1 } else { v })`
   (`window.rs:1553`), но шлёт **сырой** размер в
   `viewport.set_destination(f32::from(size.width) as i32, ...)`
   (`window.rs:1335`). `set_destination(0, h)` — протокольное нарушение
   `wp_viewport` (`invalid_size`, «Size was <= 0»), соединение убито.

То же самое заложено и на открытии: `wrap_window_options` (`frame.rs:983`)
открывает `ExclLeft` с `Size::new(px(inset_left), px(h))` — при замапленном
рейле это ноль уже в `window.open`.

Обе гипотезы брифа T322 уточнены: это не «гонка close/open» и не
«исчерпание адаптера», а **нулевой размер, который форк пропускает в
wp_viewport без клампа** (кламп есть только на ветке `set_geometry`).

## Верификация

- `cargo test -p chronos --lib` → **609 passed, 0 failed**.
- Тест реально ловит: временно отключил skip `apply_frame_inset` → тест
  краснеет дословно на `mod.rs:487` (`window.resize(Size::new(px(tabs::RAIL_WIDTH)...)`).
  Вернул skip → зелёный. Драг-путь (`workspace_view.rs`/`rail_view.rs`) по-прежнему
  полностью запрещён.
- При прогоне 35 тестов временно падали с `No space left on device` — это
  был переполненный tmpfs `/tmp` (мой лог T321 на 1.2 ГБ от флуда
  protocol-ошибок). Удалил свой лог, перегнал — зелёные. К правкам отношения нет.

## Чего НЕ смог проверить живьём

Критерий №1 брифа («20 переходов при ОБЕИХ панелях, grep = 0») **заблокирован**
фактом №2: нельзя даже открыть панели при `style = "wrap"` — открытие первой
уже валит адаптер через `sync_wrap_surfaces`. Это `frame.rs` + форк, вне
заявленной зоны («не трогать frame.rs», «в форк не полез»).

## Что требуется решить

Один из двух (оба вне зоны брифа):

1. **`frame.rs`** — `sync_wrap_surfaces` и `wrap_window_options` не должны
   давать нулевой размер полосы (кламп ≥1px, либо при нуле ресайз не делать,
   ставить только `set_exclusive_zone(0)`).
2. **Форк** — `viewport.set_destination` в `set_size_and_scale`
   (`window.rs:1335`) клампить так же, как уже клампится `set_geometry`
   (`window.rs:1553`).

После любого из них мой фикс панелей должен дать «0 ошибок» на критерии
брифа — resize панелей всегда на ненулевой размер (`RAIL_WIDTH=40`,
`CONTENT_CANVAS_WIDTH=920`, `new_h.max(100.)`).

---

# ПРИЁМКА АРХИТЕКТОРА — ПРИНЯТ. ГЛАВНОЕ: ЭТО БЫЛ МОЙ РЕГРЕСС (2026-08-19)

Код: `c51dbee8` (эррата архитектора в `frame.rs`) + `4a52e091` (правка
панелей исполнителя).

## Диагноз исполнителя верен, и он и есть главный результат

Обе гипотезы, которые я записал в бриф, опровергнуты — правильно и с
точной строкой:

- **не гонка close/open** и не исчерпание адаптера;
- падение детерминированное: при мапленном рельсе
  `wrap_inset_left/right(cfg, true) == 0`, `sync_wrap_surfaces` ресайзит
  полосу в **нулевую ширину**, а форк клампит размер только на ветке
  `set_geometry` (`Source/gpui_linux/.../wayland/window.rs:1553`,
  `map_size(|v| if v <= 0 { 1 } else { v })`) — в
  `viewport.set_destination` (`:1335`) уходит сырое значение.
  `set_destination(0, h)` = протокольное нарушение `wp_viewport`
  («Size was <= 0»), соединение убито, шелл теряет все поверхности.

Второй факт, тоже правильный: **живой сеттер `margin` в форке
отсутствует** — `layer_surface.set_margin` вызывается один раз в
`WaylandSurfaceState::new()`; живые сеттеры только `set_exclusive_zone`,
`set_exclusive_edge`, `resize`. Проверено, ссылки в отчёте точные.

И третий: боковая компонента `content_window_margin` при мапленном
рельсе сворачивается в ноль **в обоих стилях**, то есть при Hide↔Wrap
меняется только высота — живого margin и не требовалось.

## Мой регресс, воспроизведён и починен

**Проверил живьём:** на дереве после T321 одно открытие левой панели при
`style = "wrap"` убивает шелл — `0 слоёв`, `Size was <= 0` = 1,
`Protocol error` = **1 441 388**. Откатил `frame.rs` на `bbf61f02^`,
пересобрал: та же операция даёт 7 живых слоёв и ноль ошибок.

Значит это **регресс T321**, который я принял. В приёмке T321 я проверил
смену геометрии (`geometry synced`, `closed wrap surface = 0`) и **ни
разу не открыл после неё панель**. Ровно тот провал, который сам двумя
тикетами раньше записал в дисциплину: числа сошлись, результат не
посмотрел.

Эррата `c51dbee8`: в `sync_wrap_surfaces` нулевой размер больше не идёт
в `resize` (двигается только эксклюзивная зона), в `wrap_window_options`
размеры полос клампятся до 1px — зона при этом остаётся сырой, то есть
резервация ноль сохраняется.

## Критерий №1 брифа выполнен

20 переключений `frame.style` шагом 400 мс при ОБЕИХ открытых панелях:

```
слоёв живо: 9 (matte, bar, 3 excl, 2 рельса, 2 контента)
failed to open: 0    not compatible: 0    Protocol error: 0    Size was <= 0: 0
```

**Изоляция:** прогнал тот же сценарий с моей эрратой, но БЕЗ правки
панелей (`git stash`) — результат тот же, 9 слоёв и нули. То есть
падение чинит эррата, а не устранение каскада. Записываю честно, чтобы
в истории не осталось ложной причинно-следственной связи.

## Почему правка панелей всё равно принята

Она не чинит падение, но она правильная сама по себе: убирает
`close + open` двух поверхностей на каждый style-переход в пользу живого
`window.resize` — третий случай применения того же принципа (T314, T321,
теперь T322). Меньше пересозданий — меньше поверхности для будущих
граблей того же класса. Плюс уточнён сканер `window_options_have_no_resize_calls`:
запрет `resize` остался на драг-пути, исключение точечное и проверено
на покраснение.

`cargo test -p chronos --lib` 609/609 прогнан архитектором.

## Что остаётся форку

Кламп в `viewport.set_destination` (`window.rs:1335`) по-прежнему
отсутствует — мы обошли это со своей стороны, но любая другая
поверхность, которую попросят сжаться в ноль, повторит трюк. Кандидат в
тикет по `Source/gpui`, когда дойдут руки; в `TBD.md`.

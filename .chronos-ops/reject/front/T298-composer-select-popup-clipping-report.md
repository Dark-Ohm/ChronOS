# T298 — composer `Select` popup clipping

## Что сделано

`crates/app/src/side_panel_left/composer.rs` — оба кита-пикера получили
`.menu_width(...)`:

- `model_picker`: `.menu_width(px(280.))` (триггер 150px)
- `mode_picker`: `.menu_width(px(200.))` (триггер 90px)

Зона соблюдена: правка только в `composer.rs`, кит
(`../Source/gpui-component/`) и `workspace_view.rs` не тронуты.

## Почему это правильный фикс (подтверждено кодом, не гипотеза)

Кит `Select` рендерит дропдаун так (`Source/gpui-component/crates/ui/src/select.rs`):

```rust
.map(|this| match self.state.menu_width {
    Length::Auto => this.w(bounds.size.width + px(2.)),  // ← ширина триггера + 2px
    Length::Definite(w) => this.w(w),
})
```

`menu_width` по умолчанию — `Length::Auto`, а `bounds` — это bounds
триггера (захватывается `on_prepaint` в `SelectState::render`). Значит
дропдаун модели был шириной **152px** (триггер 150 + 2), режима —
**92px** (90 + 2). Длинные имена моделей/режимов резались в этой
колонке — это и есть «содержимое обрезано». Бриф §3 подтверждает этот
диагноз («если ширина попапа = ширине триггера — переопределить»), и
`Select::menu_width` — штатный способ кита задать ширину меню независимо
от триггера. Сделано.

## Что выяснил про позиционирование (важно — расходится с гипотезой брифа)

Бриф предполагал, что «курсор передаётся как anchor point по умолчанию».
**В коде этого нет.** Якорь кита жёстко зашит в `select.rs`:

```rust
deferred(
    anchored().snap_to_window_with_margin(px(8.)).child(...)
).with_priority(1)
```

`anchored()` (`Source/gpui/src/elements/anchored.rs`) по умолчанию:
`anchor = Anchor::TopLeft`, `position_mode = Window`,
`anchor_position = None`. При `None` якорная точка = `bounds.origin`
анкор-элемента (позиция самого элемента в лэйауте), **а не курсор**.
`Bounds::from_anchor_and_size(TopLeft, …)` кладёт top-left попапа в эту
точку — т.е. попап открывается **вниз от триггера**, а
`snap_to_window_with_margin(px(8.))` при выходе за низ окна **сдвигает его
вверх** (не флипует угол, а двигает, чтобы низ был `viewport.bottom − 8`).

Точка якоря в коде кита **не настраивается извне**: `Select` не
предоставляет опции «anchor»/«direction»/«open upward». Единственный
рычаг в зоне (`composer.rs`) — `menu_width` и `menu_max_h`. Поэтому
переключение «на якорь от `composer-*-picker-wrap` вверх» из `composer.rs`
**невозможно** без правки самого кита (`select.rs`), который вне зоны
тикета и вне репозитория ChronOS.

Гипотеза брифа §4 (обрезка от `workspace_view.rs` `overflow_hidden`) —
**опровергнута кодом**: попап рисуется через `deferred`, чей `prepaint`
вызывает `window.defer_draw(child, element_offset, …)` — отложенный
проход отрисовки **вне** пайнт-пасса предков, т.е. `overflow_hidden`
клипа `side-panel-left-product-clip` его не режет. `workspace_view.rs`
не трогал (нечего чинить).

## Верификация

- `cargo check -p chronos` — exit 0, по `composer.rs` ошибок/новых
  warning нет.
- `cargo build --release -p chronos` — `Finished release profile`, exit 0.

## Что НЕ сделал (честно)

- **Live grim не гонял** — в этой сессии нет живого Wayland/Hyprland.
  Пункт «весь попап и список видны целиком, длинные имена не режутся,
  попап внутри границы окна» **не подтверждён глазами**. Ширина-фикс
  подтверждён только сборкой + чтением кода кита.
- **Позиционирование не чинил.** Симптом «попап по центру курсора /
  уезжает за низ монитора» не воспроизводится из кода: якорь кита —
  от триггера, снэп в пределах `window.viewport_size()`. По геометрии
  левого контент-окна (`content_window_options`: height =
  `display_height − bar_height`, margin top = `bar_height`, bottom = 0)
  низ окна = низ монитора, а снэп держит попап в `viewport − 8px`, т.е.
  должен оставаться над нижней кромкой. Либо симптом снимается
  шириной-фиксом (узкий попап визуально «болтался» на клике и выглядел
  обрезанным), либо это отдельный баг координатного пространства
  `deferred`/`viewport_size` на layer-shell, который виден только в
  живом дебаге. Для него нужен grim с подписанными границами (бриф §1).

## Рекомендация владельцу

Прогнать live grim после ширины-фикса. Если попап всё ещё уезжает вниз
за монитор — баг в ките (`anchored` + `snap_to_window_with_margin` на
layer-shell окне), и чинить его надо либо в `select.rs` кита (вне зоны
тикета), либо заменить кит-дропдаун на нативный anchored-popup (как
sessions T287-B), но это уже отдельный тикет/решение по scope.

## Коммит

`fix(left-panel): composer Select popup no longer clipped (T298)`

# Eval: примеры форка gpui

**Когда грузить:** после изучения `references/examples-catalog.md` — проверить усвоение.

---

## Вопрос 1

**Q:** Какой метод нужен для скролла в форке gpui, и какое обязательное precondition?

**A:** `overflow_scroll()` — требует `.id("...")` перед вызовом, потому что метод живёт в `StatefulInteractiveElement`, реализованном только для `Stateful<E>`.

**Доказательство:** `Source/gpui/examples/scrollable.rs:12-14` — `.id("vertical").p_4().overflow_scroll()`.

---

## Вопрос 2

**Q:** Как создать layer-shell окно в форке? Назови минимум 3 обязательных поля.

**A:** `WindowKind::LayerShell(LayerShellOptions { namespace, anchor, margin, keyboard_interactivity, .. })`. Обязательные: `namespace` (строка), `anchor` (битовая маска `Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM`), `keyboard_interactivity` (`None`/`OnDemand`/`Exclusive`).

**Доказательство:** `Source/gpui/examples/layer_shell.rs:87-93` — полный пример.

---

## Вопрос 3

**Q:** Как получить список всех дисплеев в форке gpui?

**A:** `cx.displays()` — возвращает итератор по `DisplayId` и bounds каждого экрана.

**Доказательство:** `Source/gpui/examples/window_positioning.rs:82` — `for screen in cx.displays() { ... }`.

---

## Вопрос 4

**Q:** Чем отличается `.focus(|style| ...)` от `.focus_visible(|style| ...)`?

**A:** `.focus()` показывает стиль всегда при фокусе (и от мыши, и от клавиатуры). `.focus_visible()` — только когда фокус получен через клавиатуру (Tab).

**Доказательство:** `Source/gpui/examples/focus_visible.rs:152` — `.focus_visible(|style| style.border_4().border_color(...))`.

---

## Вопрос 5

**Q:** Как создать плавающий попап, который позиционируется относительно элемента и не обрезается границами окна?

**A:** `deferred(anchored().anchor(Anchor::...).snap_to_window_with_margin(px(...)).child(...))`.

**Доказательство:** `Source/gpui/examples/popover.rs:99-103` — `deferred(anchored().anchor(Anchor::TopLeft).snap_to_window_with_margin(px(8.)).child(popover()...)`.

---

## Вопрос 6

**Q:** Как создать виртуализированный список из 10000 элементов?

**A:** `uniform_list("id", 10000, cx.processor(|this, range, _window, _cx| { ... })).h_full()`.

**Доказательство:** `Source/gpui/examples/uniform_list.rs:14-37` и `data_table.rs:429-441`.

---

## Вопрос 7

**Q:** Как запустить анимацию, которая повторяется бесконечно?

**A:** `Animation::new(Duration::from_secs(2)).repeat().with_easing(bounce(ease_in_out))` + `.with_animation("id", anim, |el, delta| ...)`.

**Доказательство:** `Source/gpui/examples/animation.rs:78-86`.

---

## Вопрос 8

**Q:** Какой статус у `gpui_elements` в форке на 2026-07-20?

**A:** Исключён из workspace. `cargo check -p gpui_elements` → «did not match any packages». Прямой check из каталога → «believes it's in a workspace when it's not».

**Доказательство:** проверено в этой сессии — вывод `cargo check` подтверждает статус.

# T206 report

**Зона:** `crates/app/src/side_panel_right/{view,rail}.rs`. `tab/preview*`,
`tab/bar_settings*`, `bar/`, левая панель — не тронуты.

**Предшественник:** T204 `96c40d4` + errata `1d9b71b`. Эррата архитектора
(live) ломала resize: overlay absolute handle не ловил drag, новая формула
`width = right_abs − pointer_abs` давала snap-back в ~36 после rail→expand.

## Symptoms → root cause

| symptom | root cause | fix |
|---|---|---|
| **Gray lip** (серая полоса рядом с рейлом) | body div всегда имел `bg(surfaces::chrome)` — в rail-only хром просвечивал за handle | body bg + border — только когда `content_open`; rail сам даёт фон |
| **Hairline в rail-only** | `render_rail` всегда рисовал `border_l_1()` | border на rail — только когда `content_open` |
| **Snap/dead resize** (после rail→expand ширина схлопывалась в ~40) | Right-anchored окно растёт ВЛЕВО → курсор смещается внутри окна вправо на `(target - w)` px. `resize_start_x` не корректировался → `update_resize` вычислял `560 - (522 - 2) = 40` | `start_resize` после expand: `resize_start_x = start_x + (target - w)` |
| **Overlay unhittable** | absolute transparent handle без надёжного hit-path (эксперимент эрраты) | flex-колонка 4px — `on_drag` на реальном div (уже было, не трогал) |

## Changes

### `crates/app/src/side_panel_right/view.rs`

**`start_resize` — snap fix:**
Порядок изменён: rail→expand блок — теперь первый, `resize_start_x`
корректируется на смещение окна. Обычный (не rail) resize — в `else`.

```rust
// Было: resize_start_x выставлялся ДО expand, не корректировался после.
// Стало:
if w <= RAIL_ONLY_WIDTH + 1.0 {
    // expand to target …
    self.resize_start_width = Some(target);
    self.resize_start_x = Some(start_x + (target - w)); // ← КОРРЕКЦИЯ
} else {
    self.resize_start_x = Some(start_x);
    self.resize_start_width = Some(w);
}
```

**body bg — gray lip fix:**
```rust
// Было:
.bg(surfaces::chrome(&theme))
.when(content_open, |b| {
    b.border_l_1().border_color(theme.border.default)
})

// Стало:
.when(content_open, |b| {
    b.bg(surfaces::chrome(&theme))
        .border_l_1()
        .border_color(theme.border.default)
})
```

Тело прозрачное в rail-only — rail даёт свой фон через `render_rail`.
Хром + хейрлайн появляются только когда контент открыт.

**`render_rail` вызов:**
Добавлен параметр `content_open` (см. rail.rs ниже).

### `crates/app/src/side_panel_right/rail.rs`

**Сигнатура:**
```rust
pub fn render_rail(
    cx: &App,
    tabs: &[PanelTab],
    active: PanelTab,
    on_select: Rc<dyn Fn(PanelTab, &mut Window, &mut App) + 'static>,
    dock_content: bool,
    on_dock_toggle: Rc<dyn Fn(&mut Window, &mut App) + 'static>,
    content_open: bool,  // ← новый параметр
) -> impl IntoElement {
```

**border:**
```rust
// Было:
.bg(surfaces::chrome(theme))
.border_l_1()
.border_color(theme.border.default)

// Стало:
.bg(surfaces::chrome(theme))
.when(content_open, |r| {
    r.border_l_1().border_color(theme.border.default)
})
```

## Verification

```
$ cargo check -p chronos
0 errors

$ cargo test -p chronos --lib
test result: ok. 219 passed; 0 failed
```

Тесты `side_panel_right::` — все проходят (включая `rail_only_default_width`,
`drag_left_grows_right_anchored_width`, `mode_fallback_*`).

## Residual (known, accepted)

- **One-frame jank на rail→expand:** если `DragMoveEvent` приходит до того,
  как `render()` физически ресайзнул окно, координаты курсора будут в старом
  40px-окне, а `resize_start_x` уже скорректирован под новую ширину.
  `state.resize()` клампит к `RAIL_ONLY_WIDTH..MAX_WIDTH` — один кадр
  перескока, дальше окно ресайзится и всё ок. Тот же класс гонки, что левая
  панель принимает.

## Что НЕ сделано

- **Живой смок** — LIVE NOT VERIFIED (нет интерактивной сессии в этой работе).
  Статика зелёная: check + тесты. Требуется release-бинарь + визуал:
  rail-only → drag left expands, drag right shrinks, no snap-to-36,
  no gray lip, left panel unchanged.
- **Тест на коррекцию `resize_start_x`** — unit-тест требует GPUI-окружения
  с entity update + DragMoveEvent. Существующий `drag_left_grows_right_anchored_width`
  покрывает чистую delta-математику; коррекция — механика `start_resize`,
  проверяется live.
- **Левый handle** — не трогал (там transparent уже был, inner edge —
  lower risk по T204).

## Acceptance

- [x] Handle 4px flex hit strip, transparent (no gray lip)
- [x] Rail border only when content_open
- [x] Body bg only when content_open (rail provides own bg)
- [x] Resize delta math: `start_w - (current_x - start_x)` — local only
- [x] Rail→expand корректирует `resize_start_x` — no snap-to-40
- [x] Check чистая, 219 тестов зелёные
- [x] Зона: только view.rs + rail.rs (подтверждено git diff)
- [ ] LIVE smoke — не проверено

---

## Приёмка

**Коммит:** `panels : right resize stick + transparent handle (T206)`.

**Вердикт:** ACCEPTED (статика). Живой смок — отдельным прогоном release-бинаря.

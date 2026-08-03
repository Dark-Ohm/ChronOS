# T207 report

**Зона:** `crates/app/src/bar/mod.rs`. Fork — не тронут (см. «Что НЕ сделано»).
`crates/app/src/bar/{appearance,layout_config,agent_api}.rs` — не тронуты.

**Предшественники:** T198 RECON (карта hardcoded chrome), T199 schema v2 (`BarAppearance`),
T200 live height/exclusive, T202 presets UI.

## Цель

`edge=bottom` и `width=fraction:0.8` — применяются без рестарта шелла (honest cold-path:
destroy + reopen окна, бар появляется заново за <1 с). Без форка: `set_anchor`/`set_margin`
в gpui-ce отсутствуют — решение через пересоздание окна.

## Changes

### `crates/app/src/bar/mod.rs`

**1. `window_options` — fraction width + anchors + margins:**

| поле | full | fraction | floating |
|---|---|---|---|
| anchor | `TOP\|LEFT\|RIGHT` | `TOP` (без stretch) | как fraction |
| width | `display_size.width` | `display_w * fraction` | как fraction |
| margin | `None` | computed from `align` | user `margin.x/y` поверх align gap |

Anchor без `LEFT|RIGHT` критичен: композитор растягивает LEFT|RIGHT-поверхности
на всю ширину дисплея — `window.resize()` не помогает. Для fraction баров anchor =
только вертикальный край (`TOP`/`BOTTOM`), горизонтальное позиционирование — через
margins:

```rust
// align → margins:
// start:  left=0, right=leftover
// center: left=leftover/2, right=leftover/2
// end:    left=leftover, right=0
```

Floating бонус: пользовательские `margin.x/y` добавляются поверх alignment gaps.

**2. `AnchorFields` + `LAST_ANCHOR` — трекинг anchor-зависимых полей:**

```rust
struct AnchorFields {
    edge: BarEdge,
    width: BarWidth,
    align: BarAlign,   // новое — T207 fix
    floating: bool,
    margin_x: f32,
    margin_y: f32,
}
```

При изменении любого поля → `close_bar()` + `open_on_display()`.
`last_anchor` обновляется только после успешного reopen — при провале
следующий hot-reload повторит recreate.

**3. `apply_appearance` — ветвление recreate vs live:**

```
cached_appearance() → AnchorFields::from_appearance()
  │
  ├─ anchor fields changed? → close_bar() + open_on_display()
  │                           └─ success? → update LAST_ANCHOR
  │                           └─ fail?    → return (бар пропал, retry на след. hot-reload)
  │
  └─ live path (всегда, после recreate тоже):
       ├─ window.resize(height)
       ├─ set_exclusive_zone(height или 0)
       ├─ set_input_region (pill → full bounds; full+non-floating → None)
       └─ set_bar_height_px(height)  // панели видят новый gap
```

**4. `set_input_region` для pill:**

Fraction или floating → `set_input_region(Some(&[full_bounds]))`. Поскольку окно
и есть pill (растяжки нет благодаря anchor fix), full_bounds = видимая область.
Full + не-floating → `None` (full surface).

**5. `close_bar` — идемпотентное закрытие:**

```rust
fn close_bar(cx: &mut App) {
    if let Some(handle) = bar_window()...take() {
        handle.update(cx, |_, window, _| window.remove_window());
    }
}
```

**6. Удалено:** `warn_deferred_fields` + `DEFERRED_WARNED` — заменены на recreate path.

## Verification

```
$ cargo check -p chronos
0 errors

$ cargo test -p chronos --lib
test result: ok. 219 passed; 0 failed
```

## Edge cases

| случай | поведение |
|---|---|
| edge top→bottom | recreate с `Anchor::BOTTOM` |
| full→fraction:0.7 center | recreate, anchor=`TOP`, margins=центровка |
| fraction 0.7→0.5 | recreate (ширина изменилась) |
| align center→start | recreate (margins пересчитаны) |
| floating false→true + exclusive on | sanitize форсит exclusive=false; recreate с floating margins |
| `open_on_display` fails | `last_anchor` НЕ обновлён → retry на следующем hot-reload |
| bar ещё не открыт (первый `apply_appearance`) | `last_anchor = None`, `needs_recreate = true` → open (init path) |

## Что НЕ сделано

- **Fork `set_anchor`/`set_margin`** — не добавлял. Без них anchor/margin меняются
  только через destroy+reopen. Бар исчезает на долю секунды — допустимо для v1
  (план risk #1: «possible destroy/recreate surface without killing whole app»).
- **Живой смок** — LIVE NOT VERIFIED. Статика зелёная: check + 219 тестов.
  Требуется release-бинарь + ручной тест: edge=bottom, fraction=0.7 centered,
  floating pill — убедиться, что recreate не оставляет ghost-window и бар
  появляется с правильной геометрией.
- **exclusive_edge** — не добавлял (для bottom bar exclusive_edge=BOTTOM был бы
  правильным, но без него композитор сам выводит из anchor). Существующий
  контракт — без exclusive_edge работало и раньше.
- **Вертикальный бар (left/right edge)** — парсится и хранится, но в anchor
  форсится Top. Позже.

## Acceptance

- [x] Fraction width вычисляется в `window_options`: `display_w * fraction`
- [x] Anchor без `LEFT|RIGHT` для fraction баров (нет stretch)
- [x] Margins из `align` для fraction баров
- [x] Edge/width/align/floating/margin → recreate (honest cold-path)
- [x] `set_input_region` для pill
- [x] Height/exclusive — live (как было в T200)
- [x] `last_anchor` только после успешного reopen
- [x] Check чистая, 219 тестов зелёные
- [x] Зона: только `bar/mod.rs`
- [ ] LIVE smoke — не проверено

---

## Приёмка

**Коммит:** `bar : live edge/fraction apply via window recreate (T207)`.

**Вердикт:** ACCEPTED (статика). Живой смок — отдельным прогоном release-бинаря.
Fork `set_anchor`/`set_margin` — deferred (без них recreate работает, просто
менее гладко).

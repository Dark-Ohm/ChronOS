# Elements, styling, layout, scroll

**When to load:** before claiming the fork "can't" scroll, cap height, virtualize a list, truncate text, or use a Style method that "doesn't resolve". Every claim below is pinned to `Source/` (`../Source` from ChronOS) or a runnable example.

**Scope of this file:** `gpui` elements + style + layout units. Not windowing/layer-shell (see `windowing-platform.md`). Not executors/globals (see `state-async-executors.md`).

**Package specifier for checks** (required — bare `-p gpui` is ambiguous in the Source workspace):

```bash
cargo check --example scrollable \
  -p 'path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2'
```

---

## 1. Trait hierarchy (why "method not found" is usually not a missing feature)

Three independent layers share one style memory on `Div`, but **methods live on different traits** with different impl targets.

| Layer | Trait | Supertrait | What it unlocks | Impl targets (gpui) |
|---|---|---|---|---|
| Style | `Styled` | `Sized` | Tailwind-like layout/visual API via `style() -> &mut StyleRefinement` | `Div`, `Stateful<E>` (if `E: Styled`), `UniformList`, `List`, `Img`, … |
| Interact (stateless) | `InteractiveElement` | `Sized` | `id()`, hover, mouse/key, `on_scroll_wheel` | `Div`, `Stateful<E>`, `UniformList`, `Img`, `Svg` |
| Interact (stateful) | `StatefulInteractiveElement` | `InteractiveElement` | **`overflow_*_scroll`**, **`track_scroll`**, `on_click`, `active`, a11y | **`Stateful<E>` only** (+ exception: `Img`) — **not bare `Div`** |

Sources:

- `InteractiveElement` — `Source/gpui/src/elements/div.rs:699`
- `.id(...)` returns `Stateful<Self>` — `div.rs:710-714`
- `StatefulInteractiveElement` — `div.rs:1213`
- scroll methods — `div.rs:1416-1438`
- empty impl unlocking defaults — `div.rs:3752-3757`
- `Styled` — `Source/gpui/src/styled.rs:22-34`
- `Div` implements `Styled` + `InteractiveElement` only — `div.rs:1689-1699`

### Educational case: scroll

```rust
// DOES NOT COMPILE — overflow_y_scroll is on StatefulInteractiveElement;
// there is no impl StatefulInteractiveElement for Div.
div().overflow_y_scroll()

// WORKS — .id() changes the type to Stateful<Div>, which implements the trait.
div().id("log").overflow_y_scroll()
```

| Step | Expression | Type | Available traits |
|---|---|---|---|
| 1 | `div()` | `Div` | `Styled` + `InteractiveElement` |
| 2a | `.overflow_y_scroll()` | **error** | method not in scope for `Div` |
| 1′ | `div().id("x")` | `Stateful<Div>` | + `StatefulInteractiveElement` |
| 2′ | `.overflow_y_scroll()` | ok | default method at `div.rs:1429` |

Also: `.id()` stores `element_id` on `Interactivity` (`div.rs:711`, comment at `div.rs:1948-1949`) — required for the stateful subset (click, scroll offset persistence via element state).

### What does *not* need `.id()`

On plain `Div` (via `InteractiveElement`): `hover`, `on_mouse_*`, `on_key_*`, `on_scroll_wheel`, `track_focus`, tab helpers.

On plain `Div` (via `Styled`): `flex`, `p_4`, `w_full`, `max_h(...)`, `overflow_hidden`, colors, text helpers.

### Exception

`Img` implements `StatefulInteractiveElement` without the `Stateful` wrapper (`Source/gpui/src/elements/img.rs` — verified by recon; Img can call stateful methods without `.id()`). Do not generalize this to `Div`.

---

## 2. Scroll — full picture

### Setting overflow to scroll

| Method | Trait | Lines | Effect |
|---|---|---|---|
| `overflow_scroll()` | `StatefulInteractiveElement` | `div.rs:1416-1420` | `overflow.x/y = Scroll` |
| `overflow_x_scroll()` | same | `div.rs:1423-1426` | x only |
| `overflow_y_scroll()` | same | `div.rs:1429-1432` | y only |
| `track_scroll(&ScrollHandle)` | same | `div.rs:1435-1438` | share offset/state with a handle |
| `anchor_scroll(...)` | same | `div.rs:1440-1444` | nested non-immediate-child scroll target |
| `overflow_hidden` / `_x_` / `_y_` | **`Styled`** (macro) | `gpui_macros/src/styles.rs:135-151` | Hidden only — **not scroll** |
| `scrollbar_width(len)` | `Styled` | `styled.rs:68-71` | layout reservation when overflow is Scroll |
| `on_scroll_wheel` | `InteractiveElement` | `div.rs:969+` | listener only; does not implement scrolling |

`Overflow` enum values: `Visible` (default), `Clip`, `Hidden`, `Scroll` — `style.rs:1204-1219`. Styled helpers only set **Hidden**. Scroll is set only through `StatefulInteractiveElement` (or by writing style manually).

### Minimal working scroll (example)

Proven by `Source/gpui/examples/scrollable.rs` (`cargo check` green 2026-07-20):

```rust
div()
    .size_full()
    .id("vertical")      // required: unlocks StatefulInteractiveElement
    .overflow_scroll()   // both axes
    .child(/* tall content */)
```

Nested independent axes: outer `.id("vertical").overflow_scroll()`, inner `.id("horizontal").overflow_scroll()` — same example.

### `ScrollHandle` — programmatic control

Defined `div.rs:3920`. Hold on the view, pass with `.track_scroll(&self.handle)`.

| Method | Lines | Role |
|---|---|---|
| `new()` | 3930 | construct |
| `offset()` / `max_offset()` | 3935 / 3940 | current / max scroll (offset is more negative as you scroll down) |
| `top_item()` / `bottom_item()` | 3945 / 3964 | visible child indices (needs tracked handle so Div fills `child_bounds`) |
| `bounds()` / `bounds_for_item(ix)` | 3983 / 3988 | viewport / child bounds |
| `scroll_to_item(ix)` | 3993 | ensure child visible (`FirstVisible`) |
| `scroll_to_top_of_item(ix)` | 4003 | pin child to top |
| `scroll_to_bottom()` | 4063 | **flag** applied next clamp: `y = -scroll_max.y` (`div.rs:2272-2273`) |
| `set_offset(point)` | 4071 | immediate write |
| `logical_scroll_top/bottom` | 4077 / 4092 | `(child_ix, pixel offset)` |

There is **no** public `ScrollHandle::scroll_to`. Nested anchors use `ScrollAnchor::scroll_to` (`div.rs:3881`).

### Autoscroll to bottom (terminal log / streaming)

**Plain scrollable div** (one-shot stick-to-bottom after content grows):

```rust
// view field
scroll: ScrollHandle::new(),

// render
div()
    .id("log")
    .overflow_y_scroll()
    .track_scroll(&self.scroll)
    .children(/* growing lines */),

// after notify / new line
self.scroll.scroll_to_bottom();
```

`scroll_to_bottom` is a one-shot flag (`mem::take` at `div.rs:2251`). Re-call when content grows. Continuous "follow tail until user scrolls away" is **not** on `ScrollHandle` — that is `ListState::set_follow_mode(FollowMode::Tail)` (`list.rs:613-635`).

**Variable-height list** (chat/log): `ListState` + `list(...)` with `ListAlignment::Bottom` and/or `FollowMode::Tail` + `scroll_to_end()` (`list.rs:603-611`). See example `list_example.rs`.

**Uniform-height list:** `UniformListScrollHandle::scroll_to_bottom()` (`uniform_list.rs:251-254`).

### ChronOS note

Layer-shell popups that need a height cap + scroll: combine `.max_h(px(...))` (or `window.resize` for surface size) with `.id(...).overflow_y_scroll()`. The old belief that scroll is impossible was wrong; the old skill `gpui-layer-shell` still documents the wrong limitation in places (see §Traps).

---

## 3. Style — what exists (and the `max_height` claim)

### `Style` size fields (`style.rs:180-236`)

| Field | Type | Line | Meaning |
|---|---|---|---|
| `size` | `Size<Length>` | 228 | preferred w/h |
| `min_size` | `Size<Length>` | 231 | min w/h |
| `max_size` | `Size<Length>` | 234 | **max w/h** |
| `aspect_ratio` | `Option<f32>` | 236 | optional |

There is **no** Rust field named `max_height` or `max_width`. Max height is `max_size.height`.

### VERDICT on "Style has no max_height"

| Reading | Verdict |
|---|---|
| Literal: no field `max_height` | **True** (`style.rs:234` is `max_size`) |
| Practical: "cannot cap element height" | **False** — use `.max_h(...)` / `.max_h_full()` / `.max_h(px(480.))` |

Proof chain:

1. Field: `pub max_size: Size<Length>` — `style.rs:234`
2. API: macro prefix `max_h` → `max_size.height` — `gpui_macros/src/styles.rs:899-903`
3. Layout: `max_size: self.max_size.to_taffy(...)` — `taffy.rs:496`

Same pattern for `max_w` → `max_size.width` (`styles.rs:891-896`).

### How Styled methods appear

`Styled` (`styled.rs:22-34`) mixes:

- **Macro-generated:** `style_helpers!` (w/h/size/min_*/max_*/gap ramp), margin/padding/position/overflow_hidden/border/shadow/cursor/visibility
- **Hand-written:** `flex`/`grid`/`block`/`hidden`, all `flex_*`, grid placement, text helpers (`truncate`, `text_ellipsis*`, `line_clamp`), `scrollbar_width`

`grep` for `pub fn max_h` in `gpui/src` finds nothing — the method is **generated** by `gpui_macros`. Looking only at expanded source trees without macros is how "API missing" myths start.

### Overflow helpers vs enum

| Want | Use |
|---|---|
| Clip content, no scroll | `.overflow_hidden()` (`Styled`) |
| Scrollable region | `.id(...).overflow_y_scroll()` (`StatefulInteractiveElement`) |
| Both axes scroll | `.id(...).overflow_scroll()` |

### Layout modes

| Mode | API | Evidence |
|---|---|---|
| Flex | `.flex()`, `flex_col` / `flex_row`, grow/shrink/wrap | `styled.rs:45-47`, hand-written flex section; `Display::Flex` `style.rs:1131` |
| Grid | `.grid()`, `grid_cols`/`grid_rows`, spans | `styled.rs:50-54`, grid methods; taffy map `taffy.rs:511+` |
| Block | `.block()` | `styled.rs:36-40` |
| Absolute | `.absolute()` + insets (`top`/`left`/…) | position macro `styles.rs:100-126`; `Position::Absolute` `style.rs:1232+` |
| Hidden (no layout) | `.hidden()` → `Display::None` | `styled.rs:59-61` |

Trap: `Display`'s `#[default]` is **Flex** (`style.rs:1131`), but `Style::default()` sets **`display: Block`** (`style.rs:770`).

### Units (`geometry.rs`)

| Helper / type | Role | Lines |
|---|---|---|
| `px(f32)` → `Pixels` | absolute CSS-like px | `geometry.rs:3736-3737` |
| `rems(f32)` → `Rems` | root-em | `3723-3724` |
| `AbsoluteLength` | `Pixels \| Rems` | `3298-3303` |
| `relative(f32)` → `DefiniteLength::Fraction` | 0..1 of parent | `3705-3706` |
| `DefiniteLength` | absolute or fraction | `3460+` |
| `Length` | `Definite \| Auto` | `3611-3616` |

Style sizes use `Size<Length>` (auto allowed). Padding/gap use `DefiniteLength` (no auto).

---

## 4. Lists — naive children vs virtualization

### Naive children

```rust
div().id("x").overflow_y_scroll().children(all_items)
```

Works for small N. Every child is laid out every frame. Fine for tens of rows; bad for hundreds/thousands (updates popup "+N more" pattern is a symptom of this pressure).

### `uniform_list` — equal-height, range renderer

`Source/gpui/src/elements/uniform_list.rs`

```rust
uniform_list("entries", item_count, |range, window, cx| {
    range.map(|ix| /* row element */).collect::<Vec<_>>()
})
.h_full()
// optional:
.track_scroll(&self.scroll_handle)  // UniformListScrollHandle
```

| Fact | Evidence |
|---|---|
| Virtualizes: only visible range is built | module docs `uniform_list.rs:1-5`, layout path ~473-490 |
| Assumes **uniform** item height (measures one index, default 0) | docs + `item_to_measure_index` |
| Defaults `overflow.y = Scroll` | `uniform_list.rs:31-32` |
| Programmatic scroll | `UniformListScrollHandle::scroll_to_item` / `scroll_to_bottom` |
| Example | `examples/uniform_list.rs` (50 items); `examples/data_table.rs` (**10k** rows + custom scrollbar) |

If row heights differ → use `list`, not `uniform_list`.

### `list` — variable height, intrusive state

`Source/gpui/src/elements/list.rs`

```rust
// view field
list_state: ListState::new(count, ListAlignment::Bottom, px(500.)),

// render
list(self.list_state.clone(), |ix, window, cx| { /* item */ }.into_any_element())
```

| Fact | Evidence |
|---|---|
| Variable heights tracked in SumTree | module docs `list.rs:1-8` |
| Off-screen height must stay stable unless `splice`/`reset`/`remeasure` | same |
| Tail follow for logs | `FollowMode::Tail` `list.rs:113-119`, `set_follow_mode` `617-635` |
| Jump to end | `scroll_to_end` `603-611` |
| Example | `examples/list_example.rs` (variable heights + DIY scrollbar) |

### What replaces ChronOS "+N more"

Use **`uniform_list`** (fixed row height, e.g. package list) or **`list`+`ListState`** (variable). Both paint only the viewport. No need to invent a third virtualizer. Scrollbars are DIY in both APIs (see `data_table.rs` / `list_example.rs`).

---

## 5. Text

### Truncation / wrap (`styled.rs`)

| Method | Lines | Effect |
|---|---|---|
| `text_ellipsis()` | 87-92 | end ellipsis (`TextOverflow::Truncate`) |
| `text_ellipsis_start()` | 94-100 | start (paths) |
| `text_ellipsis_middle()` | 102-108 | middle (filenames) |
| `truncate()` | 137-141 | `overflow_hidden` + `nowrap` + end ellipsis |
| `line_clamp(n)` | 143-149 | max lines + hidden |
| `whitespace_normal` / `nowrap` | 76-85 | wrap control |

Canonical matrix: `examples/text_wrapper.rs` (`cargo check` green).

Truncation needs a **definite width** at layout time (`text.rs` layout path). Without bounded width, ellipsis won't appear.

### Font / mono

- `font_family(name)` — `styled.rs:708-711`
- `font(Font)` — `styled.rs:720+`
- No dedicated `.monospace()` helper. Use an explicit family (fork aliases `.ZedMono` in text system; ChronOS uses theme `font_mono` string separately).

### Selection

**Not in core `gpui` text elements.** `InteractiveText` is click/hover/tooltip only (`text.rs`). Selection demo lives in **gpui-component** (not used by ChronOS today):

- `Source/gpui-component/examples/text_selection/`
- Related window helpers under `gpui-component/crates/ui/src/window_ext.rs`

---

## 6. Examples catalog (this zone)

Checked with `cargo check --example <name> -p 'path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2'` on 2026-07-20 unless noted.

| Example | Proves | ChronOS-portable? |
|---|---|---|
| `scrollable.rs` | `.id` + `overflow_scroll`, nested axes | Yes (API). Layer-shell still needs surface sizing. |
| `uniform_list.rs` | equal-height virtualized list | Yes for bar/popup lists |
| `list_example.rs` | variable-height `ListState`, bottom align, DIY scrollbar | Yes for logs/chats |
| `data_table.rs` | 10k-row `uniform_list` + `track_scroll` + truncate cells | Yes as performance pattern |
| `text_wrapper.rs` | ellipsis / clamp / nowrap matrix | Yes |
| `text.rs` / `text_layout.rs` | fonts, highlights, decorations | Yes |
| `grid_layout.rs` | CSS grid + container_query layout switch | Yes for complex panels |
| `gradient.rs` / `opacity.rs` / `pattern.rs` / `painting.rs` | fills, opacity, canvas paths | Yes (visual) |
| `anchor.rs` / `popover.rs` | `anchored` floating content | Partial — ChronOS often uses separate layer-shell windows instead |

---

## 7. Ловушки и опровержения

| Мы думали | На самом деле | Доказательство |
|---|---|---|
| `overflow_y_scroll` в форке нет | Метод есть; живёт на `StatefulInteractiveElement`, нужен `.id(...)` | `div.rs:1429`, `3752`; пример `scrollable.rs` |
| `Style` не умеет max height | Нет поля `max_height`, есть `max_size.height` + `.max_h(...)` | `style.rs:234`, `styles.rs:899-903`, `taffy.rs:496` |
| `overflow_*` = scroll | `Styled` даёт только `overflow_*_hidden`; scroll — другой трейт | `styles.rs:135-151` vs `div.rs:1416-1432` |
| Любой interactive API требует `.id()` | Hover/mouse/key — на plain `Div`; click/scroll/active — после `.id()` | `InteractiveElement` vs `StatefulInteractiveElement` |
| `ScrollHandle::scroll_to` | Нет; есть `scroll_to_item` / `set_offset` / `scroll_to_bottom` | `div.rs:3928-4074` |
| `scroll_to_bottom` мгновенный | Флаг, применяется при clamp в prepaint | `div.rs:2251`, `2272-2273` |
| Длинный список = только «+N more» | `uniform_list` / `list` виртуализуют | `uniform_list.rs`, `list.rs`, `data_table.rs` |
| Text selection в gpui | В core нет; смотри gpui-component `text_selection` | `text.rs` vs `gpui-component/examples/text_selection` |
| Style-методы «не существуют» потому что grep пуст | Многие генерируются `gpui_macros` (`style_helpers!`) | `styled.rs:26`, `gpui_macros/src/styles.rs` |
| Скилл `gpui-layer-shell` — истина про max_h/scroll | Частично устарел: буквально «нет max_height» + «overflow_y_scroll не резолвится» без `.id()` | `~/.agents/skills/gpui-layer-shell/SKILL.md` description + lines 21, 48, 52 |

---

## 8. Quick recipes

**Scrollable panel with max height**

```rust
div()
    .id("panel")
    .w(px(300.))
    .max_h(px(400.))
    .overflow_y_scroll()
    .children(/* ... */)
```

**Scrollable panel with programmatic stick-to-bottom**

```rust
div()
    .id("log")
    .flex_1()
    .overflow_y_scroll()
    .track_scroll(&self.scroll)
    .children(/* ... */)
// after append:
self.scroll.scroll_to_bottom();
```

**Large equal-height list**

```rust
uniform_list("rows", n, |range, _, _| {
    range.map(|i| div().child(format!("{i}"))).collect()
})
.h_full()
```

**Ellipsis label**

```rust
div().w(px(120.)).child(
    div().truncate().child(long_string)  // or .text_ellipsis() with known width
)
```

---

## Not verified in this pass

- Live runtime of every example under Wayland (only `cargo check`).
- Interaction of `max_h` with layer-shell `window.resize` recipes (windowing file).
- Whether ChronOS already calls `.max_h` anywhere (out of zone for this recon).
- Full `uniform_list` decoration / `y_flipped` edge cases beyond API surface.

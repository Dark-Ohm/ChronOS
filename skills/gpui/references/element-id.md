# ElementId

`ElementId` is a unique identifier for a GPUI element. It is required for elements that need:
- Mouse event handling (`on_click`, `on_hover`, etc.)
- State storage via `window.use_keyed_state`
- Interaction tracking

## Making an Element Stateful

Call `.id()` on a `div()` to create a `Stateful<Div>`:

```rust
div().id("my-element")          // ElementId from &str
div().id(42usize)               // ElementId from usize
div().id(ElementId::from(idx))  // Explicit
```

Without `.id()`, a div cannot receive mouse events or store state.

## HARD RULE — interactive methods need `.id()` FIRST (verified by compilation)

`Div` implements ONLY the `InteractiveElement` trait. The interactive methods live on
`StatefulInteractiveElement` — which is implemented ONLY for `Stateful<E>`, NOT for bare
`Div`. So you must call `.id(...)` (which returns `Stateful<Div>`) before any of these:

- `on_click`, `on_mouse_down`, `on_hover`, `on_scroll_wheel`, `cursor_pointer`, `cursor`
- `overflow_y_scroll()`, `overflow_x_scroll()`, `overflow_scroll()`
- `track_scroll(&ScrollHandle)`

**Bare `div().overflow_y_scroll()` does NOT compile** — rustc E0599
`no method named overflow_y_scroll found for struct gpui::Div`. The working form is
`div().id("x").overflow_y_scroll()`.

> WARNING: the `layout-style.md` reference shows `.overflow_scroll()` / `.overflow_hidden()`
> on a bare `div()` in its snippets. `.overflow_hidden()` is a pure style (lives on `Styled`,
> fine on bare `Div`), but `.overflow_scroll()` / `.overflow_y_scroll()` are
> `StatefulInteractiveElement` methods and REQUIRE `.id()` first — those snippets are
> misleading. When in doubt, compile.

### Proven by throwaway probe (do this instead of reasoning about the trait block)

```rust
// pose.rs — drop in gpui/examples/, `cargo check --example pose`, then delete
#![cfg_attr(target_family = "wasm", no_main)]
use gpui::{div, prelude::*};
struct P {}
impl gpui::Render for P {
    fn render(&mut self, _w: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        div().id("x").overflow_y_scroll().on_click(|_,_,_| {})
    }
}
fn main() {}
```
- `div().id("x").overflow_y_scroll().on_click(...)` → compiles.
- `div().overflow_y_scroll()` → E0599 (fails).
Replace the method to test any other (`on_hover`, `cursor_pointer`, `track_scroll`).
This pattern caught a real false "correction" of the chronos `gpui-layer-shell` skill:
an agent reasoned from the `impl InteractiveElement for Div` block that scroll worked on a
bare div, "refuted" the skill, then a compile probe proved the skill was RIGHT. Compile,
don't infer.

## Accepted Types

```rust
impl Into<ElementId> for &str      // "my-id"
impl Into<ElementId> for String    // String::from("my-id")
impl Into<ElementId> for usize     // 0, 1, 2, ...
impl Into<ElementId> for u64
impl Into<ElementId> for SharedString
```

## Uniqueness Rules

IDs must be unique within the same **stateful parent's scope** — not globally. GPUI builds a `GlobalElementId` by chaining parent IDs:

```rust
div().id("app").child(
    div().id("list1").children(vec![
        div().id(1usize).child("Item 1"),  // GlobalId: ["app", "list1", 1]
        div().id(2usize).child("Item 2"),  // GlobalId: ["app", "list1", 2]
    ])
).child(
    div().id("list2").children(vec![
        div().id(1usize).child("Item 1"),  // GlobalId: ["app", "list2", 1] — no conflict
    ])
)
```

Items in different parent scopes can reuse simple IDs (integers, short strings).

## In Component Structs

Components always store `id: ElementId` and pass it in `new()`:

```rust
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,
    // ...
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: div().id(id),  // id applied to base
            // ...
        }
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.base  // already has .id() applied
            .on_click(/* ... */)
    }
}
```

## Usage at Call Sites

```rust
// Use unique string IDs for named components
Button::new("save-btn").label("Save")
Button::new("cancel-btn").label("Cancel")

// Use index-based IDs in lists
for (i, item) in items.iter().enumerate() {
    div().id(i)  // unique within this parent
}

// Use descriptive IDs for debugging
Input::new("search-input")
Select::new("country-select")
```

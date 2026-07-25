---
name: gpui-rsx
description: >
  Use when writing ChronOS UI with `gpui-rsx` / `rsx!`, converting mockup HTML
  (e.g. design/*.dc.html) into panel chrome, wiring `overflow_y_scroll` /
  ScrollHandle in rsx or div, fixing E0631 on `hover={…}`, E0425 `div` not
  found after rsx expand, or deciding rsx vs builder `div()`. Triggers:
  "mockup → rsx", "gpui-rsx", "rsx!", "overflowYScroll", side_panel static
  header/permission cards.
---

# gpui-rsx in ChronOS (mockup → compile)

`gpui-rsx` is a **compile-time** JSX→GPUI macro (`use gpui_rsx::rsx`). Zero
runtime. Pin: workspace `gpui-rsx` @ Chronos-GPUI rev (see root `Cargo.toml`);
crate already depends on it (`crates/app/Cargo.toml`).

**Canonical live use:** `side_panel_right/{header,permission,disks}.rs`
(rsx chrome) + `view.rs` (builder shell/scroll/dynamic). README:
`../Source/gpui-rsx/README.md`.

**Layer note:** this is the consumer-side skill. The vendored crate itself
(macro entry points incl. `rsx_expand!` for debugging, parser/codegen,
vendoring status) is covered by `gpui-rsx-markup` (fork-internals layer).

---

## When rsx vs builder `div()`

| Prefer **`rsx!`** | Prefer **builder `div()`** |
|---|---|
| Static chrome from HTML (header, labels, fixed cards) | N dynamic children with per-item heights (spectrum bars) |
| Mockup flex/pad/gap/hex 1:1 | `cx.listener` power/arm, complex closures |
| Allow/Deny / close with simple `onClick` | `transition_when` / `with_transition` animation shell |
| Wrapper that only nests other `IntoElement`s | Anything already painful in rsx — **rollback is data, not failure** |

Flagship rule from sidebar v2: **rsx for static structure, div for live meters
and listeners.** Report where you fell back.

---

## Minimal recipe (static header)

```rust
use gpui::{IntoElement, div, img, prelude::*, px, rgb};
use gpui_rsx::rsx;

pub fn render_header() -> impl IntoElement {
    rsx! {
        <div
            class="flex items-center justify-between"
            flex_none
            px={px(14.)}
            py={px(10.)}
            border_b_1
            border_color={rgb(0x23_23_36)}
        >
            <div text_size={px(11.5)} text_color={rgb(0xa6_ad_c8)}>
                {"kitty"}
            </div>
            <div
                id="side-panel-close"
                w={px(20.)} h={px(20.)}
                cursor_pointer
                text_color={rgb(0x6c_70_86)}
                hover={|s| s.bg(rgb(0x23_23_36)).text_color(rgb(0xcd_d6_f4))}
                onClick={|_ev, window, cx| { /* close */ }}
            >
                {img("icons/x.svg").w(px(12.)).h(px(12.))}
            </div>
        </div>
    }
}
```

**Blood facts (compile-breakers):**

1. **`use gpui::div` is required** even if you only write `rsx!` — macro expands
   to `div()`. Missing → `E0425 cannot find function div`.
2. **`hover={|s| …}` takes `StyleRefinement`**, not `Div`.
   `hover={|s: gpui::Div| …}` → **E0631**. Same as `.hover(|s| …)` on builders.
3. Stateful attrs (`onClick`, `hover`, `overflowYScroll`) need **`id=…`**
   (explicit preferred over auto-id).

---

## Scrollable middle (flex column)

Layer-shell surface size is fixed (`gpui-layer-shell`). Scroll is **inside**.

```rust
// struct field: scroll: ScrollHandle  (+ ScrollHandle::new() in new)
div()
    .flex().flex_col().size_full().overflow_hidden()
    .child(render_header()) // flex_none
    .child(
        div()
            .id("panel-scroll")
            .flex_1()
            .min_h(px(0.))              // required or flex never clips
            .overflow_y_scroll()        // needs .id — StatefulInteractiveElement
            .track_scroll(&self.scroll) // optional programmatic
            .flex().flex_col().gap(px(14.)).p(px(14.))
            // children…
    )
    .child(render_footer()) // flex_none
```

rsx equivalent for the scroller:

```rust
rsx! {
    <div
        id="panel-scroll"
        class="flex-1 min-h-0 flex flex-col"
        overflowYScroll
        trackScroll={&self.scroll}
        gap={px(14.)} p={px(14.)}
    >
        {/* … */}
    </div>
}
```

`div().overflow_y_scroll()` **without** `.id()` → E0599. Old DECISIONS “no
scroll” is **false** (2026-07-20).

---

## Colors: hex vs Theme

| Mode | Use |
|---|---|
| **Pixel mockup parity** (sidebar v2) | `rgb(0xRR_GG_BB)` / `rgba(…)` literals from HTML |
| Product / themeable UI | `Theme::global(cx)` tokens (`chronos_ui`) |

Do not invent Tailwind class colors for product chrome unless mockup is Tailwind.
`class="bg-[#181825]"` works in rsx but ChronOS prefers **attrs**
`bg={rgb(0x18_18_25)}` for mockup work.

---

## Attr / class cheat sheet

| HTML / intent | rsx |
|---|---|
| `display:flex; flex-direction:column` | `class="flex flex-col"` or flags `flex flex_col` |
| `gap:14px; padding:14px` | `gap={px(14.)}` `p={px(14.)}` |
| `flex:1; min-height:0` | `flex_1` + `min_h={px(0.)}` / `class="flex-1 min-h-0"` |
| `onClick` | `onClick={handler}` → `.on_click` |
| Nested element `{expr}` | `{render_foo()}` if `IntoElement` |
| Loop | `{for i in 0..n { <div key={i}>…</div> }}` — **key required** if stateful |

Full syntax: Source README. Do not copy Longbridge `gpui-component` `h_flex()` —
wrong tree (`chronos-fm`).

---

## Common mistakes

| Mistake | Fix |
|---|---|
| `hover={|s: gpui::Div| …}` | `hover={|s| s.bg(…)}` (`StyleRefinement`) |
| No `use gpui::div` | Import `div` (and often `img`, `px`, `rgb`) |
| `overflow_y_scroll` on bare div | `.id("…")` first |
| Second `on_hover` on panel root | Root owns peek debounce — see **gpui-layer-shell** Part C |
| Entire dynamic panel in one giant `rsx!` | Split: rsx chrome + div live body |
| Theme tokens when brief says “hex from mockup” | Use `rgb(0x…)` until token pass |
| Path `gpui_component` / crates.io gpui | This repo: path/git Chronos-GPUI only |

---

## Verification

- [ ] `cargo test -p chronos --bin chronos <module>` green for touched UI
- [ ] `cargo build --release -p chronos` if visual claim
- [ ] Report **rsx vs div map** for mockup ports (where 1:1, where fallback)
- [ ] Live grim vs mockup for pixel work (unit green ≠ UX)

## Related

- **gpui-layer-shell** — panel geometry, single `on_hover`, resize
- **gpui** / **chronos-gpui** — generic Element/API and fork facts
- **chronos-shell** — app/services layout, not rsx syntax

---
name: gpui-layer-shell
description: Make a gpui-ce layer-shell popup/window resize its height to fit dynamic content (fix "popup clipped at bottom" bugs). Use when a ChronOS surface has a fixed height and content overflows, or any time a layer-shell surface's height must track its children. Covers the `window.resize()` API, content-height estimation, the surface-vs-element `max_h` distinction, and the `.id()` requirement for `overflow_y_scroll`. For the full fork API map, see `chronos-gpui`.
---

# gpui-ce layer-shell rubber-band height

A layer-shell surface does **NOT** auto-size to its children. Its size comes
from `WindowBounds` at `open_window` time and is only ever changed by calling
`window.resize(Size<Pixels>)` afterwards. A fixed height + unbounded children
= content clipped at the bottom. This is the standard cause of "popup cut off"
bugs in ChronOS (notifications, tray_menu, OSD-style popups all share it).

## The fix (honest resize — preferred)

1. **Estimate content height.** Sum per-child geometry with plain constants:
   header line, title line, body line-height, action-button height, card
   padding, inter-card gap. For wrapped text, estimate `chars_per_line` from
   width (`(width - 2*pad) / approx_glyph_px`) and `lines = ceil(chars / cpl)`.
   Keep a `MIN_*` floor so a tiny notification doesn't collapse the window.
2. **Cap the height — two different things, don't confuse them.**
   - **The layer-shell *surface itself*** (the actual Wayland window) has no
     style-level cap: its size is `WindowBounds`/`resize()` only, full stop —
     see `chronos-gpui`'s `windowing-platform.md`. Clamp the value you pass to
     `resize()` (e.g. `display_h*0.4` clamped to `[160,560]`).
   - **Content *inside* that surface** (a scrollable list, a variable-length
     card) CAN be capped in style: there is no field literally named
     `max_height`, but `.max_h(px(N))` exists and works (`Style.max_size`,
     `style.rs:234`; macro `gpui_macros/src/styles.rs:899-903` — see
     `chronos-gpui`'s `elements-styling-layout.md` §3 for the full verdict).
     Combine with `.id(...).overflow_y_scroll()` for a real scrollable region
     inside a fixed-size popup — see the scroll pitfall below, it is NOT
     "impossible" here either.
3. **Resize on every content change**, before repainting:
   ```rust
   let height = { /* read global state, compute */ }.min(max_popup_height(cx));
   let _ = handle.update(cx, |_, window: &mut gpui::Window, _| {
       window.resize(Size::new(px(POPUP_WIDTH), px(height)));
   });
   let _ = handle.update(cx, |_, _window, view_cx| { view_cx.notify(); });
   ```
   Size the initial window too: `window_options(display_id, state)` should set
   `WindowBounds::Windowed(Bounds{ size: Size::new(px(W), px(estimate.min(max))) })`.
4. **Don't borrow `cx` mutably inside the `update` closure** (E0502). Compute
   `height` *before* calling `handle.update(...)` — read the global / clone
   outside the closure.

## Key APIs (gpui-ce path dep — see chronos-shell skill for the exact path)

- `Window::resize(&mut self, size: Size<Pixels>)` — `src/window.rs` (~line 2318
  in the local gpui-ce checkout). On Wayland layer-shell this flows to
  `layer_surface.set_size` (`gpui_linux/src/linux/wayland/window.rs`, ~line 1468).
  Resize works on layer-shell; it is the correct tool.
- `f32::from(pixels)` — `From<Pixels> for f32` exists (`src/geometry.rs`
  ~line 2909). `Pixels.0` is **private** — do NOT write `d.bounds().size.height.0`;
  use `f32::from(...)`.
- `gpui::Style` — no field literally named `max_height`/`max_width`, but
  `max_size: Size<Length>` (`style.rs:234`) + the `.max_h`/`.max_w` macro
  prefix (`gpui_macros/src/styles.rs:899-903`) cap an *element's* size.
  The layer-shell *surface* still has no style-level cap — that one really is
  `resize()`-only. See `chronos-gpui/references/elements-styling-layout.md`.

## Pitfalls (specific to this ChronOS gpui-ce build)

- **`overflow_y_scroll()` requires `.id()` first — it is NOT missing.**
  It lives on `StatefulInteractiveElement`, implemented only for
  `Stateful<E>` (`Source/gpui/src/elements/div.rs:3752`), so calling it on a
  bare `div()` fails with "no method" — a compile error long misread as "the
  fork lacks scroll" (it stood as canon for a full day, spread into 6 docs,
  before anyone opened the fork — see `chronos-gpui/SKILL.md` for the full
  story). Working sample shipped in the fork the whole time:
  `Source/gpui/examples/scrollable.rs` (`.id("vertical").overflow_scroll()`),
  `cargo check` green. `cursor_pointer()` compiling on a bare `div()` in
  `bar/widgets/tray.rs` was the tell — it lives on plain `InteractiveElement`
  (no `.id()` needed), which is a *different* trait than `overflow_y_scroll`'s
  `StatefulInteractiveElement` — not a version quirk, a trait boundary.
  For a real scrollable region: `.id("x")` + `.overflow_y_scroll()`, or wire
  `ScrollHandle` + `.track_scroll(&handle)` for programmatic control
  (`scroll_to_bottom()`, `div.rs:4063`). For an append-only log/terminal,
  prefer `list()` + `ListState::set_follow_mode(FollowMode::Tail)`
  (`list.rs:113`/`:617`) over manual `scroll_to_bottom` calls — see
  `chronos-gpui/references/elements-styling-layout.md`, "2. Scroll — full
  picture" → "Autoscroll to bottom (terminal log / streaming)".
- **Build via the right crate.** `cargo build -p chronos` / `cargo test -p chronos`
  isolate app-crate changes from the often-broken `chronos-services` WIP tests.
  `cargo test --workspace` may fail on a *foreign* test that is not your concern
  (e.g. `tray::menu::tests::parse_recursive_variant_wrapped` in `services/**`).
  Don't touch foreign-zone files to "fix" such failures — report and wait.
- **Inline lint `async move` errors are spurious** — the patcher's rustc lacks
  the edition-2024 context (this repo is edition 2024). Trust `cargo build`.

## Verification

- `cargo build -p chronos` (and `cargo test -p chronos --bins` / workspace)
  must be GREEN; test counts drift — do not hardcode a number.
- **UX smoke = release only** (`cargo build --release -p chronos`), per HANDOFF.
- **Visual/live smoke needs a real Wayland session** (grim before/after a
  long-body `notify-send` + two simultaneous notifications). Headless agents
  CANNOT verify the visual result — state that explicitly in the report rather
  than claiming it passes.
- Single-instance: `pkill -x chronos` before restart (not `-f`).

## Worked example

ChronOS `crates/app/src/notifications/`: `mod.rs` holds `estimate_content_height`
+ `max_popup_height(cx)` + the `resize()` call in `sync_window`; `view.rs` keeps
the `flex_col` stack unchanged (do NOT rewrite into a list — that's outside the
task's scope). Commit message shape: `notifications : попап резиновый по высоте
(фикс обрезки)`. The same fixed-height disease also exists in `tray_menu`
(240×40) — candidate for the same rubber-band treatment.

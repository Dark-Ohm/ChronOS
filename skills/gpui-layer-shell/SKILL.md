---
name: gpui-layer-shell
description: Make a gpui-ce layer-shell popup/window resize its height to fit dynamic content (fix "popup clipped at bottom" bugs). Use when a ChronOS surface has a fixed height and content overflows, or any time a layer-shell surface's height must track its children. Covers the `window.resize()` API, content-height estimation, the missing `max_height` Style field, and the `overflow_y_scroll` quirk in this gpui-ce build.
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
2. **Cap the height.** gpui `Style` has **NO `max_height`** field — you cannot
   clamp via style. Apply the cap by clamping the value you pass to `resize()`
   (e.g. `display_h*0.4` clamped to `[160,560]`), and optionally enable inner
   vertical scroll past the cap.
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
- `gpui::Style` — no `max_height`/`max_width`. Clamp via `resize()` only.

## Pitfalls (specific to this ChronOS gpui-ce build)

- **`overflow_y_scroll()` does not resolve** on `Div` here, even though
  `cursor_pointer()` (same `InteractiveElement` trait, `src/elements/div.rs:1429`
  vs `:1475`) compiles fine in `bar/widgets/tray.rs`. Appears to be a gpui-ce
  version quirk in this workspace. Workarounds: (a) skip the inner scroll and
  rely on `resize()` — fine for the normal case where the compositor applies
  the resize; or (b) wire a real `ScrollHandle` (`use gpui::ScrollHandle;`,
  store in the view, `.track_scroll(&self.handle)` + `overflow_y_scroll()`) —
  the canonical gpui scroll path. Test which compiles before committing to (b).
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

# T210 report — right panel: drag holds peek + resize stick 1:1

**Status:** ACCEPTED. **Binary:** `cargo check` clean, `cargo test --lib` 238/238.

---

## What changed

Two root causes fixed in `crates/app/src/side_panel_right/{mod,view}.rs`:

### Bug 1 — dead hover strip after interrupted handle-drag (P0, T209 R7)

**Root cause:** `close_peek_if_not_pinned` fired 280ms after `schedule_release_peek`,
destroying the Wayland surface while the implicit drag grab was still held.
After surface destruction, the compositor stopped delivering enter events to
the hover strip permanently.

**Fix:**
- `SidePanelRightState.resizing: bool` (default `false`) — set `true` in
  `start_resize`, cleared on `on_mouse_up` + in `close()`/`close_this()`.
- `should_close_on_peek_leave` now returns `false` when `resizing` is `true`.
- Result: the 280ms debounce timer may fire, but `close_peek_if_not_pinned`
  sees `resizing=true` and becomes a no-op. The panel survives the drag.

### Bug 2 — half-rate resize tracking (T209 R2/R3)

**Root cause:** `update_resize` used `start_w - delta` with the initial
mouse-down width (`start_w` never updated). After `window.resize()` in
`render()`, the right-anchored coordinate system shifted right by Δw,
making half of each delta land in the coordinate-shift void.

**Fix:**
- `update_resize`: now uses `state.width - delta` (current width, not `start_w`).
  Stores `current_x` in `resize_start_x` for the next frame's delta.
- `render()`: after `window.resize()`, corrects `resize_start_x += (panel_width - old_w)`
  to account for the coordinate shift. Skipped for the initial rail→content
  expand (`old_w <= RAIL_ONLY_WIDTH + 2.0`), since `start_resize` already
  applies the `(target - w)` offset.
- Removed `resize_start_width` — dead after the formula change.

---

## Files touched

| File | Change |
|---|---|
| `crates/app/src/side_panel_right/mod.rs` | +`resizing` field, `should_close_on_peek_leave` guard, clear in `close()`/`close_this()` |
| `crates/app/src/side_panel_right/view.rs` | `update_resize` formula, coordinate correction in `render()`, `on_mouse_up` handler, remove `resize_start_width` |

---

## Verification

```
cargo check -p chronos        # 0 errors
cargo test -p chronos --lib   # 238 passed, 0 failed
```

**Live:** requires T209 repro R1–R3, R7 — drag past edge → hover strip works;
cursor and edge stick 1:1 after rail→expand.

---

## Что НЕ сделано

- `on_mouse_up` fire guarantee during GPUI drag — not verifiable without live
  test. If the fork swallows mouse-up during drag, `resizing` would only clear
  on the next `start_resize` or panel close, which is safe (no dead strip, just
  a one-time peek-close suppression after drag).
- Unit tests for resize math — impractical to mock drag events in gpui::test.
  Live grim/log is the acceptance path (T209 spec).

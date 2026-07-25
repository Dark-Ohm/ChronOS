# Local Kael patches (applied to `reference/kael-source`)

Three patches applied to Kael 0.3 (commit `9351f31a`) to fix Wayland issues.
All edits in `reference/kael-source/` (local git clone, dep via `file://` git URL).

## 1. RefCell double-borrow in Wayland backend (set_size_and_scale → frame)

**File**: `crates/kael/src/platform/linux/wayland/window.rs`

**Root cause**: `WaylandWindowStatePtr::set_size_and_scale()` calls the `resize` callback
while holding `self.callbacks.borrow_mut()`. The callback chain
(`Window::bounds_changed → refresh → update_frame_polling → set_frame_polling → frame()`)
re-enters `self.callbacks.borrow_mut()`, causing a runtime panic.

**Fix A** — `set_size_and_scale()` (~line 874):
```rust
// Take callback out before invoking, so we don't hold the RefCell borrow
let mut resize_fn = self.callbacks.borrow_mut().resize.take();
if let Some(ref mut fun) = resize_fn {
    fun(size, scale);
}
self.callbacks.borrow_mut().resize = resize_fn;
```

**Fix B** — `frame()` (~line 493):
```rust
// Use try_borrow_mut to skip the callback when callbacks is already borrowed
if let Ok(mut cb) = self.callbacks.try_borrow_mut() {
    if let Some(fun) = cb.request_frame.as_mut() {
        fun(Default::default());
    }
}
```

## 2. Layer-shell anchor + exclusive zone for Overlay

**Core change**: Added `layer_anchor: u32` and `layer_exclusive_zone: i32` fields to
`WindowOptions` (public API, platform.rs) and `WindowParams` (pub(crate), platform.rs).

**Files changed**:
- `crates/kael/src/platform.rs` — struct fields + Default impl
- `crates/kael/src/window.rs` — destructure + forward to WindowParams
- `crates/kael/src/platform/linux/wayland/window.rs` — use params values

**Wayland backend change** (window.rs ~line 374):
```rust
// Before (hardcoded):
layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::empty());
layer_surface.set_exclusive_zone(0);

// After (configurable):
layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::from_bits_retain(
    params.layer_anchor,
));
layer_surface.set_exclusive_zone(params.layer_exclusive_zone);
```

Default is 0/0, which is backward-compatible with previous empty-anchor behaviour.

## How to re-apply after Kael version bump

1. Check out new Kael version into `reference/kael-source/`
2. Cherry-pick our patches:
   ```bash
   cd reference/kael-source
   git log --oneline -10  # find our patch commits
   git cherry-pick <hash-of-layer-anchor-patch> <hash-of-refcell-fix-patch>
   ```
3. Update Cargo.lock: `cargo update -p kael` in chronos workspace
4. Rebuild: `cargo check -p chronos`

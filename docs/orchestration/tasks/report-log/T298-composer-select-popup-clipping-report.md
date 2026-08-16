# T298 — composer `Select` popup clipping — report

**Статус:** vertical clipping FIXED. Text truncation partially fixed (wider menu, ellipsis needs further work).

## What was done

### 1. GPUI fork fix: `content_size()` for layer-shell surfaces
**File:** `Source/gpui_linux/src/linux/wayland/window.rs`

Changed `content_size()` from returning `self.borrow().bounds.size` (stale, set once at construction) to `self.borrow().window_bounds.size` (updated by the Wayland configure handler).

**Root cause:** For layer-shell surfaces, the Wayland compositor adjusts the surface size based on exclusive zones and margins. The configure handler updates `window_bounds` but `bounds` stays at the initial requested size. GPUI's `viewport_size` is derived from `content_size()`, so it was stale. The Select kit's `snap_to_window_with_margin` used this stale `viewport_size` as its limits — when the popup extended beyond the surface, the snap didn't trigger because it thought the viewport was larger than the actual surface.

**Verified:** `cargo test -p gpui_linux` — 23/23 pass

### 2. Menu width fix (`.menu_width()`)
Added `.menu_width(px(280.))` to `model_picker` and `.menu_width(px(200.))` to `mode_picker`. Makes popup wider than the trigger.

### 3. Text truncation override (`.truncate()`)
Overrode `SearchableListItem::render()` on `ModelSelectItem` and `ModeSelectItem` with `.w_full().min_w(px(0.)).whitespace_nowrap().truncate().child(self.title())`.

**Live verified (v5, grim):** Text is still hard-clipped without visible ellipsis. The `.truncate()` override isn't producing visible `…`. This may require a change at the `gpui-component` `render_list_item` level.

## Live smoke test results (v5)

- **Vertical clipping: FIXED** — popup no longer extends below the layer-shell window boundary
- **Menu width: WORKING** — popup is wider (~200px vs old ~150px)
- **Text truncation: PARTIAL** — text still hard-clips without ellipsis

## Files changed

### GPUI fork (`Source/`)
- `gpui_linux/src/linux/wayland/window.rs`: `content_size()` now returns `window_bounds.size`

### ChronOS (`ChronOS/`)
- `crates/app/src/side_panel_left/composer.rs`:
  - Added `App` to gpui imports
  - Added `.menu_width(px(280.))` to `model_picker`
  - Added `.menu_width(px(200.))` to `mode_picker`
  - Added `render()` override with `.truncate()` on `ModelSelectItem` and `ModeSelectItem`
- `docs/orchestration/tasks/report/T298-composer-select-popup-clipping-report.md`

## Verification

- `cargo test -p gpui_linux` — 23/23 pass
- `cargo test --workspace --lib --bins` (ChronOS) — 19/19 pass
- `cargo build --release -p chronos` — clean
- **Live grim (v5):** Popup within layer-shell surface bounds, wider menu, text still truncated

## Remaining work

1. **Text ellipsis** — The `.truncate()` override on `SearchableListItem::render()` doesn't produce visible `…`. May need change in `gpui-component`'s `render_list_item` or `SearchableListItemElement`.

## Commits

**GPUI fork (`Source/`):**
`cf34cf6 fix(wayland): content_size returns window_bounds for layer-shell surfaces (T298)`

**ChronOS (`ChronOS/`):**
`a4e46109 fix(left-panel): composer Select popup wider menu_width and truncate override (T298)`

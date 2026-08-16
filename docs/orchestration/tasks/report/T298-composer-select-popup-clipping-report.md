# T298 — composer `Select` popup clipping — report

**Статус:** partial fix shipped (horizontal truncation). Vertical clipping requires GPUI fork change.

## What was done

Added `.menu_width()` to both picker `Select` elements in `composer.rs`:

- `model_picker`: `.menu_width(px(280.))` — wide enough for longest model IDs (`anthropic/claude-fable`, `openai/gpt-5.6-sol-pro`)
- `mode_picker`: `.menu_width(px(200.))` — wide enough for mode names

This fixes **Корень №2** from the rejected report: text truncation in the popup's item rows. The kit's default menu width = trigger width + 2px, so with 150px/90px triggers, long names were clipped.

## What was verified

- `cargo check -p chronos` — clean (warnings only, no errors)
- `cargo test --workspace --lib --bins` — 19 passed, 0 failed
- `cargo build --release -p chronos` — clean (3m54s, warnings only)

## What was NOT done (and why)

### Vertical clipping (Корень №1) — requires GPUI fork change

The popup extending below the layer-shell surface is caused by the Select kit's `deferred(anchored().snap_to_window_with_margin(px(8.)))` mechanism using `window.viewport_size()` for snap limits. For layer-shell surfaces, `viewport_size` (= `platform_window.content_size()`) returns the bounds from `WindowParams` set at window creation, which may not match the actual compositor-committed surface bounds.

The `deferred` element correctly bypasses the parent `overflow_hidden` clip (verified by reading `deferred.rs` — `content_mask: None` means no clipping during deferred paint). The `anchored` element's snap logic correctly adjusts position when `desired.bottom() > limits.bottom()`. But if `viewport_size` returns a height larger than the actual surface (e.g., full display height instead of `panel_h`), the snap doesn't trigger.

**This cannot be fixed from `composer.rs`** — it requires either:
1. Fixing `content_size()` / `viewport_size` in the GPUI fork's Wayland window implementation
2. Or replacing the Select kit's in-window popup with a native `WindowKind::AnchoredPopup` window (as the rejected report recommended — Option 1)

The rejected report's recommendation to rewrite pickers as native `AnchoredPopup` windows remains the correct long-term fix. That is a larger refactor (300+ LOC, new module, window lifecycle management).

## Files changed

- `crates/app/src/side_panel_left/composer.rs`: added `.menu_width(px(280.))` to `model_picker`, `.menu_width(px(200.))` to `mode_picker`

## Live smoke needed

This touches layout/popup UI — a live Wayland smoke test with `grim` is needed to confirm:
1. Popup text is no longer truncated (horizontal)
2. Vertical clipping status (may still extend below surface — see above)
3. Model/mode selection still works

## Commit

`fix(left-panel): composer Select popup wider menu_width to prevent text truncation (T298)`

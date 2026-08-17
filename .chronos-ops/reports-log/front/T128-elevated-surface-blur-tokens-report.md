# T128 — Elevated-Surface Blur + Shadow Tokens

**Date:** 2026-07-25
**Status:** Code complete; debug + release build clean; 4 new `chronos-ui` token tests pass

---

## What This Task Is

Introduce a single **elevated-surface depth language** for every floating
card in the shell (popups + panel content columns) and migrate the 4 existing
popups + 2 panels off copy-pasted `BoxShadow::new(...)` / ad-hoc `paint_blur(...)`
blocks onto it. Dark scheme keeps the mockup-faithful **blur-only** recipe;
light scheme (Latte) gets the **Light C** recipe (real shadows + accent ring +
glow edge + watermark flag). This is wave 1/4 of the visual-depth initiative.

---

## Files Modified

| File | What Changed |
|---|---|
| `crates/ui/src/elevation.rs` | **NEW** — `ElevationTokens`, `BlurSpec`, `EMPTY_SHADOWS`, `Theme::elevation_popup()`, `elevation_blur_layer()`, `elevation_glow_bar()`, `elevation_watermark()`. 4 unit tests. |
| `crates/ui/src/lib.rs` | Exports `elevation` module + `ElevationTokens`/`BlurSpec`/`EMPTY_SHADOWS`. |
| `crates/app/src/volume_popup/view.rs` | Hardcoded `BoxShadow::new` drop-shadow + accent ring → `elevation_popup().shadows` + glow edge. Kept existing frosted blur layer. |
| `crates/app/src/system_popup/view.rs` | `.shadow([...])` block → token shadows; blur layer now uses `elev.blur` radius/tint/sat. Added glow edge + watermark flag. |
| `crates/app/src/updates_popup/view.rs` | **No blur before** — added frosted `elevation_blur_layer`; hard-coded shadows → token shadows + glow + watermark. |
| `crates/app/src/notifications/history_popup/view.rs` | **No blur before** — added frosted `elevation_blur_layer`; added token shadows + glow edge. |
| `crates/app/src/side_panel_left/panel.rs` | `main-content` gets `.shadow(elev.shadows)` + glow edge when `chat_open` (rail-only stays flat). Imports `Theme`. |
| `crates/app/src/side_panel_right/view.rs` | `side-panel-content-column` gets `.shadow(elev.shadows)` + glow edge when `content_open`. Imports `Theme`. |

**Not touched (intentional):** `crates/app/src/notifications/view.rs` toast cards.
They are a separate dark-only stack with their own geometry; migrating them to
the token would change their look for zero benefit (rule: "if cheap").

---

## Token Surface (`crates/ui/src/elevation.rs`)

```rust
pub struct ElevationTokens {
    pub shadows: &'static [BoxShadow], // dark: EMPTY_SHADOWS; light: 2-layer pool
    pub blur: Option<BlurSpec>,        // dark: Some; light: None
    pub glow: Option<Hsla>,            // dark: None; light: Some(accent)
    pub watermark: bool,               // dark: false; light: true
}

pub struct BlurSpec {
    pub radius: f32,    // gaussian strength
    pub sat: f32,       // saturation boost
    pub tint_alpha: f32 // color overlay alpha over backdrop
}
```

### Dark popup (blur-only, mockup-faithful)

```rust
ElevationTokens {
    shadows: &[],                          // NO drop shadow in dark — blur carries depth
    blur: Some(BlurSpec { radius: 14.0, sat: 1.15, tint_alpha: 0.45 }),
    glow: None,
    watermark: false,
}
```

### Light popup (Light C recipe)

```rust
ElevationTokens {
    shadows: &[
        // drop shadow — y6 / blur24
        BoxShadow { offset: point(px(0.), px(6.)), blur_radius: px(24.), spread_radius: px(0.), color: rgba(0x1e_1e_2e, 0.18), inset: false },
        // accent ring — inset 0 / blur0 / spread1
        BoxShadow { offset: point(px(0.), px(0.)), blur_radius: px(0.),  spread_radius: px(1.), color: accent_ring,    inset: true },
    ],
    blur: None,                            // light uses solid bg, not blur
    glow: Some(accent.with_alpha(0.5)),    // top-edge glow strip
    watermark: true,
}
```

`BoxShadow` pool is `static` (constructed via `lazy_static`), so the
`&'static [BoxShadow]` lifetime is stable.

### Helpers (no `Window` coupling — caller owns the `canvas`)

```rust
pub fn elevation_blur_layer(elev: &ElevationTokens, radius: Pixels, bg: Hsla) -> Div
pub fn elevation_glow_bar(glow: Hsla) -> Div   // 1px top-edge strip, opacity 0.4
pub fn elevation_watermark() -> Div            // corner sigil, light only
```

---

## Migration Pattern (every popup)

```rust
let elev = theme.elevation_popup();

// frosted layer (dark) — radius matches card corner
let blur_layer = elevation_blur_layer(&elev, radius_lg, panel_bg);

// card chrome
let mut card = div()
    .relative()
    .rounded(radius_lg)
    .bg(panel_bg)
    .border_1()
    .border_color(border_subtle)
    .shadow(elev.shadows.to_vec())               // dark: [] (no-op); light: 2-layer
    .when_some(elev.glow, |glow, el| el.child(elevation_glow_bar(glow)))
    .when(elev.watermark, |el| el.child(elevation_watermark()));

div().relative().size_full().child(blur_layer).child(card) ...
```

**Geometry untouched** — every popup keeps its original radius
(volume/system/history `radius_lg` = 12px, updates `radius` = 6px). Only the
depth *treatment* moved to tokens.

---

## Panels (Task 3 — elevated chrome on root card)

Left `main-content` and right `side-panel-content-column` now carry
`.shadow(elev.shadows.to_vec())` **only when expanded** (`chat_open` /
`content_open`). When in rail-only mode (collapsed strip) they stay flat — the
tab rail is chrome, not an elevated surface. Glow edge added in light scheme.
No blur on panels (they are root layer-shell windows, not floating cards).

---

## Tests Added

| Test | Location | What It Checks |
|---|---|---|
| `dark_popup_is_blur_only` | `ui/src/elevation.rs` | Dark: `shadows.is_empty()`, `blur.is_some()`, `glow.is_none()`, `!watermark` |
| `light_popup_has_shadows_and_glow` | `ui/src/elevation.rs` | Light: 2 shadows, `blur.is_none()`, `glow.is_some()`, `watermark` |
| `light_and_dark_differ_where_intended` | `ui/src/elevation.rs` | Light shadow count != dark; dark blur set, light none |
| `light_shadow_pool_is_stable` | `ui/src/elevation.rs` | 2 entries; [0] drop (y6/blur24, not inset), [1] inset ring |

**Total:** `cargo test -p chronos-ui elevation` → 4 passed.

---

## Build Status

```text
cargo check -p chronos        → clean (pre-existing warnings only)
cargo build      (-p chronos)  → clean  (35s, dev profile)
cargo test -p chronos-ui      → 4 elevation tests pass; full ui suite green
```

No `../Source/**` fork edits. `BoxShadow` import dropped from `updates_popup`
(replaced by token); `Corners`/`canvas` added where blur layers were introduced.

---

## Live Smoke Test (Pending — needs Hyprland session)

Blur + shadows cannot be verified headless. After `chronos-rebuild && chronos-restart`:

```bash
# 1) Dark scheme (default)
#    - volume popup (Meta+V): frosted blur, NO drop shadow, accent ring visible
#    - system popup (battery/lock rows): same blur-only treatment
#    - updates popup: now has blur (was missing before)
#    - notification history (bell): now has blur (was missing before)
#    - left panel chat-open: content column flat in dark (no shadow expected)
# 2) Light scheme (Latte) — toggle in settings
#    - every popup: visible drop shadow + inset accent ring + top glow strip
#    - left/right panel content columns: elevated shadow appears
# grim: dark-popups / light-popup-volume / light-panel-content
```

Report: confirm dark = blur-only (no shadow), light = shadow+ring+glow.

---

## Summary

T128 delivers the first half of the visual-depth wave: a **single token
vocabulary** for elevated surfaces (`Theme::elevation_popup()`) that encodes
the dark blur-only vs light Light-C split once, and 6 views migrated onto it
(4 popups + 2 panel content columns). Toast cards deliberately excluded.
Build is clean, 4 token tests lock the shadow pool shape. Visual confirmation
of blur/glass is pending a live Hyprland run — code compiles and tokens are
validated by unit tests, but pixels need your screen.

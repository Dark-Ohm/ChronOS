---
name: gpui-layer-shell
description: >
  Use when placing or sizing ChronOS layer-shell surfaces on Hyprland/Niri —
  full-height side panels that must clear the bar with equal top/bottom gaps,
  popups clipped at the bottom, rubber-band height via window.resize, or when
  combining on_hover with gpui-animation (single on_hover slot per element).
  Triggers: "panel overlaps bar", "unequal margins", "popup cut off",
  "debug_assert on_hover", side_panel_right geometry.
---

# gpui-ce layer-shell: height, placement, hover

Two different problems share this skill:

1. **Rubber-band height** — surface does not grow with children → clip.
2. **Full-height side placement** — panel must not cover the bar; equal air
   top and bottom of the **display**.

Do not mix recipes. Popup content height ≠ panel screen geometry.

**Canonical code:** `crates/app/src/side_panel_right/mod.rs` + `hover_strip.rs`
(placement); `crates/app/src/notifications/mod.rs` (resize).

**Layer note:** this is the consumer-side skill (ChronOS usage, measured on
Hyprland). The fork's layer-shell *API reference* (all `LayerShellOptions`
fields, cfg-gates, `LayerShellNotSupportedError`) is `layer-shell-windows`;
close/ghost bugs are `wayland-window-lifecycle` (fork-internals layer).

---

## Part A — Full-height side panel under the bar (equal display gaps)

### Goal (measured)

On pult DP-1 2560×1440, `BAR_HEIGHT = 30`:

| | |
|---|---|
| bar | `y=0 h=30` |
| panel | `y=30 h=1380` → `top_gap=30`, `bot_gap=30` |
| strip | same vertical band as panel |

`top_gap == bot_gap == BAR_HEIGHT`, and `y >= BAR_HEIGHT` (no bar overlap).

### Recipe that works (Hyprland Overlay)

```rust
// gap == BAR_HEIGHT (from chronos_luau::bar::BAR_HEIGHT)
let display_h = display_id
    .and_then(|id| cx.find_display(id))
    .or_else(|| cx.primary_display())
    .map(|d| f32::from(d.bounds().size.height))
    .unwrap_or(1080.);
let panel_h = (display_h - 2. * BAR_HEIGHT).max(100.);

WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(Bounds {
        origin: point(px(0.), px(0.)),
        size: Size::new(px(PANEL_WIDTH), px(panel_h)),
    })),
    kind: WindowKind::LayerShell(LayerShellOptions {
        // TOP|RIGHT only — NOT BOTTOM
        anchor: Anchor::TOP | Anchor::RIGHT,
        exclusive_zone: None,
        // margin top 0: bar's exclusive zone already places TOP-anchored
        // Overlay surfaces at y = BAR_HEIGHT (measured).
        margin: None,
        keyboard_interactivity: KeyboardInteractivity::None,
        ..Default::default()
    }),
    ..
}
```

Hover strip: same `anchor` / `height` / `margin` math so the 4px edge only
covers the panel band, not the bar.

### What fails (do not re-try)

| Attempt | Result on Hyprland + bar exclusive |
|---|---|
| `TOP\|BOTTOM\|RIGHT` + `margin: None` | `y≈15`, height full — **overlaps bar** |
| `TOP\|BOTTOM\|RIGHT` + equal margins `(T,0,T,0)` | stretch + exclusive **skews** gaps (e.g. top 45 / bottom 15); split of T/B only changes total, not symmetry |
| `TOP\|BOTTOM` + "skew compensation" literals | fragile; broke when bar exclusive state differed |
| `TOP\|RIGHT` + `margin top = BAR_HEIGHT` | **double** offset → `y≈60`, bottom flush (`bot_gap=0`) |

**Root cause:** with `TOP|BOTTOM` stretch, Hyprland + bar exclusive zone does
not treat top/bottom margins as equal insets from the display. With
`TOP|RIGHT` + fixed height, exclusive zone of the bar alone places the
surface under the bar; height `display − 2×gap` creates matching bottom air.

### Verification (mandatory live)

```bash
pkill -x chronos
CHRONOS_SMOKE_SIDE_PANEL=1 ./target/release/chronos &
# or open panel by product path
hyprctl layers | rg 'side_panel|namespace: bar'
# parse xywh: expect panel y == BAR_HEIGHT, bot_gap == BAR_HEIGHT on pult height
```

Headless agents cannot claim placement OK without `hyprctl layers` numbers.

---

## Part B — Rubber-band height (popups)

A layer-shell surface does **NOT** auto-size to its children. Size comes from
`WindowBounds` at `open_window` and only changes via `window.resize(...)`.
Fixed height + unbounded children = content clipped at the bottom
(notifications, tray_menu, OSD-class popups).

### Preferred fix

1. **Estimate content height** with constants (line heights, pads, gaps).
   For wrap: `cpl ≈ (width - 2*pad) / glyph_px`, `lines = ceil(chars/cpl)`.
   Keep a `MIN_*` floor.
2. **Cap** by clamping the value passed to `resize()` (no surface-level
   `max_height` style). Inside the surface, `.max_h(px(N)).overflow_hidden()`
   or `.id(..).overflow_y_scroll()` is fine for content.
3. **Resize on every content change** before repaint; compute height
   *outside* `handle.update` (E0502).
4. Initial `WindowBounds` should use the same estimate, not a magic 96px.

### APIs

- `Window::resize` → Wayland `layer_surface.set_size` (works on layer-shell).
- `f32::from(pixels)` — never `.0` on `Pixels` (private).
- `.max_h` / `.max_w` exist for **elements**; surface size is still resize-only.

### Scroll pitfall

`overflow_y_scroll()` needs `.id(...)` first — it lives on
`StatefulInteractiveElement` (`Stateful<E>`), not bare `div()`.
Sample: `Source/gpui/examples/scrollable.rs`.

---

## Part C — One `on_hover` per element (fork blood fact)

Our gpui stores a **single** `Option` hover handler per element and
`debug_assert!`s if `.on_hover` is set twice.

| Do | Don't |
|---|---|
| Root: only peek/close debounce `on_hover` | Root: `on_hover` **and** `transition_on_hover` |
| Animation: `.with_transition` + **`.transition_when(state, …)`** on an **inner** node | `.transition_on_hover` on the same node as manual debounce |
| Strip: its own window, one `on_hover` | Second hover on panel root for meters/power |

`gpui-animation`'s `AnimatedWrapper` **always** installs `on_hover` on the
wrapped child when rendered — even for state-driven transitions. Keep the
animated node separate from the debounce root.

**Canonical:** `side_panel_right/view.rs` (outer root hover / inner body
`transition_when` fade).

---

## Part D — `exclusive_zone` on a corner anchor needs `exclusive_edge`

The bar (`Anchor::LEFT|RIGHT|TOP`, stretched across a full edge) reserves
space with just `exclusive_zone: Some(px(BAR_HEIGHT))` — the direction is
unambiguous. A panel anchored to a **corner** (`Anchor::LEFT|TOP`, fixed
width AND height, not stretched) is ambiguous to wlr-layer-shell: which
edge does the zone grow from? Without `exclusive_edge`, the compositor
silently ignores the zone — no protocol error, `hyprctl monitors` just
shows `reserved` unchanged.

```rust
LayerShellOptions {
    anchor: Anchor::LEFT | Anchor::TOP,
    exclusive_zone: Some(px(width)),
    exclusive_edge: Some(Anchor::LEFT),   // single bit, must be in `anchor`
    ..
}
```

`Window::set_exclusive_zone`/`set_exclusive_edge` (`gpui/src/window.rs:
2005`/`2014`) are live-callable from `render()`, not create-time-only —
call them next to `window.resize()` if the surface's reserved width
tracks a live resize.

**Whether to use exclusive_zone at all is a UX call, not just a technical
one.** `side_panel_left` tried it (2026-07-23): verified working (tiled
windows in `hyprctl clients` genuinely shifted/shrank), then reverted the
same session — shoving tiled windows around on every open/resize of a
panel meant to stay open while working is disruptive in a way it isn't
for a bar that opens rarely. Full narrative + the hover-peek-vs-keybind-
toggle decision it's paired with: `chronos-shell` skill Gotchas, and
`docs/DECISIONS.log` 2026-07-23.

---

## Related ChronOS patterns (side panel body)

Not geometry, but same module — do not re-learn:

- **Net sampling in `render`:** use `net_stats::update_speed(..., SAMPLE_INTERVAL)`;
  push history only when sample time advances (not every paint with cache).
- **Power arm/confirm:** view-local state + `cx.listener`, timeout via
  `cx.spawn` + `match view.update` / `warn!` — never `let _ =` on fallible
  window/view update (ghost-window saga).
- **Smoke:** `CHRONOS_SMOKE_SIDE_PANEL=1` pin-open (optional env); `pkill -x chronos`.

---

## Verification checklist

- [ ] `cargo build -p chronos` green
- [ ] Live: `hyprctl layers` — panel `y >= BAR_HEIGHT`, `top_gap == bot_gap`
- [ ] No double `on_hover` on one element (grep + no `debug_assert` panic)
- [ ] UX smoke release-only for visual claims
- [ ] Single-instance: `pkill -x chronos` (not `-f`)

## Worked examples

| Surface | Path | Pattern |
|---|---|---|
| Right panel + strip | `side_panel_right/{mod,hover_strip}.rs` | Part A placement |
| Notifications | `notifications/mod.rs` | Part B resize |
| Panel view hover/anim | `side_panel_right/view.rs` | Part C hover split |

## Common mistakes

1. **TOP\|BOTTOM + equal margins** hoping for symmetry → Hyprland skews gaps.
2. **TOP\|RIGHT + margin top = BAR_HEIGHT** → double offset under exclusive bar.
3. **Pixel formula without hyprctl** → claim equal gaps from constants alone.
4. **`transition_on_hover` on root** that already has debounce → panic under debug.
5. **Pushing net history every `render`** without time gate → flat flood / wrong UI.
6. **`exclusive_zone` on a corner anchor (`LEFT|TOP`) without `exclusive_edge`**
   → silently ignored, `reserved` unchanged, no error to catch it (Part D).

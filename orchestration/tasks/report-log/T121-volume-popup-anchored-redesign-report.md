# T121 — Volume Popup: Anchored Redesign + Fork Animation/Blur Integration

**Status:** done (build green on full local fork; live Wayland smoke pending Architect)
**Stack:** Rust + GPUI (chronos fork `Source/`) + `gpui_animation` + `gpui-rsx`

---

## 1. What changed

### Visual language — matched `updates_popup` / `notifications`
The popup was rebuilt to the same premium recipe those two popups use:
- Light C card: elevated `BoxShadow` + 1px inset accent ring + top accent glow
  line + `hexagon-sigil` watermark (same code as `updates_popup/view.rs`).
- `border_1` + `radius_lg` (12px) card, `radius` (6px) inner controls.
- `font_mono` throughout, `font_sizes` tokens, `is_light` branch.
- Section dividers (`border_b_1`) between header / endpoints / footer.

### Fork skills used (the "use the animation crate" ask)
- **`gpui_animation`** (`vendored-gpui-animation` skill): `with_transition` +
  `transition_on_hover` on footer mute buttons and device rows — border/color
  morph to accent on hover with a spring ease. The device picker springs
  open/closed via `transition_when` using `EaseOutBack` (wrapped, see §3).
- **`backdrop-blur`** (`backdrop-blur` skill): real frosted glass via
  `window.paint_blur(...)` in a `canvas` paint closure behind the card. This is
  the premium touch the other popups do **not** have yet — the panel now reads
  as acrylic, not flat `bg`.

### Anchoring (unchanged from prior T121 work)
`AnchoredPopup` + LayerShell fallback, `POPUP_WIDTH = 360`, bar-widget bounds
captured via `Rc<Cell<Bounds>>` so the popup pins to the speaker icon.

---

## 2. Fork as the source of truth — whole tree in-tree

**Decision (Architect, 2026-07-25):** git deps are the wrong tool while we
iterate on the fork. The entire `gpui` graph is now developed against
`../Source/*` — no pinned git rev in active dev.

`ChronOS/Cargo.toml` `[patch."https://github.com/Dark-Ohm/Chronos-GPUI"]`
now redirects **all 16 fork crates** to `../Source/`:
`gpui, gpui_collections, gpui_derive_refineable, gpui_linux, gpui_macros,
gpui_media, gpui_platform, gpui_refineable, gpui-rsx, gpui_scheduler,
gpui_shared_string, gpui_sum_tree, gpui_util, gpui_web, gpui_wgpu,
gpui-animation`.

`Source/gpui-animation/Cargo.toml` keeps `gpui = { path = "../gpui" }` (canonical
per the `vendored-gpui-animation` skill — must stay a path dep, never a version
dep).

---

## 3. Fork deltas touched (record in `gpui-animation/PATCHES.md`)

### Delta 4 — public `init` entry point (NEW)
`TransitionRegistry::init` was `pub(crate)`, so the crate left `animation_tick`
**frozen** — every `with_transition` / `transition_on_hover` / `transition_when`
stayed dead until something called `init`. Upstream never calls it from outside.

Added to `Source/gpui-animation/src/lib.rs`:
```rust
pub fn init(window: &mut gpui::Window, cx: &mut gpui::App) {
    transition::TransitionRegistry::init(window, cx);
}
```
Booted once per session from `Bar::render` (idempotent — `AtomicBool` guard):
```rust
gpui_animation::init(window, cx);
```
This is the fix that makes the animation crate actually run — and the blocker
that would have prevented publishing the fork (private boot entry).

### EaseOutBack adapter (in `volume_popup/view.rs`, not the crate)
`EaseOutBack` is not in `gpui_animation::transition::general` (only
quad/cubic/sine/exponential). Wrapped the fork's
`gpui::easing::EasingCurve::EaseOutBack` in a local `Transition` impl
(`struct SpringBack(f32)`) so the device picker can overshoot-spring.

---

## 4. Fork API traps (re-verified, do not re-learn)
1. `rsx!` is `use gpui_rsx::rsx;` — never `gpui::rsx` (the fork doesn't
   re-export it). Mixed `rsx!` chrome + `div()` builder for stateful parts.
2. `AnimatedWrapper::on_click` is `Fn(&ClickEvent, &mut Window, &mut App)` — NOT
   compatible with `cx.listener` (which is 4-arg, last `&mut Context<Self>`).
   So `gpui_animation` wrappers are used only where the handler needs just
   `&mut App` (footer mute, device row). Title/slider keep plain `Div` +
   `.hover` + `cx.listener`, matching `updates`/`notifications`.
3. `canvas` paint closure is `(bounds, state, window, cx)` — 4 args; prepaint is
   `(bounds, window, cx)` — 3 args. `Hsla` alpha is `.alpha(a)`, not
   `.with_alpha(a)`.
4. `backdrop-blur` is a **paint-phase** primitive: call `window.paint_blur(...)`
   inside a `canvas` paint closure. No `.backdrop_blur()` style method exists.
   wgpu-only — verify on the real renderer, not headless.

---

## 5. Verification

Ad-hoc (NOT suite-green — visual/animation behavior needs a live Wayland
session):
- `cargo build --release -p chronos` → `BUILD_EXIT=0` (full local fork, all
  16 crates patched to `../Source/*`).
- `cargo test -p chronos volume` → **12 passed; 0 failed**.
- `grep` confirms: `paint_blur` blur layer in `view.rs`, public `init` in fork
  `lib.rs`, `init` called in `Bar::render`, `[patch]` redirects the whole
  fork tree (all 16 crates resolved from `../Source`).

**Blocker (Architect):** no live Hyprland/Wayland here. Frosted glass,
hover-glow spring, and spring-reveal picker must be eyeballed by running the
shell (`chronos rebuild` + click the speaker icon).

## 6. Files touched
- `crates/app/src/volume_popup/view.rs` — blur layer + `gpui_animation` + style.
- `crates/app/src/volume_popup/mod.rs` — anchored popup (prior T121).
- `crates/app/src/bar/widgets/volume.rs` — bounds capture (prior T121).
- `crates/app/src/bar/mod.rs` — `gpui_animation::init` boot.
- `crates/app/src/assets.rs` + `assets/icons/microphone*.svg` — mic icons.
- `Cargo.toml` — whole-fork `[patch]` to `../Source/*`.
- `Source/gpui-animation/src/lib.rs` — public `init` (Delta 4).
- `Source/gpui-animation/PATCHES.md` — Delta 4 recorded.

# Updates Popup — Anchored Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **ChronOS-specific note:** this repo's Architect does not dispatch Claude
> subagents — this plan is executed by a human-directed local minion via
> `orchestration/tasks/active/`, following `orchestration/agents/rules.md`
> (поимённый `git add`, no unrequested commits to master). Treat
> "subagent" below as "the assigned minion."

**Goal:** Turn `updates_popup` from a fixed-corner, clip-truncated,
MVP-looking window into the pilot for the popup-system polish pass —
anchored to its actual trigger icon via the fork's native
`WindowKind::AnchoredPopup`, real scroll instead of a hard clip, and
visual chrome matching `design/Updates Popup.dc.html` (dark + light).

**Architecture:** Trigger bounds captured via a `gpui::canvas()` overlay
in the bar widget (stashed in a shared cell, read at mouse-down time),
passed into a new `updates_popup::open(cx, anchor_rect, parent)` that
builds `WindowOptions { kind: WindowKind::AnchoredPopup(PopupOptions {
anchor: BottomRight, gravity: BottomLeft, grab: true, .. }), .. }`
instead of standalone `LayerShell`. List container swaps
`.max_h().overflow_hidden()` for `.overflow_y_scroll().id(...)`. A new
`Theme.is_light: bool` field (small, reusable beyond this popup) gates
the mockup's light-only watermark sigil / glow-top line / elevated
box-shadow.

**Tech Stack:** GPUI (this fork, `../Source/gpui`), `gpui::canvas` for
bounds capture, `WindowKind::AnchoredPopup` (`gpui/src/platform/popup.rs`),
existing `crates/app/src/updates_popup/`, `chronos_ui::Theme`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-24-updates-popup-anchored-redesign-design.md`.
- Design brief: `design/Updates Popup.dc.html` (dark reference + light
  "accepted, Light C" variant — literal hex/shadow/opacity values, do not
  re-derive by eye).
- **Spec correction found while planning:** the spec said
  `anchor: BottomRight, gravity: BottomRight`. That pushes the popup
  further right, off-screen for an icon already near the bar's right
  edge. Correct pairing per the mockup (popup's right edge aligned with
  the icon's right edge, extending down-and-left): `anchor:
  PopupAnchor::BottomRight`, `gravity: PopupGravity::BottomLeft`. This
  plan uses the corrected pairing; the spec's positioning intent (anchor
  at the trigger, not a fixed corner) is unchanged.
- **Spec correction found while planning:** the spec assumed raw
  `rgb(0x..)` hex literals (sidebar-v2 first-pass style). Actual
  `view.rs` already uses `Theme::global(cx)` tokens throughout (not
  hardcoded hex) — this is a MORE mature file than sidebar v2's first
  pass, don't regress it. This plan keeps token-based color for
  everything that already has a token; only the mockup's genuinely NEW
  decorations (watermark sigil, glow-top line, elevated shadow — light-
  theme-only, no existing token) get literal values, gated by the new
  `theme.is_light` flag.
- **`light_scheme()` already exists and is wired**
  (`crates/ui/src/theme/schemes.rs`, commits `0f0ee88`/`5bb6c77`) — this
  was misdocumented as "not started" in `roadmap.md` until corrected
  2026-07-24 during this plan's research. Both dark and light are live-
  switchable today; this plan can and should verify both, not just dark.
- `unsafe_code = "deny"` at workspace level — nothing here needs `unsafe`.
- `clippy::unwrap_used`/`expect_used` are `warn` — no new `.unwrap()`/
  `.expect()`. Task 4 touches an existing `let _ = handle.update(...)`
  pair (mod.rs, the resize-on-state-change watcher) — per the project's
  "never swallow a fallible call" rule, replace both with `.log_err()`
  since you're already touching that exact code, not left as-is.
- Existing dismiss paths (Esc, "✕" button, re-toggle bar icon, "Upgrade
  all") stay as they are — `ARCHITECTURE.md §4.1` already establishes
  "explicit dismiss only, never focus-loss" and `AnchoredPopup`'s
  `grab: true` only ADDS native other-app-click dismissal on top; it does
  not replace or need new own-app dismiss wiring for this pass.
- Fallback: if `cx.open_window(...)` returns `PopupNotSupportedError` (or
  the fork's equivalent — verify exact error type in Task 2, Step 1),
  fall back to the CURRENT standalone `LayerShell` `TOP|RIGHT` path,
  don't hard-fail.
- Live UX verification is mandatory (release build + `grim`), not unit
  tests alone — Task 5 is not optional polish.

---

### Task 1: `Theme.is_light` flag

**Files:**
- Modify: `crates/ui/src/theme/mod.rs` (add field to `Theme` struct,
  `Default` impl)
- Modify: `crates/ui/src/theme/schemes.rs` (`default_scheme()`,
  `light_scheme()`)
- Test: inline `#[cfg(test)]` in `crates/ui/src/theme/schemes.rs`
  (existing test module already there — add to it)

**Interfaces:**
- Produces: `Theme.is_light: bool` — Task 4 reads this to gate the
  watermark/glow/shadow decorations.

- [ ] **Step 1: Write the failing test**

```rust
// crates/ui/src/theme/schemes.rs — inside the existing `#[cfg(test)] mod tests` block
#[test]
fn is_light_flag_matches_scheme() {
    assert!(!default_scheme().theme.is_light);
    assert!(light_scheme().theme.is_light);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p chronos-ui is_light_flag_matches_scheme -- --nocapture`
Expected: FAIL to compile — `Theme` has no field `is_light`.

- [ ] **Step 3: Add the field**

```rust
// crates/ui/src/theme/mod.rs — add to the `Theme` struct (after `font_ui`)
pub struct Theme {
    pub bg: BgColors,
    pub text: TextColors,
    pub border: BorderColors,
    pub accent: AccentColors,
    pub status: StatusColors,
    pub interactive: InteractiveColors,
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub transparent: Hsla,
    pub font_sizes: FontSizes,
    pub font_mono: &'static str,
    pub font_ui: &'static str,
    /// True for light color schemes (Light C and any future light variant).
    /// Gates light-only decoration (watermark sigils, glow-edges) that has
    /// no dark-theme equivalent — see `updates_popup` for the first use.
    pub is_light: bool,
}
```

Set it in `Default for Theme` (the existing dark default): add
`is_light: false,` to that struct literal.

```rust
// crates/ui/src/theme/schemes.rs — in light_scheme(), after `let mut theme = Theme::default();`
theme.is_light = true;
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p chronos-ui is_light_flag_matches_scheme -- --nocapture`
Expected: `test result: ok. 1 passed; 0 failed`

- [ ] **Step 5: Full workspace check**

Run: `cargo check -p chronos`
Expected: clean — `Theme { .. }` struct literals elsewhere in the tree
either use `..Theme::default()` (unaffected) or already list every field
(if any do, this step's error tells you where to add `is_light: false`).

- [ ] **Step 6: Commit**

```bash
git add crates/ui/src/theme/mod.rs crates/ui/src/theme/schemes.rs
git commit -m "chronos_ui : Theme.is_light flag for light-only decoration"
```

---

### Task 2: Bounds capture + mouse-down trigger in the bar widget

**Files:**
- Modify: `crates/app/src/bar/widgets/updates.rs` (add bounds capture,
  switch `on_click` → `on_mouse_down`)

**Interfaces:**
- Consumes: nothing new from other tasks.
- Produces: calls `crate::updates_popup::open(cx, anchor_rect, parent)`
  (exact signature defined in Task 3 — this task calls it, Task 3 defines
  it; write this task's call site to match, then make Task 3's signature
  agree).

- [ ] **Step 1: Add a shared bounds cell and canvas overlay**

```rust
// crates/app/src/bar/widgets/updates.rs — imports (add to existing `use gpui::{...}`)
use gpui::{
    AnyElement, App, Bounds, MouseButton, Pixels, Window, canvas, div, prelude::*, px, svg,
};
use std::cell::Cell;
use std::rc::Rc;
```

Add a field to carry the captured bounds across renders — `UpdatesWidget`
is currently a unit struct (`pub struct UpdatesWidget;`), change it to
hold a shared cell:

```rust
pub struct UpdatesWidget {
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl UpdatesWidget {
    pub fn new() -> Self {
        Self {
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }
}
```

Update `register()` (bottom of file) to use `UpdatesWidget::new()`
instead of the unit-struct literal:

```rust
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(UpdatesWidget::new()));
}
```

- [ ] **Step 2: Wrap the widget row in a canvas to capture its bounds**

In `render()`, after building `row` (the existing `div()` chain with the
icon/count) but before `.into_any_element()`, wrap it so bounds are
captured every prepaint. Replace the final `row.on_click(...)
.into_any_element()` with:

```rust
        let bounds_cell = self.bounds.clone();
        div()
            .child(row.on_mouse_down(MouseButton::Left, {
                let bounds_cell = self.bounds.clone();
                move |_event, window, cx: &mut App| {
                    let anchor_rect = bounds_cell.get();
                    let parent = window.window_handle();
                    crate::updates_popup::toggle(anchor_rect, parent, window, cx);
                }
            }))
            .child(
                canvas(
                    move |bounds, _window, _cx| bounds,
                    move |bounds, captured, _window, _cx| {
                        bounds_cell.set(captured);
                        let _ = bounds;
                    },
                )
                .absolute()
                .size_full(),
            )
            .into_any_element()
```

Note: `canvas`'s `prepaint` callback receives the canvas ELEMENT's own
bounds, which is why it's laid out `.absolute().size_full()` as a sibling
overlay on top of `row` — it inherits the same box, so its captured
bounds equal the row's bounds. If `.absolute()` on a plain `div()`
doesn't position it flush over its sibling in this fork (verify live in
Task 5), the fallback is making `row` itself the canvas's paint target by
restructuring so `row` is built INSIDE the canvas's `paint` closure — flag
this in the report if the simpler sibling-overlay approach doesn't work,
don't silently ship a misaligned anchor.

- [ ] **Step 3: Run existing widget tests to confirm no regression**

Run: `cargo test -p chronos --lib bar::widgets::updates -- --nocapture`
Expected: existing `describe()`-based tests still pass unchanged (this
task doesn't touch `describe()`/`UpdatesView`).

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/bar/widgets/updates.rs
git commit -m "bar/updates : capture trigger bounds, mouse-down for anchored popup"
```

---

### Task 3: `WindowKind::AnchoredPopup` in `updates_popup`

**Files:**
- Modify: `crates/app/src/updates_popup/mod.rs`

**Interfaces:**
- Consumes: `Bounds<Pixels>` anchor rect + `AnyWindowHandle` parent from
  Task 2's call site.
- Produces: `pub fn toggle(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle, window: &mut Window, cx: &mut App)`
  replacing the current `pub fn toggle(_window: &mut Window, cx: &mut App)`.
  `open()` gains the same two new parameters.

- [ ] **Step 1: Verify the exact not-supported error type**

Run: `grep -n "PopupNotSupportedError\|fn open_window" /home/neo/projects/chronos-ecosystem/Source/gpui/src/platform/popup.rs /home/neo/projects/chronos-ecosystem/Source/gpui/src/app.rs`

Confirm the error variant's exact path (e.g. `gpui::PopupNotSupportedError`
vs nested under another error enum) before writing the match arm in Step
3 — do not guess the path.

- [ ] **Step 2: Change `window_options` to build `PopupOptions`**

```rust
// crates/app/src/updates_popup/mod.rs — imports, add:
use gpui::{
    AnyWindowHandle, PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupOptions,
};

// replace the existing `window_options` function:
fn window_options(
    anchor_rect: Bounds<Pixels>,
    parent: AnyWindowHandle,
    height: f32,
) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-updates-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomRight,
            gravity: PopupGravity::BottomLeft,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::FLIP_X,
            offset: point(px(0.), px(4.)),
            grab: true,
        }),
        ..Default::default()
    }
}

/// Fallback window options when `AnchoredPopup` isn't supported on this
/// platform — the pre-redesign standalone LayerShell path, unchanged.
fn fallback_window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-updates-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "updates-popup".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(POPUP_MARGIN_TOP), px(POPUP_MARGIN_RIGHT), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}
```

- [ ] **Step 3: Update `open()` to accept anchor/parent and fall back on error**

```rust
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) {
    AppState::aur(cx).dispatch(AurCommand::Refresh);

    if cx.global::<UpdatesPopupState>().handle.is_some() {
        return;
    }

    let count = AppState::aur(cx).get().count();
    let height = estimate_popup_height(count);

    let result = cx.open_window(window_options(anchor_rect, parent, height), |_, app_cx| {
        app_cx.new(|view_cx| UpdatesPopupView::new(view_cx))
    });

    let result = match result {
        Err(_not_supported) => {
            tracing::warn!("updates_popup: AnchoredPopup not supported on this platform, falling back to fixed-corner LayerShell");
            let display_id = pick_display(cx);
            cx.open_window(fallback_window_options(display_id, height), |_, app_cx| {
                app_cx.new(|view_cx| UpdatesPopupView::new(view_cx))
            })
        }
        ok => ok,
    };

    match result {
        Ok(new_handle) => {
            cx.global_mut::<UpdatesPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("updates_popup: failed to open popup: {err}"),
    }
}
```

Match the fallback's error arm to the EXACT error type confirmed in Step
1 — if `cx.open_window` returns a generic error that could be something
other than "not supported" (e.g. a real allocation failure), narrowing
the match to only retry on the specific not-supported variant (not any
`Err`) is more correct; adjust the `match` here accordingly once you've
read the real signature.

- [ ] **Step 4: Update `toggle()`**

```rust
pub fn toggle(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle, _window: &mut Window, cx: &mut App) {
    let is_open = cx.global::<UpdatesPopupState>().handle.is_some();
    if is_open {
        close(cx);
    } else {
        open(cx, anchor_rect, parent);
    }
}
```

- [ ] **Step 5: `cargo check` — fix call sites**

Run: `cargo check -p chronos`
Expected: the only remaining error should be Task 2's call site in
`bar/widgets/updates.rs` (already written to match this signature in
Task 2 — if Task 2 was done first, this should already be green;
if this task lands first, go back and confirm Task 2's call site matches
exactly).

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/updates_popup/mod.rs
git commit -m "updates_popup : WindowKind::AnchoredPopup with LayerShell fallback"
```

---

### Task 4: Real scroll instead of clip

**Files:**
- Modify: `crates/app/src/updates_popup/mod.rs` (`LIST_MAX_H` value,
  the two `let _ = handle.update(...)` in `init()`)
- Modify: `crates/app/src/updates_popup/view.rs` (list container,
  `UpdatesPopupView` struct)

**Interfaces:**
- Consumes: nothing new.
- Produces: nothing consumed by later tasks (Task 5 is visual-only on
  top of this).

- [ ] **Step 1: Bump `LIST_MAX_H` to match the mockup**

```rust
// crates/app/src/updates_popup/mod.rs
pub(crate) const LIST_MAX_H: f32 = 340.;
```

(`MAX_POPUP_H` recomputes automatically — it's `HEADER_BUDGET_H +
DIVIDER_H + LIST_MAX_H + FOOTER_BUDGET_H`.)

- [ ] **Step 2: Fix the swallowed-error pattern while touching this function**

```rust
// crates/app/src/updates_popup/mod.rs — inside init()'s watch() callback,
// replace both `let _ = handle.update(...)` lines:
                if let Some(handle) = handle {
                    let height = estimate_popup_height(updates_state.count());
                    handle
                        .update(cx, |_, window: &mut Window, _| {
                            window.resize(Size::new(px(POPUP_WIDTH), px(height)));
                        })
                        .log_err();
                    handle
                        .update(cx, |_, _window, view_cx| view_cx.notify())
                        .log_err();
                }
```

This needs `use gpui::LogErr as _;` or the fork's equivalent — verify the
exact trait/import path used elsewhere in the tree first:
`grep -rn "\.log_err()" crates/app/src | head -3` and match that import.

- [ ] **Step 3: Give `UpdatesPopupView` a `ScrollHandle`**

```rust
// crates/app/src/updates_popup/view.rs — imports, add ScrollHandle
use gpui::{
    AnyElement, App, Context, InteractiveElement, IntoElement, Render, ScrollHandle, Styled,
    Window, div, prelude::*, px,
};

pub struct UpdatesPopupView {
    scroll: ScrollHandle,
}

impl UpdatesPopupView {
    pub fn new(_cx: &mut App) -> Self {
        Self {
            scroll: ScrollHandle::new(),
        }
    }
}
```

- [ ] **Step 4: Replace the clip with real scroll, drop row-truncation**

The current `list` branch truncates rows to `max_visible_rows()` and
hard-clips with `.max_h().overflow_hidden()`. Replace the non-empty
branch entirely — every row renders, the container scrolls:

```rust
        let list: AnyElement = if updates.is_empty() {
            div()
                .w_full()
                .px(px(ROW_PAD_X))
                .py(px(ROW_PAD_Y))
                .text_color(text_muted)
                .child("System is up to date")
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = updates
                .iter()
                .map(|u| render_row(u, text_primary, text_muted, radius, hover, accent_hover))
                .collect();
            div()
                .id("updates-popup-list")
                .w_full()
                .max_h(px(LIST_MAX_H))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .flex_col()
                .children(rows)
                .into_any_element()
        };
```

`render()`'s signature already takes `&mut Context<Self>` as `cx` — no
signature change needed to reach `self.scroll`.

- [ ] **Step 5: Remove now-dead code**

`max_visible_rows()` and the `+N more` overflow-note branch in `view.rs`
are no longer reachable (every row renders now) — delete
`max_visible_rows()` from `mod.rs` and its import in `view.rs`. Leave
`ROW_H` — still used as the row-geometry constant for `estimate_popup_height`'s
sibling logic if anything references it; if `cargo check` flags it
unused after this deletion, remove it too.

- [ ] **Step 6: Run tests**

Run: `cargo test -p chronos --lib updates_popup -- --nocapture`
Expected: green, no reference to `max_visible_rows` remains anywhere in
the tree (`grep -rn max_visible_rows crates/` returns nothing).

- [ ] **Step 7: Commit**

```bash
git add crates/app/src/updates_popup/mod.rs crates/app/src/updates_popup/view.rs
git commit -m "updates_popup : real overflow_y_scroll instead of clip+truncate"
```

---

### Task 5: Visual polish — watermark, glow, shadow (light-only)

**Files:**
- Modify: `crates/app/src/updates_popup/view.rs`

**Interfaces:**
- Consumes: `Theme.is_light` (Task 1).
- Produces: nothing consumed elsewhere.

- [ ] **Step 1: Verify the box-shadow builder API before writing this step**

This fork's `Styled`/`Style` has a `box_shadow: Vec<BoxShadow>` field
(`Source/gpui/src/style.rs:288`) but this codebase has **no existing
`.shadow(...)`-style call to copy** — confirm the exact chainable method
(if one exists) or the `StyleRefinement` field-set path:

```bash
grep -rn "fn shadow\|box_shadow" /home/neo/projects/chronos-ecosystem/Source/gpui/src/styled.rs
```

If no convenience method exists, build it via
`.refine(&StyleRefinement { box_shadow: Some(vec![BoxShadow { .. }]), ..Default::default() })`
— read `BoxShadow`'s exact fields (`Source/gpui/src/style.rs:345`,
remember this fork adds an `inset` field vs crates.io per
`skills/fork-api-drift`) before filling them in. Do not guess field
names — this is exactly the class of error `fork-api-drift` documents.

- [ ] **Step 2: Add the watermark + glow line, gated by `is_light`**

```rust
// crates/app/src/updates_popup/view.rs — top-level render(), after computing `theme`
let is_light = theme.is_light;
```

Add near the end of `render()`, wrapping the final returned `div()` so
the decorations sit behind the content (same z-order as the mockup's
`position:absolute` watermark):

```rust
        let mut card = div()
            .relative()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg)
            .border_1()
            .border_color(border_subtle)
            .overflow_hidden();

        if is_light {
            card = card
                .child(
                    // glow-top hairline — accent gradient is not a plain
                    // gpui color, mockup uses a CSS linear-gradient; this
                    // fork's div doesn't do gradients on a 1px bg directly,
                    // so approximate with a solid accent line at low
                    // opacity (visual parity call, not pixel-identical —
                    // note this in the report if it reads wrong live).
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .right(px(0.))
                        .h(px(1.))
                        .bg(accent)
                        .opacity(0.4),
                )
                .child(
                    svg()
                        .path("icons/hexagon-sigil.svg")
                        .absolute()
                        .top(px(-30.))
                        .right(px(-30.))
                        .size(px(140.))
                        .text_color(accent)
                        .opacity(0.18),
                );
        }

        card.child(header).child(divider_line).child(list).child(footer)
```

Remove the old plain `div()...child(header)...` tail that this replaces
— this task's Step 2 code block is the new return expression, replacing
the current final statement of `render()`.

`svg` needs importing: add `svg` to the existing `use gpui::{ .. }` list
in `view.rs` if not already present (check first — `render_row` doesn't
use it, `mod.rs` does for other things, `view.rs` currently doesn't
import `svg` — add it).

- [ ] **Step 3: Apply the shadow using whatever Step 1 found**

Using the confirmed API from Step 1, apply an elevated shadow to the
outer `card` div only `if is_light` (dark variant's mockup value is
`none` — no shadow at all, matches current no-shadow dark rendering, so
gate the whole shadow application behind `if is_light`, not just the
color).

- [ ] **Step 4: Run tests**

Run: `cargo test -p chronos --lib updates_popup -- --nocapture`
Expected: still green (this task is pure rendering, no logic under test
changed).

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/updates_popup/view.rs
git commit -m "updates_popup : light-theme watermark sigil + glow-edge + shadow"
```

---

### Task 6: Live verification (mandatory, not optional)

**Files:** none (verification only).

- [ ] **Step 1: Release build**

Run: `cargo build --release -p chronos`
Expected: clean build, no new warnings.

- [ ] **Step 2: Live smoke — positioning**

```bash
pkill -x chronos 2>/dev/null
RUST_LOG=info ./target/release/chronos &
sleep 2
```

Click the updates icon in the bar (or dispatch the equivalent action).
Expected: popup opens directly below/aligned to the icon's right edge
(NOT in a fixed screen corner unrelated to the icon's actual bar
position), extending down-and-left. `hyprctl layers` (or `hyprctl
clients` if the popup registers as a toplevel under `AnchoredPopup` —
check which) confirms geometry is anchored near the icon, not at a
constant absolute position.

Grim it: name the screenshot `updates-popup-anchored-dark.png`.

- [ ] **Step 3: Live smoke — scroll**

With more pending updates than fit in 340px (fake data or a real system
with many pending packages), scroll the list. Expected: every row is
reachable via scroll, "Upgrade all" stays visible and clickable at all
times, no row is silently lost the way the pre-redesign clip lost them.

Grim it: `updates-popup-scrolled-dark.png`.

- [ ] **Step 4: Live smoke — light theme**

Switch to light scheme (`theme.toml` or however `select_scheme()` is
invoked live — check `chronos::theme_config` for the mechanism). Expected:
watermark sigil + glow-top line + elevated shadow all visible, matching
`design/Updates Popup.dc.html`'s light variant reasonably closely (exact
gradient approximation noted in Task 5 Step 2 is an acceptable, reported
deviation — everything else should match).

Grim it: `updates-popup-anchored-light.png`.

- [ ] **Step 5: Live smoke — dismiss paths**

Confirm Esc, "✕" button, and re-clicking the bar icon all close the
popup. Confirm a click on a genuinely different part of the bar (own-app,
different window) does NOT silently reach through and misbehave — per
this plan's Global Constraints, no NEW own-app dismiss wiring was added,
so this should behave identically to before the redesign; just confirm
no regression.

- [ ] **Step 6: Fallback path — compile-only check**

The `PopupNotSupportedError` fallback branch (Task 3) can't be forced to
trigger live on a working Hyprland/Wayland session — confirm it at least
compiles and is logically reachable (read the code path once more), and
say so plainly in the report rather than claiming it was exercised live.

- [ ] **Step 7: Record the result**

If Steps 2-5 hold, this plan is done. Note any visual deviation from the
mockup (especially the glow-gradient approximation from Task 5) as a
reported, not hidden, gap.

---

## Self-review

**Spec coverage:** anchored positioning (Tasks 2-3), real scroll (Task
4), visual chrome (Task 5), fallback (Task 3) — all four of the user's
original complaints (fixed corner, deferred scroll, MVP look, no content-
adaptive sizing) are addressed: fixed-corner → anchored (Task 3),
deferred scroll → real scroll (Task 4), MVP look → mockup chrome (Task
5), no adaptive sizing → same cap+scroll model the spec settled on
(Task 4), which is genuinely adaptive within the 340px budget (window
doesn't over-allocate for short lists — `estimate_popup_height` already
returns the small empty-state height when `count == 0`).

**Placeholder scan:** the two "verify exact API before coding" steps
(Task 3 Step 1 error type, Task 5 Step 1 shadow builder) are explicit
verification instructions with a concrete fallback path each, not vague
TODOs — matches the `Theme::global_static()` precedent from T112's
accepted plan.

**Type consistency:** `open(cx, anchor_rect: Bounds<Pixels>, parent:
AnyWindowHandle)` / `toggle(anchor_rect, parent, window, cx)` — same
three new parameters, same order, across Task 2's call site and Task 3's
definition.

**Two corrections made during planning, both flagged in Global
Constraints:** the spec's `anchor`/`gravity` pairing was geometrically
backwards (would push the popup off-screen), and the spec assumed hex-
literal colors where the real file already uses theme tokens. Both are
called out explicitly, not silently overridden.

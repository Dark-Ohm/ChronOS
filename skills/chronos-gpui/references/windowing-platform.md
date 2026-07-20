# Windowing platform — windows, Wayland, layer-shell, input

**When to load:** Before touching any `crates/app/src/**` surface that opens a
`Window`/`WindowKind::LayerShell`, calls `window.resize()`, reads `cx.primary_display()` /
`window.display()`, sets `keyboard_interactivity`, or binds pointer/scroll/clicks. Also
load when someone says "the fork can't do X" about windowing — every claim below is
anchored to `../Source/` at the commit this skill was written against.

**Ground-truth rule (from the SKILL.md preamble):** a "the fork cannot X" claim needs a
`file:line` from `Source/` or a runnable example. None of the negative claims below are
retellings — they were read from the fork.

---

## 0. How ChronOS opens a layer-shell surface

`WindowKind::LayerShell(LayerShellOptions { .. })` is passed in `WindowOptions.kind`
(`Source/gpui/src/window.rs` — `WindowKind` enum; `Source/gpui/src/platform/layer_shell.rs`
owns `LayerShellOptions`/`Layer`/`Anchor`/`KeyboardInteractivity`). On Wayland this routes
to `WaylandWindow::new` (`Source/gpui_linux/src/linux/wayland/window.rs:151`): the branch
`if let WindowKind::LayerShell(options) = &params.kind` creates the `zwlr_layer_surface_v1`
and pushes **initial** geometry through the protocol on creation:

- `layer_surface.set_size(width, height)` — from `params.bounds.size` (window.rs:165-167)
- `set_anchor` / `set_keyboard_interactivity` / `set_margin` / `set_exclusive_zone`
  (window.rs:169-189)

**Proof example:** `Source/gpui/examples/layer_shell.rs` is a clock on a
`LayerShell` (anchor `LEFT|RIGHT|BOTTOM`, `keyboard_interactivity: None`, namespace
`"gpui"`). `cargo check --example layer_shell -p 'path+file:///.../Source/gpui#0.2.2'`
→ green (verified 2026-07-20, only unrelated `nightly_coverage` cfg warnings).

---

## 1. `LayerShellOptions` — every field, and what it does in OUR code

Defined in `Source/gpui/src/platform/layer_shell.rs:59-77`. Mapped to the protocol in
`Source/gpui_linux/src/linux/wayland/layer_shell.rs` (`wayland_layer`, `wayland_anchor`,
`wayland_keyboard_interactivity`).

| Field | Type | Protocol effect | Notes |
|---|---|---|---|
| `namespace` | `String` | `get_layer_surface(.., namespace, ..)` | Set at creation, **immutable after** (comment at layer_shell.rs:60-61). Compositors use it for rules. |
| `layer` | `Layer` | `zwlr_layer_shell_v1::Layer` | `Background/Bottom/Top/Overlay` (default `Overlay`). layer_shell.rs:9-22. |
| `anchor` | `Anchor` (bitflags) | `zwlr_layer_surface_v1::Anchor` | `TOP/BOTTOM/LEFT/RIGHT` (1/2/4/8), combinable (`LEFT|RIGHT` stretches width). layer_shell.rs:24-39. |
| `exclusive_zone` | `Option<Pixels>` | `set_exclusive_zone(f32::from(x) as i32)` | `None` ⇒ 0 (no reserve). `exclusive_zone` reserved pixels push other surfaces away. window.rs:183-185 / `set_exclusive_zone` window.rs:458+. |
| `exclusive_edge` | `Option<Anchor>` | `apply_exclusive_edge(..)` | Which edge the exclusive zone hugs if `exclusive_zone` is set. window.rs:187-189. |
| `margin` | `Option<(Px,Px,Px,Px)>` | `set_margin(top,right,bottom,left)` | CSS order. window.rs:174-181. |
| `keyboard_interactivity` | `KeyboardInteractivity` | `set_keyboard_interactivity(..)` | `None` / `Exclusive` / `OnDemand` (default). See §4. |

**Changing these fields on a live window:** `namespace` cannot change (immutable). All
other geometry (`anchor`, `margin`, `exclusive_zone`, `keyboard_interactivity`) is set
**only at creation** in `new()` (window.rs:151-195). There is **no** code path that
re-issues `set_anchor`/`set_margin`/`set_keyboard_interactivity` after the surface is
mapped — to change them you must destroy and recreate the surface. (Dynamic resize of the
bounds is the exception — see §2.)

---

## 2. Resize & window size — `window.resize()`

**Public API:** `Window::resize(&mut self, size: Size<Pixels>)` at
`Source/gpui/src/window.rs:2318` → `self.platform_window.resize(size)`.

**Platform impl (Wayland):** `WaylandWindow::resize` at
`Source/gpui_linux/src/linux/wayland/window.rs:1340` →
`self.set_size_and_scale(Some(size), None)` (window.rs:1306). That function:
1. Early-returns if `size == state.bounds.size && scale unchanged` (window.rs:1309-1313)
   — **no-op on identical size**.
2. Writes `state.bounds.size = size` (window.rs:1314-1316).
3. Updates the viewport destination: `viewport.set_destination(w, h)` (window.rs:1331-1337)
   and fires the resize callback (window.rs:1325-1329).

**Does resize reach the Wayland protocol?** Yes, indirectly. The layer-surface size is
pushed on the next `set_geometry` call, which happens when GPUI recomputes window geometry
after the bounds change: `WaylandSurfaceState::set_geometry` (window.rs:418-431) → for
`LayerShell` it calls `layer_surface.set_size(width, height)` (window.rs:423-425). So a
`window.resize(Size)` is honored by the compositor, but the protocol `set_size` is emitted
from the geometry recompute path, **not** directly inside `WaylandWindow::resize`.

**Limits / compositor ignores:**
- A zero or negative size is a protocol error on `set_size`; GPUI clamps via
  `f32::from(size).max(1)` in the xdg/popup path (window.rs:322-324) and the layer path
  inherits bounds clamping in `default_bounds`/geometry. Don't resize to ≤0.
- A layer surface with `anchor = LEFT|RIGHT` (or `TOP|BOTTOM`) **stretches** to that
  dimension; the `height` (or `width`) you pass is then advisory for the other axis. The
  compositor owns the stretched axis.
- The window is **not** auto-sized to its children. Fixed bounds + unbounded children =
  bottom clipping — the standard "popup cut off" root cause (see §5, and the
  `gpui-layer-shell` skill for the fix pattern).

**`f32::from(pixels)` is required.** `Pixels` is `pub struct Pixels(pub(crate) f32)`
(`Source/gpui/src/geometry.rs:2677`) — the field is **private**. You cannot read
`pixels.0`. Use `f32::from(px)` (`geometry.rs:2909`) / `f32::from(size.width)`.

**`max_height` via style works (the better clamp).** `Style` has
`pub max_size: Size<Length>` (`Source/gpui/src/style.rs:234`), exposed as `.max_h()`,
`.max_w()`, `.max_size()` (`Source/gpui_macros/src/styles.rs:900/892/884`). To clip
overflowing content (e.g. a long notification body) the canonical fix is
`.max_h(px(N)).overflow_hidden()` — `overflow_hidden` is a real style method
(`styles.rs:135`, sets both axes to `Overflow::Hidden`). This is what ChronOS brief №12
used for `updates_popup`/`notifications` and it compiled + verified live. So "clamp the
height" has **two** correct tools: pass a clamped value to `resize()`, and/or set
`.max_h(...).overflow_hidden()` on the content element. Prefer the style clip when the
goal is "content never pushes siblings off-window".

---

## 3. Displays — `primary_display`, `window.display()`, `uuid`

- **`cx.primary_display()` returns `None` on Wayland.** `WaylandClient::primary_display`
  at `Source/gpui_linux/src/linux/wayland/client.rs:826-828` is literally `None`. This is
  NOT a bug — Wayland has no "primary display" concept, and our backend doesn't fake one.
  Don't call `default_bounds(display_id, cx)` expecting a real display from
  `cx.primary_display()` (window.rs:1229-1231); for layer-shell we pass
  `display_id: None` so target_output is unset and the surface lands on the
  compositor-default output (window.rs:879-882).
- **`window.display()` for a layer-shell surface is `None`** (the "blood fact").
  `Window::display_id` is filled from `platform_window.display().map(|d| d.id())`
  (window.rs:1345, and refreshed at window.rs:2293). But `WaylandWindow`'s `display` field
  is initialized to `None` (`window.rs:605`) and is only ever assigned from output
  events (`self.display = current_output`, window.rs:660) during normal output handling —
  for a freshly created layer-shell surface with no output mapping yet, `display()` returns
  `None`. `PlatformWindow::display()` (window.rs:1570-1576) maps `state.display` →
  `WaylandDisplay`. Confirmed: don't rely on `window.display()` being `Some` for
  layer-shell popups.
- **`PlatformDisplay::uuid()` exists** and is fallible (`Source/gpui/src/platform.rs:288`;
  `WaylandDisplay::uuid` at `Source/gpui_linux/src/linux/wayland/display.rs:31`). It
  derives a stable id from the output `name`. `id()` is `DisplayId::new(protocol_id)`
  (display.rs:27-29). `displays()` (client.rs:795) returns the live output list — that is
  the way to enumerate monitors in ChronOS, not `primary_display()`.
- **Monitors for wallpaper/positioning:** use `CompositorSubscriber` (DP-1/HDMI-A-1
  names) rather than re-enumerating via wayland-client — per ChronOS brief №8
  (don't pull wayland-client like waytrogen does).

---

## 4. Keyboard & focus

**`KeyboardInteractivity`** (`Source/gpui/src/platform/layer_shell.rs:43-55`):
`None` (no keyboard focus delivered), `Exclusive` (exclusive grab while above other
layer surfaces), `OnDemand` (default — focusable like a normal window). Mapped to
`zwlr_layer_surface_v1::KeyboardInteractivity` in
`Source/gpui_linux/src/linux/wayland/layer_shell.rs:18-26`.

**`Exclusive` is FORBIDDEN in ChronOS.** A `KeyboardInteractivity::Exclusive` layer
surface grabs keyboard focus and **freezes the input stack of Hyprland** (documented
blood fact, HANDOFF). ChronOS popups/bar use `None` or `OnDemand` only. The
`layer_shell.rs` example uses `None`.

**`Window::focus(handle, cx)`** (`window.rs:1910`) and **`Window::activate_window()`**
(`window.rs:5296` → `platform_window.activate()`). On Wayland,
`WaylandWindow::activate` (`window.rs:1616-1631`) requests an **xdg_activation** token
(app_id + surface + serial). Comment in-source: "the activation is likely going to be
rejected" — KWin/Mutter may use app_id to indicate attention, but it does **not** force
focus. `request_attention` is a no-op (window.rs:1633).

**Layer-shell does not participate in xdg_activation reliably** and never forcibly grabs
focus. Treat "bring to front / focus" as best-effort attention, not a guarantee — this is
why persistent popups in ChronOS open on a timer/click, not on activation.

---

## 5. Window lifecycle — `remove_window`, `Drop`, reentrancy

**`Window::remove_window(&mut self)`** (`Source/gpui/src/window.rs:1899-1901`) just sets
`self.removed = true`. The actual teardown happens in `App::update_window_id`
(window.rs:1728): it `.take()`s the window out of `cx.windows` (window.rs:1733), runs your
closure, then in the inner `trail` (window.rs:1739-1777) — if `window.removed` — it
removes `window_handles`/`windows`, fires `window_closed_observers`, and may `cx.quit()`
(window.rs:1769-1771).

**Reentrancy trap (`window not found` ghost windows).** `update_window_id` does
`let mut window = cx.windows.get_mut(id)?.take()?` (window.rs:1733) — it **empties the
slot for the duration of your closure**. If your closure is itself invoked from inside an
already-running update of that same window-id (e.g. an `on_click` that already holds
`&mut Window`), calling `handle.update(cx, |_, window, _| window.remove_window())` again
hits the **same id**, finds the slot empty → `Err("window not found")` (the `.context(...)`
at window.rs:1781), silently swallowed by `let _ =`. Result: `remove_window()` never runs,
the popup stays as a ghost, global close-state is already reset → next click opens a new
window over the old one. **Fix (ChronOS pattern):** when the callback already holds
`&mut Window`, call `window.remove_window()` directly on that reference; reserve
`close(cx)`-style `handle.update` for genuinely external paths (timers, `cx.spawn`). Grep
every `close(` / `remove_window()` call site and classify reentrant vs external.

**`Drop for WaylandWindow`** (`Source/gpui_linux/src/linux/wayland/window.rs:680-750`)
is **synchronous** and self-documents the historical bug: previously `close()`+`drop_window`
lived in a detached task and the destroy requests were never flushed, leaving ghost
layer-shell surfaces in `hyprctl layers` and late `window not found` errors. Now the drop
`state.renderer.destroy()`, destroys blur/decoration/surface_state(viewport last)/
`wl_surface` (window.rs:717-741), then `client.drop_window(&surface_id)` (window.rs:749)
to unregister from event routing **before** any later frame callback can route at the dead
surface, and flushes on the same backend connection. Log lines to expect on a clean close
(per ChronOS verification briefs): `wayland: Drop WaylandWindow surface_id=… (sync
destroy+unregister+flush)` then `wayland: protocol destroy queued …`. If you see a popup
linger in `hyprctl layers -j` after close, the close was reentrant-swallowed (see above),
not a drop bug.

---

## 6. Input — scroll / click / hover / cursor on a layer-shell surface

**Scroll path (confirmed live in ChronOS bar).** Wayland `wl_pointer` axis/frame events
arrive in `WaylandClientStatePtr` (`Source/gpui_linux/src/linux/wayland/client.rs`). On
`wl_pointer::Event::Frame` (client.rs:2179) it batches `continuous_scroll_delta` /
`discrete_scroll_delta` and, if a window is under the pointer
(`state.mouse_focused_window`), emits `PlatformInput::ScrollWheel(ScrollWheelEvent {..})`
and calls `window.handle_input(input)` (client.rs:2183-2206). So scroll **does** reach a
layer-shell surface — delivered to the window currently holding mouse focus, then
dispatched to the hovered element's `on_scroll_wheel` listener.

**`InteractiveElement::on_scroll_wheel`** (`Source/gpui/src/elements/div.rs:357-371`)
binds a `ScrollWheelEvent` listener in the **bubble** phase, gated by
`hitbox.should_handle_scroll(window)`. Available directly on `div()` (it's on
`InteractiveElement`, `div.rs:699`; `Div` implements it at `div.rs:1695`).

**Click / hover / cursor:** `on_click` (`div.rs:1475`, bubble phase), `on_hover`
(`InteractiveElement`), `cursor`/`cursor_pointer` (styles.rs:164/178) — all live on
`InteractiveElement` and are therefore usable on bare `div()`. Hover is `hitbox.is_hovered`
gated (e.g. `on_pinch`, div.rs:379). None of these require `.id()`.

---

## 7. "We thought X — actually Y" (highest-value reframes vs the `gpui-layer-shell` skill)

These correct the existing `skills/gpui-layer-shell/SKILL.md`, which was written from
retellings, not the fork:

1. **`overflow_y_scroll()` does NOT require `.id()` / `Stateful<E>`.** It is a default
   method on `InteractiveElement` (`Source/gpui/src/elements/div.rs:1429`), and `Div`
   implements `InteractiveElement` directly (`div.rs:1695`). Bare `div().overflow_y_scroll()`
   compiles — proven by `Source/gpui/examples/animation.rs:60-62` (`.flex_col().h(px(150.)).overflow_y_scroll()` with no `.id()`) and `examples/scrollable.rs` (`cargo check` green).
   What actually requires `.id()`/`Stateful` is **`track_scroll(&ScrollHandle)`**
   (`div.rs:1435`, on `StatefulInteractiveElement`, `div.rs:1213`; `Stateful<E>` impl at
   `div.rs:3752`). The old skill inverted this: it claimed scroll "lives on
   `StatefulInteractiveElement`, only for `Stateful<E>`". That is false. The real scroll
   method is `overflow_y_scroll` (or `overflow_scroll` for both axes, div.rs:1416), and it
   works on a plain `div()`. `examples/scrollable.rs` shows the canonical nested
   `id("vertical").overflow_scroll()` / `id("horizontal").overflow_scroll()` usage
   (scrollable.rs:12-29).
2. **`resize()` does NOT flow to `set_size` at window.rs:1468.** That line (window.rs:1468)
   is inside `impl HasDisplayHandle for WaylandWindow` — unrelated to resize. The real path
   is `Window::resize` (window.rs:2318) → `WaylandWindow::resize` (window.rs:1340) →
   `set_size_and_scale` (window.rs:1306) → bounds update + viewport `set_destination`,
   then the geometry recompute emits `layer_surface.set_size` (window.rs:423-425). The old
   skill cited the wrong line; the mechanism (resize → protocol set_size) is correct, the
   citation isn't.
3. **`Style` HAS a max-height mechanism.** There is no field literally named `max_height`/
   `max_width` (`style.rs` grep: 0 hits), but `Style.max_size: Size<Length>` (style.rs:234)
   is exposed as `.max_h()` / `.max_w()` / `.max_size()` (styles.rs:900/892/884). So you
   CAN clamp height via style — and `overflow_hidden` (styles.rs:135) clips the overflow.
   Brief №12's `updates_popup` fix used exactly `.max_h(px(N)).overflow_hidden()` and it
   built + verified live. The old skill's "gpui `Style` has NO `max_height`" is
   technically true but misleadingly framed; the practical answer is "use `.max_h()`".
4. **`window.display()` being `None` for layer-shell is confirmed** (§3) — the blood fact
   holds; do not depend on it.
5. **`primary_display()` returning `None` on Wayland is confirmed** (§3) — enumerate via
   `displays()` instead.

---

## 8. How this applies to ChronOS (one-line notes, not architecture)

- Popup rubber-band height: prefer `window.resize(clamped_size)` AND/OR `.max_h().overflow_hidden()`
  on the overflowing content; both are correct (§2, §5/§7.3).
- Don't recreate a surface just to change anchor/margin/exclusive_zone/keyboard_interactivity
  unless you truly must — those are creation-only (§1).
- Never use `KeyboardInteractivity::Exclusive` (§4).
- Close popups by calling `window.remove_window()` on the `&mut Window` you already hold
  in the click/event closure; only use `handle.update(...close...)` from external paths
  (§5).
- Scroll already works on layer-shell (§6); no special wiring needed for `on_scroll_wheel`.

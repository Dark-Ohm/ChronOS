# windowing-platform — eval (verifiable)

Each question has a concrete answer and the `file:line` or example that proves it.
Use these to confirm the `windowing-platform.md` reference actually teaches the fork,
not a retelling. Answers were checked against `../Source/` at the commit this skill was
written against (2026-07-20).

---

## Q1 — Where does `window.resize(Size<Pixels>)` ultimately change the Wayland layer-surface size?

**A:** `Window::resize` (`Source/gpui/src/window.rs:2318`) → `WaylandWindow::resize`
(`Source/gpui_linux/src/linux/wayland/window.rs:1340`) → `set_size_and_scale` (window.rs:1306)
writes `state.bounds.size` and updates the viewport; the geometry recompute then calls
`WaylandSurfaceState::set_geometry` (window.rs:418) → for `LayerShell`,
`layer_surface.set_size(width, height)` (window.rs:423-425).

**Proven by:** `Source/gpui/examples/layer_shell.rs` (resize-style bounds at creation) +
reading window.rs:2318/1340/1306/423.

---

## Q2 — Does `div().overflow_y_scroll()` compile WITHOUT `.id()`?

**A:** Yes. `overflow_y_scroll` is a default method on `InteractiveElement`
(`Source/gpui/src/elements/div.rs:1429`), and `Div` implements `InteractiveElement`
(`div.rs:1695`). The only scroll method that needs `.id()`/`Stateful` is `track_scroll`
(`div.rs:1435`, on `StatefulInteractiveElement`, `div.rs:1213`; `Stateful<E>` impl
`div.rs:3752`).

**Proven by:** `Source/gpui/examples/animation.rs:60-62` calls `.flex_col().h(px(150.)).overflow_y_scroll()`
with no `.id()`; `cargo check --example animation -p 'path+file:///.../Source/gpui#0.2.2'` green.

---

## Q3 — What is the correct way to clamp a popup's height in `Style`?

**A:** `Style` has no field named `max_height`, but it has `max_size: Size<Length>`
(`Source/gpui/src/style.rs:234`), exposed as `.max_h()` / `.max_w()` / `.max_size()`
(`Source/gpui_macros/src/styles.rs:900/892/884`). Pair with `.overflow_hidden()`
(styles.rs:135, sets both axes to `Overflow::Hidden`) to clip overflow. This is what
brief №12 used for `updates_popup` and it built + verified live.

**Proven by:** style.rs:234 + styles.rs:884-904 + styles.rs:135; ChronOS commit `67f7d10`
(`updates_popup` `.max_h().overflow_hidden()`).

---

## Q4 — Why does `cx.primary_display()` return `None` in ChronOS?

**A:** `WaylandClient::primary_display` (`Source/gpui_linux/src/linux/wayland/client.rs:826-828`)
is literally `None`. Wayland has no "primary display" concept and our backend doesn't
fake one. Enumerate monitors via `displays()` (client.rs:795) or `CompositorSubscriber`
names (DP-1/HDMI-A-1) instead.

**Proven by:** client.rs:826-828 (the function body).

---

## Q5 — Is `window.display()` reliable for a layer-shell surface?

**A:** No. `WaylandWindow.display` is initialized to `None` (`Source/gpui_linux/src/linux/wayland/window.rs:605`)
and only gets assigned from output events (`self.display = current_output`, window.rs:660).
`PlatformWindow::display()` (window.rs:1570) maps `state.display`; for a freshly mapped
layer-shell surface it is `None`. `Window::display_id` is filled from
`platform_window.display()` (window.rs:1345/2293), so it inherits the `None`.

**Proven by:** window.rs:605, window.rs:660, window.rs:1570-1576.

---

## Q6 — What `KeyboardInteractivity` value must ChronOS NEVER use, and why?

**A:** `KeyboardInteractivity::Exclusive` (`Source/gpui/src/platform/layer_shell.rs:48-50`).
It grabs exclusive keyboard focus and **freezes the Hyprland input stack** (documented
blood fact, HANDOFF). Use `None` (popups/bar) or `OnDemand` (default) only. Mapped to
protocol at `Source/gpui_linux/src/linux/wayland/layer_shell.rs:18-26`.

**Proven by:** layer_shell.rs:43-55 + wayland/layer_shell.rs:18-26 + HANDOFF "СИСТЕМНЫЙ БАГ"
on Exclusive freeze.

---

## Q7 — Why does `handle.update(cx, |_, w, _| w.remove_window())` inside an `on_click` sometimes silently fail with "window not found"?

**A:** `App::update_window_id` (`Source/gpui/src/window.rs:1728`) does
`cx.windows.get_mut(id)?.take()?` (window.rs:1733) — it empties the slot for the duration
of the closure. If the closure is already running inside an update of the same window-id
(the `on_click` already holds `&mut Window`), a second `handle.update` on that id finds an
empty slot → `Err("window not found")` (window.rs:1781), swallowed by `let _ =`. The popup
becomes a ghost. Fix: call `window.remove_window()` on the `&mut Window` already held.

**Proven by:** window.rs:1728-1733, 1739-1781; ChronOS HANDOFF "СИСТЕМНЫЙ БАГ remove_window".

---

## Q8 — After a clean popup close, what log lines prove the Wayland surface was destroyed synchronously?

**A:** `wayland: Drop WaylandWindow surface_id=… (sync destroy+unregister+flush)` then
`wayland: protocol destroy queued for surface_id=…`. Emitted by `Drop for WaylandWindow`
(`Source/gpui_linux/src/linux/wayland/window.rs:680-750`), which synchronously destroys
renderer/blur/decoration/surface_state/`wl_surface` (window.rs:717-741) and unregisters
via `client.drop_window` (window.rs:749).

**Proven by:** window.rs:680-750 (self-documenting `Drop` with the historical-bug comment).

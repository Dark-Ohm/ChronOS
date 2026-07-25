# GPUI popup lifecycle (canonical template)

**2026-07-25:** bar-triggered popups are **AnchoredPopup** first
(`updates` / `volume` / `system` / history), with LayerShell TOP|RIGHT as
`PopupNotSupportedError` fallback. Full skeleton + slider/slow-backend rules:
skill **`chronos-gpui-popup`**. This file keeps reentrancy / sizing notes.

Verified against `updates_popup`, `volume_popup`, `system_popup`,
`history_popup`, `tray_menu` (tray may still be fixed-corner). Copy
`crates/app/src/volume_popup/` or `updates_popup/` for the anchored shape.

## The 6-piece shape

A popup needs exactly these parts. The reentrancy guard (`close_this`) is the
one thing people get wrong and it leaves a GHOST popup (see HANDOFF.md
"СИСТЕМНЫЙ БАГ: window.remove_window()").

```rust
// 1. Global holding the open window handle + the watcher entity.
#[derive(Default)]
pub struct XxxPopupState {
    handle: Option<WindowHandle<XxxPopupView>>,
    watcher: Option<Entity<XxxPopupWatcher>>,
}
impl Global for XxxPopupState {}

pub struct XxxPopupWatcher {}  // no state of its own

// 2. Window options — TOP|RIGHT overlay, never exclusive, no keyboard.
fn window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-xxx-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "xxx-popup".to_string(),
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

// 3. open() — idempotent; dispatch any "refresh" here; mark read if inbox.
pub fn open(cx: &mut App) {
    if cx.global::<XxxPopupState>().handle.is_some() { return; }
    let display_id = pick_display(cx);
    match cx.open_window(window_options(display_id, POPUP_HEIGHT), |_, app_cx| {
        app_cx.new(|view_cx| XxxPopupView::new(view_cx))
    }) {
        Ok(h) => cx.global_mut::<XxxPopupState>().handle = Some(h),
        Err(e) => tracing::warn!("xxx_popup: failed to open: {e}"),
    }
}

// 4. close() — safe from OUTSIDE the popup (bar click / external toggle).
pub fn close(cx: &mut App) {
    if let Some(h) = cx.global_mut::<XxxPopupState>().handle.take() {
        let _ = h.update(cx, |_, window: &mut Window, _| window.remove_window());
    }
}

// 5. close_this() — ONLY from INSIDE a callback that already holds &mut Window
//    for this popup. Clear the tracked handle BEFORE remove_window() so the
//    reentrant handle.update() in close() can't silently no-op.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx.global::<XxxPopupState>().handle.as_ref()
        .map(|h| **h == this).unwrap_or(false);
    if tracked { cx.global_mut::<XxxPopupState>().handle.take(); }
    window.remove_window();
}

// 6. toggle() — bar widget on_click holds &mut Window for the BAR, not the
//    popup, so closing an open popup here goes through close(cx), not close_this.
pub fn toggle(_window: &mut Window, cx: &mut App) {
    if cx.global::<XxxPopupState>().handle.is_some() { close(cx) } else { open(cx) }
}

// init() — wire the service subscription so the popup repaints. The watcher
// entity hosts state::watch(); it calls view_cx.notify() on the open handle.
pub fn init(cx: &mut App) {
    cx.set_global(XxxPopupState::default());
    let signal = AppState::xxx(cx).subscribe();
    let watcher = cx.new(|cx| {
        state::watch(cx, signal,
            |_this: &mut XxxPopupWatcher, _s: XxxState, cx: &mut Context<XxxPopupWatcher>| {
                if let Some(h) = cx.global::<XxxPopupState>().handle.clone() {
                    let _ = h.update(cx, |_, _w, view_cx| view_cx.notify());
                }
            });
        XxxPopupWatcher {}
    });
    cx.global_mut::<XxxPopupState>().watcher = Some(watcher);
}
```

`pick_display(cx)`:
```rust
fn pick_display(cx: &App) -> Option<DisplayId> {
    cx.primary_display().map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}
```

## BORROW-CHECKER PITFALL (cost a real compile error, Hermes №14)

If you factor a card/list renderer out of `Render::render` to REUSE it in two
popups, the helper must NOT take `&mut App` / `&mut Context` as a param. The
`render` body iterates `notifications.iter().map(|n| render_card(n, &theme, ..))`
over `&cx.global::<XxxState>().current` — an IMMUTABLE borrow of `cx` that lives
for the whole `.map()`. Passing `&mut App` into the helper re-borrows `cx`
mutably DURING that iteration -> borrow conflict.

Correct shape (what `notifications::view::render_notification_card` does):
```rust
pub(crate) fn render_notification_card(
    n: &Notification,
    theme: &Theme,                       // immutable ref, fine
    close_button: Option<AnyElement>,   // pre-built outside
) -> AnyElement {
    // urgency color: match on n.urgency -> theme.status.*
    // build header/title/body/actions...
    // on_click closures CAPTURE n.id by value and use the runtime-provided
    // `cx: &mut App` from the handler signature — NOT the outer cx.
    //   .on_click(move |_e, _w, cx: &mut App| {
    //       let svc = AppState::notification(cx).clone();
    //       cx.background_spawn(async move { let _ = svc.dispatch(...).await; })
    //           .detach(); // REQUIRED — Task drop cancels the future (T120)
    //   })
    // close_button (if Some) is just .child()-ed into the header.
    card.into_any_element()
}
```
The caller builds `close_button` as a ready `AnyElement` (with its own
`on_click` capturing the id) and passes it in. Action buttons built INSIDE the
helper are fine because their `on_click` uses the runtime `cx`, not the outer
one. The key rule: **no `&mut App`/`&mut Context` parameter into a helper that
is invoked inside a `.map()` over a `cx.global()` snapshot.**

## MarkAllRead vs DismissAll (history/inbox semantics)

When adding an "inbox read" concept: do NOT reuse `DismissAll` to clear the
unread counter — `DismissAll` wipes the ephemeral notifications too (and the
history, if you naively wired it that way). Add a SEPARATE command
(`NotificationCommand::MarkAllRead`) that touches ONLY the `unread` counter and
leaves `history` intact. Opening the inbox popup is the natural place to
dispatch `MarkAllRead` so the bell dot clears the moment it's viewed. This is
the exact split done in `services/notification/mod.rs` + `history_popup/mod.rs`.

## Sizing

Layer-shell surfaces do NOT auto-size to children (no `Style::max_height` in
this fork; `overflow_y_scroll()` doesn't resolve — see
`references/notifications-module-patterns.md` §8). Use a FIXED window height
(`POPUP_HEIGHT` constant) + a hard `.max_h(CAP).overflow_hidden()` on the inner
list container. Do NOT estimate content height from per-glyph constants (that
drifted and silently clipped — Hermes #9->#11->#12; commit `67f7d10`).

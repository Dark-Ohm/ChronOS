# Notifications popup module patterns (ChronOS gpui-ce fork, 2026-07-17)

Verified facts from building `crates/app/src/notifications/` against
`../Source/gpui` (gpui-ce fork). These are NOT obvious from the upstream
`reference/gpui-shell-main` donor and were each hit + fixed during the build.

## 1. `state::watch` needs an ENTITY context, not `&mut App`

`crate::state::watch(cx, signal, on_update)` is declared
`watch<C, S, T, F>(cx: &mut Context<C>, signal, on_update)`. You CANNOT call
it with the `&mut App` you get in `init(cx: &mut App)` — `Context::new_context`
is `pub(crate)`, so there is no public way to turn `&mut App` into
`&mut Context<C>`.

**Fix:** host the watch loop on a tiny throwaway entity. `watch` ties its
update task to the entity's lifetime via `this.update`, so the entity MUST
stay alive or the subscription silently stops. Store the `Entity` in your
global.

```rust
pub fn init(cx: &mut App) {
    cx.set_global(NotificationPopupState::default());
    let signal = AppState::notification(cx).subscribe(); // see §3
    let watcher = cx.new(|cx| {
        state::watch(
            cx,                                   // &mut Context<NotificationWatcher>
            signal,
            |_this: &mut NotificationWatcher,
             state: NotificationState,
             cx: &mut Context<NotificationWatcher>| {
                cx.global_mut::<NotificationPopupState>().current = state;
                sync_window(cx);                  // free fn, takes &mut App (coerced)
            },
        );
        NotificationWatcher {}
    });
    cx.global_mut::<NotificationPopupState>().watcher = Some(watcher);
}
```

`on_update` is `Fn(&mut C, T, &mut Context<C>) + 'static`; ignoring `_this`
is fine. `sync_window(cx: &mut App)` works because `&mut Context<Watcher>`
deref-coerces to `&mut App`.

## 2. `on_click` is NOT `cx.listener` — and service `dispatch` is async

`Div::on_click` (fluent, via the `InteractiveElement` trait) has the signature
`fn on_click(self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)`.
It does NOT take a `cx.listener(...)` closure. Also `Service::dispatch` (e.g.
`NotificationSubscriber::dispatch`) is `async fn`, so you must spawn a task.

**Fix:** import `InteractiveElement` explicitly (it is NOT in `prelude::*`),
and spawn the dispatch on a background task with a cloned subscriber:

```rust
use gpui::{InteractiveElement, ...};

div()
    .cursor_pointer()
    .on_click(move |_event, _window, cx| {
        let svc = AppState::notification(cx).clone();   // NotificationSubscriber: Clone
        cx.background_spawn(async move {
            let _ = svc.dispatch(NotificationCommand::Close(id)).await;
        })
        .detach(); // REQUIRED — see blood fact below
    })
    .child("✕")
```

`background_spawn` (from `AppContext`) needs a `Send + 'static` future; the
cloned subscriber + `NotificationCommand` are both `Send`. `cursor_pointer()`
comes through the `Styled` trait (in `prelude`), but `on_click` does NOT — the
`InteractiveElement` import is mandatory or you get "method not found in
`gpui::Div`".

### Blood fact (T120, 2026-07-25): **always `.detach()`**

`background_spawn` returns a `Task`. Dropping the Task **cancels** the
future immediately (gpui_scheduler). `let _ = cx.background_spawn(...)`
or bare `background_spawn(...);` → Close / ClearHistory / MarkAllRead
**never run**. Append `.detach()` so the task outlives the click handler.
Same for history popup and ephemeral toast actions.

## 3. `AppState::services` is private — use the accessor associated functions

`crates/app/src/state.rs` exposes `AppState::notification(cx) -> &NotificationSubscriber`
(and `compositor`/`network`/`upower`) as **associated functions**, not methods.
`AppState::global(cx).notification()` does NOT compile ("associated function,
not a method"). Call `AppState::notification(cx)`. If you add a new subscriber
accessor, mirror the existing `pub fn notification(cx: &App) -> &...` shape.

## 4. `cx.notify()` only repaints the ENTITY, not a global-driven view

`Context<C>::notify()` calls `app.notify(entity_id)` — it targets the entity
`C`, not globals. A view that renders from a global snapshot will NOT repaint
just because the global changed (unless it re-reads every frame, which is
wasteful).

**Fix (chosen for notifications):** the window-driver (`sync_window`) calls
`view_cx.notify()` on the open window handle when the snapshot changes:

```rust
let _ = existing.update(cx, |_, _window, view_cx| {
    view_cx.notify();   // repaint the NotificationsView from the new global
});
```

Alternative: `cx.observe_global::<G>(|_view, _cx| {})` inside `render` — but a
subscription dropped at end of `render` does nothing; you must STORE it in the
view struct. The `view_cx.notify()`-from-driver approach avoids that.

## 5. `px` and `FontWeight` are NOT in `prelude::*`

`use gpui::{div, prelude::*, ...}` does NOT bring `px` or `FontWeight` into
scope (you get `prelude` styling methods like `bg`/`cursor_pointer`/`gap`, but
not the `px()` constructor or the `FontWeight` enum). Import them explicitly:
`use gpui::{div, px, FontWeight, prelude::*, ...};`. Symptom of missing `px`:
"cannot find function `px` in this scope" with the unhelpful hint to write
`cx(8.)`.

## 6. Layer-shell popup window options (verified against `bar`/`launcher`)

Notifications use `WindowKind::LayerShell(LayerShellOptions { ... })`:
- `Layer::Overlay`, `anchor: Anchor::TOP | Anchor::RIGHT`
- `exclusive_zone: None` — **NEVER** exclusive for popups (forbidden permanently;
  only the bar may reserve space).
- `keyboard_interactivity: KeyboardInteractivity::None` — popups are mouse-only.
- `margin: Some((px(12.), px(12.), px(12.), px(12.)))`, `namespace: "notifications"`.
- `app_id: Some("chronos-notifications".to_string())`,
  `window_background: WindowBackgroundAppearance::Transparent`.
- Close via `handle.update(cx, |_, window: &mut Window, _| window.remove_window())`
  (same shape as `launcher::close`).

## 7. Inline linters false-positive on `async move`

The repo is `edition = "2024"`. Inline/editor linters that don't pass
`--edition 2024` flag `async move` closures as "only allowed in Rust 2018 or
later" — this is a **false positive**. Trust `cargo build` / `cargo check`,
not the inline lint, when that appears. (Also recorded in the repo Gotchas
section.)

## Build note

`cargo build -p chronos` fails with `E0432` if the parallel `chronos-ui` crate
hasn't been added yet (the orchestrator owns wiring `Theme::global(cx)`). That
is expected, NOT a defect in the notifications code. Write against
`chronos_ui::Theme` (fields `bg.elevated/secondary`, `text.primary/secondary/muted`,
`border.default/focused`, `accent.primary`, `status.{success,warning,error,info}`,
`radius`, `radius_lg`) and report the `E0432` as "blocked on chronos-ui", not
"build failed".

## 8. Clipped-popup height handling — FIXED-CAP + HARD CLIP (2026-07-19)

> **DEPRECATED (do not copy):** the original §8 here taught a per-glyph
> `estimate_content_height` + `window.resize()` rubber-band. That shipped in
> notifications #9, was patched in #11, and was PROVEN WRONG in #12 (Hermes).
> The per-glyph `BODY_CHARS_PER_LINE`/line-height constants drift from the
> real rendered GPUI text metrics, so the estimate under-sizes the surface on
> long/wrapped content and the bottom still clips — silently, with no scroll.
> Pixel math on unmeasured text metrics is not reliable enough to be the only
> path to visible content. The code below is the CURRENT, canonical shape
> (matches `updates_popup`, commit `67f7d10`).

**Symptom (real bug):** a fixed-height layer-shell window clips any content
taller than it — long `body` text or a 2nd+ stacked card overflows and the
bottom is cut off. The compositor clips; the window never resizes.

**Key fork facts (still true):**
- gpui layer-shell surfaces do **NOT** auto-size to their children. Surface
  size comes from `WindowBounds` at open time; there is no
  `WindowBounds::ContentDriven` / auto-fit variant in this fork.
- `gpui` `Style` has **NO `max_height`** field (confirmed: `style.rs` only has
  `min_width`/`max_width` via `DefiniteLength`, height is unbounded). So you
  clip via `.max_h()` on the ELEMENT, not via style or the window.
- `Pixels` field `.0` is **private** — use `f32::from(px)`, never
  `d.bounds().size.height.0`.

**Fix — fixed window cap + structural clip (no text-height estimate):**

1. Open the window at a FIXED cap (`POPUP_HEIGHT = LIST_MAX_H = 360.`). The
   surface NEVER resizes with content. Do NOT call `window.resize()` in the
   snapshot watcher.
2. Wrap the content in a hard clip that cannot be defeated by real render
   height: `.max_h(px(LIST_MAX_H)).overflow_hidden()`. `max_h`/`overflow_hidden`
   RESOLVE in this fork (used by `updates_popup`/`volume_popup`/`tray_menu`/
   `osd`). Two clip levels:

   ```rust
   // inside a card: clip a long body so it doesn't bleed into the next card
   div().max_h(px(BODY_MAX_H)).overflow_hidden().text_color(...).child(body)
   // whole stack: clip the card list to the window cap
   div().flex_col().gap(px(8.))
       .max_h(px(LIST_MAX_H)).overflow_hidden()
       .children(cards)
   ```

3. Pick caps as SAFE BUDGETS, not measured sizes: `BODY_MAX_H` ≈ 4–5 lines;
   `LIST_MAX_H` ≈ the window height (older cards clipped off the bottom are
   acceptable for notifications — they expire on a timer; for `updates_popup`
   add a "+N more" note because a privileged button must stay visible).
4. `sync_window` only calls `view_cx.notify()` on snapshot change — NO resize.
5. **No inner scroll.** `overflow_hidden()` is clip-only (correct). The fork's
   `overflow_y_scroll()` does NOT resolve on `Div` in this build (same trait as
   `cursor_pointer`, which DOES resolve — a fork quirk; don't chase it). Don't
   reach for `ScrollHandle` for a popup; the hard clip is the chosen pattern.

**Why fixed-cap beats a bigger constant:** a constant still clips past its
threshold and wastes Wayland space when only one short item is up. A clip never
clips prematurely and never grows the window uncontrolled. Full current
guidance: skill `gpui-layer-shell-rubber-band`.

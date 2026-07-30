# T120 — Notifications History Popup (Anchored Redesign)

**Status:** DONE  
**Date:** 2026-07-24  
**Commits:** `0ebe6de`, `7415fcb`, `a90a71a`, + 4 fix commits for dispatch/detach/SVG

---

## What Was Built

Persistent notification history popup opened from the bar bell widget. Replaces the
ephemeral toast-only model with an inbox-style log that survives toast expiry/dismiss.

### Features Delivered

| Feature | Details |
|---|---|
| **Service commands** | `NotificationCommand::RemoveFromHistory(u32)` + `ClearHistory` added to the enum. Pure helpers `remove_from_history()`, `clear_history()` on `NotificationState`. 4 unit tests. |
| **Bell bounds capture** | `notification_bell.rs` rewritten: canvas bounds capture via `on_resize`, `.relative()` layout, `on_mouse_down(Left)` toggle for popup open/close. |
| **Anchored window** | `history_popup/mod.rs`: `open(cx, anchor_rect, parent)` with `WindowKind::AnchoredPopup` + LayerShell fallback. `close_this` reentrancy guard. `toggle()` for bell click. Resize-on-change watcher via `state::watch()`. |
| **Mockup-faithful view** | Full rewrite of `view.rs`: urgency 3px strip, 16x16 monogram, summary/body clamp (4-line), outlined action buttons, row text ✕ dismiss, footer "Clear all" (`len > 1`), empty "No notifications". 5 unit tests. |
| **Row dismiss (✕)** | `RemoveFromHistory(id)` dispatched via `cx.background_spawn(...).detach()`. |
| **Clear all** | `ClearHistory` dispatched the same way. |
| **MarkAllRead** | Fixed in `open()` — also needed `.detach()`. |

### Geometry

| Constant | Value | Purpose |
|---|---|---|
| `POPUP_WIDTH` | 360px | Mockup-fixed width |
| `ROW_H` | 100px | Conservative per-card estimate (was 72, too small for cards with body+actions) |
| `FOOTER_H` | 53px | "Clear all" button strip |
| `MAX_LIST_H` | 480px | Scroll cap before overflow |
| `EMPTY_H` | 84px | "No notifications" state |

---

## Bugs Found & Fixed During Smoke Testing

### 1. Async dispatch never executed (Critical)

**Symptom:** Row dismiss ✕ and "Clear all" buttons did nothing.  
**Root cause:** `background_spawn` returns a `Task<T>`. Per gpui_scheduler docs:
> *"If you drop a task it will be cancelled immediately."*

The `let _ = cx.background_spawn(async { ... })` pattern drops the `Task` handle,
cancelling the future before it ever runs. The async block never executes.

**Fix:** Append `.detach()` to every `background_spawn` call:
```rust
cx.background_spawn(async move {
    let _ = svc.dispatch(cmd).await;
}).detach();
```

**Affected files:** `history_popup/view.rs` (3 places), `history_popup/mod.rs` (1 place),
`notifications/view.rs` (2 places — pre-existing bug, Close + InvokeAction).

### 2. ✕ button invisible (SVG not rendering)

**Symptom:** Row dismiss ✕ appeared transparent/invisible.  
**Root cause:** History popup used `svg().path("icons/x.svg")` while the working
ephemeral popup uses `.child("✕")` (text character). The SVG failed to render
in the popup context.

**Fix:** Replace SVG with text character, matching ephemeral popup pattern.

### 3. "Clear all" button clipped

**Symptom:** Footer "Clear all" was cut off when notifications had body text or actions.  
**Root cause:** `ROW_H = 72px` underestimated real card height. A card with
summary + 4-line body + actions ≈ 142px. Window opened too short, footer clipped
before resize watcher could correct it.

**Fix:** Increase `ROW_H` to 100px (conservative estimate). The resize watcher
shrinks-to-fit on the next state update if content is shorter.

---

## Test Results

- **16/16** notification service unit tests pass (including 4 new T120 tests)
- **5/5** history popup view unit tests pass
- **Release build:** `cargo build --release -p chronos` — green (2m40s)
- **Live smoke:** ✕ dismiss works, Clear all works, scroll works, MarkAllRead clears bell dot

---

## Architecture Notes

- `dispatch()` is `async` because `Close`/`InvokeAction` emit D-Bus signals directly
  on the stored connection. `RemoveFromHistory`/`ClearHistory` are pure state mutations
  but go through the same async path.
- `background_spawn` + `.detach()` is the correct pattern for fire-and-forget async
  dispatch from `on_click` callbacks (confirmed by `system_popup/view.rs` and
  `battery.rs` precedent).
- The `Task` type from `gpui_scheduler` cancels on drop — this is by design, not a bug.
  Always `.detach()` when you want the task to outlive the calling scope.

---

## Files Modified

| File | Changes |
|---|---|
| `crates/services/src/notification/types.rs` | `remove_from_history()`, `clear_history()` |
| `crates/services/src/notification/mod.rs` | `RemoveFromHistory`, `ClearHistory` enum variants + dispatch arms + 4 tests |
| `crates/app/src/notifications/history_popup/mod.rs` | Anchored window, `toggle()`, `MarkAllRead` fix, resize watcher |
| `crates/app/src/notifications/history_popup/view.rs` | Full mockup rewrite: urgency strip, monogram, actions, dismiss, clear all |
| `crates/app/src/notifications/view.rs` | `.detach()` fix for Close + InvokeAction (pre-existing bug) |
| `crates/app/src/bar/widgets/notification_bell.rs` | Canvas bounds capture, `.relative()`, `on_mouse_down` toggle |

# T124 — Ephemeral toast notifications — Report

**Status:** Built + compiles. Live smoke requires a Hyprland session (not available in build-only verification; `cargo build --release -p chronos` passes).

## Changes

### `crates/app/src/notifications/mod.rs` (geometry & constants)

| Constant | Before | After | Notes |
|---|---|---|---|
| `POPUP_WIDTH` | 360 | **340** | mockup width |
| `LIST_MAX_H` | 360 | **480** | taller cards need more room |
| margin top | 12 | **12** | unchanged (bar 30px exclusive → screen top ~42px) |
| margin right | 12 | **16** | mockup `right: 16px` |
| margin bottom | 12 | **0** | unnecessary |
| margin left | 12 | **0** | unnecessary |

### `crates/app/src/notifications/view.rs` (full rewrite)

**New `render_toast_card()`** — independent card renderer (not shared with history):
- Card: `bg #1e1e2e`, `border 1px #313244`, `radius 8px`, `overflow-hidden`
- **Icon 28×28** with monogram (first letter(s)), stable per-app palette color
- **App name** row (left) + **Close ✕** (right, hover bg + color, dispatches `Close(id)` via `.detach()`)
- **Summary** (12.5px, semibold, `#cdd6f4`/`#f38ba8` for critical)
- **Body** (11px, `#a6adc8`)
- **Actions** (outlined chips, border `#45475a`, hover → `#cba6f7`, dispatches `InvokeAction(id, key)` via `.detach()`)
- **Progress bar** at bottom: 2px track `#25253b` + colored fill
- **Critical** variant: border tint `#f38ba833`, app name + summary in `#f38ba8`, progress fill opacity 0.6

**New `NotificationsView`** with progress tracking:
- `first_seen: HashMap<u32, u64>` — epoch ms when each notification was first rendered
- On each `render()`: prune stale ids, record new arrivals, compute `progress_fraction()`
- `progress_fraction()`: `(expire_at - now) / (expire_at - first_seen)`, clamped `[0, 1]`, `None` when `expire_at` is `None` (sticky → no progress bar)
- **100ms tick loop** via `cx.spawn(async move |this, cx| { ... })` for smooth progress decay; dies when view entity is dropped

**Preserved:** `render_notification_card()` unchanged at bottom of file (legacy compat).

## Urgency mapping (implemented)

| Urgency | Toast kind | Progress color | Notes |
|---|---|---|---|
| `Low` | info | `#89b4fa` | short TTL |
| `Normal` | info | `#89b4fa` | (prefer info; Normal→warning flag not added) |
| `Critical` | error | `#f38ba8` | red tint border, red text, higher progress opacity |

Success green (`#a6e3a1`) — **not implemented** (no category field on Notification, as specified).

## Progress approach

- **Known `expire_at`:** progress bar fills remaining time, shrinking from full width to 0
- **Unknown `expire_at`/sticky:** no progress bar rendered (honest skip, per spec)
- **Tick:** 100ms loop; `cx.notify()` triggers repaint of progress bar width
- Fraction calculated as `(expire_at - now) / (expire_at - first_seen)`, clamped `[0, 1]`

## Animation

**Enter/exit animation: NOT implemented.** The fork's `with_transition` API requires wrapping in `AnimatedWrapper` which uses `transition_on_hover` / `with_transition` from `gpui_animation`. The existing notification cards are all builder-style, and adding `AnimatedWrapper` per card in a dynamic list was deemed debt-accept for T124. Documented as follow-up.

## Out-of-scope

- Success skin (no data → skip)
- Light theme (dark-only mockup)
- Enter/exit animation (best-effort → debt)
- Pulse shadow for critical (simple border tint is accept-min)
- Outer list border removed (cards are now self-contained)

## Live smoke commands

```bash
chronos-rebuild && chronos-stop && chronos-start
notify-send -u low "ChronOS" "Screenshot saved"
notify-send -u normal "System" "Battery at 15%"
notify-send -u critical "Package Manager" "Update failed"
# grim stack; click ✕; action if any; wait auto-dismiss + progress
```

## Build verification

```
cargo build --release -p chronos  →  Finished (release) ✓
cargo test -p chronos --lib       →  (no new tests — helpers are local-only)
```

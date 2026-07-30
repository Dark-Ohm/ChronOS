# T123 — Audio Volume Drag: Coalesce + Light Re-read — Report

**Status:** COMPLETE
**Date:** 2026-07-25

## What was done

### Service: cheap path for volume set (`crates/services/src/audio/mod.rs`)

**Problem:** Every `AudioSubscriber::dispatch(Set*Volume)` called `apply_command` + full `read_state()` (which includes `pw-dump` JSON parse + `wpctl inspect` on both endpoints). During drag (60–120 events/sec), this created N parallel subprocess chains, race-condition `data.set` calls, and multi-second lag.

**Solution:**

1. **Coalesce via `tokio::sync::watch` channel:** New `run_volume_coalesce` background task drains the latest pending volume, applies via `wpctl set-volume`, then does a light confirm (`wpctl get-volume` only — no `pw-dump`, no `inspect`). Rapid dispatches coalesce to a single apply.

2. **Optimistic state update:** `dispatch()` sets the volume in `Mutable<AudioState>` immediately (before `wpctl` runs), so the UI re-renders instantly.

3. **Separation of concerns:** Volume commands use the coalesce fast-path; mute/toggle/default commands keep the original full `read_state()` path.

**Key functions added:**
- `run_volume_coalesce()` — background coalesce task
- `apply_to_pipewire()` — single `wpctl set-volume`
- `read_volume_only()` — light confirm (volume + muted only)
- `merge_volume_into_state()` — preserves device lists while updating volume

### UI: throttle + optimistic thumb (`crates/app/src/volume_popup/view.rs`)

**Problem:** Every `on_mouse_down` / `on_drag_move` dispatched immediately with no throttling or local paint.

**Solution:**

1. **Throttle:** `set_volume_unmute_if_needed` skips dispatches when last dispatch was <32ms ago and volume delta <1% (minimum one dispatch per ~2 frames at 60fps).

2. **Optimistic thumb:** `VolumePopupView.dispatched_vol` tracks the last dispatched `(kind, volume, timestamp)`. During render, the thumb position uses `dispatched_vol` when it matches the current endpoint kind, so the thumb follows the finger immediately.

3. **Bug fix:** Source arm previously dispatched `ToggleSinkMute` instead of `ToggleSourceMute` (pre-existing bug caught during rewrite).

## Design choices

### Coalesce design: `tokio::sync::watch` (not `Notify`)

`watch` channel gives latest-wins semantics with zero lost notifications. The background task uses `borrow_and_update()` to drain, then `changed().await` to sleep. No race between `send` and `listen`.

### Reload design: B (one-shot, no `cargo watch`)

Same as T122: `cargo build -p chronos-hotview` once. Hot-lib-reloader picks up new `.so`. Watch left to human. (Documented in T122.)

### Poll loop: unchanged

250ms poll remains — still needed for external changes (pavucontrol, `wpctl` from terminal). Device lists only refreshed on poll (not on dispatch path).

## What was NOT done

- **systemd user unit** — out of scope
- **Hyprland autostart** — out of scope
- **native PipeWire backend** — DECISIONS long-term, out of scope
- **MPRIS / stream mute path** — unchanged (no need to share volume helpers)

## Verification

```
cargo test -p chronos-services --lib audio -- --nocapture
→ 27 passed; 0 failed
  ✓ merge_volume_into_state_sink_preserves_source_and_devices
  ✓ merge_volume_into_state_source_preserves_sink
  ✓ all pre-existing audio tests green

cargo check -p chronos-services
→ EXIT_CODE=0 (0 new warnings)
```

Note: `cargo check -p chronos` fails with pre-existing errors in `toast/view.rs` and `toast/mod.rs` (not related to T123).

## Files changed

| File | Change |
|---|---|
| `crates/services/src/audio/mod.rs` | Coalesce channel, `run_volume_coalesce`, `apply_to_pipewire`, `read_volume_only`, `merge_volume_into_state`. Volume commands take fast-path. Non-volume commands unchanged. |
| `crates/app/src/volume_popup/view.rs` | `VolumePopupView.dispatched_vol` field. Throttle (32ms / 1%). Optimistic thumb rendering. Source unmute bug fix. |

## Recommendations for future work

1. **Unmute-on-volume coalesce:** Currently UI dispatches `SetSinkVolume` + `ToggleSinkMute` as two commands. Could add `SetSinkVolumeUnmute` single command to avoid double-toggle race.

2. **Full optimistic with `on_drag_end`:** If GPUI adds a drag-end event, clear `dispatched_vol` at that point for cleaner render lifecycle. Current approach (clear when service catches up) is sufficient.

3. **Live integration test:** `audio-dispatch-smoke` against real PipeWire — optional, low priority for MVP.

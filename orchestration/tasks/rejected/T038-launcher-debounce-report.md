<!-- T038 — migrated 2026-07-22 from orchestration/report-log/cline-report-9.md — see orchestration/tasks/MIGRATION.md -->

# Cline Report: Launcher Debounce Implementation

## Problem
With `follow_mouse=1` in Hyprland 0.55.4+, mouse-exit triggers spurious `deactivate` events.
Quick cursor movements out of the launcher caused immediate closure, making it unusable.

## Solution
Implemented 300ms debounce on focus-loss before closing the launcher, matching rofi/fuzzel UX.

## Changes Made

### crates/app/src/launcher/mod.rs
- Added `Task<()>` and `WeakEntity` to imports
- Extended `LauncherState` with:
  - `close_timer: Option<Task<()>>` — stores the debounce timer
  - `pending_close: bool` — flag to cancel close on user interaction
- Modified activation observer in `open()` to:
  - Cancel timer on reactivation (`active=true`)
  - Start 300ms timer on deactivation (`active=false`) instead of immediate close
- Modified `close_this()` to compare window handles before closing

### crates/app/src/launcher/view.rs
- In click handler: set `pending_close = false` to prevent debounce from closing window after click

## Edge Cases Handled
1. **Quick re-entry during debounce**: Timer fires but `handle.take()` returns None if window already closed
2. **Click during debounce**: `pending_close` prevents timer from closing newly-opened window
3. **Ghost window prevention**: `close_this()` checks if window is the tracked handle before acting

## Status
Build successful (warnings only from proc-macro-error2). Ready for testing on Hyprland with `follow_mouse=1`.
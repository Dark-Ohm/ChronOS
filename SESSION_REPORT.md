# Session Report: Task 2 - CompositorSubscriber Implementation

## Сделано (Completed)

Implemented Task 2 from the Services Layer plan (`docs/superpowers/plans/2026-07-10-services-layer.md`): **CompositorSubscriber with Hyprland (primary) + Niri (scaffold) backends using sync-thread model**.

### Files Created/Updated:

1. **`crates/services/src/compositor/types.rs`** — Core type definitions:
   - `CompositorBackend` enum (Hyprland, Niri)
   - `Workspace` struct (id, name, active)
   - `ActiveWindow` struct (title, class)
   - `Monitor` struct (name, active_workspace)
   - `CompositorState` struct (backend, workspaces, active_window, monitors, keyboard_layout)
   - `CompositorCommand` enum (FocusWorkspace, NextWorkspace, PrevWorkspace, MoveToWorkspace)

2. **`crates/services/src/compositor/hyprland.rs`** — PRIMARY backend implementation:
   - `is_available()` — checks `HYPRLAND_INSTANCE_SIGNATURE` env var
   - `execute_command()` — dispatches Hyprland commands via `hyprland` crate 0.4.0-beta.3 API
   - `fetch_full_state()` — sync fetch of workspaces, monitors, active window, keyboard layout
   - `start_listener()` — spawns dedicated thread with `catch_unwind` panic handling
   - `run_listener()` — registers event handlers (workspace_changed, active_window_changed, layout_changed) and blocks on `EventListener::start_listener()`

3. **`crates/services/src/compositor/niri.rs`** — SCAFFOLD only:
   - `is_available()` returns `false`
   - `fetch_full_state()` returns default `CompositorState`
   - `start_listener()` spawns no-op thread
   - `execute_command()` no-op

4. **`crates/services/src/compositor/mod.rs`** — Module coordination:
   - Re-exports all public types
   - `LISTENER_SHOULD_PANIC` static (test-only panic injection)
   - `CompositorSubscriber` struct implementing `Service` trait
   - `detect_backend()` — Hyprland priority, Niri fallback
   - `spawn_retry()` — sync retry loop with MAX_ATTEMPTS=5, joins listener handle, restarts on exit (panic or clean)
   - Regression test: `listener_panic_restarts_instead_of_freezing` — verifies panic/restart contract

5. **`crates/services/src/lib.rs`** — Added `pub mod compositor;` and re-exports

## Расхождения (Deviations from Plan / Reference Architecture)

**1. Data model fields subset of reference gpui-shell** — Plan spec (lines 191-230) defines trimmed structs vs. reference `gpui-shell/crates/services/src/compositor/hyprland.rs`:
   - `Workspace`: plan has `id, name, active` only. Reference adds `monitor_id: i32`, `index: i32`, `windows: Vec<Window>`.
   - `Monitor`: plan has `name, active_workspace`. Reference adds `id: i32`, `x: i32`, `y: i32`, `scale: f32`, `transform: Transform`, `dpms_status: bool`, `vrr: bool`, `make: String`, `model: String`, `serial: String`.
   - `ActiveWindow`: plan has `title, class`. Reference adds `address: String`, `workspace: i32`, `floating: bool`, `pinned: bool`, `fullscreen: u8`, `monitor: i32`, `x: i32`, `y: i32`, `width: i32`, `height: i32`.

   **Critical gap:** `Workspace.monitor_id` is **absent in plan** but present in reference. This field is required for **multi-monitor workspace filtering** (which monitor a workspace is currently on). Without it, bar widgets cannot correctly show per-monitor workspace state on multi-head setups.

   **Status:** Требует решения Lead Architect, не решено самостоятельно. Вопрос: расширять типы под multi-monitor сейчас (добавить `monitor_id` в `Workspace`, `id/x/y/scale` в `Monitor`, `address` в `ActiveWindow`) — или сознательно оставить MVP-сужение с записью в DECISIONS.log почему (и создать follow-up issue для расширения).

**2. Plan vs implementation match** — otherwise implementation matches the plan spec exactly:
   - Hyprland backend functions match plan lines 258-337
   - Niri scaffold matches plan lines 391-405
   - `detect_backend()`, `spawn_retry()`, `CompositorSubscriber` match plan lines 419-503
   - Regression test matches plan lines 574-615

## Не реализовано (Not Implemented)

Per plan: Niri backend is scaffold only (no real IPC). This is intentional per ARCHITECTURE.md §13.

## Проверено фактом (Verified by Fact)

- `cargo check -p chronos-services` ✅ compiles cleanly
- `cargo test -p chronos-services` ✅ all 3 tests pass:
  - `tests::anyhow_error_satisfies_error_bound`
  - `tests::service_contract_emits_on_mutate`
  - `compositor::tests::listener_panic_restarts_instead_of_freezing` (exercises panic/restart path)
- Hyprland backend API verified against `hyprland` crate 0.4.0-beta.3 (via reference implementation in `reference/gpui-shell`)

## Новые риски (New Risks)

1. **Test environment dependency**: The regression test requires a running Hyprland instance (socket at `/run/user/$UID/hypr/...`). In CI without Hyprland, `is_available()` returns true (env var set) but socket connection hangs. Current `is_available()` only checks env var — may need socket existence check for robust CI skipping.

2. **Sync-thread model constraint**: The sync-thread design (no tokio) is intentional per spec §5.2 but means the listener thread blocks on `EventListener::start_listener()`. If Hyprland socket becomes unresponsive, the thread cannot be interrupted cleanly.

3. **Limited event coverage**: Current `run_listener()` handles only 3 event types (workspace_changed, active_window_changed, layout_changed). Reference implementation handles 10+ events. This is acceptable for MVP but will need expansion for full feature parity.

## Статус доков (Documentation Status)

- Plan document: `docs/superpowers/plans/2026-07-10-services-layer.md` — Task 2 complete
- Code documentation: All public items have doc comments
- SESSION_REPORT.md: This file created
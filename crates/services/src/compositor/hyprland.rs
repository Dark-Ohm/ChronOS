//! Hyprland compositor backend — PRIMARY backend.
//!
//! VERIFY the exact `hyprland` crate API against the pinned version (docs.rs)
//! and reference/gpui-shell/crates/services/src/compositor/hyprland.rs.

use std::panic;
use std::thread;

use anyhow::Result;
use futures_signals::signal::Mutable;
use hyprland::{
    data::{Client, Clients, Devices, Monitors, Workspace as HWorkspace, Workspaces},
    event_listener::{EventListener, MonitorEventData},
    prelude::*,
    shared::WorkspaceType,
};
use tracing::{debug, error, warn};

use super::types::{
    ActiveWindow, BlurCapability, CompositorBackend, CompositorCommand, CompositorState, Monitor,
    Workspace,
};
use crate::ServiceStatus;

/// Hyprland is available when running under it (env var set by the compositor).
pub fn is_available() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// Execute a compositor command via the Hyprland control socket.
///
/// Lua-Hyprland (0.55+) wraps **everything** read from the socket in Lua, so
/// the classic `dispatch workspace N` form written by `hyprland-rs`'s
/// `Dispatch::call` is parsed as Lua and fails server-side
/// (`'expected near '4'`), making every `hyprland-rs` dispatcher silently
/// no-op. The working form is the Lua dispatcher table sent as a `/dispatch`
/// line, e.g. `hl.dsp.focus({ workspace = 4 })`. We build that line and write
/// it directly to `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`.
///
/// See `DECISIONS.log` (2026-07-17 — compositor dispatch via Lua socket).
pub fn execute_command(cmd: CompositorCommand) -> Result<()> {
    let line = command_to_socket_line(&cmd);
    if matches!(cmd, CompositorCommand::CycleKeyboardLayout) {
        // `switchxkblayout` is a hyprctl subcommand, NOT a `/dispatch`
        // dispatcher: sending it through `/dispatch` makes Lua-Hyprland parse
        // it as Lua and silently no-op (`nil` calls / `)` expected). Write the
        // raw subcommand line straight to the socket instead.
        send_raw(&line)
    } else {
        send_dispatch(&line)
    }
}

/// Pure: render a `CompositorCommand` to the Lua-Hyprland `/dispatch` line.
/// No I/O — unit-testable without a running compositor.
///
/// Workspace IDs are emitted as numbers; relative selectors (`+1`/`-1`) as
/// Lua strings (Lua-Hyprland's workspace selector grammar).
fn command_to_socket_line(cmd: &CompositorCommand) -> String {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            format!("hl.dsp.focus({{ workspace = {id} }})")
        }
        CompositorCommand::NextWorkspace => {
            "hl.dsp.focus({ workspace = \"+1\" })".to_string()
        }
        CompositorCommand::PrevWorkspace => {
            "hl.dsp.focus({ workspace = \"-1\" })".to_string()
        }
        CompositorCommand::MoveToWorkspace(id) => {
            format!("hl.dsp.window.move({{ workspace = {id} }})")
        }
        CompositorCommand::CycleKeyboardLayout => {
            "switchxkblayout all next".to_string()
        }
    }
}

// ── T266: compositor blur bridge ────────────────────────────────────────────
//
// The shell controls blur through the OPT-IN Lua module
// (`packaging/hyprland/45-surface-effects-chronos.lua`), which holds the
// named layer/window rule handles and exports `_G.chronos_set_blur_enabled`.
// We only ever call that global through `hyprctl eval` — we never write the
// user's Hyprland config. Verified live on Hyprland 0.56.2 (T266 condition A):
// `hl.layer_rule` returns a handle, `handle:set_enabled(bool)` works, globals
// persist across separate eval calls, and eval errors are surfaced (exit
// code + `error:` line).

/// Lua for probing the module global (pure — unit-testable).
pub fn blur_probe_code() -> &'static str {
    "assert(type(_G.chronos_set_blur_enabled) == 'function', 'chronos blur module missing')"
}

/// Lua for toggling the module global (pure).
pub fn blur_set_code(enabled: bool) -> String {
    format!("_G.chronos_set_blur_enabled({})", if enabled { "true" } else { "false" })
}

/// Probe whether the blur module is importable in THIS session.
///
/// Runs `hyprctl eval` synchronously (single-digit ms round trip, proven in
/// Task 0). Call from a background task, never from a render/animation path.
/// Exit success + `ok` stdout ⇒ `Available`; a missing-global eval error ⇒
/// `ModuleMissing`; not running under Hyprland ⇒ `Unsupported`.
pub fn probe_shell_blur() -> BlurCapability {
    if !is_available() {
        return BlurCapability::Unsupported;
    }
    match run_eval(blur_probe_code()) {
        EvalOutcome::Ok => BlurCapability::Available,
        EvalOutcome::Error(msg) if msg.contains("chronos blur module missing") => {
            BlurCapability::ModuleMissing
        }
        EvalOutcome::Error(_) | EvalOutcome::SpawnFailed(_) => BlurCapability::ModuleMissing,
    }
}

/// Toggle the module's blur state. The caller persists only after this
/// succeeds — on error the previous state stays active.
pub fn set_shell_blur_enabled(enabled: bool) -> anyhow::Result<()> {
    match run_eval(&blur_set_code(enabled)) {
        EvalOutcome::Ok => Ok(()),
        EvalOutcome::Error(msg) => {
            anyhow::bail!("hyprctl eval failed: {msg}")
        }
        EvalOutcome::SpawnFailed(e) => anyhow::bail!("hyprctl eval spawn failed: {e}"),
    }
}

enum EvalOutcome {
    Ok,
    Error(String),
    SpawnFailed(String),
}

fn run_eval(code: &str) -> EvalOutcome {
    let output = match std::process::Command::new("hyprctl").args(["eval", code]).output() {
        Ok(o) => o,
        Err(e) => return EvalOutcome::SpawnFailed(e.to_string()),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && stdout == "ok" {
        EvalOutcome::Ok
    } else {
        // Errors come back as `error: <lua message>` on stderr; exit code 7.
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let msg = if stderr.is_empty() { stdout } else { stderr };
        EvalOutcome::Error(msg)
    }
}

/// Path to the Hyprland control socket, or `None` if the compositor env is
/// not present (not running under Hyprland).
fn socket_path() -> Option<std::path::PathBuf> {
    let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let xdg_runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(
        std::path::Path::new(&xdg_runtime)
            .join("hypr")
            .join(signature)
            .join(".socket.sock"),
    )
}

/// Write a `/dispatch <lua>` line to the Hyprland control socket.
fn send_dispatch(line: &str) -> Result<()> {
    let path = socket_path().ok_or_else(|| {
        anyhow::anyhow!("Hyprland socket unavailable: HYPRLAND_INSTANCE_SIGNATURE / XDG_RUNTIME_DIR not set")
    })?;
    let mut stream = std::os::unix::net::UnixStream::connect(&path)
        .map_err(|e| anyhow::anyhow!("connect Hyprland socket {}: {e}", path.display()))?;
    use std::io::Write;
    stream
        .write_all(format!("/dispatch {line}\n").as_bytes())
        .map_err(|e| anyhow::anyhow!("write Hyprland socket {}: {e}", path.display()))?;
    Ok(())
}

/// Write a raw hyprctl subcommand line (e.g. `switchxkblayout all next`) to
/// the Hyprland control socket — no `/dispatch` Lua wrapping.
fn send_raw(line: &str) -> Result<()> {
    let path = socket_path().ok_or_else(|| {
        anyhow::anyhow!("Hyprland socket unavailable: HYPRLAND_INSTANCE_SIGNATURE / XDG_RUNTIME_DIR not set")
    })?;
    let mut stream = std::os::unix::net::UnixStream::connect(&path)
        .map_err(|e| anyhow::anyhow!("connect Hyprland socket {}: {e}", path.display()))?;
    use std::io::Write;
    stream
        .write_all(format!("{line}\n").as_bytes())
        .map_err(|e| anyhow::anyhow!("write Hyprland socket {}: {e}", path.display()))?;
    Ok(())
}

/// Перечитывает СПИСОК воркспейсов с композитора и кладёт его в состояние.
///
/// Почему не «переставить флаг `active` по имеющемуся списку» (так было до
/// 2026-07-20): список брался ровно один раз на старте шелла, поэтому
/// созданные позже воркспейсы не появлялись точками в баре, а опустевшие не
/// исчезали. Хуже того — при переходе НА воркспейс, созданный после старта,
/// его `id` в списке отсутствовал, и активной не подсвечивалась ни одна
/// точка. Событий `createworkspacev2`/`destroyworkspacev2` мы просто не
/// слушали.
///
/// `hint_active` — id из события смены воркспейса: он приходит раньше, чем
/// `get_active()` успевает обновиться, поэтому доверяем событию, а не опросу.
fn refresh_workspaces(data: &Mutable<CompositorState>, hint_active: Option<i32>) {
    let active_id = hint_active.or_else(|| HWorkspace::get_active().ok().map(|w| w.id));
    match Workspaces::get() {
        Ok(list) => {
            let workspaces: Vec<Workspace> = list
                .into_iter()
                .map(|w| Workspace {
                    id: w.id,
                    name: w.name,
                    active: active_id == Some(w.id),
                    monitor_id: w.monitor_id,
                })
                .collect();
            data.lock_mut().workspaces = workspaces;
        }
        Err(e) => {
            // Не роняем список в пустой: лучше показать чуть устаревшие точки,
            // чем мигнуть пустым баром на разовом сбое IPC.
            warn!("workspace refresh failed, keeping previous list: {e}");
        }
    }
}

/// Extract the active workspace id hint from a `focusedmon` event.
///
/// `MonitorEventData.workspace_name` is a [`WorkspaceType`], not an id. For a
/// numeric workspace ("2") that parses straight to the id; named ("foo") and
/// special (`special`/`special:name`) workspaces fall back to `None`, in which
/// case [`refresh_workspaces`] polls `HWorkspace::get_active()` — same as the
/// `workspace_added`/`workspace_deleted` handlers.
fn focusedmon_active_id_hint(evt: &MonitorEventData) -> Option<i32> {
    match &evt.workspace_name {
        Some(WorkspaceType::Regular(name)) => name.parse::<i32>().ok(),
        _ => None,
    }
}

/// Live on-screen position + size of a mapped window by its `xdg_toplevel`
/// class (Hyprland's `initialClass`/`class`), in Hyprland's global compositor
/// layout coordinates (same frame as `Monitors::get()`'s `x`/`y`) — the
/// caller subtracts its target output's own origin to get output-local.
///
/// Wayland's `xdg_shell` never tells a client where the compositor placed
/// it (no such event exists in the protocol) — a window opened `center =
/// true` via windowrule (e.g. the launcher) has a client-side `bounds()`
/// frozen at its *requested* geometry forever, not its real screen position.
/// This is the only source of truth for that position. Synchronous
/// (Unix-socket round trip, single-digit ms) — call from a one-off user
/// action (e.g. a right-click handler), not a render/animation path.
///
/// Returns `None` if Hyprland is unreachable or no mapped client matches
/// `class` (ambiguous with multiple matches: first one wins, same as
/// `Client::get_active` semantics elsewhere in this module — launcher-style
/// popups are single-instance by construction).
pub fn window_position(class: &str) -> Option<(i16, i16)> {
    Clients::get()
        .ok()?
        .into_iter()
        .find(|c| c.mapped && c.class == class)
        .map(|c| c.at)
}

/// Fetch the full current compositor state from Hyprland (sync).
pub fn fetch_full_state() -> Result<CompositorState> {
    let active_id = HWorkspace::get_active().ok().map(|w| w.id);
    let workspaces = Workspaces::get()?
        .into_iter()
        .map(|w| Workspace {
            id: w.id,
            name: w.name,
            active: active_id == Some(w.id),
            monitor_id: w.monitor_id,
        })
        .collect();
    let monitors = Monitors::get()?
        .into_iter()
        .map(|m| Monitor {
            name: m.name,
            active_workspace: m.active_workspace.id,
            id: m.id,
            x: m.x,
            y: m.y,
            scale: m.scale,
        })
        .collect();
    let active_window = Client::get_active().ok().flatten().map(|w| ActiveWindow {
        title: w.title,
        class: w.class,
        address: w.address.to_string(),
    });
    let keyboard_layout = Devices::get()
        .ok()
        .and_then(|d| {
            d.keyboards
                .into_iter()
                .find(|k| k.main)
                .map(|k| k.active_keymap)
        })
        .unwrap_or_else(|| "Unknown".to_string());
    Ok(CompositorState {
        backend: CompositorBackend::Hyprland,
        workspaces,
        active_window,
        monitors,
        keyboard_layout,
    })
}

/// Spawn the dedicated listener thread and **block until it exits** (panic or
/// clean). The caller (`spawn_retry`) loops on exit, so a panicking listener is
/// restarted via the retry mechanism — a panic must not freeze the service at
/// `Unavailable` (spec §4.2 / §5.2). Returns the `JoinHandle` so the caller can
/// `join()` and detect exit.
pub fn start_listener(
    data: Mutable<CompositorState>,
    status: Mutable<ServiceStatus>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| run_listener(data.clone())));
        if result.is_err() {
            error!("Hyprland listener thread panicked; caller will restart via retry");
            status.set(ServiceStatus::Unavailable);
        }
        // Thread ends here; `spawn_retry` joins and loops back to fetch+retry.
    })
}

fn run_listener(data: Mutable<CompositorState>) -> Result<()> {
    // TEST HOOK (cfg(test) only): when set, panic on entry to exercise the
    // listener-restart path in `spawn_retry`. No effect in production builds.
    // `LISTENER_SHOULD_PANIC` is defined at the `compositor` module root (see
    // the `#[cfg(test)]` block in `mod.rs`) and is reachable here via `super`.
    #[cfg(test)]
    {
        if super::LISTENER_SHOULD_PANIC.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("injected listener panic for regression test");
        }
    }
    let mut listener = EventListener::new();
    {
        let data = data.clone();
        listener.add_workspace_changed_handler(move |evt| {
            debug!("workspace changed: {:?}", evt.name);
            refresh_workspaces(&data, Some(evt.id));
        });
    }
    {
        let data = data.clone();
        listener.add_active_monitor_changed_handler(move |evt| {
            debug!("active monitor changed: {:?}", evt.monitor_name);
            // `focusedmon` fires when focus moves to a monitor whose workspace
            // is already active (Hyprland sends no `workspace` event then), so
            // the bar never refreshed and the blue dot stayed on the old
            // monitor's workspace (T330). Re-read the active workspace and
            // trust the event's workspace id where it is a plain number.
            refresh_workspaces(&data, focusedmon_active_id_hint(&evt));
        });
    }
    {
        let data = data.clone();
        listener.add_workspace_added_handler(move |evt| {
            debug!("workspace added: {:?}", evt.name);
            refresh_workspaces(&data, None);
        });
    }
    {
        let data = data.clone();
        listener.add_workspace_deleted_handler(move |evt| {
            debug!("workspace deleted: {:?}", evt.name);
            refresh_workspaces(&data, None);
        });
    }
    {
        let data = data.clone();
        listener.add_active_window_changed_handler(move |evt| {
            let mut s = data.lock_mut();
            s.active_window = evt.map(|w| ActiveWindow {
                title: w.title,
                class: w.class,
                address: w.address.to_string(),
            });
        });
    }
    {
        let data = data.clone();
        listener.add_layout_changed_handler(move |evt| {
            let mut s = data.lock_mut();
            s.keyboard_layout = evt.layout_name;
        });
    }
    listener.start_listener()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::CompositorCommand;

    #[test]
    fn command_to_socket_line_formats_every_variant() {
        assert_eq!(
            command_to_socket_line(&CompositorCommand::FocusWorkspace(4)),
            "hl.dsp.focus({ workspace = 4 })"
        );
        assert_eq!(
            command_to_socket_line(&CompositorCommand::NextWorkspace),
            "hl.dsp.focus({ workspace = \"+1\" })"
        );
        assert_eq!(
            command_to_socket_line(&CompositorCommand::PrevWorkspace),
            "hl.dsp.focus({ workspace = \"-1\" })"
        );
        assert_eq!(
            command_to_socket_line(&CompositorCommand::MoveToWorkspace(7)),
            "hl.dsp.window.move({ workspace = 7 })"
        );
        assert_eq!(
            command_to_socket_line(&CompositorCommand::CycleKeyboardLayout),
            "switchxkblayout all next"
        );
    }

    // ── T266 blur bridge (pure command rendering — no live compositor) ──

    #[test]
    fn blur_eval_lines_are_lua_not_legacy_dispatch() {
        assert_eq!(
            blur_probe_code(),
            "assert(type(_G.chronos_set_blur_enabled) == 'function', 'chronos blur module missing')"
        );
        assert_eq!(blur_set_code(true), "_G.chronos_set_blur_enabled(true)");
        assert_eq!(blur_set_code(false), "_G.chronos_set_blur_enabled(false)");
    }

    // NOTE: `probe_shell_blur()`/`set_shell_blur_enabled()` are I/O against a
    // live compositor — covered by the manual Task 0/6 live runs, not by
    // unit tests (the test binary must not reach the real session).

    #[test]
    fn negative_workspace_id_renders_as_number() {
        // MoveToWorkspace with a negative/special id still emits a number,
        // matching Lua-Hyprland's workspace selector grammar.
        assert_eq!(
            command_to_socket_line(&CompositorCommand::FocusWorkspace(-2)),
            "hl.dsp.focus({ workspace = -2 })"
        );
    }

    // ── T330: `focusedmon` → active-workspace hint (pure, no socket) ──

    #[test]
    fn focusedmon_hint_parses_numeric_workspace() {
        let evt = MonitorEventData {
            monitor_name: "DP-1".into(),
            workspace_name: Some(WorkspaceType::Regular("2".into())),
        };
        assert_eq!(focusedmon_active_id_hint(&evt), Some(2));
    }

    #[test]
    fn focusedmon_hint_falls_back_for_named_special_and_missing() {
        let named = MonitorEventData {
            monitor_name: "DP-1".into(),
            workspace_name: Some(WorkspaceType::Regular("foo".into())),
        };
        assert_eq!(focusedmon_active_id_hint(&named), None);

        let special = MonitorEventData {
            monitor_name: "DP-1".into(),
            workspace_name: Some(WorkspaceType::Special(Some("magic".into()))),
        };
        assert_eq!(focusedmon_active_id_hint(&special), None);

        let missing = MonitorEventData {
            monitor_name: "DP-1".into(),
            workspace_name: None,
        };
        assert_eq!(focusedmon_active_id_hint(&missing), None);
    }

    #[test]
    fn active_monitor_handler_registers_without_socket() {
        // `EventListener::new()` + handler registration must not touch the
        // compositor socket (only `start_listener` does). Guards that the
        // `add_active_monitor_changed_handler` wiring compiles and registers.
        let mut listener = EventListener::new();
        listener.add_active_monitor_changed_handler(|_evt| {});
        let _ = listener;
    }
}

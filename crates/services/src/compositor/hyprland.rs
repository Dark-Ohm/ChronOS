//! Hyprland compositor backend — PRIMARY backend.
//!
//! VERIFY the exact `hyprland` crate API against the pinned version (docs.rs)
//! and reference/gpui-shell/crates/services/src/compositor/hyprland.rs.

use std::panic;
use std::thread;

use anyhow::Result;
use futures_signals::signal::Mutable;
use hyprland::{
    data::{Client, Devices, Monitors, Workspace as HWorkspace, Workspaces},
    event_listener::EventListener,
    prelude::*,
};
use tracing::{debug, error, warn};

use super::types::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, Monitor, Workspace,
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
    send_dispatch(&line)
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
    }

    #[test]
    fn negative_workspace_id_renders_as_number() {
        // MoveToWorkspace with a negative/special id still emits a number,
        // matching Lua-Hyprland's workspace selector grammar.
        assert_eq!(
            command_to_socket_line(&CompositorCommand::FocusWorkspace(-2)),
            "hl.dsp.focus({ workspace = -2 })"
        );
    }
}

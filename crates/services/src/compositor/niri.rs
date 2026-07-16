//! Niri backend — SCAFFOLD ONLY (ARCHITECTURE.md §13): no real IPC.

use anyhow::Result;
use futures_signals::signal::Mutable;
use std::thread;

use super::types::{CompositorCommand, CompositorState};
use crate::ServiceStatus;

/// Niri is not available (scaffold only).
pub fn is_available() -> bool {
    false
}

/// Returns default (empty) compositor state.
pub fn fetch_full_state() -> Result<CompositorState> {
    Ok(CompositorState::default())
}

/// No-op listener (Niri not wired).
pub fn start_listener(
    _data: Mutable<CompositorState>,
    _status: Mutable<ServiceStatus>,
) -> thread::JoinHandle<()> {
    thread::spawn(|| {
        // No-op: Niri not wired (scaffold only).
    })
}

/// No-op command execution.
pub fn execute_command(_cmd: CompositorCommand) -> Result<()> {
    Ok(())
}

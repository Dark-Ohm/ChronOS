//! Power actions: log out (Hyprland session exit), restart, shutdown.
//!
//! **No `Service` impl.** Every other subscriber in this crate reacts to
//! external state (battery level, network connectivity, ...). Power
//! actions are pure one-shot commands with nothing to observe — modeling
//! this as `Service<Data = ()>` would be a trait implemented for the sake
//! of consistency, not because it carries meaning. Keep it a plain struct.
//!
//! **"Switch user" is intentionally absent.** There is no login/display
//! manager on this system to hand a session to (see
//! `docs/superpowers/specs/2026-07-20-right-side-panel-design.md` §3.4) —
//! the UI ships a disabled button, this service has nothing to back it.

use std::process::Command;

use tracing::warn;

/// `hyprctl dispatch exit` — ends the current Hyprland session. What
/// happens next (return to a TTY prompt, respawn) is decided by whatever
/// launched Hyprland, not by this service.
pub fn log_out_command() -> (&'static str, Vec<&'static str>) {
    ("hyprctl", vec!["dispatch", "exit"])
}

/// `systemctl reboot`.
pub fn restart_command() -> (&'static str, Vec<&'static str>) {
    ("systemctl", vec!["reboot"])
}

/// `systemctl poweroff`.
pub fn shutdown_command() -> (&'static str, Vec<&'static str>) {
    ("systemctl", vec!["poweroff"])
}

/// `loginctl lock-session` — locks the session through systemd-logind. This is
/// the only lock backend the tree has (no hyprlock/swaylock wrapper shipped);
/// logind forwards to whatever lock helper the session configured.
pub fn lock_command() -> (&'static str, Vec<&'static str>) {
    ("loginctl", vec!["lock-session"])
}

/// `systemctl suspend` (sleep to RAM).
pub fn suspend_command() -> (&'static str, Vec<&'static str>) {
    ("systemctl", vec!["suspend"])
}

/// `systemctl hibernate` (sleep to disk).
pub fn hibernate_command() -> (&'static str, Vec<&'static str>) {
    ("systemctl", vec!["hibernate"])
}

fn spawn_command((bin, args): (&'static str, Vec<&'static str>)) {
    match Command::new(bin).args(&args).spawn() {
        Ok(_) => {}
        Err(e) => warn!("power: failed to spawn `{bin} {args:?}`: {e}"),
    }
}

#[derive(Clone, Default)]
pub struct PowerSubscriber;

impl PowerSubscriber {
    pub fn new() -> Self {
        Self
    }

    /// Fire-and-forget. Caller (UI) is responsible for confirming with the
    /// user before calling this — this service does not gate on anything.
    pub fn log_out(&self) {
        spawn_command(log_out_command());
    }

    pub fn restart(&self) {
        spawn_command(restart_command());
    }

    pub fn shutdown(&self) {
        spawn_command(shutdown_command());
    }

    /// Lock the session via logind (`loginctl lock-session`).
    pub fn lock(&self) {
        spawn_command(lock_command());
    }

    /// Sleep to RAM (`systemctl suspend`).
    pub fn suspend(&self) {
        spawn_command(suspend_command());
    }

    /// Sleep to disk (`systemctl hibernate`).
    pub fn hibernate(&self) {
        spawn_command(hibernate_command());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_out_command_is_hyprctl_dispatch_exit() {
        assert_eq!(log_out_command(), ("hyprctl", vec!["dispatch", "exit"]));
    }

    #[test]
    fn restart_command_is_systemctl_reboot() {
        assert_eq!(restart_command(), ("systemctl", vec!["reboot"]));
    }

    #[test]
    fn shutdown_command_is_systemctl_poweroff() {
        assert_eq!(shutdown_command(), ("systemctl", vec!["poweroff"]));
    }

    #[test]
    fn lock_command_is_loginctl_lock_session() {
        assert_eq!(lock_command(), ("loginctl", vec!["lock-session"]));
    }

    #[test]
    fn suspend_command_is_systemctl_suspend() {
        assert_eq!(suspend_command(), ("systemctl", vec!["suspend"]));
    }

    #[test]
    fn hibernate_command_is_systemctl_hibernate() {
        assert_eq!(hibernate_command(), ("systemctl", vec!["hibernate"]));
    }
}

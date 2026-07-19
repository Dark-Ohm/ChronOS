//! Gaming mode — global toggle state + apply/revert via `hyprctl eval`.
//!
//! On Hyprland 0.55+ with the **Lua** config layer (this machine), the legacy
//! `hyprctl keyword …` command is rejected: "keyword can't work with non-legacy
//! parsers. Use eval." The working runtime-set path (verified 2026-07-19 by
//! the Architect on this session) is:
//!
//! ```text
//! hyprctl eval 'hl.config({ animations = { enabled = false },
//!   decoration = { blur = { enabled = false } },
//!   general = { allow_tearing = true } })'
//! ```
//!
//! Revert restores the session default (animations/blur on, tearing off).
//!
//! In addition to the compositor toggles, gaming mode:
//! 1. Forces the UPower power profile to `Performance` (and restores the
//!    previous profile on revert).
//! 2. Sets a DND flag (`dnd: true`) that the notifications module can read
//!    to suppress ephemeral popups. The notifications module is **not**
//!    modified in this task — the flag is a standalone Global that Hermes'
//!    parallel notification-history work can subscribe to later without
//!    editing her files (avoids shared-file collisions per the zone rules).
//!
//! **Not implemented in MVP** (per ZED.md brief): hiding bar/dock. The brief
//! calls this chicken-egg — without the bar you can't open the popup to
//! toggle gaming mode off. Tracked as a follow-up TODO.

use std::process::Command;

use gpui::{App, AppContext, Global};
use tracing::{info, warn};

use chronos_services::{PowerProfile, Service, UPowerSubscriber};

use crate::state::AppState;
use crate::system_popup::{SystemPopupState, view::SystemPopupView};

/// `hyprctl eval` payload for gaming mode ON (verified live 2026-07-19).
const HYPRCTL_GAMING_ON: &str = "hl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false } }, general = { allow_tearing = true } })";

/// `hyprctl eval` payload for gaming mode OFF — restores the session default.
const HYPRCTL_GAMING_OFF: &str = "hl.config({ animations = { enabled = true }, decoration = { blur = { enabled = true } }, general = { allow_tearing = false } })";

/// Global gaming-mode state. Stored as a GPUI global so any view (bar widget,
/// popup, future notifications integration) can read it without plumbing.
#[derive(Clone, Copy, Debug, Default)]
pub struct GamingModeState {
    /// Is gaming mode currently active?
    pub active: bool,
    /// DND flag — `true` while gaming mode is on. Notifications reads this
    /// (when wired) to suppress ephemeral popups.
    pub dnd: bool,
    /// Power profile captured at the moment gaming mode was turned on, so OFF
    /// can restore it. `None` when gaming mode is off.
    pub previous_profile: Option<PowerProfile>,
}

impl Global for GamingModeState {}

impl GamingModeState {
    pub fn init(cx: &mut App) {
        cx.set_global(Self::default());
    }

    /// Repaint the open system popup (if any) so the gaming toggle knob moves
    /// immediately. Called after the global is flipped in `apply`/`revert`.
    /// Safe no-op when the popup is closed.
    fn repaint_popup(cx: &mut App) {
        if let Some(handle) = cx.global::<SystemPopupState>().handle.clone() {
            let _ = handle.update(cx, |view: &mut SystemPopupView, _window, view_cx| {
                view_cx.notify();
            });
        }
    }

    pub fn is_active(cx: &App) -> bool {
        cx.global::<Self>().active
    }

    pub fn is_dnd(cx: &App) -> bool {
        cx.global::<Self>().dnd
    }
}

/// Toggle gaming mode. Applies/reverts the compositor config + power profile
/// asynchronously so the GPUI click handler never blocks on `hyprctl`/D-Bus.
pub fn toggle(cx: &mut App) {
    let current = cx.global::<GamingModeState>().active;
    if current {
        revert(cx);
    } else {
        apply(cx);
    }
}

fn apply(cx: &mut App) {
    info!("gaming mode: apply() entered");
    let upower = AppState::upower(cx).clone();
    let previous_profile = upower.get().power_profile;

    cx.global_mut::<GamingModeState>().active = true;
    cx.global_mut::<GamingModeState>().dnd = true;
    cx.global_mut::<GamingModeState>().previous_profile = Some(previous_profile);

    // Repaint the popup so the toggle knob moves immediately — the global flip
    // above is synchronous, but no signal re-renders the view on its own.
    GamingModeState::repaint_popup(cx);

    // 1. Compositor: animations off, blur off, allow_tearing on.
    cx.background_spawn(async move {
        match run_hyprctl_eval(HYPRCTL_GAMING_ON).await {
            Ok(()) => info!("gaming mode: hyprctl eval ON applied"),
            Err(e) => warn!("gaming mode: hyprctl eval ON failed: {e:?}"),
        }
        // 2. Power profile → Performance.
        match upower.set_power_profile(PowerProfile::Performance).await {
            Ok(()) => info!("gaming mode: power profile set to Performance"),
            Err(e) => warn!("gaming mode: set_power_profile failed: {e:?}"),
        }
    })
    .detach();
}

fn revert(cx: &mut App) {
    info!("gaming mode: revert() entered");
    let upower = AppState::upower(cx).clone();
    let restore_profile = cx
        .global::<GamingModeState>()
        .previous_profile
        .unwrap_or(PowerProfile::Balanced);

    cx.global_mut::<GamingModeState>().active = false;
    cx.global_mut::<GamingModeState>().dnd = false;
    cx.global_mut::<GamingModeState>().previous_profile = None;

    // Repaint the popup so the toggle knob moves back immediately.
    GamingModeState::repaint_popup(cx);

    cx.background_spawn(async move {
        match run_hyprctl_eval(HYPRCTL_GAMING_OFF).await {
            Ok(()) => info!("gaming mode: hyprctl eval OFF applied"),
            Err(e) => warn!("gaming mode: hyprctl eval OFF failed: {e:?}"),
        }
        // Restore the previous power profile (or Balanced if unknown).
        match upower.set_power_profile(restore_profile).await {
            Ok(()) => info!("gaming mode: power profile restored to {restore_profile:?}"),
            Err(e) => warn!("gaming mode: restore power profile failed: {e:?}"),
        }
    })
    .detach();
}

/// Run `hyprctl eval '<payload>'` and return the status. `hyprctl` is fast
/// (<10ms typically) but we still run it on the background executor to keep
/// the click handler snappy and to never block the GPUI thread on a subprocess.
///
/// We use `std::thread::spawn` + a oneshot channel instead of
/// `tokio::task::spawn_blocking` because the GPUI background executor is not a
/// tokio runtime context — `spawn_blocking` would panic / hang here. zbus calls
/// (UPower) work because zbus is async and spawns its own runtime; a blocking
/// std::process::Command needs a real OS thread.
async fn run_hyprctl_eval(payload: &'static str) -> anyhow::Result<()> {
    info!("gaming mode: run_hyprctl_eval entered, payload len={}", payload.len());
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = Command::new("hyprctl").args(["eval", payload]).status();
        let _ = tx.send(result);
    });
    let status = rx.await??;
    if !status.success() {
        anyhow::bail!("hyprctl eval exited {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_off() {
        let s = GamingModeState::default();
        assert!(!s.active);
        assert!(!s.dnd);
        assert_eq!(s.previous_profile, None);
    }

    #[test]
    fn payloads_are_distinct() {
        // Sanity: ON and OFF payloads must differ — a copy-paste mistake
        // here would silently make gaming mode a no-op.
        assert_ne!(HYPRCTL_GAMING_ON, HYPRCTL_GAMING_OFF);
        assert!(HYPRCTL_GAMING_ON.contains("enabled = false"));
        assert!(HYPRCTL_GAMING_OFF.contains("enabled = true"));
    }

    #[test]
    fn on_payload_targets_all_three_options() {
        // Verified live 2026-07-19: all three compositor toggles must be in
        // the eval payload or gaming mode is partial.
        assert!(HYPRCTL_GAMING_ON.contains("animations"));
        assert!(HYPRCTL_GAMING_ON.contains("blur"));
        assert!(HYPRCTL_GAMING_ON.contains("allow_tearing"));
    }
}

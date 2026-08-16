//! Shared power-action model: the action enum + arm/confirm state machine.
//!
//! Extracted from `side_panel_right/power_row.rs` (T265-F) so the launcher
//! header can reuse the *same* arm logic instead of copying the enum. The
//! right panel footer uses the confirming subset (LogOut / Restart /
//! Shutdown); the launcher adds Lock / Sleep / Hibernate as one-click actions.
//!
//! The state machine is tiny and deliberately UI-free: `on_click` arms,
//! `is_confirming_click` detects a second click on the same action, and
//! `on_timeout` disarms after `ARM_TIMEOUT`.

use std::time::Duration;

/// Confirm window: a first click arms, a second click on the same action
/// within this window confirms; the arm lapses back to Idle afterwards.
pub const ARM_TIMEOUT: Duration = Duration::from_secs(3);

/// A destructive/system power action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    Lock,
    LogOut,
    Sleep,
    Hibernate,
    Restart,
    Shutdown,
}

impl PowerAction {
    /// Actions that must be confirmed with a second click (the dangerous
    /// session-ending ones). Lock / Sleep / Hibernate are one-click.
    pub fn needs_confirm(self) -> bool {
        matches!(
            self,
            PowerAction::LogOut | PowerAction::Restart | PowerAction::Shutdown
        )
    }

    /// Header-tile label.
    pub fn label(self) -> &'static str {
        match self {
            PowerAction::Lock => "Lock",
            PowerAction::LogOut => "Log out",
            PowerAction::Sleep => "Sleep",
            PowerAction::Hibernate => "Hibernate",
            PowerAction::Restart => "Restart",
            PowerAction::Shutdown => "Shutdown",
        }
    }
}

/// Arm/confirm state for a `PowerAction` (right panel + launcher share it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArmState {
    #[default]
    Idle,
    Armed(PowerAction),
}

/// A click on `clicked` always (re)arms that action.
pub fn on_click(_current: ArmState, clicked: PowerAction) -> ArmState {
    ArmState::Armed(clicked)
}

/// True when the user has clicked the *same* armed action again — confirm.
pub fn is_confirming_click(current: &ArmState, clicked: PowerAction) -> bool {
    *current == ArmState::Armed(clicked)
}

/// Disarm after the confirm window lapses.
pub fn on_timeout(_current: ArmState) -> ArmState {
    ArmState::Idle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_idle_arms_that_action() {
        let mut arm = ArmState::Idle;
        arm = on_click(arm, PowerAction::Restart);
        assert_eq!(arm, ArmState::Armed(PowerAction::Restart));
    }

    #[test]
    fn clicking_the_same_armed_action_again_confirms() {
        let arm = ArmState::Armed(PowerAction::Restart);
        assert!(is_confirming_click(&arm, PowerAction::Restart));
    }

    #[test]
    fn clicking_a_different_action_while_armed_rearms_to_the_new_one() {
        let mut arm = ArmState::Armed(PowerAction::Restart);
        assert!(!is_confirming_click(&arm, PowerAction::Shutdown));
        arm = on_click(arm, PowerAction::Shutdown);
        assert_eq!(arm, ArmState::Armed(PowerAction::Shutdown));
    }

    #[test]
    fn timeout_disarms_to_idle() {
        let arm = ArmState::Armed(PowerAction::LogOut);
        assert_eq!(on_timeout(arm), ArmState::Idle);
    }

    #[test]
    fn only_dangerous_actions_need_confirmation() {
        assert!(!PowerAction::Lock.needs_confirm());
        assert!(!PowerAction::Sleep.needs_confirm());
        assert!(!PowerAction::Hibernate.needs_confirm());
        assert!(PowerAction::LogOut.needs_confirm());
        assert!(PowerAction::Restart.needs_confirm());
        assert!(PowerAction::Shutdown.needs_confirm());
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(PowerAction::Lock.label(), "Lock");
        assert_eq!(PowerAction::Shutdown.label(), "Shutdown");
    }
}

//! Power row: Switch user (always disabled — no login manager), Log out,
//! Restart, Shutdown. Arm/confirm instead of a modal: first click arms
//! (label → "Confirm?" for 3s), second click within the window executes,
//! anything else disarms.

use std::time::Duration;

use chronos_ui::Theme;
use gpui::{Context, IntoElement, div, prelude::*, px};

use crate::side_panel_right::view::SidePanelRightView;

pub(crate) const ARM_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    LogOut,
    Restart,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArmState {
    #[default]
    Idle,
    Armed(PowerAction),
}

pub fn on_click(_current: ArmState, clicked: PowerAction) -> ArmState {
    ArmState::Armed(clicked)
}

pub fn is_confirming_click(current: &ArmState, clicked: PowerAction) -> bool {
    *current == ArmState::Armed(clicked)
}

pub fn on_timeout(_current: ArmState) -> ArmState {
    ArmState::Idle
}

fn label_for(action: PowerAction, arm: &ArmState) -> &'static str {
    if *arm == ArmState::Armed(action) {
        "Confirm?"
    } else {
        match action {
            PowerAction::LogOut => "Log out",
            PowerAction::Restart => "Restart",
            PowerAction::Shutdown => "Power",
        }
    }
}

fn power_button(action: PowerAction, label: &str, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    let armed = matches!(label, "Confirm?");
    div()
        .id(("power-btn", action as usize))
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.))
        .p(px(10.))
        .rounded(theme.radius)
        .cursor_pointer()
        .text_color(if armed {
            theme.status.warning
        } else {
            theme.text.secondary
        })
        .text_size(px(8.5))
        .font_family(theme.font_mono)
        .hover(|s| s.bg(theme.interactive.hover))
        .when(armed, |d| d.bg(theme.bg.elevated))
        .child(label.to_string())
}

pub fn render_power_row(
    theme: &Theme,
    arm: ArmState,
    cx: &mut Context<SidePanelRightView>,
) -> impl IntoElement {
    div()
        .flex()
        .gap(px(2.))
        .mt_auto()
        .child(
            // Switch user — always disabled, never armed, no listener.
            // Same tile shape as the three action buttons so the row reads as
            // four even buttons.
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.))
                .p(px(10.))
                .rounded(theme.radius)
                .text_color(theme.text.disabled)
                .text_size(px(8.5))
                .font_family(theme.font_mono)
                .child("Switch user"),
        )
        .child(
            power_button(
                PowerAction::LogOut,
                label_for(PowerAction::LogOut, &arm),
                theme,
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.on_power_click(PowerAction::LogOut, cx);
            })),
        )
        .child(
            power_button(
                PowerAction::Restart,
                label_for(PowerAction::Restart, &arm),
                theme,
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.on_power_click(PowerAction::Restart, cx);
            })),
        )
        .child(
            power_button(
                PowerAction::Shutdown,
                label_for(PowerAction::Shutdown, &arm),
                theme,
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.on_power_click(PowerAction::Shutdown, cx);
            })),
        )
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
}

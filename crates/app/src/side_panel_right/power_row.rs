//! Power footer grid: Switch / Log out / Restart / Power.
//! Visual from mockup (4-col grid, red Power). Arm/confirm 3s preserved.
//!
//! The `PowerAction`/`ArmState` model + arm helpers live in `crate::power`
//! (shared with the launcher header, T265-F) — this file is UI only.

use chrono::{Datelike, Local};
use gpui::{Context, IntoElement, div, img, prelude::*, px};
use chronos_ui::Theme;

use crate::power::{ArmState, PowerAction};
use crate::side_panel_right::surfaces;
use crate::side_panel_right::view::SidePanelRightView;

fn label_for(action: PowerAction, arm: &ArmState) -> &'static str {
    if *arm == ArmState::Armed(action) {
        "Confirm?"
    } else {
        match action {
            PowerAction::LogOut => "Log out",
            PowerAction::Restart => "Restart",
            PowerAction::Shutdown => "Power",
            // Never rendered in this footer (launcher-only), kept for an
            // exhaustive match over the shared enum.
            PowerAction::Lock => "Lock",
            PowerAction::Sleep => "Sleep",
            PowerAction::Hibernate => "Hibernate",
        }
    }
}

/// Russian month abbreviations (same as bar clock).
const MONTHS_RU: [&str; 12] = [
    "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
];

fn clock_label() -> String {
    let now = Local::now();
    let month_idx = now.month0() as usize;
    format!(
        "{} · {} {}",
        now.format("%H:%M"),
        now.day(),
        MONTHS_RU.get(month_idx).copied().unwrap_or("???"),
    )
}

fn power_tile(
    theme: &Theme,
    id: &'static str,
    icon: &'static str,
    label: &str,
    danger: bool,
    disabled: bool,
    armed: bool,
) -> gpui::Stateful<gpui::Div> {
    let border = if danger || armed {
        theme.status.error
    } else {
        theme.text.disabled
    };
    let text = if disabled {
        theme.text.muted
    } else if danger || armed {
        theme.status.error
    } else {
        theme.text.primary
    };
    // Danger/armed wash: status.error at ~12% alpha (works on light+dark).
    let danger_wash = theme.status.error.opacity(0.12);

    div()
        .id(id)
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.))
        .py(px(5.))
        .rounded(px(5.))
        .border_1()
        .border_color(border)
        .text_color(text)
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .when(!disabled, |d| {
            d.cursor_pointer().hover(|s| {
                if danger {
                    s.bg(danger_wash)
                } else {
                    s.bg(theme.border.subtle).border_color(theme.accent.primary)
                }
            })
        })
        .when(armed, |d| d.bg(danger_wash))
        .child(img(icon).w(px(10.)).h(px(10.)))
        .child(label.to_string())
}

/// Full footer: net status line (static summary + live clock) + power grid.
pub fn render_footer(
    net_summary: &str,
    arm: ArmState,
    cx: &mut Context<SidePanelRightView>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let clock = clock_label();

    div()
        .flex_none()
        .border_t_1()
        .border_color(theme.border.subtle)
        .bg(surfaces::card(&theme))
        .px(px(12.))
        .py(px(10.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(10.5))
                .text_color(theme.text.muted)
                .child(net_summary.to_string())
                .child(div().font_family("JetBrains Mono").child(clock)),
        )
        .child(
            div()
                .flex()
                .gap(px(4.))
                // Switch — always disabled
                .child(power_tile(
                    &theme,
                    "power-switch",
                    "icons/users.svg",
                    "Switch",
                    false,
                    true,
                    false,
                ))
                .child(
                    power_tile(
                        &theme,
                        "power-logout",
                        "icons/sign-out.svg",
                        label_for(PowerAction::LogOut, &arm),
                        false,
                        false,
                        arm == ArmState::Armed(PowerAction::LogOut),
                    )
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.on_power_click(PowerAction::LogOut, cx);
                    })),
                )
                .child(
                    power_tile(
                        &theme,
                        "power-restart",
                        "icons/arrows-clockwise.svg",
                        label_for(PowerAction::Restart, &arm),
                        false,
                        false,
                        arm == ArmState::Armed(PowerAction::Restart),
                    )
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.on_power_click(PowerAction::Restart, cx);
                    })),
                )
                .child(
                    power_tile(
                        &theme,
                        "power-shutdown",
                        "icons/power.svg",
                        label_for(PowerAction::Shutdown, &arm),
                        true,
                        false,
                        arm == ArmState::Armed(PowerAction::Shutdown),
                    )
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.on_power_click(PowerAction::Shutdown, cx);
                    })),
                ),
        )
}

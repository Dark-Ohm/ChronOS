//! Shared power + performance controls for the right-side System surface.
//!
//! Extracted from `system_popup/view.rs` (T291): the power-profile 3-segment
//! switch and the Perf Gaming toggle used to live only inside the bar popup.
//! They now render on the System tab (`side_panel_right/tab/system.rs`) as
//! System-style cards, and the popup keeps brightness only. Behaviour is 1:1
//! with the popup originals — same services (`AppState::upower`,
//! `GamingModeState`), same click arms.
//!
//! Styling matches the System tab's content cards (`surfaces::card` fill +
//! `border.subtle`), not the raw 280px popup block.

use gpui::{AnyElement, App, FontWeight, Hsla, SharedString, div, prelude::*, px};

use chronos_services::{PowerProfile, UPowerData};
use chronos_ui::{Theme, on_fill};

use crate::state::AppState;

/// Power-profile control as a System card. `upower` carries the current
/// profile; clicking a segment dispatches `set_power_profile` (optimistic UI
/// is unnecessary — UPower round-trips and the tab repaints on the service
/// signal via `SystemTab`'s upower watch).
pub fn render_power_profile_card(upower: &UPowerData, theme: &Theme) -> AnyElement {
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let accent = theme.accent.primary;
    let hover = theme.interactive.hover;
    let radius = theme.radius;

    let current = upower.power_profile;

    let title = div()
        .w_full()
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(text_primary)
        .child("Power profile");

    let segments: [(PowerProfile, &'static str); 3] = [
        (PowerProfile::PowerSaver, "Quiet"),
        (PowerProfile::Balanced, "Balanced"),
        (PowerProfile::Performance, "Performance"),
    ];

    let mut row = div()
        .w_full()
        .flex()
        .rounded(radius)
        .overflow_hidden()
        .border_1()
        .border_color(text_muted.alpha(0.3));
    for (profile, label) in segments {
        let is_active = current == profile;
        let seg_bg = if is_active { accent } else { gpui::transparent_black() };
        let color = if is_active {
            on_fill(accent)
        } else {
            text_muted
        };
        let id: SharedString = format!("power-profile-{label}").into();
        row = row.child(
            div()
                .id(id)
                .flex_1()
                .text_center()
                .py(px(6.))
                .text_color(color)
                .bg(seg_bg)
                .cursor_pointer()
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .hover(move |s| if is_active { s } else { s.bg(hover) })
                .child(label)
                .on_click(move |_event, _window, cx: &mut App| {
                    let upower = AppState::upower(cx).clone();
                    let target = profile;
                    cx.background_spawn(async move {
                        match upower.set_power_profile(target).await {
                            Ok(()) => tracing::info!("power_controls: set power profile to {target:?}"),
                            Err(e) => tracing::error!("power_controls: set power profile failed: {e:?}"),
                        }
                    })
                    .detach();
                }),
        );
    }

    card(theme)
        .gap(px(9.))
        .child(title)
        .child(row)
        .into_any_element()
}

/// Perf Gaming toggle as a System card. Clicking the knob flips the global
/// `GamingModeState` (compositor config + forced Performance profile); the
/// tab repaints on the resulting UPower signal.
pub fn render_gaming_mode_card(active: bool, theme: &Theme) -> AnyElement {
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let accent = theme.accent.primary;
    let hover = theme.interactive.hover;

    let title_row = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(text_primary)
                .child("Gaming mode"),
        )
        .child(toggle_switch(active, accent, hover));

    let effect = "Performance profile · No animations · Do Not Disturb · Hide bar/dock · VSync forced";

    card(theme)
        .gap(px(8.))
        .child(title_row)
        .child(
            div()
                .text_color(text_muted)
                .text_size(px(10.5))
                .line_height(px(16.))
                .child(effect),
        )
        .into_any_element()
}

/// System-card shell for a power control (consistent with `mpris_card` /
/// `wallpaper_card` / `disks` in the right panel).
fn card(theme: &Theme) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .rounded(px(9.))
        .overflow_hidden()
        .bg(crate::side_panel_right::surfaces::card(theme))
        .border_1()
        .border_color(theme.border.subtle)
        .p(px(12.))
}

/// Gaming-mode knob — identical geometry/logic to the popup origin
/// (`system_popup/view.rs::toggle_switch`), minus the unused `radius`.
fn toggle_switch(active: bool, accent: Hsla, hover: Hsla) -> AnyElement {
    let track_bg = if active { accent } else { hover };
    let knob_left = if active { px(17.) } else { px(2.) };
    let knob_color = on_fill(track_bg);

    div()
        .id("gaming-mode-toggle")
        .w(px(34.))
        .h(px(19.))
        .rounded(px(10.))
        .bg(track_bg)
        .cursor_pointer()
        .hover(move |s| s)
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(knob_left)
                .w(px(15.))
                .h(px(15.))
                .rounded(px(8.))
                .bg(knob_color),
        )
        .on_click(move |_event, _window, cx: &mut App| {
            crate::gaming_mode::toggle(cx);
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Smoke: both cards must build from default state without panicking. The
    // real interaction (dispatch + repaint) is covered live; this guards the
    // render path against field/theme drift.
    #[test]
    fn cards_build_from_default_state() {
        let theme = Theme::default();
        let upower = UPowerData::default();
        let _ = render_power_profile_card(&upower, &theme);
        let _ = render_gaming_mode_card(false, &theme);
        let _ = render_gaming_mode_card(true, &theme);
    }
}

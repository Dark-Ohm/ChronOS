//! Battery widget for the bar — shows percentage + charging icon + power profile.

use gpui::{AnyElement, App, Window, div, prelude::*, px, svg};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{profile_to_str, Service, PowerProfile};
use chronos_ui::Theme;

use crate::state::AppState;

pub struct BatteryWidget;

impl BarWidget for BatteryWidget {
    fn name(&self) -> &str {
        "battery"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let upower = AppState::upower(cx);
        let data = upower.get();

        // Desktop without battery: render empty div, don't show a fake 0% icon.
        // The honest `has_battery` flag comes from UPower
        // (EnumerateDevices finds a Battery device, or DisplayDevice.IsPresent).
        // The old heuristic (Unknown + 0%) is kept as a second line of defence
        // in case the DBus detection ever regresses to Unknown/0 on a real box.
        if !data.has_battery
            || upower.status() == chronos_services::ServiceStatus::Unavailable
            || (data.state == chronos_services::BatteryState::Unknown
                && data.battery_percent == 0.0)
        {
            return div().into_any_element();
        }

        let percent = data.battery_percent.round() as u32;
        let icon_path = match data.state {
            chronos_services::BatteryState::Charging => "icons/battery-charging.svg",
            chronos_services::BatteryState::Full => "icons/battery-charging.svg",
            _ => "icons/battery.svg",
        };

        /// Cycle to the next power profile: Performance → Balanced → PowerSaver → Performance.
        fn cycle_profile(current: PowerProfile) -> PowerProfile {
            match current {
                PowerProfile::Performance => PowerProfile::Balanced,
                PowerProfile::Balanced => PowerProfile::PowerSaver,
                PowerProfile::PowerSaver => PowerProfile::Performance,
            }
        }

        let profile_icon = match data.power_profile {
            PowerProfile::Performance => "icons/bolt.svg",
            PowerProfile::Balanced => "icons/bolt.svg",
            PowerProfile::PowerSaver => "icons/bolt.svg",
        };

        let theme = Theme::global(cx);
        let color = if percent <= 15 {
            theme.status.error
        } else if percent <= 30 {
            theme.status.warning
        } else {
            theme.status.success
        };

        div()
            .id("bar-battery")
            .flex()
            .items_center()
            .gap(px(4.))
            .cursor_pointer()
            .px(px(6.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(svg().path(icon_path).size(px(13.)).text_color(color))
            .child(
                div()
                    .child(format!("{percent}%"))
                    .text_color(color)
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.sm),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(svg().path(profile_icon).size(px(10.)).text_color(theme.text.muted))
                    .child(
                        div()
                            .child(profile_to_str(data.power_profile))
                            .text_color(theme.text.muted)
                            .text_size(theme.font_sizes.sm),
                    ),
            )
            .on_click(move |_event, _window, cx| {
                let upower = AppState::upower(cx);
                let current = upower.get().power_profile;
                let next = cycle_profile(current);
                let svc = upower.clone();
                cx.background_spawn(async move {
                    match svc.set_power_profile(next).await {
                        Ok(()) => tracing::info!("battery widget: set power profile to {:?}", next),
                        Err(e) => tracing::error!("battery widget: failed to set power profile: {e:?}"),
                    }
                })
                .detach();
            })
            .into_any_element()
    }
}

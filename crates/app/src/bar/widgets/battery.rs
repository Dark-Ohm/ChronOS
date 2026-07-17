//! Battery widget for the bar — shows percentage + charging icon.

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::Service;
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

        // Desktop without battery: render empty, don't panic.
        // UPower daemon is alive on desktops (DisplayDevice always exists),
        // but data shows percent=0.0, state=Unknown — that's "no battery".
        if upower.status() == chronos_services::ServiceStatus::Unavailable
            || (data.state == chronos_services::BatteryState::Unknown
                && data.battery_percent == 0.0)
        {
            return div().into_any_element();
        }

        let percent = data.battery_percent.round() as u32;
        let icon = match data.state {
            chronos_services::BatteryState::Charging => "⚡",
            chronos_services::BatteryState::Full => "⚡",
            _ => "🔋",
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
            .flex()
            .items_center()
            .gap(px(4.))
            .child(div().child(format!("{icon} {percent}%")).text_color(color))
            .into_any_element()
    }
}

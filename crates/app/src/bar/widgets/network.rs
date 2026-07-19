//! Network widget for the bar — shows wired / wifi / disconnected state.

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{ConnectivityState, NetworkData, Service, ServiceStatus};
use chronos_ui::Theme;

use crate::state::AppState;

/// Max SSID length shown before truncation.
const MAX_SSID_LEN: usize = 16;

/// Pure description of what the widget should display, decoupled from GPUI so
/// the branching logic is unit-testable without a window/app context.
#[derive(Debug, PartialEq, Eq)]
enum NetworkView {
    /// Service not ready (Initializing/Failed) or unknown connectivity.
    Stub,
    /// No connectivity at all.
    Disconnected,
    /// Wired connection (full connectivity, no wifi SSID).
    Wired,
    /// Wifi connection with SSID (already truncated) and strength 0..=100.
    Wifi { ssid: String, strength: u8 },
}

fn describe(data: &NetworkData, status: ServiceStatus) -> NetworkView {
    if matches!(
        status,
        ServiceStatus::Initializing | ServiceStatus::Unavailable | ServiceStatus::Degraded(_)
    ) || data.connectivity == ConnectivityState::Unknown
    {
        return NetworkView::Stub;
    }

    if data.connectivity == ConnectivityState::None {
        return NetworkView::Disconnected;
    }

    if data.connectivity == ConnectivityState::Full && data.wired {
        return NetworkView::Wired;
    }

    let ssid = data.wifi_ssid.clone().unwrap_or_else(|| "wifi".to_string());
    let ssid = if ssid.chars().count() > MAX_SSID_LEN {
        let truncated: String = ssid.chars().take(MAX_SSID_LEN).collect();
        format!("{truncated}…")
    } else {
        ssid
    };

    NetworkView::Wifi {
        ssid,
        strength: data.wifi_strength.unwrap_or(0),
    }
}

/// Strength buckets for the wifi icon (0..=100 → 4 levels).
fn strength_icon(strength: u8) -> &'static str {
    match strength {
        0..=24 => "󰤯",
        25..=49 => "󰤟",
        50..=74 => "󰤢",
        _ => "󰤨",
    }
}

pub struct NetworkWidget;

impl BarWidget for NetworkWidget {
    fn name(&self) -> &str {
        "network"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let net = AppState::network(cx);
        let theme = Theme::global(cx);

        match describe(&net.get(), net.status()) {
            NetworkView::Stub => div()
                .flex()
                .items_center()
                .child(div().child("󰕭").text_color(theme.text.muted))
                .into_any_element(),
            NetworkView::Disconnected => div()
                .flex()
                .items_center()
                .child(div().child("󰕭 off").text_color(theme.text.muted))
                .into_any_element(),
            NetworkView::Wired => div()
                .flex()
                .items_center()
                .child(div().child("󰈀 eth").text_color(theme.text.secondary))
                .into_any_element(),
            NetworkView::Wifi { ssid, strength } => div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .child(strength_icon(strength))
                        .text_color(theme.text.secondary),
                )
                .child(div().child(ssid).text_color(theme.text.secondary))
                .into_any_element(),
        }
    }
}

/// Register the network widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(NetworkWidget));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> NetworkData {
        NetworkData::default()
    }

    #[test]
    fn stub_when_initializing() {
        assert_eq!(
            describe(&data(), ServiceStatus::Initializing),
            NetworkView::Stub
        );
    }

    #[test]
    fn stub_when_unavailable() {
        assert_eq!(
            describe(&data(), ServiceStatus::Unavailable),
            NetworkView::Stub
        );
    }

    #[test]
    fn stub_when_degraded() {
        assert_eq!(
            describe(&data(), ServiceStatus::Degraded("no wifi hw".into())),
            NetworkView::Stub
        );
    }

    #[test]
    fn stub_when_unknown_connectivity() {
        assert_eq!(
            describe(&data(), ServiceStatus::Available),
            NetworkView::Stub
        );
    }

    #[test]
    fn disconnected_on_none() {
        let mut d = data();
        d.connectivity = ConnectivityState::None;
        assert_eq!(
            describe(&d, ServiceStatus::Available),
            NetworkView::Disconnected
        );
    }

    #[test]
    fn wired_on_full_with_wired_flag() {
        let mut d = data();
        d.connectivity = ConnectivityState::Full;
        d.wired = true;
        assert_eq!(describe(&d, ServiceStatus::Available), NetworkView::Wired);
    }

    #[test]
    fn full_without_wired_flag_is_wifi() {
        // Regression guard: Full connectivity with no SSID but `wired == false`
        // must NOT be reported as Wired (this is exactly the false-positive the
        // old `Full && ssid.is_none()` heuristic produced).
        let mut d = data();
        d.connectivity = ConnectivityState::Full;
        assert_eq!(
            describe(&d, ServiceStatus::Available),
            NetworkView::Wifi {
                ssid: "wifi".into(),
                strength: 0,
            }
        );
    }

    #[test]
    fn wifi_shows_ssid_and_strength() {
        let mut d = data();
        d.connectivity = ConnectivityState::Full;
        d.wifi_ssid = Some("HomeNet".into());
        d.wifi_strength = Some(80);
        assert_eq!(
            describe(&d, ServiceStatus::Available),
            NetworkView::Wifi {
                ssid: "HomeNet".into(),
                strength: 80
            }
        );
    }

    #[test]
    fn wifi_truncates_long_ssid() {
        let mut d = data();
        d.connectivity = ConnectivityState::Full;
        d.wifi_ssid = Some("ThisIsAVeryLongSSIDName".into());
        d.wifi_strength = Some(50);
        match describe(&d, ServiceStatus::Available) {
            NetworkView::Wifi { ssid, .. } => {
                assert!(ssid.ends_with('…') && ssid.chars().count() == MAX_SSID_LEN + 1)
            }
            other => panic!("expected Wifi, got {other:?}"),
        }
    }

    #[test]
    fn strength_icon_buckets() {
        assert_eq!(strength_icon(0), "󰤯");
        assert_eq!(strength_icon(24), "󰤯");
        assert_eq!(strength_icon(25), "󰤟");
        assert_eq!(strength_icon(49), "󰤟");
        assert_eq!(strength_icon(50), "󰤢");
        assert_eq!(strength_icon(74), "󰤢");
        assert_eq!(strength_icon(75), "󰤨");
        assert_eq!(strength_icon(100), "󰤨");
    }
}

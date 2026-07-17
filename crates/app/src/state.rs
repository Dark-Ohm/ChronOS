//! Application-wide runtime state stored as a GPUI global.

use futures_signals::signal::{Signal, SignalExt};
use futures_util::stream::StreamExt;
use gpui::{App, Context, Global};

use chronos_services::Services;

/// Global runtime state shared across views/widgets.
#[derive(Clone)]
pub struct AppState {
    services: Services,
}

impl Global for AppState {}

impl AppState {
    /// Initialize the global app state from constructed services.
    pub fn init(services: Services, cx: &mut App) {
        cx.set_global(Self { services });
    }

    #[inline(always)]
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    #[inline(always)]
    pub fn compositor(cx: &App) -> &chronos_services::CompositorSubscriber {
        &Self::global(cx).services.compositor
    }

    #[inline(always)]
    pub fn network(cx: &App) -> &chronos_services::NetworkSubscriber {
        &Self::global(cx).services.network
    }

    #[inline(always)]
    pub fn notification(cx: &App) -> &chronos_services::NotificationSubscriber {
        &Self::global(cx).services.notification
    }

    #[inline(always)]
    pub fn upower(cx: &App) -> &chronos_services::UPowerSubscriber {
        &Self::global(cx).services.upower
    }

    #[inline(always)]
    pub fn tray(cx: &App) -> &chronos_services::TraySubscriber {
        &Self::global(cx).services.tray
    }

    #[inline(always)]
    pub fn audio(cx: &App) -> &chronos_services::AudioSubscriber {
        &Self::global(cx).services.audio
    }

    #[inline(always)]
    pub fn applications(cx: &App) -> &chronos_services::ApplicationsSubscriber {
        &Self::global(cx).services.applications
    }

    #[inline(always)]
    pub fn wallpaper(cx: &App) -> &chronos_services::WallpaperSubscriber {
        &Self::global(cx).services.wallpaper
    }

    #[inline(always)]
    pub fn mpris(cx: &App) -> &chronos_services::MprisSubscriber {
        &Self::global(cx).services.mpris
    }
}

/// Watch a signal and apply updates to component state.
///
/// `S: Signal<Item = T> + Unpin + 'static` — satisfied by the `impl Signal + Unpin`
/// returned from `Service::subscribe()` (spec §4).
pub fn watch<C, S, T, F>(cx: &mut Context<C>, signal: S, on_update: F)
where
    C: 'static,
    S: Signal<Item = T> + Unpin + 'static,
    T: Clone + 'static,
    F: Fn(&mut C, T, &mut Context<C>) + 'static,
{
    cx.spawn(async move |this, cx| {
        let mut stream = signal.to_stream();
        while let Some(data) = stream.next().await {
            if this
                .update(cx, |this, cx| {
                    on_update(this, data.clone(), cx);
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::{
        CompositorSubscriber, NetworkSubscriber, ServiceStatus, UPowerSubscriber,
    };

    /// Test that AppState module compiles and functions exist
    #[test]
    fn app_state_module_compiles() {
        // Verify the module structure and function signatures are correct
        let _ = AppState::init;
        let _ = AppState::global;
        let _ = AppState::compositor;
        let _ = AppState::network;
        let _ = AppState::upower;
        let _ = AppState::applications;

        assert!(true);
    }

    /// Test that AppState accessors return the correct subscriber types
    #[test]
    fn app_state_accessor_types() {
        // Just verify the accessor function signatures are correct
        let _compositor_fn = AppState::compositor;
        let _network_fn = AppState::network;
        let _upower_fn = AppState::upower;

        // Verify they return the expected types (via type inference)
        fn _check_compositor(_: fn(&gpui::App) -> &chronos_services::CompositorSubscriber) {}
        fn _check_network(_: fn(&gpui::App) -> &chronos_services::NetworkSubscriber) {}
        fn _check_upower(_: fn(&gpui::App) -> &chronos_services::UPowerSubscriber) {}

        _check_compositor(AppState::compositor);
        _check_network(AppState::network);
        _check_upower(AppState::upower);

        assert!(true);
    }

    /// Test ServiceStatus variants are accessible
    #[test]
    fn service_status_variants() {
        let _ = ServiceStatus::Available;
        let _ = ServiceStatus::Unavailable;
        let _ = ServiceStatus::Initializing;
        let _ = ServiceStatus::Degraded(String::new());
        assert!(true);
    }

    /// Test subscriber types are accessible
    #[test]
    fn subscriber_types_accessible() {
        let _ = std::any::type_name::<CompositorSubscriber>();
        let _ = std::any::type_name::<NetworkSubscriber>();
        let _ = std::any::type_name::<UPowerSubscriber>();
        assert!(true);
    }
}

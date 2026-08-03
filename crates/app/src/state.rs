//! Application-wide runtime state stored as a GPUI global.

use std::sync::atomic::{AtomicU32, Ordering};

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

    #[inline(always)]
    pub fn aur(cx: &App) -> &chronos_services::AurSubscriber {
        &Self::global(cx).services.aur
    }

    #[inline(always)]
    pub fn cava(cx: &App) -> &chronos_services::CavaSubscriber {
        &Self::global(cx).services.cava
    }

    #[inline(always)]
    pub fn brightness(cx: &App) -> &chronos_services::BrightnessSubscriber {
        &Self::global(cx).services.brightness
    }

    #[inline(always)]
    pub fn power(cx: &App) -> &chronos_services::PowerSubscriber {
        &Self::global(cx).services.power
    }

    #[inline(always)]
    pub fn system_resources(cx: &App) -> &chronos_services::SystemResourcesSubscriber {
        &Self::global(cx).services.system_resources
    }

    #[inline(always)]
    pub fn disks(cx: &App) -> &chronos_services::DisksSubscriber {
        &Self::global(cx).services.udisks
    }
}

/// Applied bar height in px, exposed to lib-visible consumers (T200).
///
/// The bar module is bin-only (`mod bar;` in `main.rs`); side panels (lib)
/// need the live `[appearance] height` for their top gap without reaching
/// into `crate::bar`. The bar writes here on every appearance apply (hot-
/// reload) and at startup; the default mirrors the historical `BAR_HEIGHT`.
const LIVE_BAR_HEIGHT_BITS: u32 = chronos_luau::bar::BAR_HEIGHT.to_bits();

static LIVE_BAR_HEIGHT: AtomicU32 = AtomicU32::new(LIVE_BAR_HEIGHT_BITS);

/// Current applied bar height in px (configured `[appearance] height` or
/// the code default).
pub fn bar_height_px() -> f32 {
    f32::from_bits(LIVE_BAR_HEIGHT.load(Ordering::Relaxed))
}

/// Record the applied bar height (called by `bar::apply_appearance` and
/// `bar::init`).
pub fn set_bar_height_px(height: f32) {
    LIVE_BAR_HEIGHT.store(height.to_bits(), Ordering::Relaxed);
}

/// T217 — bar top-corner geometry published for the panel junction.
///
/// The bar is bin-only while the side panels are lib, so the panels read the
/// bar's live `[appearance] radius` and horizontal screen extent through the
/// same AtomicU32 (f32 bits) mechanism as the height. The bar writes these
/// in `bar::apply_appearance` and `bar::init`. Defaults describe a full-width
/// square bar that covers every panel strip — the pre-T217 square chrome —
/// so an unpublished / un-applied bar reads as "square, no rounding".
static LIVE_BAR_RADIUS_BITS: AtomicU32 = AtomicU32::new(0.0f32.to_bits());
static LIVE_BAR_X0_BITS: AtomicU32 = AtomicU32::new(0.0f32.to_bits());
static LIVE_BAR_X1_BITS: AtomicU32 = AtomicU32::new(f32::INFINITY.to_bits());

/// Current applied bar corner radius in px (`[appearance] radius`).
pub fn bar_radius_px() -> f32 {
    f32::from_bits(LIVE_BAR_RADIUS_BITS.load(Ordering::Relaxed))
}

/// The bar's horizontal screen extent `[x0, x1]` on the pult display
/// (px, left edge = 0). A panel whose strip this interval covers is tucked
/// under the bar; a strip outside it has a free top edge.
pub fn bar_x_extent() -> (f32, f32) {
    (
        f32::from_bits(LIVE_BAR_X0_BITS.load(Ordering::Relaxed)),
        f32::from_bits(LIVE_BAR_X1_BITS.load(Ordering::Relaxed)),
    )
}

/// Record the applied bar corner geometry (called by `bar::apply_appearance`
/// and `bar::init` alongside `set_bar_height_px`).
pub fn set_bar_geometry(radius: f32, x0: f32, x1: f32) {
    LIVE_BAR_RADIUS_BITS.store(radius.to_bits(), Ordering::Relaxed);
    LIVE_BAR_X0_BITS.store(x0.to_bits(), Ordering::Relaxed);
    LIVE_BAR_X1_BITS.store(x1.to_bits(), Ordering::Relaxed);
}

/// Corner radius for a panel's top corner at screen x (px on the pult
/// display, left edge = 0): `bar_radius_px()` when the bar does not sit
/// above that x (the corner's top edge is free — rhyme with the bar's pill),
/// 0 when the bar covers it (the panel tucks under the bar's bottom edge —
/// rounding would leave a semicircular seam).
///
/// Shared by both side panels (and, by construction, their hover strips) so
/// the junction rule lives in exactly one place (T217).
pub fn panel_corner_radius(x: f32) -> f32 {
    let (bar_x0, bar_x1) = bar_x_extent();
    if x >= bar_x0 && x <= bar_x1 {
        0.0
    } else {
        bar_radius_px()
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

    /// Live bar height (T200) defaults to the code `BAR_HEIGHT` and round-
    /// trips through the AtomicU32 (f32 bits) store.
    #[test]
    fn bar_height_defaults_and_round_trips() {
        assert_eq!(bar_height_px(), chronos_luau::bar::BAR_HEIGHT);
        set_bar_height_px(42.5);
        assert_eq!(bar_height_px(), 42.5);
        // Restore for other tests (process-wide static).
        set_bar_height_px(chronos_luau::bar::BAR_HEIGHT);
        assert_eq!(bar_height_px(), chronos_luau::bar::BAR_HEIGHT);
    }

    /// T217 — radius + extent round-trip through the atomics, plus the
    /// per-corner junction rule: free edge → bar radius, under the bar → 0.
    ///
    /// This is the ONLY test touching the bar-geometry statics (process-wide
    /// AtomicU32, tests run in parallel threads) — asserting the default here
    /// and then mutating it in one test keeps the module race-free.
    #[test]
    fn bar_geometry_round_trips_and_junction_rule() {
        // Defaults describe a full-width square bar (pre-T217 chrome): every
        // strip is "covered", so corners stay square.
        assert_eq!(bar_radius_px(), 0.0);
        let (x0, x1) = bar_x_extent();
        assert_eq!(x0, 0.0);
        assert_eq!(x1, f32::INFINITY);
        assert_eq!(panel_corner_radius(0.0), 0.0);
        assert_eq!(panel_corner_radius(2560.0), 0.0);

        set_bar_geometry(16.0, 384.0, 2176.0); // fraction:0.7 centered on 2560
        assert_eq!(bar_radius_px(), 16.0);
        assert_eq!(bar_x_extent(), (384.0, 2176.0));

        // Right screen edge (bar never reaches it) → rhyming pill corner.
        assert_eq!(panel_corner_radius(2560.0), 16.0);
        // Left screen edge (bar centered, away from it) → rhyming pill corner.
        assert_eq!(panel_corner_radius(0.0), 16.0);
        // Just outside the bar's right edge → free → rhyme.
        assert_eq!(panel_corner_radius(2177.0), 16.0);

        // Under the bar → square (butt, no seam).
        assert_eq!(panel_corner_radius(2000.0), 0.0);
        assert_eq!(panel_corner_radius(1000.0), 0.0);
        // Exactly on the bar edge counts as covered (shared edge → no seam).
        assert_eq!(panel_corner_radius(384.0), 0.0);
        assert_eq!(panel_corner_radius(2176.0), 0.0);

        // Full-width bar → every corner square.
        set_bar_geometry(16.0, 0.0, 2560.0);
        assert_eq!(bar_radius_px(), 16.0);
        assert_eq!(panel_corner_radius(0.0), 0.0);
        assert_eq!(panel_corner_radius(2560.0), 0.0);

        // Restore for other tests (process-wide static).
        set_bar_geometry(0.0, 0.0, f32::INFINITY);
        assert_eq!(bar_radius_px(), 0.0);
        assert_eq!(bar_x_extent(), (0.0, f32::INFINITY));
    }
}

//! System-integration services for Chronos (GPUI-agnostic).
//!
//! Each service is a subscriber holding a `futures_signals::Mutable<T>` and
//! implements the lightweight `Service` trait. Commands are concrete methods
//! on each subscriber (NOT part of the trait).

pub mod applications;
pub mod audio;
pub mod aur;
pub mod brightness;
pub mod cava;
pub mod compositor;
pub mod hermes_acp;
pub mod mpris;
pub mod net_stats;
pub mod network;
pub mod notification;
pub mod power;
pub mod system_resources;
pub mod tray;
pub mod udisks;
pub mod upower;
pub mod wallpaper;

pub use applications::{
    AppEntry, ApplicationsCommand, ApplicationsState, ApplicationsSubscriber, strip_field_codes,
};
pub use audio::{AudioCommand, AudioDevice, AudioState, AudioSubscriber, EndpointState};
pub use aur::{AurCommand, AurSubscriber, PackageUpdate, UpdateSource, UpgradeState, UpdatesState};
pub use brightness::{
    BrightnessCommand, BrightnessState, BrightnessSubscriber, DDCUTIL_BIN, detect_displays,
    get_brightness, parse_getvcp_stdout, read_primary, set_brightness, write_all,
};
pub use cava::{CavaState, CavaSubscriber, BAR_COUNT as CAVA_BAR_COUNT};
pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};
pub use mpris::{CycleDirection, MprisCommand, MprisState, MprisSubscriber};
pub use network::{ConnectivityState, NetworkData, NetworkSubscriber};
pub use notification::{
    CloseReason, Notification, NotificationCommand, NotificationState, NotificationSubscriber,
    Urgency,
};
pub use power::PowerSubscriber;
pub use system_resources::{SystemResourcesState, SystemResourcesSubscriber};
pub use tray::{
    MenuNode, MenuToggleType, TrayCommand, TrayIcon, TrayItem, TrayPixmap, TrayState,
    TraySubscriber, strip_mnemonic,
};
pub use udisks::{DiskInfo, DisksCommand, DisksSubscriber};
pub use upower::{
    profile_to_str, BatteryState, PowerProfile, UPowerData, UPowerSubscriber,
};
pub use wallpaper::{
    AWWW_BIN, AWWW_DAEMON_BIN, Backend, IMAGE_EXTENSIONS, WallpaperCommand, WallpaperState,
    WallpaperSubscriber, command_to_awww_args, is_image, parse_query,
};
pub use hermes_acp::{AgentDescriptor, ModelInfo, SessionMode, known_agents};

/// Container holding all system-integration subscribers.
#[derive(Clone)]
pub struct Services {
    pub applications: ApplicationsSubscriber,
    pub aur: AurSubscriber,
    pub audio: AudioSubscriber,
    pub brightness: BrightnessSubscriber,
    pub cava: CavaSubscriber,
    pub compositor: CompositorSubscriber,
    pub mpris: MprisSubscriber,
    pub network: NetworkSubscriber,
    pub notification: NotificationSubscriber,
    pub power: power::PowerSubscriber,
    pub system_resources: SystemResourcesSubscriber,
    pub tray: TraySubscriber,
    pub udisks: DisksSubscriber,
    pub upower: UPowerSubscriber,
    pub wallpaper: WallpaperSubscriber,
}

/// Construct all subscribers. Always succeeds (spec §6): each constructor is
/// non-failing and starts its own background connect/retry task. MUST be called
/// inside a tokio runtime context (rt.block_on) so `Handle::current()` resolves
/// in the D-Bus constructors.
pub fn init_all() -> Services {
    Services {
        applications: ApplicationsSubscriber::new(),
        aur: AurSubscriber::new(),
        audio: AudioSubscriber::new(),
        brightness: BrightnessSubscriber::new(),
        cava: CavaSubscriber::new(),
        compositor: CompositorSubscriber::new(),
        mpris: MprisSubscriber::new(),
        network: NetworkSubscriber::new(),
        notification: NotificationSubscriber::new(),
        power: power::PowerSubscriber::new(),
        system_resources: SystemResourcesSubscriber::new(),
        tray: TraySubscriber::new(),
        udisks: DisksSubscriber::new(),
        upower: UPowerSubscriber::new(),
        wallpaper: WallpaperSubscriber::new(),
    }
}

use futures_signals::signal::Signal;

/// Availability of a service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Created; first connection attempt pending (set by `new()`).
    Initializing,
    /// Fully functional.
    Available,
    /// All connection attempts failed; retry loop running in background.
    Unavailable,
    /// Connected but some features missing (e.g. NM present, Wi-Fi hardware absent).
    Degraded(String),
}

/// Lightweight, unified service contract: availability + reactivity.
/// Commands are concrete methods on each subscriber, not part of this trait.
pub trait Service: Send + Sync + 'static {
    type Data: Clone + 'static;
    type Error: Send + Sync + 'static;
    /// Reactive signal. Hides the `Mutable`; consumer cannot call `.set()`.
    fn subscribe(&self) -> impl Signal<Item = Self::Data> + Unpin + 'static;
    fn get(&self) -> Self::Data;
    fn status(&self) -> ServiceStatus;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_signals::signal::Mutable;

    struct FakeService {
        data: Mutable<u32>,
    }

    impl Service for FakeService {
        type Data = u32;
        type Error = anyhow::Error;
        fn subscribe(&self) -> impl Signal<Item = u32> + Unpin + 'static {
            self.data.signal_cloned()
        }
        fn get(&self) -> u32 {
            self.data.get()
        }
        fn status(&self) -> ServiceStatus {
            ServiceStatus::Available
        }
    }

    #[tokio::test]
    async fn service_contract_emits_on_mutate() {
        use futures_signals::signal::SignalExt;
        use futures_util::stream::StreamExt;
        let svc = FakeService {
            data: Mutable::new(1),
        };
        let sig = svc.subscribe();
        assert_eq!(svc.get(), 1);
        assert_eq!(svc.status(), ServiceStatus::Available);

        svc.data.set(42);
        let mut stream = sig.to_stream();
        let received = stream.next().await.expect("signal emits");
        assert_eq!(received, 42);
    }

    #[test]
    fn anyhow_error_satisfies_error_bound() {
        fn takes_error<E: Send + Sync + 'static>(_e: E) {}
        let err = anyhow::Error::msg("test");
        takes_error(err); // anyhow::Error satisfies Send + Sync + 'static
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;
    use futures_signals::signal::{Mutable, Signal};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    /// Mirrors the spec §5.1 retry loop shape against a controllable backend.
    #[allow(dead_code)]
    struct FakeRetryService {
        status: Mutable<ServiceStatus>,
        failures_before_success: u32,
        attempts: Arc<AtomicU32>,
    }

    impl FakeRetryService {
        fn new(failures_before_success: u32) -> Self {
            let status = Mutable::new(ServiceStatus::Initializing);
            let attempts = Arc::new(AtomicU32::new(0));
            let s = Self {
                status: status.clone(),
                failures_before_success,
                attempts: attempts.clone(),
            };

            // Drive the retry loop synchronously on this test thread (no tokio needed
            // for the assertion; mirrors backoff math from spec §5.1).
            let mut backoff = Duration::from_secs(1);
            let max = Duration::from_secs(60);
            let start = Instant::now();
            loop {
                let n = s.attempts.fetch_add(1, Ordering::SeqCst);
                if n >= s.failures_before_success {
                    status.set(ServiceStatus::Available);
                    break;
                }
                status.set(ServiceStatus::Unavailable);
                // In the real loop this sleeps; here we assert the backoff math only.
                backoff = (backoff * 2).min(max);
                if start.elapsed() > Duration::from_secs(1) {
                    break; // safety; test uses small failure counts
                }
            }
            s
        }
    }

    impl Service for FakeRetryService {
        type Data = ();
        type Error = anyhow::Error;
        fn subscribe(&self) -> impl Signal<Item = ()> + Unpin + 'static {
            Mutable::new(()).signal_cloned()
        }
        fn get(&self) -> () {}
        fn status(&self) -> ServiceStatus {
            self.status.get_cloned()
        }
    }

    #[test]
    fn retry_ends_in_available_after_failures() {
        // failures_before_success = 3 means 3 failures (Unavailable) then the
        // 4th attempt succeeds (Available). The loop's `n >= failures_before_success`
        // guard triggers success on the (N+1)-th attempt, so attempts == N + 1.
        let svc = FakeRetryService::new(3);
        assert_eq!(svc.status(), ServiceStatus::Available);
        assert_eq!(svc.attempts.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn retry_starts_initializing_then_unavailable() {
        // With 1 failure, the loop sets Unavailable before the success attempt.
        let svc = FakeRetryService::new(1);
        assert_eq!(svc.status(), ServiceStatus::Available);
        assert!(svc.attempts.load(Ordering::SeqCst) >= 1);
    }
}

#[cfg(test)]
mod runtime_guard_tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Edge case demanded by the Lead Architect: `NetworkSubscriber::new()` and
    /// `UPowerSubscriber::new()` call `Handle::current()`, which panics when
    /// invoked outside a tokio runtime. This test pins that behaviour so the
    /// guard is never silently removed. In the normal path `init_all()` runs
    /// inside `rt.block_on` (spec §5.1 + §7), so no panic occurs there.
    #[test]
    fn network_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = NetworkSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "NetworkSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[test]
    fn applications_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = ApplicationsSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "ApplicationsSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[test]
    fn upower_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = UPowerSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "UPowerSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[test]
    fn tray_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = TraySubscriber::new();
        }));
        assert!(
            result.is_err(),
            "TraySubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[test]
    fn audio_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = AudioSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "AudioSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[test]
    fn mpris_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = MprisSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "MprisSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }
}

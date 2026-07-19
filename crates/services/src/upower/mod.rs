//! Power service via UPower (D-Bus, system bus).
//!
//! ASYNC TEMPLATE (spec §5.1): same connect+retry loop shape as
//! NetworkSubscriber. `new()` captures `Handle::current()` and `tokio::spawn`s
//! the loop; `init_all()` (spec §7) calls this inside `rt.block_on`.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use futures_util::stream::StreamExt;
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::{Connection, proxy};

use crate::Service;
use crate::ServiceStatus;
pub use types::{BatteryState, PowerProfile, UPowerData};

/// Map our PowerProfile enum to the string expected by power-profiles-daemon.
pub fn profile_to_str(p: PowerProfile) -> &'static str {
    match p {
        PowerProfile::Performance => "performance",
        PowerProfile::Balanced => "balanced",
        PowerProfile::PowerSaver => "power-saver",
    }
}

pub mod types;

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/devices/DisplayDevice"
)]
trait DisplayDevice {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<i32>;
    /// True when a battery is physically present. On desktops the
    /// DisplayDevice exists but reports `IsPresent == false` → no battery.
    #[zbus(property)]
    fn is_present(&self) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    fn enumerate_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait UPowerDevice {
    #[zbus(property)]
    fn type_(&self) -> zbus::Result<u32>;
}

#[proxy(
    interface = "net.hadess.PowerProfiles",
    default_service = "net.hadess.PowerProfiles",
    default_path = "/net/hadess/PowerProfiles"
)]
trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;
}

fn map_state(s: i32) -> BatteryState {
    match s {
        1 => BatteryState::Charging,
        2 => BatteryState::Discharging,
        3 => BatteryState::Empty,
        4 => BatteryState::Full,
        _ => BatteryState::Unknown,
    }
}

/// Map power-profiles-daemon profile string to our enum.
/// Unknown values default to Performance with a warning.
fn map_profile(s: String) -> PowerProfile {
    match s.as_str() {
        "performance" => PowerProfile::Performance,
        "balanced" => PowerProfile::Balanced,
        "power-saver" => PowerProfile::PowerSaver,
        other => {
            warn!("Unknown power profile '{}', defaulting to Performance", other);
            PowerProfile::Performance
        }
    }
}

/// UPower device Type values (from the spec's `PowerSource` enum).
const UPOWER_DEVICE_TYPE_BATTERY: u32 = 2;

/// Honest "a battery is present" detection.
///
/// Preferred signal: any UPower device enumerated with Type=Battery.
/// Fallback: the DisplayDevice's `IsPresent` property (false on a desktop
/// whose DisplayDevice is a synthetic stub). This replaces the bar widget's
/// fragile guess (`state == Unknown && percent == 0.0`).
async fn detect_has_battery(conn: &Connection) -> bool {
    // Fallback check first (cheap, single property read on a path we already
    // hold a proxy for): DisplayDevice.IsPresent.
    if let Ok(dev) = DisplayDeviceProxy::new(conn).await {
        if let Ok(present) = dev.is_present().await {
            if present {
                return true;
            }
        }
    }

    // Primary check: enumerate devices and look for a real battery.
    let Ok(up) = UPowerProxy::new(conn).await else {
        return false;
    };
    let Ok(devices) = up.enumerate_devices().await else {
        return false;
    };
    for path in devices {
        let Some(builder) = UPowerDeviceProxy::builder(conn)
            .destination("org.freedesktop.UPower")
            .ok()
            .and_then(|b| b.path(path).ok())
        else {
            continue;
        };
        let Ok(proxy) = builder.build().await else {
            continue;
        };
        if let Ok(t) = proxy.type_().await {
            if t == UPOWER_DEVICE_TYPE_BATTERY {
                return true;
            }
        }
    }
    false
}

#[derive(Clone)]
pub struct UPowerSubscriber {
    data: Mutable<UPowerData>,
    status: Mutable<ServiceStatus>,
    /// Stored so future command methods (`set_power_profile`, when wired in a
    /// follow-up spec) can reuse the live D-Bus connection. Currently unused —
    /// retained per plan.
    conn: Mutable<Option<Connection>>,
}

impl UPowerSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`, so the
    /// runtime is present there.
    pub fn new() -> Self {
        let data = Mutable::new(UPowerData::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);

        // `Handle::current()` panics if there is no tokio runtime. `init_all()`
        // (spec §7) calls this inside `rt.block_on`, so the runtime is present.
        // We don't use the handle directly (the spawned future already runs in
        // the runtime), but the call is the documented guard against calling
        // `new()` outside a runtime.
        let _handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone(), conn.clone()));

        Self { data, status, conn }
    }

    /// Set the active power profile via power-profiles-daemon.
    pub async fn set_power_profile(&self, profile: PowerProfile) -> anyhow::Result<()> {
        let conn = self
            .conn
            .get_cloned()
            .ok_or_else(|| anyhow::anyhow!("UPowerSubscriber not connected yet"))?;
        let profiles = PowerProfilesProxy::new(&conn).await?;
        let profile_str = profile_to_str(profile);
        profiles.set_active_profile(profile_str).await?;
        Ok(())
    }
}

impl Service for UPowerSubscriber {
    type Data = UPowerData;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = UPowerData> + Unpin + 'static {
        self.data.signal_cloned()
    }
    fn get(&self) -> UPowerData {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Async connect + retry loop (spec §5.1). Same shape as
/// NetworkSubscriber::run. Exponential backoff 1s→2s→…→60s, infinite retries.
/// Internal `connect_timeout` (~5s) so a hung bus never blocks the loop.
async fn run(
    data: Mutable<UPowerData>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    let mut backoff = Duration::from_secs(1);

    loop {
        let connect = async {
            let conn = Connection::system().await?;
            let dev = DisplayDeviceProxy::new(&conn).await?;
            let profiles = PowerProfilesProxy::new(&conn).await?;
            let percentage = dev.percentage().await.unwrap_or(0.0);
            let state = dev
                .state()
                .await
                .map(map_state)
                .unwrap_or(BatteryState::Unknown);
            let profile = profiles
                .active_profile()
                .await
                .map(map_profile)
                .unwrap_or_default();
            Ok::<_, anyhow::Error>((conn, dev, profiles, percentage, state, profile))
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok((conn, dev, profiles, percentage, state, profile))) => {
                let has_battery = detect_has_battery(&conn).await;
                data.set(UPowerData {
                    battery_percent: percentage,
                    state,
                    has_battery,
                    power_profile: profile,
                });
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn));
                info!("UPowerSubscriber connected");

                // Subscribe to percentage + state + profile property changes; on stream
                // error, break to retry.
                let percentage_stream = dev.receive_percentage_changed().await;
                let state_stream = dev.receive_state_changed().await;
                let profile_stream = profiles.receive_active_profile_changed().await;
                let mut percentage_stream = std::pin::pin!(percentage_stream);
                let mut state_stream = std::pin::pin!(state_stream);
                let mut profile_stream = std::pin::pin!(profile_stream);
                loop {
                    // Wait for any property stream to emit; re-read all on
                    // change (cheap, and keeps data consistent).
                    let next = tokio::select! {
                        _ = percentage_stream.next() => true,
                        _ = state_stream.next() => true,
                        _ = profile_stream.next() => true,
                    };
                    if !next {
                        break; // all streams ended cleanly
                    }
                    let percentage = dev.percentage().await.unwrap_or(0.0);
                    let state = dev
                        .state()
                        .await
                        .map(map_state)
                        .unwrap_or(BatteryState::Unknown);
                    let profile = profiles
                        .active_profile()
                        .await
                        .map(map_profile)
                        .unwrap_or_default();
                    // has_battery is static for the lifetime of the machine; no
                    // need to re-poll it on every property change.
                    data.set(UPowerData {
                        battery_percent: percentage,
                        state,
                        power_profile: profile,
                        ..data.get_cloned()
                    });
                }
                status.set(ServiceStatus::Unavailable);
            }
            Ok(Err(e)) => {
                warn!("UPowerSubscriber connect failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
            }
            Err(_) => {
                warn!("UPowerSubscriber connect timed out, retrying");
                status.set(ServiceStatus::Unavailable);
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_profile_known_values() {
        assert_eq!(map_profile("performance".to_string()), PowerProfile::Performance);
        assert_eq!(map_profile("balanced".to_string()), PowerProfile::Balanced);
        assert_eq!(map_profile("power-saver".to_string()), PowerProfile::PowerSaver);
    }

    #[test]
    fn map_profile_unknown_defaults_to_performance() {
        assert_eq!(map_profile("unknown".to_string()), PowerProfile::Performance);
        assert_eq!(map_profile("".to_string()), PowerProfile::Performance);
    }

    #[test]
    fn profile_to_str_roundtrip() {
        assert_eq!(profile_to_str(PowerProfile::Performance), "performance");
        assert_eq!(profile_to_str(PowerProfile::Balanced), "balanced");
        assert_eq!(profile_to_str(PowerProfile::PowerSaver), "power-saver");
    }
}

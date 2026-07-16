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

pub mod types;

#[proxy(
    interface = "org.freedesktop.UPower.DisplayDevice",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/devices/DisplayDevice"
)]
trait DisplayDevice {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<i32>;
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

#[derive(Clone)]
pub struct UPowerSubscriber {
    data: Mutable<UPowerData>,
    status: Mutable<ServiceStatus>,
    /// Stored so future command methods (`set_power_profile`, when wired in a
    /// follow-up spec) can reuse the live D-Bus connection. Currently unused —
    /// retained per plan.
    #[allow(dead_code)]
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

    pub async fn set_power_profile(&self, _profile: PowerProfile) -> anyhow::Result<()> {
        // Deferred: real PowerProfiles proxy wiring lands in a follow-up spec.
        anyhow::bail!("UPowerSubscriber::set_power_profile deferred to a follow-up spec")
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
            let percentage = dev.percentage().await.unwrap_or(0.0);
            let state = dev
                .state()
                .await
                .map(map_state)
                .unwrap_or(BatteryState::Unknown);
            Ok::<_, anyhow::Error>((conn, dev, percentage, state))
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok((conn, dev, percentage, state))) => {
                data.set(UPowerData {
                    battery_percent: percentage,
                    state,
                    ..UPowerData::default()
                });
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn));
                info!("UPowerSubscriber connected");

                // Subscribe to percentage + state property changes; on stream
                // error, break to retry.
                let percentage_stream = dev.receive_percentage_changed().await;
                let state_stream = dev.receive_state_changed().await;
                let mut percentage_stream = std::pin::pin!(percentage_stream);
                let mut state_stream = std::pin::pin!(state_stream);
                loop {
                    // Wait for either property stream to emit; re-read both on
                    // any change (cheap, and keeps data consistent).
                    let next = tokio::select! {
                        _ = percentage_stream.next() => true,
                        _ = state_stream.next() => true,
                    };
                    if !next {
                        break; // both streams ended cleanly
                    }
                    let percentage = dev.percentage().await.unwrap_or(0.0);
                    let state = dev
                        .state()
                        .await
                        .map(map_state)
                        .unwrap_or(BatteryState::Unknown);
                    data.set(UPowerData {
                        battery_percent: percentage,
                        state,
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

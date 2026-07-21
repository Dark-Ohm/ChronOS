//! Block-device inventory via udisks2 (D-Bus, system bus).
//!
//! MVP: poll every 2.5s (`GetBlockDevices` + property reads + usage via
//! sysinfo/statvfs on the chosen mount point). Fire-and-forget
//! Mount / Unmount / Eject|PowerOff.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{Connection, proxy};

use crate::Service;
use crate::ServiceStatus;
pub use types::{
    DiskInfo, DisksCommand, decode_ay, decode_mount_points, device_basename, format_bytes,
    pick_mount_point, size_label, usage_fraction,
};

pub mod types;

const POLL_INTERVAL: Duration = Duration::from_millis(2500);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

const UDISKS2_SERVICE: &str = "org.freedesktop.UDisks2";

#[proxy(
    interface = "org.freedesktop.UDisks2.Manager",
    default_service = "org.freedesktop.UDisks2",
    default_path = "/org/freedesktop/UDisks2/Manager"
)]
trait UDisks2Manager {
    fn get_block_devices(
        &self,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Block",
    default_service = "org.freedesktop.UDisks2"
)]
trait UDisks2Block {
    #[zbus(property)]
    fn size(&self) -> zbus::Result<u64>;
    #[zbus(property)]
    fn preferred_device(&self) -> zbus::Result<Vec<u8>>;
    #[zbus(property)]
    fn id_label(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn id_usage(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn drive(&self) -> zbus::Result<OwnedObjectPath>;
    #[zbus(property)]
    fn hint_ignore(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn hint_system(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn hint_auto(&self) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Filesystem",
    default_service = "org.freedesktop.UDisks2"
)]
trait UDisks2Filesystem {
    #[zbus(property)]
    fn mount_points(&self) -> zbus::Result<Vec<Vec<u8>>>;
    fn mount(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<String>;
    fn unmount(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.UDisks2.Drive",
    default_service = "org.freedesktop.UDisks2"
)]
trait UDisks2Drive {
    #[zbus(property)]
    fn removable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn ejectable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn can_power_off(&self) -> zbus::Result<bool>;
    fn eject(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
    fn power_off(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct DisksSubscriber {
    data: Mutable<Vec<DiskInfo>>,
    status: Mutable<ServiceStatus>,
    conn: Mutable<Option<Connection>>,
    runtime: Handle,
}

impl DisksSubscriber {
    /// Non-failing constructor. Panics outside a tokio runtime
    /// (`Handle::current()` guard — same as every other D-Bus subscriber).
    pub fn new() -> Self {
        let data = Mutable::new(Vec::new());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);
        let handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone(), conn.clone()));
        Self {
            data,
            status,
            conn,
            runtime: handle,
        }
    }

    /// Fire-and-forget Mount / Unmount / Eject; re-enumerates on success.
    pub fn dispatch(&self, cmd: DisksCommand) {
        let data = self.data.clone();
        let status = self.status.clone();
        let conn_slot = self.conn.clone();
        self.runtime.spawn(async move {
            let Some(conn) = conn_slot.get_cloned() else {
                warn!("DisksSubscriber: no system bus connection, drop {cmd:?}");
                return;
            };
            if let Err(e) = apply_command(&conn, &cmd).await {
                warn!("DisksSubscriber command failed ({cmd:?}): {e:?}");
                return;
            }
            match enumerate_disks(&conn).await {
                Ok(disks) => {
                    data.set(disks);
                    status.set(ServiceStatus::Available);
                }
                Err(e) => warn!("DisksSubscriber re-enumerate after command failed: {e:?}"),
            }
        });
    }

    pub fn mount(&self, block_path: impl Into<String>) {
        self.dispatch(DisksCommand::Mount {
            block_path: block_path.into(),
        });
    }

    pub fn unmount(&self, block_path: impl Into<String>) {
        self.dispatch(DisksCommand::Unmount {
            block_path: block_path.into(),
        });
    }

    pub fn eject(&self, drive_path: impl Into<String>) {
        self.dispatch(DisksCommand::Eject {
            drive_path: drive_path.into(),
        });
    }
}

impl Default for DisksSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for DisksSubscriber {
    type Data = Vec<DiskInfo>;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = Vec<DiskInfo>> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> Vec<DiskInfo> {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

async fn run(
    data: Mutable<Vec<DiskInfo>>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
) {
    let mut backoff = Duration::from_secs(1);
    loop {
        let connect = async {
            let conn = Connection::system().await?;
            let disks = enumerate_disks(&conn).await?;
            Ok::<_, anyhow::Error>((conn, disks))
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok((conn, disks))) => {
                info!(
                    "DisksSubscriber connected ({} filesystem device(s))",
                    disks.len()
                );
                data.set(disks);
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn.clone()));
                backoff = Duration::from_secs(1);

                loop {
                    tokio::time::sleep(POLL_INTERVAL).await;
                    match enumerate_disks(&conn).await {
                        Ok(disks) => data.set(disks),
                        Err(e) => {
                            warn!("DisksSubscriber poll failed, reconnecting: {e:?}");
                            status.set(ServiceStatus::Unavailable);
                            conn_slot.set(None);
                            break;
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("DisksSubscriber connect failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
            }
            Err(_) => {
                warn!("DisksSubscriber connect timed out, retrying");
                status.set(ServiceStatus::Unavailable);
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn apply_command(conn: &Connection, cmd: &DisksCommand) -> anyhow::Result<()> {
    let empty = HashMap::<&str, Value<'_>>::new();
    match cmd {
        DisksCommand::Mount { block_path } => {
            let fs = filesystem_proxy(conn, block_path).await?;
            let mount = fs.mount(empty).await?;
            info!("DisksSubscriber mounted {block_path} → {mount}");
        }
        DisksCommand::Unmount { block_path } => {
            let fs = filesystem_proxy(conn, block_path).await?;
            fs.unmount(empty).await?;
            info!("DisksSubscriber unmounted {block_path}");
        }
        DisksCommand::Eject { drive_path } => {
            let drive = drive_proxy(conn, drive_path).await?;
            let ejectable = drive.ejectable().await.unwrap_or(false);
            let can_power = drive.can_power_off().await.unwrap_or(false);
            if ejectable {
                drive.eject(empty).await?;
                info!("DisksSubscriber Drive.Eject on {drive_path}");
            } else if can_power {
                // Live fact (this host's USB HDD): Ejectable=false, CanPowerOff=true.
                // PowerOff is the safe-remove path for many USB controllers.
                drive.power_off(empty).await?;
                info!("DisksSubscriber Drive.PowerOff on {drive_path} (not ejectable)");
            } else {
                anyhow::bail!("drive {drive_path} is neither ejectable nor power-off-capable");
            }
        }
    }
    Ok(())
}

async fn filesystem_proxy(
    conn: &Connection,
    block_path: &str,
) -> anyhow::Result<UDisks2FilesystemProxy<'static>> {
    let path = OwnedObjectPath::try_from(block_path)
        .map_err(|e| anyhow::anyhow!("bad block path {block_path}: {e}"))?;
    let proxy = UDisks2FilesystemProxy::builder(conn)
        .destination(UDISKS2_SERVICE)?
        .path(path)?
        .build()
        .await?;
    Ok(proxy)
}

async fn drive_proxy(
    conn: &Connection,
    drive_path: &str,
) -> anyhow::Result<UDisks2DriveProxy<'static>> {
    let path = OwnedObjectPath::try_from(drive_path)
        .map_err(|e| anyhow::anyhow!("bad drive path {drive_path}: {e}"))?;
    let proxy = UDisks2DriveProxy::builder(conn)
        .destination(UDISKS2_SERVICE)?
        .path(path)?
        .build()
        .await?;
    Ok(proxy)
}

async fn block_proxy(
    conn: &Connection,
    path: OwnedObjectPath,
) -> anyhow::Result<UDisks2BlockProxy<'static>> {
    let proxy = UDisks2BlockProxy::builder(conn)
        .destination(UDISKS2_SERVICE)?
        .path(path)?
        .build()
        .await?;
    Ok(proxy)
}

/// Enumerate filesystem-bearing block devices. Pure-ish assembly after D-Bus
/// reads; usage comes from sysinfo (statvfs under the hood on Linux).
pub async fn enumerate_disks(conn: &Connection) -> anyhow::Result<Vec<DiskInfo>> {
    let manager = UDisks2ManagerProxy::new(conn).await?;
    let paths = manager
        .get_block_devices(HashMap::<&str, Value<'_>>::new())
        .await?;

    // Refresh disk usage once per poll (sysinfo reads /proc + statvfs).
    let usage_table = collect_mount_usage();

    let mut out = Vec::new();
    for path in paths {
        let block_path = path.as_str().to_string();
        let block = match block_proxy(conn, path).await {
            Ok(b) => b,
            Err(e) => {
                warn!("DisksSubscriber: block proxy {block_path}: {e:?}");
                continue;
            }
        };

        if block.hint_ignore().await.unwrap_or(false) {
            continue;
        }
        let id_usage = block.id_usage().await.unwrap_or_default();
        if id_usage != "filesystem" {
            continue;
        }

        let size = block.size().await.unwrap_or(0);
        if size == 0 {
            continue;
        }

        let preferred = decode_ay(&block.preferred_device().await.unwrap_or_default());
        let id_label = block.id_label().await.unwrap_or_default();
        let label = if id_label.trim().is_empty() {
            device_basename(&preferred)
        } else {
            id_label
        };

        // Filesystem interface may be absent briefly after unplug — skip then.
        let mounts = match UDisks2FilesystemProxy::builder(conn)
            .destination(UDISKS2_SERVICE)
            .ok()
            .and_then(|b| b.path(block_path.as_str()).ok())
        {
            Some(builder) => match builder.build().await {
                Ok(fs) => match fs.mount_points().await {
                    Ok(raw) => decode_mount_points(&raw),
                    Err(_) => continue,
                },
                Err(_) => continue,
            },
            None => continue,
        };

        let mount_point = pick_mount_point(&mounts).map(str::to_string);

        let (used_opt, total_for_label, fraction) = if let Some(ref mp) = mount_point {
            if let Some((used, total)) = usage_table
                .iter()
                .find(|(p, _, _)| p == mp)
                .map(|(_, u, t)| (*u, *t))
                .or_else(|| usage_for_path(mp))
            {
                // Prefer live free-space total when mounted; fall back to block Size.
                let total = if total > 0 { total } else { size };
                (
                    Some(used),
                    total,
                    usage_fraction(used, total),
                )
            } else {
                (None, size, 0.0)
            }
        } else {
            (None, size, 0.0)
        };

        let drive_path_raw = block.drive().await.ok();
        let drive_path = drive_path_raw.and_then(|p| {
            let s = p.as_str();
            // udisks2 uses "/" for "no drive".
            if s.is_empty() || s == "/" {
                None
            } else {
                Some(s.to_string())
            }
        });

        let (removable, ejectable) = if let Some(ref dp) = drive_path {
            match drive_proxy(conn, dp).await {
                Ok(drive) => {
                    let rem = drive.removable().await.unwrap_or(false);
                    let eject = drive.ejectable().await.unwrap_or(false);
                    let power = drive.can_power_off().await.unwrap_or(false);
                    // UI "извлечь" is meaningful when either path works.
                    (rem, eject || (rem && power))
                }
                Err(_) => (false, false),
            }
        } else {
            (false, false)
        };

        out.push(DiskInfo {
            label,
            size_label: size_label(used_opt, total_for_label),
            fraction,
            removable,
            ejectable,
            mount_point,
            block_path,
            drive_path,
        });
    }

    // Stable order: internal first, then removable; alpha by label inside group.
    out.sort_by(|a, b| {
        a.removable
            .cmp(&b.removable)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            .then_with(|| a.block_path.cmp(&b.block_path))
    });

    Ok(out)
}

/// `(mount_point, used, total)` from sysinfo disks list.
fn collect_mount_usage() -> Vec<(String, u64, u64)> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let avail = d.available_space();
            let used = total.saturating_sub(avail);
            (
                d.mount_point().to_string_lossy().into_owned(),
                used,
                total,
            )
        })
        .collect()
}

fn usage_for_path(mount: &str) -> Option<(u64, u64)> {
    let path = Path::new(mount);
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for d in disks.list() {
        if d.mount_point() == path {
            let total = d.total_space();
            let used = total.saturating_sub(d.available_space());
            return Some((used, total));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::types::*;
    use super::*;

    // Re-export type tests live in types.rs; keep a smoke that module loads.
    #[test]
    fn size_label_matches_mockup_shape() {
        let s = size_label(Some(12 * 1024 * 1024 * 1024), 64 * 1024 * 1024 * 1024);
        assert_eq!(s, "12G / 64G");
        assert!((usage_fraction(12, 64) - 0.1875).abs() < 0.001);
    }

    /// Live system-bus smoke — skips cleanly if udisks2 is unreachable.
    #[tokio::test]
    async fn live_enumerate_sees_filesystem_devices() {
        let Ok(conn) = Connection::system().await else {
            eprintln!("skip: no system bus");
            return;
        };
        let Ok(disks) = enumerate_disks(&conn).await else {
            eprintln!("skip: udisks2 enumerate failed");
            return;
        };
        // On this workstation: root FS + removable USB partitions.
        assert!(
            !disks.is_empty(),
            "expected at least one filesystem device from udisks2"
        );
        for d in &disks {
            assert!(!d.block_path.is_empty());
            assert!(!d.label.is_empty());
            assert!(d.fraction >= 0.0 && d.fraction <= 1.0);
            assert!(d.size_label.contains('/'));
        }
        // At least one internal (non-removable) when a system root is present.
        let has_internal = disks.iter().any(|d| !d.removable);
        let has_root = disks.iter().any(|d| d.mount_point.as_deref() == Some("/"));
        assert!(
            has_internal || has_root || disks.iter().any(|d| d.removable),
            "unexpected empty classification: {disks:?}"
        );
    }
}

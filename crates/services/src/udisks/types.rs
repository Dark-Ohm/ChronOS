//! Disk inventory types for the udisks2 subscriber.

/// One filesystem-bearing block device (partition or whole-disk FS).
#[derive(Clone, Debug, PartialEq)]
pub struct DiskInfo {
    /// Display name: IdLabel, else device basename (`sdb1`).
    pub label: String,
    /// Human usage line: `"318G / 512G"`. Unmounted → `"— / 512G"`.
    pub size_label: String,
    /// Used / total in `[0.0, 1.0]`. Unmounted → `0.0`.
    pub fraction: f32,
    /// From `Drive.Removable` (USB/hotplug). Internal disks are `false`.
    pub removable: bool,
    /// Drive can be safely removed: `Ejectable` or `CanPowerOff`.
    pub ejectable: bool,
    /// Primary mount path if mounted.
    pub mount_point: Option<String>,
    /// D-Bus object path of the block device (`…/block_devices/sdb1`).
    pub block_path: String,
    /// D-Bus object path of the parent drive, if any.
    pub drive_path: Option<String>,
}

/// Fire-and-forget disk actions (mounted on `DisksSubscriber::dispatch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisksCommand {
    Mount { block_path: String },
    Unmount { block_path: String },
    /// Safe-remove: `Drive.Eject` when ejectable, else `Drive.PowerOff`.
    Eject { drive_path: String },
}

/// Format byte count as a short human label (`512G`, `32M`, `4.0K`).
///
/// Uses 1024-base (same as `df -h` on this host). Values ≥ 10 units drop
/// the fractional part; smaller ones keep one decimal when needed for
/// readability under 10.
pub fn format_bytes(n: u64) -> String {
    const K: f64 = 1024.0;
    let n = n as f64;
    if n >= K * K * K * K {
        fmt_unit(n / (K * K * K * K), 'T')
    } else if n >= K * K * K {
        fmt_unit(n / (K * K * K), 'G')
    } else if n >= K * K {
        fmt_unit(n / (K * K), 'M')
    } else if n >= K {
        fmt_unit(n / K, 'K')
    } else {
        format!("{n:.0}B")
    }
}

fn fmt_unit(v: f64, unit: char) -> String {
    if v >= 10.0 {
        format!("{v:.0}{unit}")
    } else {
        // Trim trailing ".0" for clean mockup-style labels.
        let s = format!("{v:.1}");
        if s.ends_with(".0") {
            format!("{}{unit}", &s[..s.len() - 2])
        } else {
            format!("{s}{unit}")
        }
    }
}

/// `"used / total"` size line. Unmounted (no used sample) → `"— / total"`.
pub fn size_label(used: Option<u64>, total: u64) -> String {
    match used {
        Some(u) => format!("{} / {}", format_bytes(u), format_bytes(total)),
        None => format!("— / {}", format_bytes(total)),
    }
}

/// Used/total clamped to `[0.0, 1.0]`. Zero total → `0.0`.
pub fn usage_fraction(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        ((used as f64) / (total as f64)).clamp(0.0, 1.0) as f32
    }
}

/// Prefer `/`, else the shortest mount path (stable, human-readable).
pub fn pick_mount_point(mounts: &[String]) -> Option<&str> {
    if mounts.is_empty() {
        return None;
    }
    if let Some(root) = mounts.iter().find(|m| m.as_str() == "/") {
        return Some(root.as_str());
    }
    mounts.iter().min_by_key(|m| m.len()).map(String::as_str)
}

/// Decode udisks2 `ay` (null-terminated byte path) → UTF-8 string.
pub fn decode_ay(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Decode `aay` MountPoints.
pub fn decode_mount_points(raw: &[Vec<u8>]) -> Vec<String> {
    raw.iter()
        .map(|b| decode_ay(b))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Device basename from `/dev/sdb1` → `sdb1`.
pub fn device_basename(preferred: &str) -> String {
    preferred
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(preferred)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_common_sizes() {
        assert_eq!(format_bytes(0), "0B");
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(1024), "1K");
        assert_eq!(format_bytes(32 * 1024 * 1024), "32M");
        assert_eq!(format_bytes(512 * 1024 * 1024 * 1024), "512G");
        // ~465.8 GiB USB
        assert_eq!(format_bytes(500_073_238_528), "466G");
    }

    #[test]
    fn size_label_mounted_and_unmounted() {
        assert_eq!(
            size_label(Some(318 * 1024 * 1024 * 1024), 512 * 1024 * 1024 * 1024),
            "318G / 512G"
        );
        assert_eq!(size_label(None, 64 * 1024 * 1024 * 1024), "— / 64G");
    }

    #[test]
    fn usage_fraction_clamps() {
        assert!((usage_fraction(50, 100) - 0.5).abs() < f32::EPSILON);
        assert_eq!(usage_fraction(0, 0), 0.0);
        assert_eq!(usage_fraction(200, 100), 1.0);
    }

    #[test]
    fn pick_mount_point_prefers_root() {
        let mps = vec![
            "/home".into(),
            "/".into(),
            "/var/tmp".into(),
        ];
        assert_eq!(pick_mount_point(&mps), Some("/"));
    }

    #[test]
    fn pick_mount_point_shortest_when_no_root() {
        let mps = vec![
            "/run/media/neo/Ventoy".into(),
            "/run/media/neo/VTOYEFI".into(),
        ];
        // both longer; shortest wins (VTOYEFI path is longer actually)
        // /run/media/neo/Ventoy = 22, VTOYEFI = 23
        assert_eq!(pick_mount_point(&mps), Some("/run/media/neo/Ventoy"));
        assert_eq!(pick_mount_point(&[]), None);
    }

    #[test]
    fn decode_ay_and_mount_points() {
        assert_eq!(decode_ay(b"/dev/sdb1\0"), "/dev/sdb1");
        assert_eq!(decode_ay(b"/boot"), "/boot");
        let raw = vec![
            b"/run/media/neo/Ventoy\0".to_vec(),
            b"/\0".to_vec(),
        ];
        assert_eq!(
            decode_mount_points(&raw),
            vec!["/run/media/neo/Ventoy", "/"]
        );
    }

    #[test]
    fn device_basename_strips_dev() {
        assert_eq!(device_basename("/dev/sdb1"), "sdb1");
        assert_eq!(device_basename("sdb1"), "sdb1");
    }
}

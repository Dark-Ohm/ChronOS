//! System info provider (`i:`): read-only hostname / kernel / compositor.

use super::{ProviderAction, ProviderResult};

pub fn results() -> Vec<ProviderResult> {
    vec![
        row("hostname", hostname()),
        row("kernel", kernel()),
        row("compositor", compositor()),
    ]
}

fn row(id: &str, value: String) -> ProviderResult {
    ProviderResult {
        id: format!("sys-{id}"),
        label: value,
        detail: Some(id.to_string()),
        glyph: 'ℹ',
        action: ProviderAction::None,
    }
}

fn hostname() -> String {
    read_trimmed("/proc/sys/kernel/hostname").unwrap_or_else(|| "unknown".into())
}

fn kernel() -> String {
    read_trimmed("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".into())
}

/// Compositor is detected from the environment the same way the compositor
/// service does (Niri is scaffold-only in the tree, but the env check is kept
/// for symmetry).
fn compositor() -> String {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        "Hyprland".into()
    } else if std::env::var_os("NIRI_SOCKET").is_some() {
        "Niri".into()
    } else {
        "unknown".into()
    }
}

fn read_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

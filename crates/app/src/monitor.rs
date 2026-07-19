//! Pult (control) monitor selection — single source of truth for chrome display.
//!
//! Config: `~/.config/chronos/monitor.toml`
//! ```toml
//! chrome_monitor = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
//! ```
//!
//! Fallback: largest display by area. Auto-designates on first run.

use std::path::PathBuf;

use gpui::{App, DisplayId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct MonitorConfig {
    chrome_monitor: Option<String>,
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/monitor.toml")
}

fn load_config() -> MonitorConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => MonitorConfig {
            chrome_monitor: None,
        },
    }
}

fn save_config(config: &MonitorConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!("monitor: mkdir {} failed: {e}", parent.display());
            return;
        }
    }
    match toml::to_string_pretty(config) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                tracing::warn!("monitor: write {} failed: {e}", path.display());
            }
        }
        Err(e) => tracing::warn!("monitor: serialize config failed: {e}"),
    }
}

/// DisplayId пультового монитора (chrome).
///
/// Resolution order:
/// 1. `monitor.toml` uuid matches a live display → use it
/// 2. Fallback: largest display by area
/// 3. Fallback: first display
/// 4. None only if no displays at all
pub fn pult_display(cx: &App) -> Option<DisplayId> {
    let displays = cx.displays();
    if displays.is_empty() {
        return None;
    }

    let cfg = load_config();

    // Try uuid match from config.
    if let Some(ref expected_uuid) = cfg.chrome_monitor {
        for d in &displays {
            if let Ok(uuid) = d.uuid() {
                if uuid.to_string() == *expected_uuid {
                    return Some(d.id());
                }
            }
        }
        tracing::warn!(
            "monitor: configured uuid {} not found among {} displays, using fallback",
            expected_uuid,
            displays.len()
        );
    }

    // Fallback: largest display by area.
    let mut best = &displays[0];
    let mut best_area = 0.0;
    for d in &displays {
        let bounds = d.bounds();
        let area = f64::from(bounds.size.width) * f64::from(bounds.size.height);
        if area > best_area {
            best_area = area;
            best = d;
        }
    }

    // Auto-designate: write the winning uuid to config.
    if let Ok(uuid) = best.uuid() {
        let uuid_str = uuid.to_string();
        if cfg.chrome_monitor.as_deref() != Some(&uuid_str) {
            tracing::info!("monitor: auto-designating {} as pult display", uuid_str);
            save_config(&MonitorConfig {
                chrome_monitor: Some(uuid_str),
            });
        }
    }

    Some(best.id())
}

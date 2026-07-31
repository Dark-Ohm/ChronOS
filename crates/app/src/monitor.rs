//! Pult (control) monitor selection — single source of truth for chrome display.
//!
//! Config: `~/.config/chronos/monitor.toml`
//! ```toml
//! chrome_monitor = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
//! ```
//!
//! Fallback: largest display by area. Auto-designates on first run.
//!
//! Hotplug: periodic check detects when the configured display disappears
//! and surfaces a notification via the existing notification service.
//! Shell continues on fallback display; scene state is preserved.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, DisplayId, PlatformDisplay};
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

/// Pure function: find the index of the display with the largest area.
///
/// Takes an iterator of `(width, height)` pairs and returns the index of
/// the one with the greatest product. Returns `None` for an empty iterator.
/// Tie-breaking: first occurrence wins (stable).
pub fn largest_display_index(
    dimensions: impl Iterator<Item = (f64, f64)>,
) -> Option<usize> {
    // Uses `reduce` with `>` (strictly greater) to match the original
    // `pult_display` behavior: first occurrence wins on equal areas.
    dimensions
        .enumerate()
        .reduce(|acc, item| {
            let (_, (w1, h1)) = acc;
            let (_, (w2, h2)) = item;
            if w2 * h2 > w1 * h1 { item } else { acc }
        })
        .map(|(i, _)| i)
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
    let best_idx = largest_display_index(
        displays.iter().map(|d| {
            let b = d.bounds();
            (f64::from(b.size.width), f64::from(b.size.height))
        }),
    )
    .unwrap_or(0);
    let best = &displays[best_idx];

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

/// Resolved pult display as a `PlatformDisplay` object.
///
/// Full fallback chain: configured UUID → largest by area → first →
/// `cx.primary_display()`. Use this instead of manual
/// `find_display(id).or_else(|| primary_display())`.
pub fn pult_display_info(cx: &App) -> Option<Rc<dyn PlatformDisplay>> {
    pult_display(cx)
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
}

/// Start periodic hotplug watcher.
///
/// Checks every 3 seconds if the configured display is still present.
/// When it disappears: logs a warning and shows a notification.
/// When it reappears: logs info and shows a notification.
fn start_hotplug_watcher(cx: &mut App) {
    let cfg = load_config();
    let Some(uuid) = cfg.chrome_monitor else {
        tracing::info!("monitor: no configured display, hotplug watcher not started");
        return;
    };

    cx.spawn(async move |cx| {
        let mut was_present = true;
        loop {
            cx.background_executor()
                .timer(Duration::from_secs(3))
                .await;

            let is_present = cx.update(|cx: &mut App| {
                cx.displays().iter().any(|d| {
                    d.uuid()
                        .ok()
                        .map(|u| u.to_string() == uuid)
                        .unwrap_or(false)
                })
            });

            if was_present && !is_present {
                tracing::warn!(
                    "monitor: configured display {} disconnected, shell using fallback",
                    uuid
                );
                let _ = cx.update(|cx: &mut App| {
                    crate::notifications::push_internal(
                        cx,
                        "Display disconnected",
                        &format!(
                            "Display {}… disconnected. Shell on fallback.",
                            &uuid[..8.min(uuid.len())]
                        ),
                    );
                });
            } else if !was_present && is_present {
                tracing::info!("monitor: configured display {} reconnected", uuid);
                let _ = cx.update(|cx: &mut App| {
                    crate::notifications::push_internal(
                        cx,
                        "Display reconnected",
                        &format!("Display {}… is back.", &uuid[..8.min(uuid.len())]),
                    );
                });
            }

            was_present = is_present;
        }
    })
    .detach();
}

/// Initialize the monitor module. Call once at startup from `main.rs`.
pub fn init(cx: &mut App) {
    start_hotplug_watcher(cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largest_display_index_empty() {
        assert_eq!(largest_display_index(std::iter::empty()), None);
    }

    #[test]
    fn largest_display_index_single() {
        assert_eq!(
            largest_display_index([(1920.0, 1080.0)].into_iter()),
            Some(0)
        );
    }

    #[test]
    fn largest_display_index_picks_largest() {
        let dims = [(1920.0, 1080.0), (2560.0, 1440.0), (1280.0, 720.0)];
        assert_eq!(largest_display_index(dims.into_iter()), Some(1));
    }

    #[test]
    fn largest_display_index_equal_areas_first_wins() {
        let dims = [(1920.0, 1080.0), (1080.0, 1920.0)];
        assert_eq!(largest_display_index(dims.into_iter()), Some(0));
    }

    #[test]
    fn largest_display_index_first_is_largest() {
        let dims = [(3840.0, 2160.0), (1920.0, 1080.0)];
        assert_eq!(largest_display_index(dims.into_iter()), Some(0));
    }
}

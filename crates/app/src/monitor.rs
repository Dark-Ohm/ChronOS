//! Pult (control) monitor selection — single source of truth for chrome display.
//!
//! Config: `~/.config/chronos/monitor.toml`
//! ```toml
//! chrome_monitor = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
//! ```
//!
//! Fallback: largest display by area. Auto-designates on first run (writes
//! the winning uuid to the config so subsequent boots are deterministic).
//!
//! Resolution layers (single source of truth — surfaces must never call
//! `cx.primary_display()` directly):
//!
//! 1. `pult_display_id_or_primary(cx)` — flat `Option<DisplayId>`,
//!    surfaces pass this to `WindowOptions.display_id`. Used by the 8
//!    chrome surfaces (bar / both side panels / hover strips / dock menu /
//!    desktop terminal).
//! 2. `pult_display_info(cx)` — same chain, returns
//!    `Option<Rc<dyn PlatformDisplay>>` for callers that want the full
//!    object (e.g. to read bounds without a second `find_display`).
//!
//! Both honour the same chain:
//!   configured uuid → largest by area → `cx.primary_display()`.
//!
//! Hotplug: periodic check detects when the configured display disappears
//! and surfaces a notification via the existing notification service.
//! Shell continues on fallback display; scene state is preserved.
//!
//! Stability (T245, 2026-08-05): `uuid` is STABLE. The fork derives it as
//! `UUIDv5(NAMESPACE_DNS, wl_output name)`, i.e. a pure function of the
//! connector name ("DP-1"/"HDMI-A-1" on Hyprland) — NOT a session-order
//! or hotplug-sequence index. The 2026-08-05 incident (shell permanently
//! on the smaller HDMI-A-1) was an auto-designate ratchet: a transiently
//! absent DP-1 (DPMS sleep at night / late `wl_output` Done past the
//! 100 ms bar-init call) made the largest-area fallback pick HDMI-A-1 and
//! the then-current code rewrote the config to it forever. Fixed by gating
//! auto-designate to a true first run — an existing config is
//! authoritative and never overwritten. To change the pult display, edit
//! `monitor.toml`; delete it to re-designate on the next boot.

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

/// Pure gate: may `pult_display` write a new designation to `monitor.toml`?
///
/// Only a true first run (no configured uuid) may auto-designate. Once a
/// uuid exists it is the source of truth; a transiently incomplete display
/// set must never overwrite it (see module docs — T245 ratchet).
fn should_auto_designate(existing: Option<&str>, winner_uuid: Option<&str>) -> bool {
    existing.is_none() && winner_uuid.is_some()
}

/// Pure resolver: pick the index of the chrome monitor from a precomputed
/// summary list. The caller owns `cx.displays()` and pre-extracts uuid +
/// area; this function only does the decision, no side effects.
///
/// Chain:
///   1. uuid match → that display's index
///   2. → largest by area (via [`largest_display_index`])
///
/// Empty input always returns `None`, even when a `configured_uuid` is
/// supplied — there is simply nothing to address.
fn resolve_pult_index(
    displays: &[(Option<&str>, f64)],
    configured_uuid: Option<&str>,
) -> Option<usize> {
    if let Some(expected) = configured_uuid {
        if let Some(idx) = displays.iter().position(|(uuid, _)| {
            uuid.map(|u| u == expected).unwrap_or(false)
        }) {
            return Some(idx);
        }
    }
    largest_display_index(displays.iter().map(|(_, area)| (1.0, *area)))
}

/// DisplayId пультового монитора (chrome).
///
/// Resolution order:
/// 1. `monitor.toml` uuid matches a live display → use it
/// 2. Fallback: largest display by area (auto-designate on first run)
/// 3. `None` only if no displays at all
///
/// On first run (or any run where the configured uuid is missing), the
/// largest-area winner is written to `monitor.toml` so subsequent boots
/// are deterministic. This is an intentional side effect, not a bug:
/// "auto-designates on first run" per spec §3.6. Once configured, every
/// later call goes through step 1 and returns without side effects.
pub fn pult_display(cx: &App) -> Option<DisplayId> {
    let displays = cx.displays();
    if displays.is_empty() {
        return None;
    }

    let cfg = load_config();

    // Build a slim summary of every display: (uuid_str_opt, width*height).
    // This decouples the decision logic from the live `cx` — see
    // [`resolve_pult_index`] for the pure half.
    let summaries: Vec<(Option<String>, f64)> = displays
        .iter()
        .map(|d| {
            let uuid = d.uuid().ok().map(|u| u.to_string());
            let b = d.bounds();
            let area = f64::from(b.size.width) * f64::from(b.size.height);
            (uuid, area)
        })
        .collect();
    let summary_refs: Vec<(Option<&str>, f64)> = summaries
        .iter()
        .map(|(u, a)| (u.as_deref(), *a))
        .collect();

    for (i, (d, (uuid, area))) in displays.iter().zip(&summaries).enumerate() {
        tracing::debug!(
            "monitor: display[{i}] id={:?} uuid={uuid:?} bounds={:?} area={area}",
            d.id(),
            d.bounds(),
        );
    }

    let Some(idx) = resolve_pult_index(&summary_refs, cfg.chrome_monitor.as_deref()) else {
        return None;
    };
    let best = &displays[idx];
    let best_uuid_str = best.uuid().ok().map(|u| u.to_string());

    // How was the winner chosen? Distinguishes "no config yet" (first run)
    // from "configured uuid present but not live" — both fall back to
    // largest-by-area, but only the latter deserves a warning.
    let configured_expected = cfg.chrome_monitor.as_deref();
    let matched_configured = configured_expected
        .is_some_and(|expected| best_uuid_str.as_deref() == Some(expected));
    let resolution = match configured_expected {
        Some(_) if matched_configured => "configured-uuid",
        Some(_) => "fallback-config-mismatch",
        None => "fallback-no-config",
    };
    tracing::info!(
        "monitor: pult display resolved id={:?} uuid={:?} area={} via {} ({} live displays)",
        best.id(),
        best_uuid_str,
        summaries[idx].1,
        resolution,
        displays.len()
    );

    // Surface a single warning when we expected a specific uuid but
    // couldn't find it (helps debug "shell landed on the wrong monitor").
    if let Some(expected) = configured_expected {
        if !matched_configured {
            tracing::warn!(
                "monitor: configured uuid {} not found among {} displays, using fallback",
                expected,
                displays.len()
            );
        }
    }

    // Auto-designate: write the winner to config ONLY on a true first run
    // (no config yet). Once a uuid is configured it is authoritative and is
    // never overwritten — a transiently incomplete display set must not
    // ratchet the shell onto the wrong monitor (T245: config flipped to
    // HDMI-A-1 at 02:52 when DP-1 was momentarily absent).
    if let Some(uuid_str) = best_uuid_str {
        if should_auto_designate(cfg.chrome_monitor.as_deref(), Some(uuid_str.as_str())) {
            tracing::info!(
                "monitor: first run — auto-designating {} as pult display",
                uuid_str
            );
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
/// `cx.primary_display()`. Use this when the caller needs the full object
/// (e.g. to read bounds without a second `find_display`).
pub fn pult_display_info(cx: &App) -> Option<Rc<dyn PlatformDisplay>> {
    pult_display(cx)
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
}

/// Flat `DisplayId` helper with the same fallback chain as
/// [`pult_display_info`]. Surfaces that just need an id for
/// `WindowOptions.display_id` use this — no `find_display` or
/// `primary_display` boilerplate per surface.
pub fn pult_display_id_or_primary(cx: &App) -> Option<DisplayId> {
    pult_display_info(cx).map(|d| d.id())
}

/// Periodic hotplug watcher.
///
/// Runs for the lifetime of the shell. Every 3 seconds:
///   1. Re-reads `monitor.toml` so a fresh machine picks up the
///      auto-designated uuid within ≤3 s of `bar::init` writing it.
///   2. If a uuid is configured, checks whether that display is still
///      present in `cx.displays()`.
///   3. On a present → absent transition: warns + pushes a notification.
///   4. On an absent → present transition: infos + pushes a notification.
///
/// Starts unconditionally — `monitor::init` is called *before* `bar::init`
/// (see `main.rs`), and the only writes to the config happen as a
/// side-effect of `pult_display`/`bar::init`. So on a fresh machine the
/// first few ticks see no watched uuid and quietly loop; the moment the
/// config is populated, monitoring begins. This is correct and avoids
/// the race where the watcher bails before its job exists.
fn start_hotplug_watcher(cx: &mut App) {
    cx.spawn(async move |cx| {
        let mut last_present: Option<bool> = None;
        loop {
            cx.background_executor()
                .timer(Duration::from_secs(3))
                .await;

            // Re-read cfg each tick so a fresh install picks up the
            // auto-designation from `bar::init` within ~3 s.
            let cfg = load_config();
            let Some(uuid) = cfg.chrome_monitor.clone() else {
                // No config yet — keep polling. Don't reset `last_present`
                // (we never sampled), so the first uuid-match tick won't
                // fire a spurious "reconnected" notification either way.
                continue;
            };

            let is_present = cx.update(|cx: &mut App| {
                cx.displays().iter().any(|d| {
                    d.uuid()
                        .ok()
                        .map(|u| u.to_string() == uuid)
                        .unwrap_or(false)
                })
            });

            if last_present == Some(is_present) {
                continue;
            }

            // Only notify on *real* transitions. The first sample is not a
            // transition (`last_present: None` is "haven't sampled yet");
            // firing a "Display reconnected" toast on every shell start
            // would be user-visible noise (caught by T166 live boot log,
            // 2026-07-31: spurious info on cold start).
            match last_present {
                Some(_prev) if _prev != is_present => {
                    if !is_present {
                        tracing::warn!(
                            "monitor: configured display {} disconnected, shell using fallback",
                            uuid
                        );
                        let preview = uuid.chars().take(8).collect::<String>();
                        cx.update(|cx: &mut App| {
                            crate::notifications::push_internal(
                                cx,
                                "Display disconnected",
                                &format!(
                                    "Display {}… disconnected. Shell on fallback.",
                                    preview
                                ),
                            );
                        });
                    } else {
                        tracing::info!(
                            "monitor: configured display {} reconnected",
                            uuid
                        );
                        let preview = uuid.chars().take(8).collect::<String>();
                        cx.update(|cx: &mut App| {
                            crate::notifications::push_internal(
                                cx,
                                "Display reconnected",
                                &format!("Display {}… is back.", preview),
                            );
                        });
                    }
                }
                _ => {}
            }

            last_present = Some(is_present);
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

    // -------- resolve_pult_index --------

    /// Build a summaries slice that borrows directly from the input — no
    /// copies, no `Box::leak`, no allocation. The returned Vec lives as
    /// long as both inputs; tests construct them with literal arrays so
    /// the lifetime is the test body.
    fn make_displays<'a>(
        uuids: &'a [&'a str],
        areas: &'a [f64],
    ) -> Vec<(Option<&'a str>, f64)> {
        uuids
            .iter()
            .zip(areas.iter().copied())
            .map(|(u, a)| (Some(*u), a))
            .collect()
    }

    // -------- should_auto_designate --------

    #[test]
    fn should_auto_designate_only_on_first_run() {
        // No config + resolvable winner: first run, may designate.
        assert!(should_auto_designate(None, Some("winner")));
        // No config + no uuid: nothing to write.
        assert!(!should_auto_designate(None, None));
        // Existing config is authoritative — never rewritten, even if the
        // live winner differs (T245 ratchet: HDMI-A-1 over DP-1).
        assert!(!should_auto_designate(Some("dp-1"), Some("hdmi-a-1")));
        assert!(!should_auto_designate(Some("dp-1"), Some("dp-1")));
        assert!(!should_auto_designate(Some("dp-1"), None));
    }

    #[test]
    fn resolve_pult_index_empty_even_with_config() {
        assert_eq!(resolve_pult_index(&[], None), None);
        assert_eq!(resolve_pult_index(&[], Some("anything")), None);
    }

    #[test]
    fn resolve_pult_index_no_config_picks_largest() {
        let d = make_displays(
            &["a", "b", "c"],
            &[1920.0 * 1080.0, 2560.0 * 1440.0, 1280.0 * 720.0],
        );
        assert_eq!(resolve_pult_index(&d, None), Some(1));
    }

    #[test]
    fn resolve_pult_index_uuid_match_wins_regardless_of_area() {
        let d = make_displays(
            &["small", "big"],
            &[1280.0 * 720.0, 3840.0 * 2160.0],
        );
        assert_eq!(resolve_pult_index(&d, Some("small")), Some(0));
        assert_eq!(resolve_pult_index(&d, Some("big")), Some(1));
    }

    #[test]
    fn resolve_pult_index_uuid_missing_falls_back_to_largest() {
        let d = make_displays(
            &["a", "b"],
            &[1920.0 * 1080.0, 2560.0 * 1440.0],
        );
        assert_eq!(resolve_pult_index(&d, Some("never-seen")), Some(1));
    }

    #[test]
    fn resolve_pult_index_skips_displays_without_uuid() {
        // Display with no uuid can't match a configured uuid; it can still
        // win as the largest fallback.
        let d: Vec<(Option<&str>, f64)> = vec![
            (None, 100.0),
            (Some("real"), 1920.0 * 1080.0),
        ];
        assert_eq!(resolve_pult_index(&d, Some("real")), Some(1));
        assert_eq!(resolve_pult_index(&d, None), Some(1));
    }

    #[test]
    fn resolve_pult_index_equal_areas_first_wins() {
        let d: Vec<(Option<&str>, f64)> = vec![
            (Some("a"), 1920.0 * 1080.0),
            (Some("b"), 1080.0 * 1920.0),
        ];
        assert_eq!(resolve_pult_index(&d, Some("a")), Some(0));
        assert_eq!(resolve_pult_index(&d, None), Some(0));
    }
}

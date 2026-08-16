//! Frecency (frequency × recency) store for the launcher.
//!
//! T275 Часть C: the launcher must surface recently/frequently launched apps
//! on top when the query is empty, and use frecency as a secondary sort key
//! when the user is actually typing (nucleo relevance stays primary).
//!
//! Storage: `~/.config/chronos/frecency.toml` (TOML, like the dock config),
//! held in a process-global cache. Launches update the in-memory cache
//! immediately; disk writes are coalesced (at most one write per
//! `SAVE_DEBOUNCE`) and flushed on shell shutdown — never one write per
//! launch (per T275).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::AppEntry;

/// Max one disk write per this window — batches rapid launches.
const SAVE_DEBOUNCE: Duration = Duration::from_secs(3);
/// Frecency recency half-life (days). A launch's weight halves every 7 days.
const HALF_LIFE_DAYS: f64 = 7.0;
const SECS_PER_DAY: f64 = 86_400.0;

/// One app's launch history.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FrecencyEntry {
    pub launch_count: u32,
    /// Unix timestamp (seconds) of the most recent launch.
    pub last_launched_at: i64,
}

/// Serializable payload — only the history map is persisted.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct FrecencyData {
    pub entries: HashMap<String, FrecencyEntry>,
}

struct Store {
    data: FrecencyData,
    dirty: bool,
    last_save: Instant,
}

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();

fn store() -> &'static Mutex<Store> {
    STORE.get_or_init(|| Mutex::new(load()))
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos/frecency.toml")
}

fn load() -> Store {
    let path = config_path();
    let data = match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => FrecencyData::default(),
    };
    Store {
        data,
        dirty: false,
        last_save: Instant::now(),
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Current unix timestamp (seconds) — exposed so callers rank against a
/// single consistent "now" within one refresh.
pub fn now() -> i64 {
    now_secs()
}

fn save_locked(data: &FrecencyData) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(data).expect("FrecencyData is always serializable");
    std::fs::write(path, content)
}

/// Record a launch: bump count, refresh timestamp, persist coalesced.
pub fn record_launch(id: &str) {
    if id.is_empty() {
        return;
    }
    let mut s = store().lock().unwrap();
    let entry = s.data.entries.entry(id.to_string()).or_default();
    entry.launch_count += 1;
    entry.last_launched_at = now_secs();
    s.dirty = true;
    // Coalesce: only touch disk if the last write was long enough ago.
    if s.last_save.elapsed() >= SAVE_DEBOUNCE {
        if let Err(err) = save_locked(&s.data) {
            tracing::warn!("frecency: failed to save store: {err}");
        }
        s.dirty = false;
        s.last_save = Instant::now();
    }
}

/// Force a persist — call on shell shutdown so the last coalesce window
/// isn't lost.
pub fn flush() {
    let mut s = store().lock().unwrap();
    if s.dirty {
        if let Err(err) = save_locked(&s.data) {
            tracing::warn!("frecency: failed to flush store: {err}");
        }
        s.dirty = false;
        s.last_save = Instant::now();
    }
}

/// Snapshot of the history map for ranking.
pub fn cached() -> FrecencyData {
    store().lock().unwrap().data.clone()
}

/// Pure frecency weight: `launch_count × 2^(−age_days / HALF_LIFE_DAYS)`
/// (exponential recency decay, 7-day half-life). Recent launches dominate
/// a stale-but-frequent history.
pub fn score(entry: &FrecencyEntry, now: i64) -> f64 {
    let age_days = (now - entry.last_launched_at).max(0) as f64 / SECS_PER_DAY;
    entry.launch_count as f64 * 2f64.powf(-age_days / HALF_LIFE_DAYS)
}

/// Rank entries for the launcher.
///
/// Each entry arrives with its nucleo relevance `score` (from
/// `FuzzySearch::results`) so frecency can act as a *secondary* key without
/// ever inverting relevance:
///
/// * empty query → nucleo has no signal, so frecency is the **primary** order
///   (recently/frequently launched on top), name as a stable tie-break.
/// * non-empty query → nucleo relevance score is the **primary** key,
///   frecency the **secondary** tie-breaker. A frequent-but-unrelated app can
///   therefore never jump above the one the user actually typed.
pub fn rank(
    mut entries: Vec<(AppEntry, f32)>,
    pattern: &str,
    data: &FrecencyData,
    now: i64,
) -> Vec<AppEntry> {
    let weight = |id: &str| -> f64 {
        data.entries
            .get(id)
            .map(|e| score(e, now))
            .unwrap_or(0.0)
    };

    if pattern.trim().is_empty() {
        entries.sort_by(|a, b| {
            weight(&b.0.id)
                .partial_cmp(&weight(&a.0.id))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.name.cmp(&b.0.name))
        });
    } else {
        entries.sort_by(|a, b| {
            // Primary: nucleo relevance (higher score first).
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                // Secondary tie-break: frecency.
                .then_with(|| {
                    weight(&b.0.id)
                        .partial_cmp(&weight(&a.0.id))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
    }
    entries.into_iter().map(|(e, _)| e).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `(AppEntry, score)` pair for ranking tests.
    fn e(id: &str, name: &str, score: f32) -> (AppEntry, f32) {
        (
            AppEntry {
                id: id.into(),
                name: name.into(),
                exec: "x".into(),
                ..AppEntry::default()
            },
            score,
        )
    }

    #[test]
    fn recent_beats_frequent() {
        // A: 10 launches a month ago. B: 2 launches yesterday.
        // Recency must dominate, so B ranks above A.
        let day = 86_400i64;
        let now = 1_700_000_000i64;
        let mut data = FrecencyData::default();
        data.entries.insert(
            "frequent".into(),
            FrecencyEntry {
                launch_count: 10,
                last_launched_at: now - 30 * day,
            },
        );
        data.entries.insert(
            "recent".into(),
            FrecencyEntry {
                launch_count: 2,
                last_launched_at: now - 1 * day,
            },
        );

        let sa = score(&data.entries["frequent"], now);
        let sb = score(&data.entries["recent"], now);
        assert!(
            sb > sa,
            "recency must beat stale frequency: recent={sb:.3} frequent={sa:.3}"
        );
    }

    #[test]
    fn empty_query_sorts_by_frecency() {
        let day = 86_400i64;
        let now = 1_700_000_000i64;
        let mut data = FrecencyData::default();
        data.entries.insert(
            "old".into(),
            FrecencyEntry {
                launch_count: 10,
                last_launched_at: now - 30 * day,
            },
        );
        data.entries.insert(
            "new".into(),
            FrecencyEntry {
                launch_count: 2,
                last_launched_at: now - 1 * day,
            },
        );

        // Scores are irrelevant on an empty query (nucleo has no signal).
        let entries = vec![e("old", "Old", 0.0), e("new", "New", 0.0)];

        let ranked = rank(entries, "", &data, now);
        assert_eq!(ranked[0].id, "new", "recently launched must be first when query is empty");
    }

    #[test]
    fn nonempty_query_keeps_relevance_primary() {
        // "relevant" has a far higher nucleo score than "boring" even though
        // "boring" was launched 100× more recently. A typed query must keep
        // the relevance-led order; frecency must NEVER invert it.
        let now = 1_700_000_000i64;
        let mut data = FrecencyData::default();
        data.entries.insert(
            "boring".into(),
            FrecencyEntry {
                launch_count: 100,
                last_launched_at: now,
            },
        );
        data.entries.insert(
            "relevant".into(),
            FrecencyEntry {
                launch_count: 1,
                last_launched_at: now - 100 * 86_400,
            },
        );

        // Relevance order: relevant first (higher score), boring second.
        let entries = vec![e("relevant", "Relevant", 100.0), e("boring", "Boring", 1.0)];

        let ranked = rank(entries, "rel", &data, now);
        assert_eq!(
            ranked[0].id, "relevant",
            "typed query must keep nucleo relevance primary; frecency must not invert it"
        );
        assert_eq!(
            ranked[1].id, "boring",
            "the less-relevant but frequent app must stay below the typed match"
        );
    }

    #[test]
    fn frecency_breaks_tie_on_equal_relevance() {
        // Both apps share the same nucleo score, so frecency decides — the
        // freshly-launched one must win the tie.
        let day = 86_400i64;
        let now = 1_700_000_000i64;
        let mut data = FrecencyData::default();
        data.entries.insert(
            "stale".into(),
            FrecencyEntry {
                launch_count: 10,
                last_launched_at: now - 30 * day,
            },
        );
        data.entries.insert(
            "fresh".into(),
            FrecencyEntry {
                launch_count: 2,
                last_launched_at: now - 1 * day,
            },
        );

        let entries = vec![e("stale", "Stale", 50.0), e("fresh", "Fresh", 50.0)];
        let ranked = rank(entries, "x", &data, now);
        assert_eq!(
            ranked[0].id, "fresh",
            "with equal nucleo score, the more recently launched app must win the tie"
        );
    }

    #[test]
    fn score_is_zero_for_unknown_app() {
        let data = FrecencyData::default();
        assert_eq!(score(&FrecencyEntry::default(), 1_700_000_000), 0.0);
    }
}

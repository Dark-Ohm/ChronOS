use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};

use chronos_services::AppEntry;
use std::sync::Arc;

/// Search ranking tiers (T265-A). Lower = better:
/// exact Name > prefix Name > substring Name > GenericName/Comment/Keywords/Exec
/// > fuzzy.
const TIER_EXACT: u8 = 0;
const TIER_PREFIX: u8 = 1;
const TIER_SUBSTRING: u8 = 2;
const TIER_OTHER_FIELDS: u8 = 3;
const TIER_FUZZY: u8 = 4;

/// Number of tiers, used to encode the tier as the dominant part of the score.
const TIER_COUNT: u8 = 5;
/// Tier weight so a better tier can never be outranked by a lower one, no
/// matter how many items nucleo matched. `frecency::rank` uses this score as
/// its primary key on typed queries, so frecency can't promote a fuzzy match
/// above an exact Name either.
const TIER_STRIDE: f32 = 1000.0;

pub struct FuzzySearch {
    nucleo: Nucleo<u32>,
    items: Vec<AppEntry>,
    pattern: String,
}

impl FuzzySearch {
    pub fn new() -> Self {
        Self {
            nucleo: Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1),
            items: Vec::new(),
            pattern: String::new(),
        }
    }

    pub fn set_items(&mut self, entries: Vec<AppEntry>) {
        self.items = entries;
        self.nucleo.restart(true);
        for (i, entry) in self.items.iter().enumerate() {
            let haystack = haystack(entry);
            let _ = self.nucleo.injector().push(i as u32, move |_, cols| {
                cols[0] = Utf32String::from(haystack.as_str());
            });
        }
    }

    pub fn update_pattern(&mut self, pattern: &str) {
        self.pattern = pattern.to_string();
        self.nucleo
            .pattern
            .reparse(0, pattern, CaseMatching::Smart, Normalization::Never, false);
    }

    /// Return up to `max` matches as `(entry, score)`.
    ///
    /// The score is tier-encoded: `(TIER_COUNT - tier) * TIER_STRIDE` plus a
    /// within-tier component derived from nucleo's relevance order, so a
    /// strictly-better tier always scores higher than a worse one regardless of
    /// the within-tier component. `frecency::rank` consumes this score as its
    /// primary key on typed queries.
    pub fn results(&mut self, max: usize) -> Vec<(AppEntry, f32)> {
        self.nucleo.tick(10);

        let snapshot = self.nucleo.snapshot();
        let count = snapshot.matched_item_count() as usize;
        if count == 0 {
            return Vec::new();
        }

        // Nucleo yields candidates in its own relevance order; we re-rank them
        // over the snapshot by an explicit match-tier (T265-A), keeping
        // nucleo's order as the within-tier tie-break.
        let mut ranked: Vec<(AppEntry, u8, usize)> = snapshot
            .matched_items(0..count as u32)
            .enumerate()
            .filter_map(|(pos, item)| {
                let entry = self.items.get(*item.data as usize)?.clone();
                let tier = match_tier(&entry, &self.pattern);
                Some((entry, tier, pos))
            })
            .collect();

        ranked.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));

        let total = ranked.len();
        ranked
            .into_iter()
            .take(max)
            .map(|(entry, tier, pos)| {
                let score = (TIER_COUNT - tier) as f32 * TIER_STRIDE + (total - pos) as f32;
                (entry, score)
            })
            .collect()
    }
}

/// Searchable haystack: `name\0generic_name\0comment\0keywords...\0exec`.
/// The `\0` separators stop a match from spanning field boundaries.
fn haystack(entry: &AppEntry) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(4 + entry.keywords.len());
    parts.push(entry.name.as_str());
    if let Some(g) = entry.generic_name.as_deref() {
        parts.push(g);
    }
    if let Some(c) = entry.comment.as_deref() {
        parts.push(c);
    }
    parts.extend(entry.keywords.iter().map(|s| s.as_str()));
    parts.push(entry.exec.as_str());
    parts.join("\0")
}

/// Match tier for `pattern` against an entry's searchable fields (T265-A).
fn match_tier(entry: &AppEntry, pattern: &str) -> u8 {
    let pat = pattern.trim().to_lowercase();
    if pat.is_empty() {
        return TIER_FUZZY;
    }
    let name = entry.name.to_lowercase();
    if name == pat {
        return TIER_EXACT;
    }
    if name.starts_with(&pat) {
        return TIER_PREFIX;
    }
    if name.contains(&pat) {
        return TIER_SUBSTRING;
    }
    let in_other = entry
        .generic_name
        .as_deref()
        .map(|s| s.to_lowercase().contains(&pat))
        .unwrap_or(false)
        || entry
            .comment
            .as_deref()
            .map(|s| s.to_lowercase().contains(&pat))
            .unwrap_or(false)
        || entry.keywords.iter().any(|k| k.to_lowercase().contains(&pat))
        || entry.exec.to_lowercase().contains(&pat);
    if in_other {
        TIER_OTHER_FIELDS
    } else {
        TIER_FUZZY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::applications::frecency::{self, FrecencyData, FrecencyEntry};

    fn make_entries() -> Vec<AppEntry> {
        vec![
            AppEntry {
                exec: "/usr/bin/firefox".into(),
                ..AppEntry::fixture("firefox", "Firefox")
            },
            AppEntry {
                exec: "/usr/bin/thunderbird".into(),
                ..AppEntry::fixture("thunderbird", "Thunderbird")
            },
            AppEntry {
                exec: "/usr/bin/nautilus".into(),
                ..AppEntry::fixture("files", "Files")
            },
        ]
    }

    #[test]
    fn exact_match() {
        let mut search = FuzzySearch::new();
        search.set_items(make_entries());
        search.update_pattern("firefox");
        let results = search.results(10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "firefox");
    }

    #[test]
    fn fuzzy_match() {
        let mut search = FuzzySearch::new();
        search.set_items(make_entries());
        search.update_pattern("ffx");
        let results = search.results(10);
        assert!(results.iter().any(|e| e.0.id == "firefox"));
    }

    #[test]
    fn empty_pattern_returns_all() {
        let mut search = FuzzySearch::new();
        search.set_items(make_entries());
        search.update_pattern("");
        let results = search.results(10);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn no_match_returns_empty() {
        let mut search = FuzzySearch::new();
        search.set_items(make_entries());
        search.update_pattern("zzzzz");
        let results = search.results(10);
        assert!(results.is_empty());
    }

    #[test]
    fn keyword_match_finds_entry_by_keyword_not_name() {
        // The query matches only the `Keywords` field of the first entry; its
        // name ("Alacritty") has nothing in common with "terminal".
        let entries = vec![
            AppEntry {
                keywords: vec!["terminal".into(), "shell".into()],
                ..AppEntry::fixture("alacritty", "Alacritty")
            },
            AppEntry {
                keywords: vec!["music".into()],
                ..AppEntry::fixture("rhythmbox", "Rhythmbox")
            },
        ];
        let mut search = FuzzySearch::new();
        search.set_items(entries);
        search.update_pattern("terminal");
        let results = search.results(10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "alacritty");
    }

    #[test]
    fn comment_is_searchable() {
        let entries = vec![
            AppEntry {
                comment: Some("A fast terminal emulator".into()),
                ..AppEntry::fixture("alacritty", "Alacritty")
            },
            AppEntry::fixture("firefox", "Firefox"),
        ];
        let mut search = FuzzySearch::new();
        search.set_items(entries);
        search.update_pattern("emulator");
        let results = search.results(10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "alacritty");
    }

    #[test]
    fn exact_name_beats_fuzzy() {
        // "abc" is an exact Name for `plain`, but only a fuzzy subsequence
        // (skipping the spaces) for `spaced`.
        let entries = vec![
            AppEntry::fixture("spaced", "a b c"),
            AppEntry::fixture("plain", "abc"),
        ];
        let mut search = FuzzySearch::new();
        search.set_items(entries);
        search.update_pattern("abc");
        let results = search.results(10);
        assert_eq!(results[0].0.id, "plain", "exact Name must rank above fuzzy");
        assert_eq!(results[1].0.id, "spaced");
    }

    #[test]
    fn prefix_beats_substring() {
        // "fir" prefixes `firefox` but is only a substring of `zfire`.
        let entries = vec![
            AppEntry::fixture("zfire", "zfire"),
            AppEntry::fixture("firefox", "firefox"),
        ];
        let mut search = FuzzySearch::new();
        search.set_items(entries);
        search.update_pattern("fir");
        let results = search.results(10);
        assert_eq!(results[0].0.id, "firefox", "prefix must rank above substring");
    }

    #[test]
    fn substring_beats_other_fields() {
        // "term" is a substring of `terminal`, but only a keyword of `alacritty`.
        let entries = vec![
            AppEntry {
                keywords: vec!["term".into()],
                ..AppEntry::fixture("alacritty", "Alacritty")
            },
            AppEntry::fixture("terminal", "terminal"),
        ];
        let mut search = FuzzySearch::new();
        search.set_items(entries);
        search.update_pattern("term");
        let results = search.results(10);
        assert_eq!(results[0].0.id, "terminal", "substring of Name must beat keyword match");
    }

    #[test]
    fn frecency_does_not_override_exact_name() {
        // `term` is an exact Name; `firefox` is only a fuzzy match. Frecency
        // heavily favors firefox — it must still rank below the exact name.
        let entries = vec![
            AppEntry {
                exec: "/usr/bin/firefox".into(),
                ..AppEntry::fixture("firefox", "Firefox")
            },
            AppEntry::fixture("term", "term"),
        ];
        let mut search = FuzzySearch::new();
        search.set_items(entries);
        search.update_pattern("term");
        let raw = search.results(10);

        let now = frecency::now();
        let mut data = FrecencyData::default();
        data.entries.insert(
            "firefox".into(),
            FrecencyEntry {
                launch_count: 1000,
                last_launched_at: now,
            },
        );
        data.entries.insert(
            "term".into(),
            FrecencyEntry {
                launch_count: 1,
                last_launched_at: 0,
            },
        );

        let ranked = frecency::rank(raw, "term", &data, now);
        assert_eq!(
            ranked[0].id, "term",
            "frecency must not promote a fuzzy match above an exact Name"
        );
    }
}

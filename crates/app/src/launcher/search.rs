use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};

use super::entry::DesktopEntry;
use std::sync::Arc;

pub struct FuzzySearch {
    nucleo: Nucleo<u32>,
    items: Vec<DesktopEntry>,
}

impl FuzzySearch {
    pub fn new() -> Self {
        Self {
            nucleo: Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1),
            items: Vec::new(),
        }
    }

    pub fn set_items(&mut self, entries: Vec<DesktopEntry>) {
        self.items = entries;
        self.nucleo.restart(true);
        for (i, entry) in self.items.iter().enumerate() {
            let _ = self.nucleo.injector().push(i as u32, |_, cols| {
                cols[0] = Utf32String::from(entry.name.as_str());
            });
        }
    }

    pub fn update_pattern(&mut self, pattern: &str) {
        self.nucleo
            .pattern
            .reparse(0, pattern, CaseMatching::Smart, Normalization::Never, false);
    }

    pub fn results(&mut self, max: usize) -> Vec<&DesktopEntry> {
        self.nucleo.tick(10);

        let snapshot = self.nucleo.snapshot();
        let count = snapshot.matched_item_count() as usize;
        let max = max.min(count);
        let mut matched = Vec::new();
        for item in snapshot.matched_items(0..max as u32) {
            let idx = *item.data as usize;
            if let Some(entry) = self.items.get(idx) {
                matched.push(entry);
            }
        }
        matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entries() -> Vec<DesktopEntry> {
        vec![
            DesktopEntry {
                id: "firefox".into(),
                name: "Firefox".into(),
                exec: "/usr/bin/firefox".into(),
                icon: None,
                terminal: false,
                no_display: false,
            },
            DesktopEntry {
                id: "thunderbird".into(),
                name: "Thunderbird".into(),
                exec: "/usr/bin/thunderbird".into(),
                icon: None,
                terminal: false,
                no_display: false,
            },
            DesktopEntry {
                id: "files".into(),
                name: "Files".into(),
                exec: "/usr/bin/nautilus".into(),
                icon: None,
                terminal: false,
                no_display: false,
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
        assert_eq!(results[0].id, "firefox");
    }

    #[test]
    fn fuzzy_match() {
        let mut search = FuzzySearch::new();
        search.set_items(make_entries());
        search.update_pattern("ffx");
        let results = search.results(10);
        assert!(results.iter().any(|e| e.id == "firefox"));
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
}

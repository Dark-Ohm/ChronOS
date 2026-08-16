//! Launcher grid geometry, category building and 2D navigation (T265-B).
//!
//! Pure helpers only — no GPUI types. The view renders cells itself (the
//! current list was plain `div` + `ScrollHandle`, and the kit's `virtual_list`
//! is a private module, so the grid keeps that same non-virtualized approach).

use chronos_services::AppEntry;

/// Grid columns for the 720px card: 7 cells of `CELL_WIDTH` + 6 gaps of
/// `GRID_GAP` = 664px, which fits the 704px content width (720 − 2×8 padding).
pub const GRID_COLUMNS: usize = 7;
pub const CELL_WIDTH: f32 = 88.;
pub const CELL_HEIGHT: f32 = 88.;
pub const GRID_GAP: f32 = 8.;
/// Rows advanced by PageUp/PageDown (≈ the visible grid height at a 560px window).
pub const PAGE_ROWS: usize = 4;

/// A 2D cursor move over a row-major grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Move2D {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Clamp a flat grid index after a 2D move. `selected` is the current flat
/// index, `columns` the column count, `len` the item count, `page_rows` the
/// PageUp/PageDown stride in rows.
pub fn move_2d(selected: usize, columns: usize, len: usize, mv: Move2D, page_rows: usize) -> usize {
    if len == 0 || columns == 0 {
        return 0;
    }
    let last = len - 1;
    let col = selected % columns;
    match mv {
        Move2D::Up => selected.saturating_sub(columns),
        Move2D::Down => (selected + columns).min(last),
        Move2D::Left => {
            if col > 0 {
                selected - 1
            } else {
                selected
            }
        }
        Move2D::Right => {
            if col + 1 < columns && selected + 1 <= last {
                selected + 1
            } else {
                selected
            }
        }
        Move2D::Home => 0,
        Move2D::End => last,
        Move2D::PageUp => selected.saturating_sub(columns * page_rows),
        Move2D::PageDown => (selected + columns * page_rows).min(last),
    }
}

/// Main Categories from the freedesktop Desktop Menu Specification — the ones
/// meant for user-facing menus. Everything else (`IDE`, `TextEditor`, `Qt`,
/// `GTK`, `Building`, `Debugger`, …) is an "Additional Category" for distro
/// menu merging, not for display. T297: filter the bar down from the raw
/// category dump ("тьма") to these.
const MAIN_CATEGORIES: &[&str] = &[
    "AudioVideo",
    "Development",
    "Education",
    "Game",
    "Graphics",
    "Network",
    "Office",
    "Science",
    "Settings",
    "System",
    "Utility",
];

/// Distinct Main Categories present in `entries`, with counts, sorted by count
/// descending then name ascending. Non-main categories are dropped; an entry
/// whose categories are all non-main simply doesn't appear on the bar (it stays
/// reachable under "All"). Empty categories never appear.
pub fn build_categories(entries: &[AppEntry]) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for entry in entries {
        for cat in &entry.categories {
            if MAIN_CATEGORIES.contains(&cat.as_str()) {
                *counts.entry(cat.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut out: Vec<(String, usize)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out
}

/// Filter entries by category; `None` = "All" (every entry).
pub fn filter_by_category(entries: &[AppEntry], category: Option<&str>) -> Vec<AppEntry> {
    match category {
        None => entries.to_vec(),
        Some(cat) => entries
            .iter()
            .filter(|e| e.categories.iter().any(|c| c == cat))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, categories: &[&str]) -> AppEntry {
        AppEntry {
            categories: categories.iter().map(|s| s.to_string()).collect(),
            ..AppEntry::fixture(id, id)
        }
    }

    #[test]
    fn move_2d_walks_rows_and_clamps_edges() {
        let cols = 3;
        let len = 8; // cells 0..7
        assert_eq!(move_2d(0, cols, len, Move2D::Right, 4), 1);
        assert_eq!(move_2d(2, cols, len, Move2D::Right, 4), 2, "last column does not wrap");
        assert_eq!(move_2d(0, cols, len, Move2D::Down, 4), 3);
        assert_eq!(move_2d(7, cols, len, Move2D::Down, 4), 7, "bottom edge clamps");
        assert_eq!(move_2d(4, cols, len, Move2D::Left, 4), 3);
        assert_eq!(move_2d(3, cols, len, Move2D::Left, 4), 3, "first column does not wrap");
        assert_eq!(move_2d(3, cols, len, Move2D::Up, 4), 0);
        assert_eq!(move_2d(0, cols, len, Move2D::Up, 4), 0, "top edge clamps");
    }

    #[test]
    fn move_2d_home_end_page() {
        let cols = 3;
        let len = 20;
        assert_eq!(move_2d(11, cols, len, Move2D::Home, 4), 0);
        assert_eq!(move_2d(11, cols, len, Move2D::End, 4), 19);
        assert_eq!(move_2d(11, cols, len, Move2D::PageUp, 4), 0, "page up clamps at top");
        assert_eq!(move_2d(11, cols, len, Move2D::PageDown, 4), 19);
    }

    #[test]
    fn move_2d_empty_grid_returns_zero() {
        assert_eq!(move_2d(5, 3, 0, Move2D::Down, 4), 0);
    }

    #[test]
    fn build_categories_counts_sorts_and_drops_empty() {
        let entries = vec![
            entry("a", &["Development", "System"]),
            entry("b", &["Development"]),
            entry("c", &["Network"]),
            entry("d", &[""]),
            entry("e", &[]),
        ];
        let cats = build_categories(&entries);
        assert_eq!(
            cats,
            vec![
                ("Development".to_string(), 2),
                ("Network".to_string(), 1),
                ("System".to_string(), 1),
            ]
        );
    }

    #[test]
    fn build_categories_drops_non_main_but_keeps_main() {
        // Additional Categories (IDE/TextEditor/GTK) are dropped; the Main
        // Category (Development) on the same entry survives. An entry whose
        // categories are ALL additional does not leak into the bar at all.
        let entries = vec![
            entry("a", &["Development", "IDE", "GTK"]),
            entry("b", &["IDE", "TextEditor"]),
            entry("c", &["Graphics", "2DGraphics"]),
        ];
        let cats = build_categories(&entries);
        assert_eq!(
            cats,
            vec![("Development".to_string(), 1), ("Graphics".to_string(), 1)]
        );
    }

    #[test]
    fn build_categories_empty_input() {
        assert!(build_categories(&[]).is_empty());
    }

    #[test]
    fn filter_by_category_all_vs_specific() {
        let dev = entry("dev", &["Dev"]);
        let web = entry("web", &["Web"]);
        let both = entry("both", &["Dev", "Web"]);
        let entries = vec![dev.clone(), web.clone(), both.clone()];

        assert_eq!(filter_by_category(&entries, None), entries);
        assert_eq!(filter_by_category(&entries, Some("Dev")), vec![dev, both]);
        assert!(filter_by_category(&entries, Some("Game")).is_empty());
    }
}

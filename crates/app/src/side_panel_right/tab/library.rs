//! Library tab — Gamer at-rest hub: list, pin and launch detected games (§4.2).
//!
//! Game source: `ApplicationsState` filtered by `is_game_entry` (T184/T187).
//! Pin/recent bookkeeping persists to `~/.config/chronos/games.toml` via
//! `GamesConfig` (T187). No fake artwork or playtime (§13) — a row is a name,
//! a launch click, and a pin toggle.
//!
//! Sections: Pinned (curated order) → Recent (newest first) → All games
//! (alphabetical). A game appears in at most one section (pinned wins, then
//! recent, then all) so the list never repeats.

use std::collections::HashSet;

use chronos_services::applications::is_game_entry;
use chronos_services::{AppEntry, Service};
use chronos_ui::Theme;
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::games_config::GamesConfig;
use crate::launcher::launch::launch;
use crate::side_panel_right::tabs::PanelTab;
use crate::state::{self, AppState};
use super::ui;

pub struct LibraryTab {
    /// All detected games (`is_game_entry == true`), sorted by display name.
    games: Vec<AppEntry>,
    /// Pinned + recent bookkeeping, loaded from `games.toml`.
    config: GamesConfig,
    scroll: ScrollHandle,
}

impl LibraryTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Seed from the current snapshot, then track desktop-entry changes
        // (inotify rescans republish the full entry list).
        let signal = AppState::applications(cx).subscribe();
        state::watch(cx, signal, |this: &mut Self, data, cx| {
            this.set_games(data.entries, cx);
        });

        let mut this = Self {
            games: Vec::new(),
            config: GamesConfig::load(),
            scroll: ScrollHandle::new(),
        };
        this.set_games(AppState::applications(cx).get().entries, cx);
        this
    }

    fn set_games(&mut self, entries: Vec<AppEntry>, cx: &mut Context<Self>) {
        self.games = filter_games(entries);
        cx.notify();
    }

    fn launch_game(&mut self, entry: &AppEntry, cx: &mut Context<Self>) {
        if let Err(e) = launch(&entry.exec) {
            tracing::error!("library: failed to launch {}: {e:#}", entry.name);
            return;
        }
        self.config.touch_recent(&entry.id);
        if let Err(e) = self.config.save() {
            tracing::warn!("library: failed to save games.toml after launch: {e}");
        }
        cx.notify();
    }

    fn toggle_pin(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.config.is_pinned(id) {
            self.config.unpin(id);
        } else {
            self.config.pin(id);
        }
        if let Err(e) = self.config.save() {
            tracing::warn!("library: failed to save games.toml after pin toggle: {e}");
        }
        cx.notify();
    }

    /// One row. The launch area is the flex-1 clickable; the pin toggle is a
    /// separate sibling button so the two click handlers never nest.
    fn game_row(
        entry: &AppEntry,
        pinned: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let launch_entry = entry.clone();
        let pin_id = entry.id.clone();

        let launch_area = div()
            .id(SharedString::from(format!("library-launch-{}", entry.id)))
            .flex_1()
            .min_w(px(0.))
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .py(px(5.))
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(theme.interactive.hover))
            .on_click(cx.listener(move |this, _ev, _w, cx| {
                this.launch_game(&launch_entry, cx);
            }))
            .child(
                div()
                    .w(px(20.))
                    .h(px(20.))
                    .rounded_md()
                    .bg(theme.bg.elevated)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text.muted)
                            .child(initial(&entry.name).to_string()),
                    ),
            )
            .child(
                div()
                    .min_w(px(0.))
                    .text_size(px(12.))
                    .text_color(theme.text.primary)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(entry.name.clone()),
            );

        let pin_btn = div()
            .id(SharedString::from(format!("library-pin-{}", entry.id)))
            .px(px(8.))
            .py(px(5.))
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(theme.interactive.hover))
            .on_click(cx.listener(move |this, _ev, _w, cx| {
                this.toggle_pin(&pin_id, cx);
            }))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(if pinned {
                        theme.accent.primary
                    } else {
                        theme.text.muted
                    })
                    .child(if pinned { "★" } else { "☆" }),
            );

        div()
            .flex()
            .items_center()
            .child(launch_area)
            .child(pin_btn)
            .into_any_element()
    }
}

impl Render for LibraryTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let sections = compute_sections(&self.games, &self.config);

        let header = div()
            .px(px(12.))
            .py(px(10.))
            .border_b_1()
            .border_color(theme.border.subtle)
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text.primary)
                    .child(format!("Library · {} games", self.games.len())),
            );

        let mut list = div()
            .id("library-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .px(px(6.))
            .py(px(4.));

        if self.games.is_empty() {
            list = list.child(empty_state(&theme));
        } else {
            if !sections.pinned.is_empty() {
                list = list.child(section_header("Pinned", &theme));
                for entry in &sections.pinned {
                    list = list.child(Self::game_row(entry, true, &theme, &mut *cx));
                }
            }
            if !sections.recent.is_empty() {
                list = list.child(section_header("Recent", &theme));
                for entry in &sections.recent {
                    list = list.child(Self::game_row(entry, false, &theme, &mut *cx));
                }
            }
            if !sections.all.is_empty() {
                list = list.child(section_header("All games", &theme));
                for entry in &sections.all {
                    list = list.child(Self::game_row(entry, false, &theme, &mut *cx));
                }
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(list)
    }
}

// ---------------------------------------------------------------------------
// Pure helpers — no cx/AppState, so the business logic is unit-testable
// (same precedent as `system.rs::format_net_pair`).
// ---------------------------------------------------------------------------

/// Filter to games and sort by display name (case-insensitive).
fn filter_games(mut entries: Vec<AppEntry>) -> Vec<AppEntry> {
    entries.retain(is_game_entry);
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}

/// Display sections. A game appears in at most one section: pinned wins, then
/// recent, then the alphabetical "all" bucket. Pinned/recent ids with no
/// matching detected game are dropped (stale config entry).
struct Sections {
    pinned: Vec<AppEntry>,
    recent: Vec<AppEntry>,
    all: Vec<AppEntry>,
}

fn compute_sections(games: &[AppEntry], config: &GamesConfig) -> Sections {
    let by_id = |id: &str| games.iter().find(|g| g.id == id).cloned();

    let pinned: Vec<AppEntry> = config
        .pinned
        .iter()
        .filter_map(|id| by_id(id))
        .collect();

    let pinned_ids: HashSet<&str> = config.pinned.iter().map(|s| s.as_str()).collect();

    let recent: Vec<AppEntry> = config
        .recent
        .iter()
        .filter(|r| !pinned_ids.contains(r.id.as_str()))
        .filter_map(|r| by_id(&r.id))
        .collect();

    let shown: HashSet<&str> = pinned
        .iter()
        .chain(recent.iter())
        .map(|g| g.id.as_str())
        .collect();

    let all: Vec<AppEntry> = games
        .iter()
        .filter(|g| !shown.contains(g.id.as_str()))
        .cloned()
        .collect();

    Sections { pinned, recent, all }
}

fn section_header(label: &str, theme: &Theme) -> AnyElement {
    div()
        .px(px(8.))
        .pt(px(10.))
        .pb(px(4.))
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text.muted)
        .child(label.to_string())
        .into_any_element()
}

/// Hero canon via `ui::empty_state_hero` (T269): the tab's own rail icon per
/// the 2026-08-13 ruling, so an empty Library reads as the same family as an
/// unimplemented tab. `px(20)`/`py(40)` stay on the wrapper — the state sits
/// inside the scrollable list, not on a full-size surface.
fn empty_state(theme: &Theme) -> AnyElement {
    div()
        .w_full()
        .px(px(20.))
        .py(px(40.))
        .child(ui::empty_state_hero(
            *theme,
            PanelTab::Library.icon_path(),
            "No games detected",
            "Games appear from XDG .desktop files with Categories=Game, or Steam steam://rungameid shortcuts.",
            ui::NoteSeverity::Muted,
            None,
        ))
        .into_any_element()
}

/// First uppercase letter of a name, for the avatar tile.
fn initial(name: &str) -> char {
    name.chars().next().unwrap_or('?').to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str, name: &str) -> AppEntry {
        AppEntry {
            id: id.into(),
            name: name.into(),
            exec: format!("steam steam://rungameid/{id}"),
            icon: None,
            terminal: false,
            categories: vec!["Game".into()],
        }
    }

    fn non_game(id: &str, name: &str, exec: &str, categories: &[&str]) -> AppEntry {
        AppEntry {
            id: id.into(),
            name: name.into(),
            exec: exec.into(),
            icon: None,
            terminal: false,
            categories: categories.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn filter_games_excludes_steam_client_and_non_games_and_sorts() {
        // T184/T187: id=="steam" is the client (Categories=Game) but NOT a
        // game; firefox has no Game category; CS2 + PUBG are real games.
        let steam = non_game("steam", "Steam", "/usr/bin/steam %U", &["Game", "Network"]);
        let firefox = non_game("firefox", "Firefox", "/usr/bin/firefox", &["Network"]);
        let cs2 = game("Counter-Strike 2", "Counter-Strike 2");
        let pubg = game("PUBG BATTLEGROUNDS", "PUBG: BATTLEGROUNDS");

        let out = filter_games(vec![steam, pubg.clone(), cs2.clone(), firefox]);
        assert_eq!(out.len(), 2, "only the two real games survive");
        // Sorted case-insensitively by name: "counter-strike..." < "pubg...".
        assert_eq!(out[0].id, "Counter-Strike 2");
        assert_eq!(out[1].id, "PUBG BATTLEGROUNDS");
    }

    #[test]
    fn filter_games_empty_input() {
        assert!(filter_games(Vec::new()).is_empty());
    }

    #[test]
    fn compute_sections_pinned_wins_over_recent_and_all() {
        let games = vec![
            game("cs2", "CS2"),
            game("pubg", "PUBG"),
            game("scum", "SCUM"),
            game("dota", "Dota 2"),
        ];
        let mut config = GamesConfig::default();
        config.pin("cs2");
        config.touch_recent("pubg");
        config.touch_recent("cs2"); // cs2 both pinned and recent — pinned wins
        let s = compute_sections(&games, &config);

        assert_eq!(
            s.pinned.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
            vec!["cs2"]
        );
        assert_eq!(
            s.recent.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
            vec!["pubg"]
        );
        let all: Vec<&str> = s.all.iter().map(|g| g.id.as_str()).collect();
        assert!(all.contains(&"scum"));
        assert!(all.contains(&"dota"));
        assert!(!all.contains(&"cs2"));
        assert!(!all.contains(&"pubg"));
    }

    #[test]
    fn compute_sections_drops_stale_pinned_and_recent_ids() {
        let games = vec![game("cs2", "CS2")];
        let mut config = GamesConfig::default();
        config.pin("cs2");
        config.pin("nonexistent-game"); // stale pin
        config.touch_recent("ghost"); // stale recent
        let s = compute_sections(&games, &config);
        assert_eq!(s.pinned.len(), 1);
        assert_eq!(s.pinned[0].id, "cs2");
        assert!(s.recent.is_empty(), "stale recent id must be dropped");
    }

    #[test]
    fn compute_sections_empty_games_yields_all_empty() {
        let config = GamesConfig::default();
        let s = compute_sections(&[], &config);
        assert!(s.pinned.is_empty());
        assert!(s.recent.is_empty());
        assert!(s.all.is_empty());
    }

    #[test]
    fn compute_sections_pinned_keeps_config_order() {
        let games = vec![game("a", "A"), game("b", "B"), game("c", "C")];
        let mut config = GamesConfig::default();
        config.pin("c");
        config.pin("a"); // pinned order is c, a — not alphabetical
        let s = compute_sections(&games, &config);
        assert_eq!(
            s.pinned.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
            vec!["c", "a"],
            "pinned section must preserve games.toml insertion order"
        );
    }

    #[test]
    fn initial_uppercases_first_char() {
        assert_eq!(initial("Counter-Strike"), 'C');
        assert_eq!(initial("pubg"), 'P');
        assert_eq!(initial(""), '?');
    }
}

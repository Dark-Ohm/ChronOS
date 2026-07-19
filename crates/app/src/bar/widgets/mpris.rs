//! MPRIS media-player widget — track label + play/pause + multi-player scroll.

use gpui::{AnyElement, App, ScrollDelta, ScrollWheelEvent, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{CycleDirection, MprisCommand, MprisState, Service};
use chronos_ui::Theme;

use crate::state::AppState;

/// Max characters for the `title — artist` label before ellipsis.
const MAX_LABEL_CHARS: usize = 40;

/// Pure description of what the widget should display.
#[derive(Debug, PartialEq, Eq)]
enum MprisView {
    /// No MPRIS player on the bus — render nothing.
    Hidden,
    /// Active player with play/pause icon + truncated track label.
    Track {
        icon: &'static str,
        label: String,
        playing: bool,
        /// Multi-player hint when `player_count > 1` (e.g. `‹2/3›`).
        multi: Option<String>,
    },
}

fn describe(state: &MprisState) -> MprisView {
    if !state.has_player {
        return MprisView::Hidden;
    }
    // Idle player with no metadata (e.g. a browser with no media loaded):
    // showing "▶ Unknown" is noise — hide until something is actually queued.
    if !state.playing && state.title.is_empty() && state.artist.is_empty() {
        return MprisView::Hidden;
    }
    let icon = if state.playing { "⏸" } else { "▶" };
    let label = if state.title.is_empty() && state.artist.is_empty() {
        truncate_chars(&state.player_id, MAX_LABEL_CHARS)
    } else {
        format_track_label(&state.title, &state.artist, MAX_LABEL_CHARS)
    };
    let multi = multi_player_indicator(state.player_count, state.player_index);
    MprisView::Track {
        icon,
        label,
        playing: state.playing,
        multi,
    }
}

/// `‹i/n›` when more than one player is live; otherwise hidden.
pub fn multi_player_indicator(player_count: usize, player_index: usize) -> Option<String> {
    if player_count <= 1 {
        return None;
    }
    let idx = if player_index == 0 { 1 } else { player_index };
    Some(format!("‹{idx}/{player_count}›"))
}

/// Build `title — artist` (or just title / just artist) and hard-truncate.
pub fn format_track_label(title: &str, artist: &str, max_chars: usize) -> String {
    let raw = match (title.is_empty(), artist.is_empty()) {
        (true, true) => "Unknown".to_string(),
        (false, true) => title.to_string(),
        (true, false) => artist.to_string(),
        (false, false) => format!("{title} — {artist}"),
    };
    truncate_chars(&raw, max_chars)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let keep = max_chars - 1;
    let truncated: String = s.chars().take(keep).collect();
    format!("{truncated}…")
}

/// Map scroll delta to cycle direction. Scroll up (negative y) → Next player.
fn scroll_cycle_direction(delta: &ScrollDelta) -> Option<CycleDirection> {
    let y = match delta {
        ScrollDelta::Lines(p) => p.y as f64,
        ScrollDelta::Pixels(p) => f64::from(p.y),
    };
    if y < 0.0 {
        Some(CycleDirection::Next)
    } else if y > 0.0 {
        Some(CycleDirection::Prev)
    } else {
        None
    }
}

pub struct MprisWidget;

impl BarWidget for MprisWidget {
    fn name(&self) -> &str {
        "mpris"
    }

    /// Center: track labels are long; Right is already crowded (net/tray/vol).
    fn section(&self) -> BarSection {
        BarSection::Center
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let state = AppState::mpris(cx).get();
        let theme = Theme::global(cx);

        match describe(&state) {
            MprisView::Hidden => div().into_any_element(),
            MprisView::Track {
                icon,
                label,
                playing,
                multi,
            } => {
                let color = if playing {
                    theme.text.secondary
                } else {
                    theme.text.muted
                };
                let mut row = div()
                    .id("bar-mpris")
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(theme.radius)
                    .child(div().child(icon.to_string()).text_color(color))
                    .child(div().child(label).text_color(color));
                if let Some(hint) = multi {
                    row = row.child(
                        div()
                            .child(hint)
                            .text_color(theme.text.muted)
                            .text_xs(),
                    );
                }
                row.on_click(|_event, _window, cx: &mut App| {
                    AppState::mpris(cx).dispatch(MprisCommand::PlayPause);
                })
                .on_scroll_wheel(|event: &ScrollWheelEvent, _window, cx: &mut App| {
                    if let Some(dir) = scroll_cycle_direction(&event.delta) {
                        AppState::mpris(cx).dispatch(MprisCommand::CyclePlayer(dir));
                    }
                })
                .into_any_element()
            }
        }
    }
}

/// Register the MPRIS widget with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(MprisWidget));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};

    fn track_state(
        title: &str,
        artist: &str,
        playing: bool,
        player_count: usize,
        player_index: usize,
    ) -> MprisState {
        MprisState {
            title: title.into(),
            artist: artist.into(),
            playing,
            has_player: true,
            player_count,
            player_index,
            player_id: "mock".into(),
        }
    }

    #[test]
    fn hidden_without_player() {
        assert_eq!(describe(&MprisState::default()), MprisView::Hidden);
    }

    #[test]
    fn playing_shows_pause_icon() {
        let state = track_state("Track", "Artist", true, 1, 1);
        match describe(&state) {
            MprisView::Track {
                icon,
                label,
                playing,
                multi,
            } => {
                assert_eq!(icon, "⏸");
                assert!(playing);
                assert_eq!(label, "Track — Artist");
                assert!(multi.is_none());
            }
            other => panic!("expected Track, got {other:?}"),
        }
    }

    #[test]
    fn paused_shows_play_icon() {
        let state = track_state("T", "", false, 1, 1);
        match describe(&state) {
            MprisView::Track { icon, playing, multi, .. } => {
                assert_eq!(icon, "▶");
                assert!(!playing);
                assert!(multi.is_none());
            }
            other => panic!("expected Track, got {other:?}"),
        }
    }

    #[test]
    fn multi_indicator_when_two_players() {
        let state = track_state("A", "B", true, 2, 1);
        match describe(&state) {
            MprisView::Track { multi, .. } => {
                assert_eq!(multi.as_deref(), Some("‹1/2›"));
            }
            other => panic!("expected Track, got {other:?}"),
        }
    }

    #[test]
    fn multi_indicator_hidden_for_single() {
        assert_eq!(multi_player_indicator(0, 0), None);
        assert_eq!(multi_player_indicator(1, 1), None);
        assert_eq!(multi_player_indicator(3, 2).as_deref(), Some("‹2/3›"));
    }

    #[test]
    fn format_title_and_artist() {
        assert_eq!(format_track_label("Song", "Band", 40), "Song — Band");
    }

    #[test]
    fn format_title_only() {
        assert_eq!(format_track_label("Only Title", "", 40), "Only Title");
    }

    #[test]
    fn format_artist_only() {
        assert_eq!(format_track_label("", "Only Artist", 40), "Only Artist");
    }

    #[test]
    fn format_unknown_when_empty() {
        assert_eq!(format_track_label("", "", 40), "Unknown");
    }

    #[test]
    fn format_truncates_long() {
        let long = "a".repeat(50);
        let out = format_track_label(&long, "", 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn scroll_up_cycles_next() {
        assert_eq!(
            scroll_cycle_direction(&ScrollDelta::Lines(point(0.0, -1.0))),
            Some(CycleDirection::Next)
        );
        assert_eq!(
            scroll_cycle_direction(&ScrollDelta::Pixels(point(px(0.), px(-10.)))),
            Some(CycleDirection::Next)
        );
    }

    #[test]
    fn scroll_down_cycles_prev() {
        assert_eq!(
            scroll_cycle_direction(&ScrollDelta::Lines(point(0.0, 1.0))),
            Some(CycleDirection::Prev)
        );
        assert_eq!(
            scroll_cycle_direction(&ScrollDelta::Pixels(point(px(0.), px(10.)))),
            Some(CycleDirection::Prev)
        );
    }

    #[test]
    fn scroll_zero_is_noop() {
        assert_eq!(
            scroll_cycle_direction(&ScrollDelta::Lines(point(0.0, 0.0))),
            None
        );
    }
}

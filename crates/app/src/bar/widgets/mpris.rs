//! MPRIS media-player widget — track label + play/pause.

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_services::{MprisCommand, MprisState, Service};
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
    },
}

fn describe(state: &MprisState) -> MprisView {
    if !state.has_player {
        return MprisView::Hidden;
    }
    let icon = if state.playing { "⏸" } else { "▶" };
    let label = format_track_label(&state.title, &state.artist, MAX_LABEL_CHARS);
    MprisView::Track {
        icon,
        label,
        playing: state.playing,
    }
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
            MprisView::Track { icon, label, playing } => {
                let color = if playing {
                    theme.text.secondary
                } else {
                    theme.text.muted
                };
                div()
                    .id("bar-mpris")
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .px(px(6.))
                    .py(px(2.))
                    .rounded(theme.radius)
                    .child(div().child(icon.to_string()).text_color(color))
                    .child(div().child(label).text_color(color))
                    .on_click(|_event, _window, cx: &mut App| {
                        AppState::mpris(cx).dispatch(MprisCommand::PlayPause);
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

    #[test]
    fn hidden_without_player() {
        assert_eq!(describe(&MprisState::default()), MprisView::Hidden);
    }

    #[test]
    fn playing_shows_pause_icon() {
        let state = MprisState {
            title: "Track".into(),
            artist: "Artist".into(),
            playing: true,
            has_player: true,
        };
        match describe(&state) {
            MprisView::Track { icon, label, playing } => {
                assert_eq!(icon, "⏸");
                assert!(playing);
                assert_eq!(label, "Track — Artist");
            }
            other => panic!("expected Track, got {other:?}"),
        }
    }

    #[test]
    fn paused_shows_play_icon() {
        let state = MprisState {
            title: "T".into(),
            artist: String::new(),
            playing: false,
            has_player: true,
        };
        match describe(&state) {
            MprisView::Track { icon, playing, .. } => {
                assert_eq!(icon, "▶");
                assert!(!playing);
            }
            other => panic!("expected Track, got {other:?}"),
        }
    }

    #[test]
    fn format_title_and_artist() {
        assert_eq!(
            format_track_label("Song", "Band", 40),
            "Song — Band"
        );
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
}

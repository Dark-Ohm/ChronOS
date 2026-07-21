//! MPRIS card for the right side panel. Uses only the fields
//! `MprisState` actually exposes (`title`/`artist`/`playing`/`player_id`)
//! — no album art URL, no position/length exist in this service today
//! (confirmed during planning, see plan Global Constraints). This card
//! ships a static gradient placeholder swatch instead of real art, and
//! has no progress bar / timecode row.

use gpui::{Context, IntoElement, div, prelude::*, px, rgb};

use chronos_services::MprisState;
use chronos_ui::Theme;

use crate::side_panel_right::view::SidePanelRightView;
use crate::state::AppState;

/// Accent for art swatch + play control (plan: cyan `0x5fd3e8`, not a rainbow).
const ACCENT_CYAN: u32 = 0x5f_d3_e8;

fn display_title(state: &MprisState) -> &str {
    if state.has_player {
        if state.title.is_empty() {
            if state.artist.is_empty() {
                state.player_id.as_str()
            } else {
                state.artist.as_str()
            }
        } else {
            state.title.as_str()
        }
    } else {
        "No player"
    }
}

fn display_artist(state: &MprisState) -> &str {
    if state.has_player {
        state.artist.as_str()
    } else {
        ""
    }
}

pub fn render_mpris_card(
    state: &MprisState,
    theme: &Theme,
    _cx: &mut Context<SidePanelRightView>,
) -> impl IntoElement {
    let player_id_for_mute = state.player_id.clone();
    let playing = state.playing;
    let has_player = state.has_player;

    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(14.))
        .rounded(theme.radius)
        .bg(theme.bg.tertiary)
        .border_1()
        .border_color(theme.border.default)
        .child(
            div()
                .flex()
                .gap(px(12.))
                .child(
                    // Placeholder swatch — no real album art source exists
                    // (see module doc comment). Do not wire an image fetch
                    // here without first extending `MprisState` with an
                    // art URL field in a follow-up spec.
                    div()
                        .w(px(64.))
                        .h(px(64.))
                        .flex_shrink_0()
                        .rounded(theme.radius)
                        .bg(rgb(ACCENT_CYAN)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(px(4.))
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .font_family(theme.font_ui)
                                .text_size(px(13.))
                                .text_color(theme.text.primary)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(display_title(state).to_string()),
                        )
                        .child(
                            div()
                                .font_family(theme.font_ui)
                                .text_size(px(11.5))
                                .text_color(theme.text.secondary)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(display_artist(state).to_string()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .child(transport_btn(
                    "mpris-prev",
                    "<",
                    theme,
                    has_player,
                    move |_, _, cx| {
                        AppState::mpris(cx).dispatch(chronos_services::MprisCommand::Previous);
                    },
                ))
                .child(transport_btn(
                    "mpris-playpause",
                    if playing { "||" } else { ">" },
                    theme,
                    has_player,
                    move |_, _, cx| {
                        AppState::mpris(cx).dispatch(chronos_services::MprisCommand::PlayPause);
                    },
                ))
                .child(transport_btn(
                    "mpris-next",
                    ">",
                    theme,
                    has_player,
                    move |_, _, cx| {
                        AppState::mpris(cx).dispatch(chronos_services::MprisCommand::Next);
                    },
                ))
                .child(
                    div()
                        .id("mpris-mute")
                        .ml_auto()
                        .w(px(28.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .cursor_pointer()
                        .text_color(theme.text.muted)
                        .hover(|s| s.bg(theme.interactive.hover))
                        .child("M")
                        .on_click(move |_, _window, cx| {
                            if player_id_for_mute.is_empty() {
                                tracing::debug!(
                                    "side_panel_right: mute ignored — empty player_id"
                                );
                                return;
                            }
                            tracing::info!(
                                "side_panel_right: mute toggle for player_id={player_id_for_mute}"
                            );
                            AppState::audio(cx)
                                .toggle_stream_mute_for_player(player_id_for_mute.clone());
                        }),
                ),
        )
}

fn transport_btn(
    id: &'static str,
    label: &'static str,
    theme: &Theme,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let is_play = id == "mpris-playpause";
    let mut el = div()
        .id(id)
        .w(px(if is_play { 38. } else { 32. }))
        .h(px(if is_play { 38. } else { 32. }))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(if is_play {
            theme.accent.primary
        } else {
            theme.text.primary
        })
        .child(label);

    if is_play {
        el = el.bg(theme.bg.elevated);
    }

    el = el.hover(|s| s.bg(theme.interactive.hover));

    if enabled {
        el.on_click(on_click)
    } else {
        el.opacity(0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::MprisState;

    #[test]
    fn no_player_shows_placeholder_title() {
        let state = MprisState::default();
        assert_eq!(display_title(&state), "No player");
    }

    #[test]
    fn active_player_shows_real_title() {
        let state = MprisState {
            title: "Colour Temperature".into(),
            artist: "Ambient Systems".into(),
            playing: true,
            has_player: true,
            player_count: 1,
            player_index: 1,
            player_id: "vivaldi".into(),
        };
        assert_eq!(display_title(&state), "Colour Temperature");
    }

    #[test]
    fn empty_title_falls_back_to_artist_or_id() {
        let mut state = MprisState {
            title: String::new(),
            artist: "Only Artist".into(),
            playing: false,
            has_player: true,
            player_count: 1,
            player_index: 1,
            player_id: "mpv".into(),
        };
        assert_eq!(display_title(&state), "Only Artist");
        state.artist.clear();
        assert_eq!(display_title(&state), "mpv");
    }
}

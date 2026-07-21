//! Media card for the right side panel — mockup layout (16:9 art, progress
//! stub, transport tray). Live: title + prev/play/next + mute. Static: art
//! placeholder, progress bar + timecode (no Position/length in MprisState).

use gpui::{Context, IntoElement, div, img, prelude::*, px, relative, rgb, rgba};

use chronos_services::MprisState;

use crate::side_panel_right::view::SidePanelRightView;
use crate::state::AppState;

const ACCENT: u32 = 0x00_7a_cc;
const CARD_BG: u32 = 0x1e_1e_2e;
const BORDER: u32 = 0x23_23_36;
const TEXT: u32 = 0xcd_d6_f4;
const MUTED: u32 = 0x6c_70_86;
const TRAY_BG: u32 = 0x15_15_1f;
const TRAY_BORDER: u32 = 0x26_26_3a;
const TRACK: u32 = 0x31_32_44;

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

pub fn render_mpris_card(
    state: &MprisState,
    _cx: &mut Context<SidePanelRightView>,
) -> impl IntoElement {
    let player_id_for_mute = state.player_id.clone();
    let playing = state.playing;
    let has_player = state.has_player;
    let title = display_title(state).to_string();
    let play_icon = if playing {
        "icons/pause.svg"
    } else {
        "icons/play.svg"
    };

    div()
        .flex()
        .flex_col()
        .rounded(px(9.))
        .overflow_hidden()
        .bg(rgb(CARD_BG))
        .border_1()
        .border_color(rgb(BORDER))
        // ~16:9 of 352 content width
        .child(
            div()
                .w_full()
                .h(px(198.))
                .bg(rgb(0x00_00_00))
                .flex()
                .items_center()
                .justify_center()
                .relative()
                .child(
                    img("icons/play.svg")
                        .w(px(34.))
                        .h(px(34.))
                        .text_color(rgb(0xe0_e0_e6))
                        .opacity(0.9),
                )
                .child(
                    div()
                        .absolute()
                        .bottom(px(8.))
                        .right(px(10.))
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(0xe0_e0_e6))
                        .bg(rgba(0x0000_0080))
                        .px(px(5.))
                        .py(px(1.))
                        .rounded(px(4.))
                        .child("-14:22"),
                ),
        )
        .child(
            div()
                .px(px(11.))
                .py(px(9.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(TEXT))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .mb(px(6.))
                        .child(title),
                )
                .child(
                    div()
                        .h(px(3.))
                        .w_full()
                        .rounded(px(2.))
                        .bg(rgb(TRACK))
                        .mb(px(8.))
                        .child(
                            div()
                                .h_full()
                                .w(relative(0.38))
                                .rounded(px(2.))
                                .bg(rgb(ACCENT)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .p(px(3.))
                        .rounded(px(8.))
                        .bg(rgb(TRAY_BG))
                        .border_1()
                        .border_color(rgb(TRAY_BORDER))
                        .child(tray_icon_btn(
                            "mpris-mute",
                            "icons/speaker-mute.svg",
                            1.0,
                            false,
                            has_player,
                            move |_, _, cx| {
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
                            },
                        ))
                        .child(tray_divider())
                        .child(tray_icon_btn(
                            "mpris-prev",
                            "icons/skip-back.svg",
                            1.0,
                            false,
                            has_player,
                            move |_, _, cx| {
                                AppState::mpris(cx)
                                    .dispatch(chronos_services::MprisCommand::Previous);
                            },
                        ))
                        .child(tray_play(play_icon, has_player))
                        .child(tray_divider())
                        .child(tray_icon_btn(
                            "mpris-next",
                            "icons/skip-forward.svg",
                            1.0,
                            false,
                            has_player,
                            move |_, _, cx| {
                                AppState::mpris(cx)
                                    .dispatch(chronos_services::MprisCommand::Next);
                            },
                        )),
                ),
        )
}

fn tray_divider() -> impl IntoElement {
    div()
        .w(px(1.))
        .h(px(16.))
        .bg(rgb(TRAY_BORDER))
}

fn tray_icon_btn(
    id: &'static str,
    icon: &'static str,
    flex: f32,
    is_play: bool,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let icon_size = if is_play { 17. } else { 14. };
    let el = div()
        .id(id)
        .flex_grow(flex)
        .h(px(30.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(if is_play {
            rgb(0x18_18_25)
        } else {
            rgb(MUTED)
        })
        .when(is_play, |d| d.bg(rgb(ACCENT)))
        .hover(|s| {
            if is_play {
                s.bg(rgb(0xcb_a6_f7))
            } else {
                s.bg(rgb(0x25_25_3a)).text_color(rgb(0xf2_f2_f5))
            }
        })
        .child(img(icon).w(px(icon_size)).h(px(icon_size)));

    if enabled {
        el.on_click(on_click)
    } else {
        el.opacity(0.4)
    }
}

fn tray_play(icon_path: &'static str, enabled: bool) -> impl IntoElement {
    let mut el = div()
        .id("mpris-playpause")
        .flex_grow(1.4)
        .h(px(30.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(rgb(ACCENT))
        .text_color(rgb(0x18_18_25))
        .hover(|s| s.bg(rgb(0xcb_a6_f7)))
        .child(
            img(icon_path)
                .w(px(17.))
                .h(px(17.))
                .text_color(rgb(0x18_18_25)),
        );

    if enabled {
        el = el.on_click(move |_, _, cx| {
            AppState::mpris(cx).dispatch(chronos_services::MprisCommand::PlayPause);
        });
    } else {
        el = el.opacity(0.4);
    }
    el
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

//! Media card for the right side panel — 16:9 art, live progress, transport.
//!
//! Live: title, album art (`file://` only), position/length progress + remaining
//! timecode, prev/play/next + stream mute. http(s) art URLs stay on placeholder
//! (no network fetch in this track).

use std::path::{Path, PathBuf};

use gpui::{
    Context, ImageSource, IntoElement, ObjectFit, div, img, prelude::*, px, relative, rgb, rgba,
};

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

/// Decode `file://…` art URLs to a local path. http(s) / empty / garbage → None.
pub fn art_file_path(art_url: Option<&str>) -> Option<PathBuf> {
    let uri = art_url?;
    file_uri_to_path(uri).filter(|p| p.is_file())
}

/// `file:///home/x/my%20dir` → `/home/x/my dir`. Non-file schemes → None.
pub fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut chars = encoded.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next()?;
            let lo = chars.next()?;
            let hex = [hi, lo];
            let hex = std::str::from_utf8(&hex).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

/// `position / length` clamped to [0, 1]. Missing / non-positive length → None.
pub fn progress_ratio(position_us: Option<i64>, length_us: Option<i64>) -> Option<f32> {
    let length = length_us.filter(|l| *l > 0)?;
    let position = position_us.unwrap_or(0).max(0);
    let ratio = position as f64 / length as f64;
    Some(ratio.clamp(0.0, 1.0) as f32)
}

/// Remaining time as mockup `-M:SS` (or `-H:MM:SS` when ≥1h). None if no length.
pub fn remaining_timecode(position_us: Option<i64>, length_us: Option<i64>) -> Option<String> {
    let length = length_us.filter(|l| *l > 0)?;
    let position = position_us.unwrap_or(0).max(0);
    let remain_us = (length - position).max(0);
    Some(format_remaining_us(remain_us))
}

fn format_remaining_us(us: i64) -> String {
    let total_secs = us / 1_000_000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("-{hours}:{mins:02}:{secs:02}")
    } else {
        format!("-{mins}:{secs:02}")
    }
}

fn render_art_frame(
    art_path: Option<&Path>,
    position_us: Option<i64>,
    length_us: Option<i64>,
) -> impl IntoElement {
    let timecode = remaining_timecode(position_us, length_us);

    let frame = div()
        .w_full()
        .h(px(198.))
        .bg(rgb(0x00_00_00))
        .flex()
        .items_center()
        .justify_center()
        .relative()
        .overflow_hidden();

    let frame = if let Some(path) = art_path {
        let src: ImageSource = path.to_path_buf().into();
        frame.child(
            img(src)
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Cover),
        )
    } else {
        frame.child(
            img("icons/play.svg")
                .w(px(34.))
                .h(px(34.))
                .text_color(rgb(0xe0_e0_e6))
                .opacity(0.9),
        )
    };

    frame.when_some(timecode, |d, code| {
        d.child(
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
                .child(code),
        )
    })
}

fn render_progress_bar(ratio: Option<f32>) -> impl IntoElement {
    let fill = ratio.unwrap_or(0.0);
    div()
        .h(px(3.))
        .w_full()
        .rounded(px(2.))
        .bg(rgb(TRACK))
        .mb(px(8.))
        .child(
            div()
                .h_full()
                .w(relative(fill))
                .rounded(px(2.))
                .bg(rgb(ACCENT)),
        )
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
    let art_path = art_file_path(state.art_url.as_deref());
    let ratio = progress_ratio(state.position_us, state.length_us);

    div()
        .flex()
        .flex_col()
        .rounded(px(9.))
        .overflow_hidden()
        .bg(rgb(CARD_BG))
        .border_1()
        .border_color(rgb(BORDER))
        // ~16:9 of 352 content width
        .child(render_art_frame(
            art_path.as_deref(),
            state.position_us,
            state.length_us,
        ))
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
                .child(render_progress_bar(ratio))
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
            ..Default::default()
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
            ..Default::default()
        };
        assert_eq!(display_title(&state), "Only Artist");
        state.artist.clear();
        assert_eq!(display_title(&state), "mpv");
    }

    #[test]
    fn file_uri_to_path_decodes_percent() {
        let p = file_uri_to_path("file:///tmp/my%20cover.png").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/my cover.png"));
    }

    #[test]
    fn file_uri_rejects_http() {
        assert!(file_uri_to_path("https://cdn.example/art.jpg").is_none());
        assert!(art_file_path(Some("https://cdn.example/art.jpg")).is_none());
    }

    #[test]
    fn art_file_path_requires_existing_file() {
        // Machine-local path used as fixture elsewhere; must exist.
        let path = "/usr/share/pixmaps/archlinux-logo.png";
        assert!(
            Path::new(path).is_file(),
            "fixture image missing on host: {path}"
        );
        let uri = format!("file://{path}");
        assert_eq!(
            art_file_path(Some(&uri)).as_deref(),
            Some(Path::new(path))
        );
        assert!(art_file_path(Some("file:///no/such/cover.png")).is_none());
    }

    #[test]
    fn progress_ratio_clamps_and_hides_without_length() {
        assert_eq!(progress_ratio(Some(50), None), None);
        assert_eq!(progress_ratio(Some(50), Some(0)), None);
        let r = progress_ratio(Some(50_000_000), Some(100_000_000)).unwrap();
        assert!((r - 0.5).abs() < 1e-5);
        assert_eq!(progress_ratio(Some(200), Some(100)).unwrap(), 1.0);
        assert_eq!(progress_ratio(Some(0), Some(100)).unwrap(), 0.0);
    }

    #[test]
    fn remaining_timecode_formats_like_mockup() {
        // length 14:22, position 0 → -14:22
        let len = (14 * 60 + 22) * 1_000_000;
        assert_eq!(
            remaining_timecode(Some(0), Some(len)).as_deref(),
            Some("-14:22")
        );
        // 1:00 remaining
        assert_eq!(
            remaining_timecode(Some(len - 60_000_000), Some(len)).as_deref(),
            Some("-1:00")
        );
        assert_eq!(remaining_timecode(Some(10), None), None);
    }
}

//! System popup view — brightness fill-bar + steppers, 3-segment power
//! profile switch, gaming-mode toggle + effect string.
//!
//! Visual spec: `design/System Popup.dc.html` + `design.md` §6. Structure
//! mirrors `volume_popup/view.rs` (header + ✕, three blocks separated by
//! `bg.secondary` dividers, `border.subtle` 1px frame).

use gpui::{
    AnyElement, App, Context, InteractiveElement, IntoElement, Render, SharedString, Styled,
    Window, div, prelude::*, px,
};

use chronos_services::{BrightnessCommand, PowerProfile, Service, UPowerData};
use chronos_ui::Theme;

use crate::state::AppState;
use crate::system_popup::{close_this, gaming_mode};

const PAD: f32 = 12.;
const TRACK_W: f32 = 200.;
const TRACK_H: f32 = 6.;
const STEP: i8 = 5;

pub struct SystemPopupView;

impl SystemPopupView {
    pub fn new(_cx: &mut App) -> Self {
        Self
    }
}

impl Render for SystemPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let brightness = AppState::brightness(cx).get();
        let upower = AppState::upower(cx).get();
        let gaming_active = gaming_mode::GamingModeState::is_active(cx);

        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let hover = theme.interactive.hover;
        let accent = theme.accent.primary;
        let border_subtle = theme.border.subtle;

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(PAD))
            .py(px(8.))
            .child(div().text_color(text_primary).child("System"))
            .child(
                div()
                    .id("system-popup-close")
                    .cursor_pointer()
                    .px(px(6.))
                    .rounded(radius)
                    .text_color(text_muted)
                    .hover(|s| s.bg(hover))
                    .child("✕")
                    .on_click(|_event, window, cx: &mut App| {
                        tracing::info!("system_popup: ✕ close clicked");
                        close_this(window, cx);
                    }),
            );

        let divider_line = div().w_full().h(px(1.)).bg(divider);

        div()
            .flex_col()
            .w(px(300.))
            .rounded(radius_lg)
            .bg(bg)
            .border_1()
            .border_color(border_subtle)
            .overflow_hidden()
            .child(header)
            .child(divider_line)
            .child(brightness_block(
                &brightness,
                text_primary,
                text_muted,
                text_secondary,
                accent,
                divider,
                radius,
                hover,
                cx,
            ))
            .child(div().w_full().h(px(1.)).bg(divider))
            .child(power_profile_block(
                &upower,
                text_primary,
                text_muted,
                accent,
                hover,
                radius,
                cx,
            ))
            .child(div().w_full().h(px(1.)).bg(divider))
            .child(gaming_mode_block(
                gaming_active,
                text_primary,
                text_muted,
                accent,
                divider,
                radius,
                hover,
                cx,
            ))
    }
}

fn brightness_block(
    brightness: &chronos_services::BrightnessState,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    text_secondary: gpui::Hsla,
    accent: gpui::Hsla,
    bar_track: gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    cx: &mut Context<SystemPopupView>,
) -> AnyElement {
    let available = brightness.available;
    let value = brightness.value;
    let fraction = if available {
        f32::from(value).clamp(0.0, 100.0) / 100.0
    } else {
        0.0
    };
    let fill_w = TRACK_W * fraction;
    let percent_label = if available {
        format!("{value}%")
    } else {
        "n/a".to_string()
    };
    let label_color = if available { text_primary } else { text_muted };
    let value_color = if available {
        text_secondary
    } else {
        text_muted
    };
    let bar_fill = if available { accent } else { text_muted };

    let title_row = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.))
                .child(div().text_color(label_color).child("☀ Brightness")),
        )
        .child(div().text_color(value_color).child(percent_label));

    let track = div()
        .w(px(TRACK_W))
        .h(px(TRACK_H))
        .rounded(radius)
        .bg(bar_track)
        .overflow_hidden()
        .child(div().h_full().w(px(fill_w)).rounded(radius).bg(bar_fill));

    let minus_disabled = !available;
    let plus_disabled = !available;

    let steppers = div().flex().items_center().gap(px(6.)).child(
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .id("brightness-minus")
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(radius)
                    .text_color(if minus_disabled {
                        text_muted
                    } else {
                        text_secondary
                    })
                    .hover(move |s| if !minus_disabled { s.bg(hover) } else { s })
                    .child("−5%")
                    .on_click(move |_event, _window, cx: &mut App| {
                        tracing::info!("system_popup: brightness −5% clicked (available={})", !minus_disabled);
                        if !minus_disabled {
                            AppState::brightness(cx).dispatch(BrightnessCommand::Step(-STEP));
                        }
                    }),
            )
            .child(track)
            .child(
                div()
                    .id("brightness-plus")
                    .cursor_pointer()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(radius)
                    .text_color(if plus_disabled {
                        text_muted
                    } else {
                        text_secondary
                    })
                    .hover(move |s| if !plus_disabled { s.bg(hover) } else { s })
                    .child("+5%")
                    .on_click(move |_event, _window, cx: &mut App| {
                        tracing::info!("system_popup: brightness +5% clicked (available={})", !plus_disabled);
                        if !plus_disabled {
                            AppState::brightness(cx).dispatch(BrightnessCommand::Step(STEP));
                        }
                    }),
            ),
    );

    // Suppress unused-variable warning for `cx` when no listener is needed.
    let _ = cx;

    div()
        .w_full()
        .flex_col()
        .gap(px(8.))
        .px(px(PAD))
        .py(px(10.))
        .child(title_row)
        .child(steppers)
        .into_any_element()
}

fn power_profile_block(
    upower: &UPowerData,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    accent: gpui::Hsla,
    hover: gpui::Hsla,
    radius: gpui::Pixels,
    cx: &mut Context<SystemPopupView>,
) -> AnyElement {
    let current = upower.power_profile;

    // Mockup labels: Quiet / Balanced / Performance.
    // Mapping: Quiet = PowerSaver, Balanced = Balanced, Performance = Performance.
    let segments: [(PowerProfile, &'static str); 3] = [
        (PowerProfile::PowerSaver, "Quiet"),
        (PowerProfile::Balanced, "Balanced"),
        (PowerProfile::Performance, "Performance"),
    ];

    let title = div()
        .w_full()
        .flex()
        .items_center()
        .child(div().text_color(text_primary).child("Power profile"));

    let mut row = div()
        .w_full()
        .flex()
        .rounded(radius)
        .overflow_hidden()
        .border_1()
        .border_color(hover);
    for (profile, label) in segments {
        let is_active = current == profile;
        let bg = if is_active {
            accent
        } else {
            gpui::transparent_black()
        };
        // Текст ПОВЕРХ accent-заливки — через on_fill (не theme.text.*):
        // в Light C text.primary тёмный и на #007acc ещё читается, но
        // STYLE.md запрещает text-токены на насыщенной заливке; dark
        // text.primary == paper-полюс on_fill → пиксель тот же.
        let color = if is_active {
            chronos_ui::on_fill(accent)
        } else {
            text_muted
        };
        let id: SharedString = format!("power-profile-{label}").into();
        row = row.child(
            div()
                .id(id)
                .flex_1()
                .text_center()
                .py(px(6.))
                .text_color(color)
                .bg(bg)
                .cursor_pointer()
                .hover(move |s| if is_active { s } else { s.bg(hover) })
                .child(label)
                .on_click(move |_event, _window, cx: &mut App| {
                    tracing::info!("system_popup: power profile segment clicked: {profile:?}");
                    let upower = AppState::upower(cx).clone();
                    let target = profile;
                    cx.background_spawn(async move {
                        match upower.set_power_profile(target).await {
                            Ok(()) => {
                                tracing::info!("system_popup: set power profile to {target:?}")
                            }
                            Err(e) => {
                                tracing::error!("system_popup: set power profile failed: {e:?}")
                            }
                        }
                    })
                    .detach();
                }),
        );
    }

    let _ = cx;

    div()
        .w_full()
        .flex_col()
        .gap(px(9.))
        .px(px(PAD))
        .py(px(10.))
        .child(title)
        .child(row)
        .into_any_element()
}

fn gaming_mode_block(
    active: bool,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    accent: gpui::Hsla,
    _divider: gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    cx: &mut Context<SystemPopupView>,
) -> AnyElement {
    let title_row = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(div().text_color(text_primary).child("Gaming mode"))
        .child(toggle_switch(active, accent, hover, radius, cx));

    // Effect string — matches the design mockup. "Hide bar/dock" is omitted
    // per the brief (chicken-egg: without the bar you can't reopen the popup
    // to toggle gaming mode off). Tracked as a follow-up TODO.
    let effect = if active {
        "Performance · No animations · No blur · Allow tearing · DND"
    } else {
        "Performance profile · No animations · No blur · Allow tearing · DND"
    };

    let _ = cx;

    div()
        .w_full()
        .flex_col()
        .gap(px(8.))
        .px(px(PAD))
        .py(px(10.))
        .child(title_row)
        .child(div().text_color(text_muted).text_xs().child(effect))
        .into_any_element()
}

/// iOS-style toggle: 34×19 pill, 15px knob, accent track when on.
fn toggle_switch(
    active: bool,
    accent: gpui::Hsla,
    hover: gpui::Hsla,
    radius: gpui::Pixels,
    cx: &mut Context<SystemPopupView>,
) -> AnyElement {
    let track_bg = if active { accent } else { hover };
    let knob_left = if active { px(17.) } else { px(2.) };
    // Кружок лежит ПОВЕРХ трека — контраст считаем от трека, не от схемы:
    // светло-серый кружок пропадал на светлом треке в схеме Light C.
    let knob_color = chronos_ui::on_fill(track_bg);

    let _ = cx;

    div()
        .id("gaming-mode-toggle")
        .w(px(34.))
        .h(px(19.))
        .rounded(px(10.))
        .bg(track_bg)
        .cursor_pointer()
        .hover(move |s| s)
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(knob_left)
                .w(px(15.))
                .h(px(15.))
                .rounded(px(8.))
                .bg(knob_color),
        )
        .on_click(move |_event, _window, cx: &mut App| {
            tracing::info!("system_popup: gaming toggle clicked");
            crate::system_popup::gaming_mode::toggle(cx);
        })
        .into_any_element()
}

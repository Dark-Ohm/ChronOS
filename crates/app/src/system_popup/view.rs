//! System popup view — brightness slider + steppers.
//!
//! T291: the power-profile switch and gaming-mode toggle moved to the System
//! tab (`power_controls.rs`); this popup now shows the header + brightness
//! only. Visual spec: `design/System Popup.dc.html`. Structure mirrors
//! `volume_popup/view.rs` (backdrop blur, Light C watermark + shadow,
//! header + ✕, border_1 + radius_lg).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, BoxShadow, Context, Corners, DragMoveEvent, EmptyView,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, Pixels, Render, SharedString,
    Styled, Window, canvas, div, img, prelude::*, px, rgba, svg,
};

use chronos_services::{BrightnessCommand, Service, UPowerData};
use chronos_ui::{Theme, WindowRootExt, elevation_apply_light_chrome, elevation_blur_layer};
use crate::motion;
use crate::state::AppState;
use crate::system_popup::{close_this, POPUP_WIDTH};

const PAD: f32 = 14.;
const TRACK_H: f32 = 4.;
const THUMB: f32 = 13.;
const STEP: i8 = 5;

/// Drag marker for the brightness slider only (do not reuse volume markers).
pub struct BrightnessSliderDrag;

pub struct SystemPopupView {
    /// Optimistic brightness 0..=100 while UI is ahead of the service snapshot.
    dispatched_brightness: Option<u8>,
    /// Live layout bounds of the brightness track (window coords) for hit→frac.
    track_bounds: Rc<Cell<Bounds<Pixels>>>,
    /// View-driven enter progress 0..=1 (T129).
    enter_t: f32,
}

impl SystemPopupView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        motion::arm_enter_progress(cx, |this, t| {
            this.enter_t = t;
        });
        Self {
            dispatched_brightness: None,
            track_bounds: Rc::new(Cell::new(Bounds::default())),
            enter_t: 0.0,
        }
    }
}

impl Render for SystemPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let brightness = AppState::brightness(cx).get();
        // Service now optimistically sets `value` on Set/Step, so UI preview
        // can clear when it matches — no multi-minute stale re-read storms.
        if self
            .dispatched_brightness
            .is_some_and(|d| d == brightness.value)
        {
            self.dispatched_brightness = None;
        }
        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let divider = theme.border.default;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let hover = theme.interactive.hover;
        let accent = theme.accent.primary;
        let border_subtle = theme.border.subtle;
        let font_mono = theme.font_mono;

        let elev = theme.elevation_popup();
        let blur_layer = elevation_blur_layer(&elev, radius_lg);

        let card = div()
            .window_font(&theme)
            .relative()
            .flex_col()
            .w(px(POPUP_WIDTH))
            .rounded(radius_lg)
            .bg(bg.alpha(0.82))
            .border_1()
            .border_color(border_subtle)
            .shadow(elev.shadows.to_vec())
            .child(blur_layer)
            .overflow_hidden();
        let mut card = elevation_apply_light_chrome(&elev, card);

        let card = card
            .child(header(text_primary, text_muted, hover, radius))
            .child(div().w_full().h(px(1.)).bg(divider))
            .child(brightness_block(
                &brightness,
                self.dispatched_brightness,
                self.track_bounds.clone(),
                text_primary,
                text_muted,
                text_secondary,
                accent,
                hover,
                radius,
                font_mono,
                cx,
            ));

        motion::apply_enter_rise(card, self.enter_t)
    }
}

fn header(
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    hover: gpui::Hsla,
    radius: gpui::Pixels,
) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px(px(PAD))
        .py(px(12.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(text_primary)
                .child("System"),
        )
        .child(
            div()
                .id("system-popup-close")
                .w(px(22.))
                .h(px(22.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_color(text_muted)
                .hover(|s| s.bg(hover))
                .child(img("icons/x.svg").w(px(13.)).h(px(13.)))
                .on_click(|_event, window, cx: &mut App| {
                    close_this(window, cx);
                }),
        )
        .into_any_element()
}

fn brightness_block(
    brightness: &chronos_services::BrightnessState,
    dispatched_brightness: Option<u8>,
    track_bounds: Rc<Cell<Bounds<Pixels>>>,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    text_secondary: gpui::Hsla,
    accent: gpui::Hsla,
    hover: gpui::Hsla,
    radius: gpui::Pixels,
    font_mono: &'static str,
    cx: &mut Context<SystemPopupView>,
) -> AnyElement {
    let available = brightness.available;
    let actual_value = brightness.value;

    // Optimistic value: thumb + label follow the finger; fill can lag on DDC.
    let display_value = dispatched_brightness.unwrap_or(actual_value);

    let fraction = if available {
        f32::from(display_value).clamp(0.0, 100.0) / 100.0
    } else {
        0.0
    };

    // Track width from live layout (between −/+), not full popup width.
    let measured_w = f32::from(track_bounds.get().size.width);
    let track_w = if measured_w > 1.0 {
        measured_w
    } else {
        // First frame before canvas paint — approximate content width.
        POPUP_WIDTH - 2.0 * PAD - 22.0 - 22.0 - 16.0
    };
    let fill_w = track_w * fraction;

    let percent_label = if available {
        format!("{display_value}%")
    } else {
        "n/a".to_string()
    };
    let label_color = if available { text_primary } else { text_muted };
    let value_color = text_muted;
    let bar_fill = if available { text_primary } else { text_muted };
    let track_bg = if available {
        text_muted.alpha(0.3)
    } else {
        text_muted
    };

    let minus_disabled = !available;
    let plus_disabled = !available;

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
                .child(
                    svg()
                        .path("icons/brightness.svg")
                        .size(px(15.))
                        .text_color(text_muted),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(label_color)
                        .child("Brightness"),
                ),
        )
        .child(
            div()
                .font_family(font_mono)
                .text_size(px(11.))
                .text_color(value_color)
                .child(percent_label),
        );

    // ── Slider: click + drag (bounds-local frac, not full-window PAD math) ──
    let slider_id: SharedString = "brightness-slider".into();
    let bounds_for_mouse = track_bounds.clone();
    let mouse_listener = cx.listener(
        move |this: &mut SystemPopupView,
              ev: &MouseDownEvent,
              _window,
              cx: &mut Context<SystemPopupView>| {
            let frac = brightness_frac_from_bounds(f32::from(ev.position.x), &bounds_for_mouse.get());
            set_brightness_from_frac(this, frac, cx);
        },
    );
    let bounds_for_drag = track_bounds.clone();
    let drag_listener = cx.listener(
        move |this: &mut SystemPopupView,
              ev: &DragMoveEvent<BrightnessSliderDrag>,
              _window,
              cx: &mut Context<SystemPopupView>| {
            let frac =
                brightness_frac_from_bounds(f32::from(ev.event.position.x), &bounds_for_drag.get());
            set_brightness_from_frac(this, frac, cx);
        },
    );

    let bounds_cell = track_bounds;
    let slider = div()
        .id(slider_id)
        .flex_1()
        .h(px(THUMB + 8.))
        .flex()
        .items_center()
        .cursor_pointer()
        .relative()
        .child(
            canvas(
                move |bounds, _window, _cx| bounds,
                move |_bounds, captured, _window, _cx| {
                    bounds_cell.set(captured);
                },
            )
            .absolute()
            .size_full(),
        )
        .on_mouse_down(MouseButton::Left, mouse_listener)
        .on_drag(BrightnessSliderDrag, |_, _, _, cx| cx.new(|_| EmptyView))
        .on_drag_move(drag_listener)
        .child(
            div()
                .w_full()
                .h(px(TRACK_H))
                .rounded(px(3.))
                .bg(track_bg)
                .relative()
                .child(
                    div()
                        .absolute()
                        .left(px(0.))
                        .top(px(0.))
                        .bottom(px(0.))
                        .w(px(fill_w.max(0.)))
                        .rounded(px(3.))
                        .bg(bar_fill),
                )
                .child(
                    div()
                        .absolute()
                        .top(px((TRACK_H - THUMB) / 2.))
                        .left(px(fill_w.max(0.) - THUMB / 2.))
                        .size(px(THUMB))
                        .rounded(px(THUMB / 2.))
                        .bg(text_primary),
                ),
        );

    let control_row = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .id("brightness-minus")
                .w(px(22.))
                .h(px(22.))
                .rounded(radius)
                .flex()
                .items_center()
                .justify_center()
                .flex_none()
                .cursor_pointer()
                .text_color(if minus_disabled { text_muted } else { text_secondary })
                .hover(move |s| if !minus_disabled { s.bg(hover) } else { s })
                .child(
                    svg()
                        .path("icons/minus.svg")
                        .size(px(11.))
                        .text_color(if minus_disabled { text_muted } else { text_secondary }),
                )
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    if minus_disabled {
                        return;
                    }
                    // Absolute Set (not Step): service also optimistically
                    // steps from its value — double-step if we dispatch Step.
                    let base = this
                        .dispatched_brightness
                        .unwrap_or(AppState::brightness(cx).get().value);
                    let next = (i32::from(base) - i32::from(STEP)).clamp(0, 100) as u8;
                    this.dispatched_brightness = Some(next);
                    AppState::brightness(cx).dispatch(BrightnessCommand::Set(next));
                    cx.notify();
                })),
        )
        .child(slider)
        .child(
            div()
                .id("brightness-plus")
                .w(px(22.))
                .h(px(22.))
                .rounded(radius)
                .flex()
                .items_center()
                .justify_center()
                .flex_none()
                .cursor_pointer()
                .text_color(if plus_disabled { text_muted } else { text_secondary })
                .hover(move |s| if !plus_disabled { s.bg(hover) } else { s })
                .child(
                    svg()
                        .path("icons/plus.svg")
                        .size(px(11.))
                        .text_color(if plus_disabled { text_muted } else { text_secondary }),
                )
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    if plus_disabled {
                        return;
                    }
                    let base = this
                        .dispatched_brightness
                        .unwrap_or(AppState::brightness(cx).get().value);
                    let next = (i32::from(base) + i32::from(STEP)).clamp(0, 100) as u8;
                    this.dispatched_brightness = Some(next);
                    AppState::brightness(cx).dispatch(BrightnessCommand::Set(next));
                    cx.notify();
                })),
        );

    div()
        .w_full()
        .flex_col()
        .gap(px(8.))
        .px(px(PAD))
        .py(px(14.))
        .child(title_row)
        .child(control_row)
        .into_any_element()
}

/// Pointer window-x → 0..=1 using the **measured track** bounds (between −/+),
/// not the full popup content width (that bug made drag jump and fight the thumb).
fn brightness_frac_from_bounds(x: f32, bounds: &Bounds<Pixels>) -> f64 {
    let left = f32::from(bounds.origin.x);
    let w = f32::from(bounds.size.width);
    if w <= 1.0 {
        return 0.0;
    }
    ((x - left) / w).clamp(0.0, 1.0) as f64
}

/// Set brightness from a slider fraction (0..=1). UI paints optimistically;
/// service coalesces DDC writes (latest-wins) so we can dispatch freely.
fn set_brightness_from_frac(
    this: &mut SystemPopupView,
    frac: f64,
    cx: &mut Context<SystemPopupView>,
) {
    let value = (frac * 100.0).round().clamp(0.0, 100.0) as u8;
    this.dispatched_brightness = Some(value);
    AppState::brightness(cx).dispatch(BrightnessCommand::Set(value));
    cx.notify();
}


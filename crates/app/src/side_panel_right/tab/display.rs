//! Display tab — brightness slider + wallpaper controls (T290).
//!
//! Moved here from the now-deleted system popup (`system_popup::view::
//! brightness_block`) and the right System tab's wallpaper card. Brightness
//! semantics are preserved exactly: slider + % label, latest-wins/debounce
//! dispatch, `AppState::brightness`, no per-sample ddcutil spawn. The tab
//! hosts its OWN brightness subscription (via `state::watch`) so it repaints
//! on service updates — the popup's global watcher is gone with the popup.

use std::cell::Cell;
use std::rc::Rc;

use chronos_services::{BrightnessCommand, BrightnessState, Service, WallpaperState};
use chronos_ui::{Theme, WindowRootExt};
use gpui::{
    AnyElement, App, Bounds, Context, DragMoveEvent, ElementId, IntoElement, MouseButton,
    MouseDownEvent, Pixels, Render, SharedString, Styled, Window, canvas, div, prelude::*, px, svg,
};

use crate::side_panel_right::surfaces;
use crate::state::{self, AppState};

const PAD: f32 = 14.;
const TRACK_H: f32 = 4.;
const THUMB: f32 = 13.;
const STEP: i8 = 5;
/// First-frame fallback for the measured slider track width (before the
/// canvas reports real layout bounds). One-frame approximation only — the
/// `track_bounds` canvas overrides it on the next paint.
const FALLBACK_TRACK_W: f32 = 352.;

/// Drag marker for the brightness slider only (do not reuse volume markers).
pub struct BrightnessSliderDrag;

pub struct DisplayTab {
    /// Optimistic brightness 0..=100 while UI is ahead of the service snapshot.
    dispatched_brightness: Option<u8>,
    /// Live layout bounds of the brightness track (window coords) for hit→frac.
    track_bounds: Rc<Cell<Bounds<Pixels>>>,
    /// Live wallpaper state (mirrors the service; refreshed by `state::watch`).
    wallpaper: WallpaperState,
    /// Whether `waytrogen` is installed (drives the gallery button vs CTA).
    waytrogen_available: bool,
}

impl DisplayTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Host our own brightness subscription — the popup's global watcher is
        // gone, so nothing else repaints us on brightness changes.
        let brightness_signal = AppState::brightness(cx).subscribe();
        state::watch(cx, brightness_signal, |_this: &mut Self, _brightness: BrightnessState, cx| {
            cx.notify();
        });

        let wallpaper_signal = AppState::wallpaper(cx).subscribe();
        state::watch(cx, wallpaper_signal, |this: &mut Self, data: WallpaperState, cx| {
            this.wallpaper = data;
            cx.notify();
        });

        Self {
            dispatched_brightness: None,
            track_bounds: Rc::new(Cell::new(Bounds::default())),
            wallpaper: AppState::wallpaper(cx).get(),
            waytrogen_available: crate::wallpaper_ctl::waytrogen_available(),
        }
    }
}

impl Render for DisplayTab {
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

        div()
            .id("display-tab")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(14.))
            .child(brightness_block(
                &brightness,
                self.dispatched_brightness,
                self.track_bounds.clone(),
                cx,
            ))
            .child(render_wallpaper_card(
                &self.wallpaper,
                self.waytrogen_available,
                cx,
            ))
            .into_any_element()
    }
}

fn brightness_block(
    brightness: &BrightnessState,
    dispatched_brightness: Option<u8>,
    track_bounds: Rc<Cell<Bounds<Pixels>>>,
    cx: &mut Context<DisplayTab>,
) -> AnyElement {
    let theme = *Theme::global(cx);
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let text_secondary = theme.text.secondary;
    let hover = theme.interactive.hover;
    let radius = theme.radius;
    let font_mono = theme.font_mono;

    let available = brightness.available;
    let actual_value = brightness.value;

    // Optimistic value: thumb + label follow the finger; fill can lag on DDC.
    let display_value = dispatched_brightness.unwrap_or(actual_value);

    let fraction = if available {
        f32::from(display_value).clamp(0.0, 100.0) / 100.0
    } else {
        0.0
    };

    // Track width from live layout (between −/+), not full panel width.
    let measured_w = f32::from(track_bounds.get().size.width);
    let track_w = if measured_w > 1.0 {
        measured_w
    } else {
        FALLBACK_TRACK_W
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

    // ── Slider: click + drag (bounds-local frac, not full-panel PAD math) ──
    let slider_id: SharedString = "brightness-slider".into();
    let bounds_for_mouse = track_bounds.clone();
    let mouse_listener = cx.listener(
        move |this: &mut DisplayTab,
              ev: &MouseDownEvent,
              _window,
              cx: &mut Context<DisplayTab>| {
            let frac = brightness_frac_from_bounds(f32::from(ev.position.x), &bounds_for_mouse.get());
            set_brightness_from_frac(this, frac, cx);
        },
    );
    let bounds_for_drag = track_bounds.clone();
    let drag_listener = cx.listener(
        move |this: &mut DisplayTab,
              ev: &DragMoveEvent<BrightnessSliderDrag>,
              _window,
              cx: &mut Context<DisplayTab>| {
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
        .on_drag(BrightnessSliderDrag, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
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
/// not the full panel content width (that bug made drag jump and fight the thumb).
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
fn set_brightness_from_frac(this: &mut DisplayTab, frac: f64, cx: &mut Context<DisplayTab>) {
    let value = (frac * 100.0).round().clamp(0.0, 100.0) as u8;
    this.dispatched_brightness = Some(value);
    AppState::brightness(cx).dispatch(BrightnessCommand::Set(value));
    cx.notify();
}

// ─────────────────────────────────────────────────────────────────────────
// Wallpaper / waytrogen companion card — ported verbatim from the right
// System tab's `wallpaper_card.rs` (single copy now lives here; the right
// System tab no longer renders it).
// ─────────────────────────────────────────────────────────────────────────

/// Wallpaper path display — truncate to basename for the card.
fn wallpaper_label(state: &WallpaperState) -> String {
    match &state.current {
        Some(path) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            // Show parent dir hint if short enough.
            if let Some(parent) = path.parent() {
                let parent_name = parent
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !parent_name.is_empty() && parent_name.len() < 20 {
                    format!("{parent_name}/{name}")
                } else {
                    name
                }
            } else {
                name
            }
        }
        None => "not set".to_string(),
    }
}

pub(crate) fn render_wallpaper_card(
    state: &WallpaperState,
    waytrogen_available: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let label = wallpaper_label(state);

    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(12.))
        .rounded(px(9.))
        .bg(surfaces::card(&theme))
        .border_1()
        .border_color(theme.border.subtle)
        // Title row
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.status.info)
                        .child("Wallpapers"),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(theme.text.muted)
                        .child(label),
                ),
        )
        // Button row
        .child(
            div()
                .flex()
                .gap(px(6.))
                .child(action_button(
                    ElementId::Name(SharedString::from("wallpaper-next")),
                    "Next",
                    &theme,
                    {
                        move |_, _, cx: &mut gpui::App| {
                            tracing::info!("wallpaper_card: Next clicked");
                            crate::wallpaper_ctl::next(cx);
                        }
                    },
                ))
                .when(waytrogen_available, |row| {
                    row.child(action_button(
                        ElementId::Name(SharedString::from("wallpaper-gallery")),
                        "Open waytrogen",
                        &theme,
                        {
                            move |_, _, cx: &mut gpui::App| {
                                tracing::info!("wallpaper_card: Open waytrogen clicked");
                                if let Err(e) = crate::wallpaper_ctl::open_waytrogen_gallery() {
                                    tracing::warn!("wallpaper_card: {e}");
                                    return;
                                }
                                // Delayed resync (same idea as IPC gallery arm).
                                let wallpaper = crate::state::AppState::wallpaper(cx).clone();
                                cx.spawn(async move |cx| {
                                    cx.background_executor()
                                        .timer(std::time::Duration::from_secs(3))
                                        .await;
                                    wallpaper.refresh();
                                })
                                .detach();
                            }
                        },
                    ))
                })
                .when(!waytrogen_available, |row| row.child(install_cta(&theme))),
        )
}

fn action_button(
    id: ElementId,
    label: &'static str,
    theme: &Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .py(px(6.))
        .rounded(px(5.))
        .border_1()
        .border_color(theme.text.disabled)
        .text_size(px(10.))
        .text_color(theme.text.primary)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.border.default))
        .on_click(on_click)
        .child(label)
}

fn install_cta(theme: &Theme) -> impl IntoElement {
    div()
        .flex_1()
        .py(px(6.))
        .rounded(px(5.))
        .border_1()
        .border_color(theme.text.disabled)
        .text_size(px(9.5))
        .text_color(theme.text.muted)
        .flex()
        .items_center()
        .justify_center()
        .child("waytrogen not found — yay -S waytrogen")
}

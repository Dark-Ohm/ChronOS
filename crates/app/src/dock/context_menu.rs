//! Dock context menu — right-click popup on a pinned icon.
//!
//! Simple layer-shell popup with a single "Unpin" item. Follows the
//! `tray_menu` window-lifecycle pattern (Global state, close_this guard).

use std::time::Duration;

use gpui::{
    App, AsyncApp, Bounds, Context, DisplayId, Global, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*,
    px,
};
use gpui_animation::animation::TransitionExt;

use chronos_ui::{Theme, WindowRootExt, elevation_apply_light_chrome, elevation_blur_layer};

use crate::dock::config::DockConfig;
use crate::dock::signal::notify_config_changed;
use crate::motion;

/// Context menu dimensions (px).
const MENU_WIDTH: f32 = 140.;
/// Fixed row height (px) — design `.ci { height: 34px }`.
const ROW_H: f32 = 34.;
/// Top margin — bar height + small gap so popup sits below the bar.
const MENU_MARGIN_TOP: f32 = 36.;
/// Accent-bar geometry — design `.ci::before { top:7px; bottom:7px; width:2px;
/// border-radius:0 2px 2px 0 }`; rest inset (12px → half-height) is the
/// scaleY(.5) stand-in (fork has no element `scale()`).
const BAR_TOP: f32 = 7.;
const BAR_REST_INSET: f32 = 12.;

/// Global state for the dock context menu popup.
#[derive(Default)]
pub struct DockMenuState {
    /// Window handle while the menu is open; `None` when closed.
    handle: Option<WindowHandle<DockMenuView>>,
    /// The entry id that was right-clicked (for unpin action).
    entry_id: Option<String>,
    /// Generation guard for auto-close.
    close_generation: u64,
}

impl Global for DockMenuState {}

impl DockMenuState {
    pub fn entry_id(&self) -> Option<&str> {
        self.entry_id.as_deref()
    }

    /// Test-only: stamp `entry_id` without opening a window (avoids Theme/Wayland).
    #[cfg(test)]
    pub fn set_entry_id_for_test(&mut self, id: Option<String>) {
        self.entry_id = id;
    }
}

pub struct DockMenuView {
    /// View-driven enter progress 0..=1 (anchored popups — `with_animation`
    /// is invisible on live Hyprland; see `motion::arm_enter_progress`).
    enter_t: f32,
}

impl DockMenuView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Menu enter follows the reference `ctx-in` curve (`cubic-bezier(.2,.8,.2,1)`,
        // `.12s`) — the popups' EaseOutBack overshoot would feel out of place here.
        motion::arm_enter_progress_with(
            cx,
            Duration::from_millis(motion::MENU_ENTER_MS),
            motion::ease_menu_enter,
            |view, t| {
                view.enter_t = t;
            },
        );
        cx.set_global(crate::dock::signal::DockMenuHoverSignal(false));
        Self { enter_t: 0.0 }
    }
}

impl Render for DockMenuView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let entry_id = cx.global::<DockMenuState>().entry_id.clone();

        let Some(_entry_id) = entry_id else {
            return div().into_any_element();
        };

        // One popup component with the tray menu: same elevated-surface shell
        // (same bg, radius, border, depth blur/shadow, Light-C chrome) so the
        // dock context menu doesn't read as a different widget. Content stays a
        // single "Unpin" item.
        let bg = theme.bg.primary;
        let text = theme.text.primary;
        let hover_bg = theme.interactive.hover;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let border_subtle = theme.border.subtle;
        let accent = theme.accent.primary;
        let hovered = cx.global::<crate::dock::signal::DockMenuHoverSignal>().0;

        let elev = theme.elevation_popup();
        let blur_layer = elevation_blur_layer(&elev, radius_lg);

        let mut card = div()
            .window_font(theme)
            .relative()
            .flex_col()
            .w(px(MENU_WIDTH))
            .rounded(radius_lg)
            .bg(bg.alpha(0.94))
            .border_1()
            .border_color(border_subtle)
            .shadow(elev.shadows.to_vec())
            .overflow_hidden();
        card = elevation_apply_light_chrome(&elev, card);

        // Enter-animation (view-driven — anchored popups don't animate on map).
        //
        // Row: the 2px accent-bar is ALWAYS present (hidden rest state in the
        // base chain — no flash) and morphs via `transition_when_else`; the
        // hover wash morphs the same way (design `.ci` `transition: background
        // .12s ease`). Hover state lives in the `DockMenuHoverSignal` global:
        // `on_hover` here only gets `&mut App`, so it writes the global and
        // marks this window's view dirty to flip the condition.
        motion::apply_enter_menu(
            card.child(blur_layer).child(
                div()
                    .id("dock-menu-unpin")
                    .w_full()
                    .h(px(ROW_H))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(10.))
                    .rounded(radius)
                    .relative()
                    .child(
                        div()
                            .id("dock-menu-bar")
                            .absolute()
                            .left(px(0.))
                            .w(px(2.))
                            .rounded_tr(px(2.))
                            .rounded_br(px(2.))
                            .bg(accent)
                            .opacity(0.0)
                            .top(px(BAR_REST_INSET))
                            .bottom(px(BAR_REST_INSET))
                            .with_transition("dock-menu-bar")
                            .transition_when_else(
                                hovered,
                                Duration::from_millis(motion::MENU_ENTER_MS),
                                motion::MenuEase,
                                |s| s.opacity(1.0).top(px(BAR_TOP)).bottom(px(BAR_TOP)),
                                |s| {
                                    s.opacity(0.0)
                                        .top(px(BAR_REST_INSET))
                                        .bottom(px(BAR_REST_INSET))
                                },
                            ),
                    )
                    .child(div().text_sm().text_color(text).child("Unpin"))
                    .cursor_pointer()
                    .bg(gpui::transparent_black())
                    .with_transition("dock-menu-unpin")
                    .on_hover(|hovered, _window, cx: &mut App| {
                        cx.set_global(crate::dock::signal::DockMenuHoverSignal(*hovered));
                        // Flip the accent-bar condition. `on_hover` here only
                        // gets `&mut App`, and `window.current_view()` panics
                        // outside paint/prepaint — so we use the same
                        // `refresh_windows` the gpui_animation tick uses to
                        // drive re-renders: the next draw re-renders this
                        // window's root view, `render` reads the flipped
                        // global, and `transition_when_else` can animate.
                        cx.refresh_windows();
                    })
                    .transition_when_else(
                        hovered,
                        Duration::from_millis(motion::MENU_ENTER_MS),
                        motion::MenuEase,
                        move |s| s.bg(hover_bg),
                        |s| s.bg(gpui::transparent_black()),
                    )
                    .on_click(move |_event, window, cx: &mut App| {
                        // Read entry_id from global before clearing.
                        let id = cx
                            .global::<DockMenuState>()
                            .entry_id
                            .clone()
                            .unwrap_or_default();

                        // Clear global state.
                        {
                            let state = cx.global_mut::<DockMenuState>();
                            state.entry_id = None;
                            state.close_generation = state.close_generation.wrapping_add(1);
                        }

                        // Unpin: remove from config, save, rebuild dock.
                        let mut config = DockConfig::load();
                        config.unpin(&id);
                        if let Err(e) = config.save() {
                            tracing::error!("dock: failed to save config after unpin: {e}");
                        }

                        // Update the cached config.
                        crate::dock::config::update_cache(config);

                        // Notify dock views to rebuild.
                        notify_config_changed(cx);

                        // Close popup.
                        window.remove_window();
                    }),
            ),
            self.enter_t,
        )
        .into_any_element()
    }
}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display_id_or_primary(cx)
}

/// Layer-shell options for the context menu: centered horizontally,
/// anchored TOP, positioned just below the bar.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(
                (display_size.width - px(MENU_WIDTH)) / 2.,
                px(MENU_MARGIN_TOP),
            ),
            size: Size::new(px(MENU_WIDTH), px(ROW_H)),
        })),
        app_id: Some("chronos-dock-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "dock-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP,
            exclusive_zone: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the context menu for `entry_id`. If already open for the same entry,
/// close it (toggle). If open for a different entry, switch.
pub fn open(cx: &mut App, entry_id: String) {
    let already = cx
        .global::<DockMenuState>()
        .entry_id
        .as_ref()
        .map(|s| *s == entry_id)
        .unwrap_or(false);
    if already {
        close(cx);
        return;
    }

    let state = cx.global_mut::<DockMenuState>();
    state.entry_id = Some(entry_id);
    state.close_generation = state.close_generation.wrapping_add(1);
    let generation = state.close_generation;
    drop(state);

    let handle = cx.global::<DockMenuState>().handle.clone();
    match handle {
        Some(existing) => {
            let _ = existing.update(cx, |_, _window, view_cx| {
                view_cx.notify();
            });
        }
        None => {
            let display_id = pick_display(cx);
            match cx.open_window(window_options(display_id, cx), |_, app_cx| {
                app_cx.new(|view_cx| DockMenuView::new(view_cx))
            }) {
                Ok(new_handle) => {
                    cx.global_mut::<DockMenuState>().handle = Some(new_handle);
                }
                Err(err) => tracing::warn!("dock context menu: failed to open: {err}"),
            }
        }
    }

    schedule_autoclose(cx, generation);
}

/// Close the context menu (clears state + destroys window).
pub fn close(cx: &mut App) {
    let state = cx.global_mut::<DockMenuState>();
    state.entry_id = None;
    state.close_generation = state.close_generation.wrapping_add(1);
    if let Some(handle) = state.handle.take() {
        let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
    }
}

/// Auto-close after 5 seconds (shorter than tray_menu — small menu).
fn schedule_autoclose(cx: &mut App, generation: u64) {
    cx.spawn(async move |app_cx: &mut AsyncApp| {
        app_cx
            .background_executor()
            .timer(Duration::from_secs(5))
            .await;
        app_cx.update(|app_cx| {
            if app_cx.global::<DockMenuState>().close_generation != generation {
                return;
            }
            close(app_cx);
        });
    })
    .detach();
}

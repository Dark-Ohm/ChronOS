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

use chronos_ui::Theme;

use crate::dock::config::DockConfig;
use crate::dock::signal::notify_config_changed;

/// Context menu dimensions (px).
const MENU_WIDTH: f32 = 140.;
const MENU_HEIGHT: f32 = 40.;
/// Top margin — bar height + small gap so popup sits below the bar.
const MENU_MARGIN_TOP: f32 = 36.;

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
}

pub struct DockMenuView;

impl DockMenuView {
    pub fn new(_cx: &mut App) -> Self {
        Self
    }
}

impl Render for DockMenuView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let entry_id = cx.global::<DockMenuState>().entry_id.clone();

        let Some(_entry_id) = entry_id else {
            return div().into_any_element();
        };

        let bg = theme.bg.elevated;
        let text = theme.text.primary;
        let hover_bg = theme.interactive.hover;
        let radius = theme.radius;

        div()
            .flex_col()
            .w(px(MENU_WIDTH))
            .h(px(MENU_HEIGHT))
            .rounded(radius)
            .bg(bg)
            .child(
                div()
                    .id("dock-menu-unpin")
                    .w_full()
                    .h_full()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .rounded(radius)
                    .cursor_pointer()
                    .hover(|s| s.bg(hover_bg))
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
                            state.close_generation = state
                                .close_generation
                                .wrapping_add(1);
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
                    })
                    .child(div().text_sm().text_color(text).child("Unpin")),
            )
            .into_any_element()
    }
}

fn pick_display(cx: &App) -> Option<DisplayId> {
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}

/// Layer-shell options for the context menu: centered horizontally,
/// anchored TOP, positioned just below the bar.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
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
            size: Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
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

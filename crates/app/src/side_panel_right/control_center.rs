//! Control-center popup (T305) — the 8 settings tabs plus the new Media tab
//! live in ONE anchored slide-popup opened from the right rail's icons.
//!
//! Owner's revision of T305 decision #1 (2026-08-18): the settings icons
//! STAY on the rail as entry points — clicking one opens this popup on that
//! tab instead of switching the panel's content. The rail never creates a
//! content panel for them; `TabContent` entities for these tabs exist only
//! here (decision #2 — no entity sharing between rail and popup).
//!
//! Window: a plain layer-shell `Overlay` (TOP|RIGHT, never exclusive, no
//! keyboard grab) positioned by margins computed from the icon's **live**
//! bounds captured at click time — no cached geometry, no `window.bounds()`
//! (the centered-window trap). Enter motion is view-driven
//! (`motion::arm_enter_progress` + `apply_enter_from_right`) — the fork's
//! `with_animation`/`transition_when` don't animate anchored popups on live
//! Hyprland (motion.rs:11-14).
//!
//! The popup closes when the rail un-maps (both `close` and `close_this` in
//! `side_panel_right/mod.rs` hook here) and when the same icon is re-clicked
//! (toggle). Clicking a *different* settings icon remaps the popup (margins
//! are creation-time state — same destroy-and-remap as `tray_menu::toggle`).

use std::collections::HashMap;

use gpui::{
    App, Bounds, Context, Global, Pixels, Size, Subscription, WeakEntity, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    div, layer_shell::*,
    point,
    prelude::*,
    px, svg,
};

use gpui_component::Root;

use chronos_ui::{Theme, WindowRootExt};

use crate::motion;
use crate::side_panel_right::surfaces;
use crate::side_panel_right::tab::TabContent;
use crate::side_panel_right::tabs::PanelTab;
use crate::side_panel_right::{RAIL_WIDTH, panel_edge_gap};

/// Gap (px) between the rail and the popup card.
const POPUP_GAP: f32 = 8.;
/// Fixed popup height (px). Content scrolls internally
/// (`overflow_y_scroll` on the body) — never resize this window vertically.
const POPUP_HEIGHT: f32 = 560.;
/// Width bounds (px) — derived from the hosted tabs' `preferred_content_width`
/// (320 for binds/ACP … 440 for Display), clamped to keep the card compact.
const MIN_POPUP_W: f32 = 320.;
const MAX_POPUP_W: f32 = 440.;

/// The tabs owned by the control-center popup (entry icons stay on the rail —
/// owner's revision of T305 decision #1).
pub(crate) fn is_popup_tab(tab: PanelTab) -> bool {
    matches!(
        tab,
        PanelTab::System
            | PanelTab::Media
            | PanelTab::Updates
            | PanelTab::Notifications
            | PanelTab::Display
            | PanelTab::EditorSettings
            | PanelTab::HyprlandBinds
            | PanelTab::AcpSettings
            | PanelTab::LauncherSettings
    )
}

/// Tab-bar order inside the popup — reference mapping (Dashboard / Media /
/// Performance / Workspaces) over the actual tab set.
pub(crate) const POPUP_TABS: [PanelTab; 9] = [
    PanelTab::System,
    PanelTab::Media,
    PanelTab::Updates,
    PanelTab::Notifications,
    PanelTab::Display,
    PanelTab::EditorSettings,
    PanelTab::HyprlandBinds,
    PanelTab::AcpSettings,
    PanelTab::LauncherSettings,
];

fn popup_width_for(tab: PanelTab) -> f32 {
    tab.preferred_content_width().clamp(MIN_POPUP_W, MAX_POPUP_W)
}

/// Global state for the control-center popup.
#[derive(Default)]
pub(crate) struct ControlCenterState {
    handle: Option<WindowHandle<Root>>,
    /// Weak handle to the live view, kept so the window-closed subscription
    /// can be cleared and the rail can read `active_tab` for its highlight.
    view: Option<WeakEntity<ControlCenterView>>,
    /// Tab currently shown in the popup (valid only while `handle` is set).
    active_tab: PanelTab,
    /// Clears stale state when the compositor destroys the window.
    window_closed_subscription: Option<Subscription>,
}

impl Global for ControlCenterState {}

pub(crate) fn init(cx: &mut App) {
    cx.set_global(ControlCenterState::default());
    tracing::info!("control_center: initialized");
}

/// The popup's currently open tab, if the popup is open — drives the rail's
/// active-icon highlight.
pub(crate) fn active_tab(cx: &App) -> Option<PanelTab> {
    let state = cx.global::<ControlCenterState>();
    state.handle.as_ref().map(|_| state.active_tab)
}

fn window_options(
    display_id: Option<gpui::DisplayId>,
    anchor: Bounds<Pixels>,
    width: f32,
) -> WindowOptions {
    // T311 D3: the right margin is the per-side wrap inset, not the
    // generic thickness — collapses to 0 when the right rail is mapped,
    // stays at full `wrap.thickness` when the rail is gone.
    let inset = crate::frame::wrap_inset_right_cached(crate::frame::rail_mapped(
        crate::frame::FrameSide::Right,
    ));
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(width), px(POPUP_HEIGHT)),
        })),
        app_id: Some("chronos-control-center".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "control_center".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            // `-1` is the wlr-layer-shell opt-out: the rail reserves its
            // width as an exclusive zone on this edge, and without `-1` the
            // compositor auto-offsets this popup by that reservation ON TOP
            // of the explicit margin below (double offset — the exact trap
            // documented on `content_window_options`). Never reserves space
            // itself (popups must not push clients).
            exclusive_zone: Some(px(-1.)),
            // Live geometry: the rail's top is the bar's exclusive zone
            // (`panel_edge_gap`), and `anchor` is the clicked icon's bounds
            // captured in the rail window at click time. Right margin = frame
            // inset + rail width + a breathing gap — the card sits flush left
            // of the rail, aligned with the icon.
            margin: Some((
                px(panel_edge_gap() + f32::from(anchor.origin.y)),
                px(inset + RAIL_WIDTH + POPUP_GAP),
                px(0.),
                px(0.),
            )),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the popup on `tab`, anchored to the rail icon's live bounds. No-op if
/// already open (use [`toggle`] for same/different-tab semantics).
pub(crate) fn open(bounds: Bounds<Pixels>, tab: PanelTab, cx: &mut App) {
    if cx.global::<ControlCenterState>().handle.is_some() {
        return;
    }
    let width = popup_width_for(tab);
    let display_id = crate::monitor::pult_display_id_or_primary(cx);
    let mut opened_view: Option<WeakEntity<ControlCenterView>> = None;
    let result = cx.open_window(window_options(display_id, bounds, width), |window, view_cx| {
        let view = view_cx.new(|view_cx| ControlCenterView::new(tab, view_cx));
        opened_view = Some(view.downgrade());
        view_cx.new(|view_cx| {
            Root::new(view, window, view_cx)
                .bordered(false)
                .bg(gpui::transparent_black())
        })
    });

    match result {
        Ok(handle) => {
            let window_id = handle.window_id();
            let window_closed_subscription = cx.on_window_closed(move |cx, closed_id| {
                if closed_id != window_id {
                    return;
                }
                let state = cx.global_mut::<ControlCenterState>();
                state.handle = None;
                state.view = None;
                state.window_closed_subscription = None;
            });
            let state = cx.global_mut::<ControlCenterState>();
            state.handle = Some(handle);
            state.view = opened_view;
            state.active_tab = tab;
            state.window_closed_subscription = Some(window_closed_subscription);
            tracing::info!(tab = tab.label(), "control_center: popup opened");
        }
        Err(err) => tracing::warn!("control_center: failed to open popup: {err}"),
    }
    cx.refresh_windows();
}

/// Close the popup (clears state + destroys the window). Safe from contexts
/// that do NOT already hold `&mut Window` for the popup.
pub(crate) fn close(cx: &mut App) {
    let state = cx.global_mut::<ControlCenterState>();
    let handle = state.handle.take();
    state.view = None;
    state.window_closed_subscription = None;
    if let Some(handle) = handle {
        let result = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
        if let Err(e) = result {
            tracing::warn!(
                "control_center: close remove_window failed ({e}) — possible ghost"
            );
        }
    }
    cx.refresh_windows();
}

/// Close the popup from inside a callback that already holds `&mut Window`
/// for this popup's window-id (reentrancy guard, `ARCHITECTURE.md §4.1`).
#[allow(dead_code)] // reserved for a future in-popup ✕ control
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<ControlCenterState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        let state = cx.global_mut::<ControlCenterState>();
        state.handle.take();
        state.view = None;
        state.window_closed_subscription = None;
    }
    window.remove_window();
}

/// Rail-icon entry: same icon re-click closes; a different settings icon
/// remaps the popup onto it (margins are creation-time state).
pub(crate) fn toggle(bounds: Bounds<Pixels>, tab: PanelTab, cx: &mut App) {
    let state = cx.global::<ControlCenterState>();
    let (is_open, open_tab) = (state.handle.is_some(), state.active_tab);
    match (is_open, open_tab == tab) {
        (true, true) => close(cx),
        (true, false) => {
            close(cx);
            open(bounds, tab, cx);
        }
        (false, _) => open(bounds, tab, cx),
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub struct ControlCenterView {
    active_tab: PanelTab,
    /// Width of the card/window (px) — follows the active tab's preferred
    /// width, resized on tab switch (mirrors the right panel's per-tab width).
    width: f32,
    /// Lazy tab-content cache — the popup's exclusive entity instances
    /// (T305 decision #2: one per tab, created here, dropped with the popup).
    tabs: HashMap<PanelTab, TabContent>,
    /// View-driven enter progress 0..=1 (motion.rs — anchored popups).
    enter_t: f32,
}

impl ControlCenterView {
    pub fn new(tab: PanelTab, cx: &mut Context<Self>) -> Self {
        motion::arm_enter_progress(cx, |this, t| {
            this.enter_t = t;
        });
        Self {
            active_tab: tab,
            width: popup_width_for(tab),
            tabs: HashMap::new(),
            enter_t: 0.0,
        }
    }

    fn ensure_tab(&mut self, tab: PanelTab, cx: &mut Context<Self>) -> TabContent {
        self.tabs
            .entry(tab)
            .or_insert_with(|| TabContent::create(tab, cx))
            .clone()
    }

    fn on_tab_click(&mut self, tab: PanelTab, window: &mut Window, cx: &mut Context<Self>) {
        if tab == self.active_tab {
            return;
        }
        self.active_tab = tab;
        self.width = popup_width_for(tab);
        // The window is resized to the tab's preferred width (per-tab width
        // canon from the right panel) — height never changes.
        window.resize(Size::new(px(self.width), px(POPUP_HEIGHT)));
        cx.global_mut::<ControlCenterState>().active_tab = tab;
        cx.notify();
        tracing::info!(tab = tab.label(), "control_center: switched tab");
    }
}

/// Render a cached `TabContent` as an element — `TabContent` is a registry
/// (T304 invariant), not itself `IntoElement`; each variant holds the entity.
fn render_tab_content(content: &TabContent) -> gpui::AnyElement {
    match content {
        TabContent::System(e) => e.clone().into_any_element(),
        TabContent::Files(e) => e.clone().into_any_element(),
        TabContent::Terminal(e) => e.clone().into_any_element(),
        TabContent::Build(e) => e.clone().into_any_element(),
        TabContent::Preview(e) => e.clone().into_any_element(),
        TabContent::Library(e) => e.clone().into_any_element(),
        TabContent::HyprBinds(e) => e.clone().into_any_element(),
        TabContent::BarSettings(e) => e.clone().into_any_element(),
        TabContent::AcpSettings(e) => e.clone().into_any_element(),
        TabContent::Display(e) => e.clone().into_any_element(),
        TabContent::Updates(e) => e.clone().into_any_element(),
        TabContent::Notifications(e) => e.clone().into_any_element(),
        TabContent::LauncherSettings(e) => e.clone().into_any_element(),
        TabContent::Media(e) => e.clone().into_any_element(),
        TabContent::Placeholder(e) => e.clone().into_any_element(),
    }
}

impl Render for ControlCenterView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let active = self.active_tab;
        let content = self.ensure_tab(active, cx);
        let elev = theme.elevation_popup();

        let card = div()
            .window_font(&theme)
            .relative()
            .flex()
            .flex_col()
            .w(px(self.width))
            .h_full()
            .rounded(theme.radius_lg)
            .bg(theme.surface_color(surfaces::content(&theme)))
            .border_1()
            .border_color(theme.border.subtle)
            .shadow(elev.shadows.to_vec())
            .overflow_hidden()
            .child(tab_bar(active, &theme, cx))
            // Body scrolls internally (fixed window height — the footer
            // clip trap; `overflow_y_scroll` needs a stateful id).
            .child(
                div()
                    .id("control-center-body")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(render_tab_content(&content)),
            );

        // Slide in from the rail edge (right) — the fork's popup enter canon.
        motion::apply_enter_from_right(card, self.enter_t)
    }
}

/// Horizontal icon row switching the popup's tab (reference tab bar).
fn tab_bar(active: PanelTab, theme: &Theme, cx: &Context<ControlCenterView>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .px(px(10.))
        .py(px(8.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .children(POPUP_TABS.iter().map(move |&tab| {
            let is_active = tab == active;
            let listener = cx.listener(move |this, _ev: &gpui::ClickEvent, window, cx| {
                this.on_tab_click(tab, window, cx);
            });
            div()
                .id(("control-center-tab", tab as usize))
                .size(px(26.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .bg(if is_active {
                    theme.interactive.hover
                } else {
                    gpui::transparent_black()
                })
                .hover(|s| s.bg(theme.interactive.hover))
                .on_click(listener)
                .child(
                    svg()
                        .path(tab.icon_path())
                        .size(px(16.))
                        .text_color(if is_active {
                            theme.text.primary
                        } else {
                            theme.text.muted
                        }),
                )
        }))
}

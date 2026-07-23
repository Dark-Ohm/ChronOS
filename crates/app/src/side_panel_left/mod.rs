mod state;
mod panel;
pub mod sessions_list;
mod chat_view;
mod composer;
mod tool_card;

pub use state::{PanelState, SidePanelLeftState};

use chronos_luau::bar::BAR_HEIGHT;
use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

const PANEL_WIDTH: f32 = 352.;
const PANEL_EDGE_GAP: f32 = BAR_HEIGHT;

#[derive(Default)]
pub struct SidePanelLeftState_ {
    handle: Option<WindowHandle<SidePanelLeft>>,
    pinned: bool,
    peek_generation: u64,
}

impl Global for SidePanelLeftState_ {}

fn display_height(display_id: Option<DisplayId>, cx: &App) -> f32 {
    display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1080.)
}

fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_h = display_height(display_id, cx);
    let panel_h = (display_h - PANEL_EDGE_GAP).max(100.);
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(PANEL_WIDTH), px(panel_h)),
        })),
        app_id: Some("chronos-side-panel-left".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_left".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::LEFT | Anchor::TOP,
            exclusive_zone: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub struct SidePanelLeft {
    state: state::SidePanelLeftState,
    sessions: Vec<sessions_list::SessionItem>,
}

impl Render for SidePanelLeft {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        panel::render_panel(self, _window, cx)
    }
}

impl SidePanelLeft {
    fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            state: state::SidePanelLeftState::new(),
            sessions: Vec::new(),
        }
    }

    fn toggle_collapse(&mut self, cx: &mut Context<Self>) {
        self.state.sessions_collapsed = !self.state.sessions_collapsed;
        cx.notify();
    }

    fn create_new_session(&mut self, cx: &mut Context<Self>) {
        let id = uuid::Uuid::new_v4().to_string();
        let title = format!("Session {}", self.sessions.len() + 1);
        self.sessions.push(sessions_list::SessionItem {
            id: id.clone(),
            title,
            active: true,
        });
        // Deactivate previous active sessions
        for s in self.sessions.iter_mut().rev().skip(1) {
            s.active = false;
        }
        self.state.active_session_id = Some(id);
        cx.notify();
    }

    fn select_session(&mut self, session_id: &str, cx: &mut Context<Self>) {
        for s in &mut self.sessions {
            s.active = s.id == session_id;
        }
        self.state.active_session_id = Some(session_id.to_string());
        cx.notify();
    }
}

fn open_window(cx: &mut App, pinned: bool) {
    if cx.global::<SidePanelLeftState_>().handle.is_some() {
        if pinned {
            cx.global_mut::<SidePanelLeftState_>().pinned = true;
            tracing::info!("side_panel_left: upgraded peek → pinned");
        }
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    match cx.open_window(window_options(display_id, cx), |_, view_cx| {
        view_cx.new(|cx| SidePanelLeft::new(cx))
    }) {
        Ok(handle) => {
            let state = cx.global_mut::<SidePanelLeftState_>();
            state.handle = Some(handle);
            state.pinned = pinned;
            tracing::info!(
                "side_panel_left: opened ({})",
                if pinned { "pinned" } else { "peek" }
            );
        }
        Err(err) => tracing::warn!(
            "side_panel_left: failed to open ({}): {err}",
            if pinned { "pinned" } else { "peek" }
        ),
    }
}

pub fn open_pinned(cx: &mut App) {
    open_window(cx, true);
}

pub fn open_peek(cx: &mut App) {
    open_window(cx, false);
}

pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<SidePanelLeftState_>().handle.take() {
        cx.global_mut::<SidePanelLeftState_>().pinned = false;
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => {
                tracing::info!("side_panel_left: closed");
            }
            Err(e) => tracing::warn!(
                "side_panel_left: close() could not reach the window ({e}) — possible ghost"
            ),
        }
    } else {
        cx.global_mut::<SidePanelLeftState_>().pinned = false;
    }
}

pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<SidePanelLeftState_>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        let state = cx.global_mut::<SidePanelLeftState_>();
        state.handle.take();
        state.pinned = false;
    }
    window.remove_window();
    tracing::info!("side_panel_left: close_this");
}

pub fn toggle(_window: &mut Window, cx: &mut App) {
    if cx.global::<SidePanelLeftState_>().handle.is_some() {
        close(cx);
    } else {
        open_pinned(cx);
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(SidePanelLeftState_::default());
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
        cx.update(|cx| {
            if std::env::var_os("CHRONOS_SMOKE_SIDE_PANEL_LEFT").is_some() {
                open_pinned(cx);
            }
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_starts_as_peek() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.state, PanelState::Peek);
    }

    #[test]
    fn state_default_width() {
        let state = state::SidePanelLeftState::new();
        assert_eq!(state.width, 352.0);
    }
}

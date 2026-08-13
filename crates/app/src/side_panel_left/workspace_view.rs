//! T278 / Slice A1 — the content window's root view.
//!
//! Mirrors `side_panel_right::view::SidePanelRightView`'s role (T276):
//! the entity that owns the fixed-canvas content surface, sets the
//! Wayland input region, hosts the product body, and runs the resize
//! handle. The rail (`rail_view::RailView`) lives in a separate window
//! and reaches this view through a `WeakEntity` stored on
//! `SidePanelLeftState_`.
//!
//! Resize contract (spec §3.2): the visible slice is **left-aligned**
//! inside the 920 px canvas (x = 0), the input region matches it, and a
//! transparent 4 px grab sits on the outer (right) edge of the visible
//! slice at `resize_handle_x(visible_w)`. `window.resize()` is forbidden
//! on this surface — drag only mutates `SidePanelLeftState_.panel_width`
//! and re-issues `set_input_region` on the next paint.

use gpui::{
    App, Bounds, Context, Entity, IntoElement, Render, Subscription, WeakEntity, Window,
    div, prelude::*, px,
};

use chronos_ui::Theme;

use crate::side_panel_left::SidePanelLeft;
use crate::side_panel_left::state::geometry;
use crate::side_panel_left::tabs::{
    CONTENT_CANVAS_WIDTH, MAX_PANEL_WIDTH, RAIL_WIDTH, RESIZE_HANDLE_WIDTH,
};
use crate::side_panel_left::LeftPanelResize;

/// The content window's root view. Hosts the legacy `SidePanelLeft`
/// product body as a child entity; that body still owns chat history,
/// composer state, ACP/Hermes client, etc.
pub struct WorkspaceView {
    /// T278: the legacy product-state child. It no longer owns a
    /// `WindowHandle`, width, dock, exclusive zone, or resize — it is
    /// rendered as a sub-element of this view, with state mirrored from
    /// `SidePanelLeftState_` on every render.
    pub(crate) content: Entity<SidePanelLeft>,
    /// Cache of the last `interactive_w` pushed to `set_input_region`.
    /// Only re-issues when it changes — avoids a Wayland round-trip per
    /// paint (T276 pattern).
    last_visible_width: Option<f32>,
    /// Resize drag bookkeeping — `start_x` is the pointer x at mouse-
    /// down (inside the fixed canvas), `start_width` is the panel width
    /// at that moment. Both are `Some` only while a drag is active.
    resize_start_x: Option<f32>,
    resize_start_width: Option<f32>,
    /// Set by `focus_composer_pending` (called from IPC paths that don't
    /// have a `&mut Window` in scope). Consumed by `render`, which holds
    /// the window and can call `window.focus(...)` directly.
    focus_composer_pending: bool,
    /// Held only to keep the subscription alive; not read.
    _sub: Subscription,
}

impl WorkspaceView {
    pub fn new(content: Entity<SidePanelLeft>, cx: &mut Context<Self>) -> Self {
        // Mirror SoT → product child whenever either side changes. Side
        // updates (dock toggle, width change, project switch) fire
        // `cx.notify()`; the subscription here repaints content + this
        // view together.
        let sub = cx.observe(&content, |_, _, cx| cx.notify());
        Self {
            content,
            last_visible_width: None,
            resize_start_x: None,
            resize_start_width: None,
            focus_composer_pending: false,
            _sub: sub,
        }
    }

    /// Read the panel width currently rendered. Exposed for tests and
    /// for IPC paths that need to read the live width without re-querying
    /// the global.
    pub fn panel_width(&self, cx: &App) -> f32 {
        cx.global::<crate::side_panel_left::SidePanelLeftState_>()
            .panel_width
    }

    /// Set panel width and mirror it into the legacy child. Used by IPC
    /// (`expand_with_composer`, `compose_and_send`) to dock the chat
    /// column at the remembered/preferred width without reaching through
    /// a window handle.
    pub fn set_panel_width(
        &mut self,
        new_width: f32,
        dock: bool,
        cx: &mut Context<Self>,
    ) {
        // Read the resulting panel_width + dock off the global before
        // mutating the child — `self.content.update(cx, ...)` borrows cx
        // mutably for the duration of the closure, and we cannot touch
        // `cx.global(...)` again until that borrow ends.
        let (panel_width, dock_value) = {
            let state = cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>();
            state.ensure_content_width(new_width);
            state.dock_content = dock;
            state.last_exclusive_zone = None;
            (state.panel_width, state.dock_content)
        };
        self.content.update(cx, |child, _cx| {
            child.state.width = panel_width;
            child.state.dock_chat = dock_value;
            child.state.remembered_chat_width = Some(panel_width);
        });
        cx.notify();
    }

    /// Schedule a composer focus on the next render. Used by IPC paths
    /// (`expand_with_composer`, `compose_and_send`) that run in `App`
    /// context with no `&mut Window` in scope. The render path holds
    /// the window and calls `window.focus(...)` directly.
    pub fn request_focus_composer(&mut self, cx: &mut Context<Self>) {
        self.content.update(cx, |child, _cx| {
            child.composer_focused = true;
        });
        self.focus_composer_pending = true;
        cx.notify();
    }

    fn perform_focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus_composer_pending {
            return;
        }
        self.focus_composer_pending = false;
        self.content.update(cx, |child, cx| {
            window.focus(&child.composer_focus, cx);
            child.start_blink(cx);
        });
    }

    fn start_resize(&mut self, start_x: f32, cx: &mut Context<Self>) {
        cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>().resizing = true;
        let width_now = cx.global::<crate::side_panel_left::SidePanelLeftState_>().panel_width;
        self.resize_start_x = Some(start_x);
        self.resize_start_width = Some(width_now);
        tracing::debug!(
            grab_x = start_x,
            start_w = width_now,
            "side_panel_left: resize drag started"
        );
    }

    fn update_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let (Some(start_x), Some(start_width)) = (self.resize_start_x, self.resize_start_width)
        else {
            return;
        };
        let target = geometry::resize_target_width(start_width, start_x, current_x);
        let state = cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>();
        let old_w = state.panel_width;
        state.resize(target);
        if state.panel_width >= crate::side_panel_left::tabs::SOFT_OPEN_MIN_WIDTH {
            if let Some(tab) = resizable_active(state.active_tab) {
                state.remembered_widths.set(tab, state.panel_width);
            }
        }
        let new_w = state.panel_width;
        // Cancel any pending peek-close — the cursor is now over the
        // panel, so a stale hover-leave from the rail must not win.
        crate::side_panel_left::hold_peek(cx);
        tracing::trace!(
            current_x,
            start_x,
            start_width,
            old_w,
            new_w,
            "side_panel_left: resize drag move"
        );
        cx.notify();
    }

    fn end_resize(&mut self, cx: &mut Context<Self>) {
        cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>().resizing = false;
        self.resize_start_x = None;
        self.resize_start_width = None;
        tracing::info!("side_panel_left: resize drag ended (mouse-up)");
        cx.notify();
    }
}

/// Helper: which `LeftTab`s carry resizable runtime width memory?
/// Returns `Some(tab)` for resizable tabs, `None` for fixed-width tabs.
/// (`fixed_width_tabs_only` documents the inverse for symmetry.)
fn resizable_active(tab: crate::side_panel_left::tabs::LeftTab) -> Option<crate::side_panel_left::tabs::LeftTab> {
    if tab.is_resizable() { Some(tab) } else { None }
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Mirror SoT → legacy child. Both reads happen here so the
        // legacy render path's `chat_open`/`dock_chat` checks see the
        // latest values without needing to know about the SoT.
        let so = cx.global::<crate::side_panel_left::SidePanelLeftState_>();
        let panel_w = so.panel_width;
        let dock = so.dock_content;
        let resizing = so.resizing;
        let active_tab = so.active_tab;
        let remembered = so.remembered_widths;
        drop(so);
        self.content.update(cx, |child, _cx| {
            child.state.width = panel_w;
            child.state.dock_chat = dock;
            child.state.remembered_chat_width = Some(panel_w);
        });

        // Drain any pending focus request — only fires once per request.
        self.perform_focus_composer(window, cx);

        // Visible slice + input region (LEFT axis: starts at x = 0).
        let visible_w = geometry::visible_content_width(panel_w);
        let content_open = dock || visible_w > 1.0;
        let interactive_w = geometry::content_interactive_width(visible_w, resizing);

        if self.last_visible_width != Some(interactive_w) {
            let canvas_h = f32::from(window.bounds().size.height);
            let regions = geometry::content_input_region(visible_w, canvas_h, resizing);
            window.set_input_region(Some(&regions));
            self.last_visible_width = Some(interactive_w);
        }

        let handle_x = geometry::resize_handle_x(visible_w);
        let show_handle = active_tab.is_resizable() && (visible_w > 1.0 || resizing);

        let theme = *Theme::global(cx);
        let _ = remembered;
        let _ = theme;

        // Resize mouse-down/up/drag handlers — built here because Rust
        // 2024 RPIT capture rules would conflict if any other RPIT-returning
        // builder ran in between (matches the right panel's pattern).
        let resize_mouse_down = cx.listener(
            |this, ev: &gpui::MouseDownEvent, _window, cx| {
                this.start_resize(f32::from(ev.position.x), cx);
            },
        );
        let resize_drag_move = cx.listener(
            |this, ev: &gpui::DragMoveEvent<LeftPanelResize>, _window, cx| {
                this.update_resize(f32::from(ev.event.position.x), cx);
            },
        );
        let resize_mouse_up = cx.listener(
            |this, _ev: &gpui::MouseUpEvent, _window, cx| this.end_resize(cx),
        );

        div()
            .id("side-panel-left-content-root")
            .size_full()
            .relative()
            .flex()
            .flex_row()
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    crate::side_panel_left::hold_peek(cx);
                } else {
                    crate::side_panel_left::schedule_release_peek(cx);
                }
            })
            // The legacy SidePanelLeft renders its product body inside
            // the visible slice. Empty area to the right is left
            // transparent (Wayland input region already excludes it).
            .child(self.content.clone())
            .when(show_handle, |root| {
                root.child(
                    div()
                        .id("side-panel-left-resize-handle")
                        .absolute()
                        .left(px(handle_x))
                        .top(px(0.))
                        .w(px(RESIZE_HANDLE_WIDTH))
                        .h_full()
                        .cursor_col_resize()
                        .on_mouse_down(gpui::MouseButton::Left, resize_mouse_down)
                        .on_mouse_up(gpui::MouseButton::Left, resize_mouse_up)
                        .on_drag(LeftPanelResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
                        .on_drag_move(resize_drag_move),
                )
            })
    }
}

// Silence unused — `Bounds` is re-exported via gpui::prelude in the
// render path's element tree, but the linter doesn't see that.
const _: fn() = || {
    let _ = Bounds::<gpui::Pixels>::default;
};

/// Reference the constants in tests so a stray rename triggers a compile
/// failure here, not a silent null-op.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_constants_match_tabs_constants() {
        assert_eq!(CONTENT_CANVAS_WIDTH, MAX_PANEL_WIDTH - RAIL_WIDTH);
        assert!(RAIL_WIDTH > 0.0);
        assert!(RESIZE_HANDLE_WIDTH > 0.0);
        assert!(RESIZE_HANDLE_WIDTH < RAIL_WIDTH);
    }
}
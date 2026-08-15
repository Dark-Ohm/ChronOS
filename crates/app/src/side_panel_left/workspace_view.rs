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
    AnyElement, App, Bounds, Context, Entity, IntoElement, Render, Subscription, Window,
    div, prelude::*, px,
};

use chronos_ui::{Theme, WindowRootExt};

use crate::side_panel_left::ChatTab;
use crate::side_panel_left::state::geometry;
use crate::side_panel_left::tabs::{self, RESIZE_HANDLE_WIDTH};
use crate::side_panel_left::LeftPanelResize;

/// The content window's root view. Hosts the legacy `ChatTab`
/// product body as a child entity; that body still owns chat history,
/// composer state, ACP/Hermes client, etc.
pub struct WorkspaceView {
    /// T278: the legacy chat child — always alive (owns ACP/Hermes
    /// clients, chat history, composer state). No `WindowHandle`, width,
    /// dock, exclusive zone, or resize — rendered as a sub-element, with
    /// state mirrored from `SidePanelLeftState_` on every render.
    pub(crate) chat: Entity<ChatTab>,
    /// T279 / Task 4: lazy-created secondary tabs (created on first
    /// selection, retained for reuse). Project/Sessions own their list
    /// state; shells are stateless labels.
    sessions: Option<Entity<tabs::SessionsTab>>,
    project: Option<Entity<tabs::ProjectTab>>,
    /// B/C shell tabs keyed by `LeftTab` (Plan/Tools/Skills/ContextFiles/
    /// Archive) — created on first selection, reused after.
    shells: std::collections::HashMap<tabs::LeftTab, Entity<tabs::ShellTab>>,
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
    /// Held only to keep the subscriptions alive; not read.
    _subs: Vec<Subscription>,
}

impl WorkspaceView {
    pub fn new(content: Entity<ChatTab>, cx: &mut Context<Self>) -> Self {
        // Mirror SoT → product child whenever either side changes. Side
        // updates (dock toggle, width change, project switch) fire
        // `cx.notify()`; the subscription here repaints content + this
        // view together.
        let sub = cx.observe(&content, |_, _, cx| cx.notify());
        Self {
            chat: content,
            sessions: None,
            project: None,
            shells: std::collections::HashMap::new(),
            last_visible_width: None,
            resize_start_x: None,
            resize_start_width: None,
            focus_composer_pending: false,
            _subs: vec![sub],
        }
    }

    /// Read the panel width currently rendered. Exposed for tests and
    /// for IPC paths that need to read the live width without re-querying
    /// the global.
    pub fn panel_width(&self, cx: &App) -> f32 {
        cx.global::<crate::side_panel_left::SidePanelLeftState_>()
            .panel_width
    }

    /// Apply a width/dock/tab change and mirror the width into the legacy
    /// child. Used by IPC (`expand_with_composer`, `compose_and_send`) to
    /// dock the chat column at the remembered/preferred width without
    /// reaching through a window handle.
    ///
    /// T281 / Task 7: takes `active_tab` too. The previous signature left
    /// `SidePanelLeftState_.active_tab` untouched, so `expand_with_composer`
    /// / `compose_and_send` from a non-Chat tab silently focused/wrote into
    /// the `ChatTab` entity while the screen kept showing whatever tab was
    /// already active — `render`'s match only paints Chat when
    /// `active_tab == LeftTab::Chat`. Callers now always pass
    /// `LeftTab::Chat` (via `tabs::workspace_transition`'s
    /// `ExpandComposer`/`ComposeAndSend` arm).
    pub fn set_panel_width(
        &mut self,
        new_width: f32,
        dock: bool,
        active_tab: crate::side_panel_left::tabs::LeftTab,
        cx: &mut Context<Self>,
    ) {
        // Read the resulting panel_width + dock off the global before
        // mutating the child — `self.chat.update(cx, ...)` borrows cx
        // mutably for the duration of the closure, and we cannot touch
        // `cx.global(...)` again until that borrow ends.
        let (panel_width, dock_value) = {
            let state = cx.global_mut::<crate::side_panel_left::SidePanelLeftState_>();
            state.ensure_content_width(new_width);
            state.dock_content = dock;
            state.active_tab = active_tab;
            state.last_exclusive_zone = None;
            (state.panel_width, state.dock_content)
        };
        self.chat.update(cx, |child, _cx| {
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
        self.chat.update(cx, |child, _cx| {
            child.composer_focused = true;
        });
        self.focus_composer_pending = true;
        cx.notify();
    }

    /// T279 / Task 4 — thin dispatcher for `SessionsTab` events. The real
    /// reducer is `crate::side_panel_left::select_session` (free function on
    /// `&mut App`, T278 lesson), so the transition is unit-testable without
    /// a live `ChatTab` entity.
    pub fn on_sessions_event(&mut self, event: crate::side_panel_left::tabs::SessionsEvent, cx: &mut Context<Self>) {
        use crate::side_panel_left::tabs::SessionsEvent;
        match event {
            SessionsEvent::SelectThread(id) => {
                crate::side_panel_left::select_session(id, cx);
                // Plan line 603: "Any transition to Chat through Sessions ...
                // focuses the composer after the content window exists."
                // `self` is the WorkspaceView (already leased here), so
                // `request_focus_composer` cannot double-lease `content_view`.
                self.request_focus_composer(cx);
            }
            SessionsEvent::CreateThread => {
                // T279 round 2: "+ New" is a real reducer now — it opens
                // Chat AND mints a fresh thread in the live `ChatTab`
                // (`create_new_session`), not a bare tab switch.
                crate::side_panel_left::create_thread(cx);
            }
        }
        cx.notify();
    }

    /// T279 / Task 4 — thin dispatcher for `ProjectTab` events. The real
    /// reducers are free functions on `&mut App` (the T278 lesson):
    /// `switch_project` / `remove_project_scope` on the left,
    /// `side_panel_right::open_files_at` / `open_terminal_at` on the right.
    pub fn on_project_event(&mut self, event: crate::side_panel_left::tabs::ProjectEvent, cx: &mut Context<Self>) {
        use crate::side_panel_left::tabs::ProjectEvent;
        match event {
            // Select and Add run the SAME transaction — clear the chat
            // column + session scope, set the new path, reset the Sessions
            // selection. `Add`'s config persist already happened inside
            // `project_switcher::add_project`; Select persists after this
            // handler returns (`ProjectTab` click order: emit first,
            // `set_active` last).
            ProjectEvent::Select(path) | ProjectEvent::Add(path) => {
                crate::side_panel_left::switch_project(path.clone(), cx);
                // T280: scope the Sessions list to the new project and drop
                // the old selection. (`self.sessions` is a separate entity
                // from `self`, so this lease is safe here.)
                if let Some(sessions) = &self.sessions {
                    sessions.update(cx, |tab, cx| tab.set_project(path, cx));
                }
            }
            ProjectEvent::Remove(path) => {
                // Config removal lives in `project_switcher` (the domain
                // owner); the coordinator clears the chat/session scope
                // only when the removed path WAS the active project.
                crate::project_switcher::remove_project(&path.to_string_lossy(), cx);
                crate::side_panel_left::remove_project_scope(path, cx);
                // T280: if the removed project was active, reset the
                // Sessions list scope + selection too.
                if let Some(sessions) = &self.sessions {
                    sessions.update(cx, |tab, cx| tab.clear_for_project(cx));
                }
            }
            ProjectEvent::OpenInFiles(path) => {
                crate::side_panel_right::open_files_at(path, cx);
            }
            ProjectEvent::OpenInTerminal(path) => {
                crate::side_panel_right::open_terminal_at(path, cx);
            }
        }
        cx.notify();
    }

    fn perform_focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus_composer_pending {
            return;
        }
        self.focus_composer_pending = false;
        self.chat.update(cx, |child, cx| {
            window.focus(&child.composer_focus, cx);
            child.start_blink(cx);
        });
    }

    /// T279 / Task 4 — lazy-create the Sessions tab entity on first
    /// activation, retain for reuse. Entity creation + observe happen
    /// here in render-context but BEFORE the `div()` element builder is
    /// constructed — keeps the `.when` closure a pure clone+attach.
    /// Chat is always alive; secondary tabs are created on demand.
    fn ensure_sessions(&mut self, cx: &mut Context<Self>) -> Entity<tabs::SessionsTab> {
        if self.sessions.is_none() {
            let coordinator = cx.weak_entity();
            let entity = cx.new(|_| tabs::SessionsTab::new(coordinator));
            self._subs.push(cx.observe(&entity, |_, _, cx| cx.notify()));
            self.sessions = Some(entity);
        }
        self.sessions
            .as_ref()
            .expect("sessions tab created above")
            .clone()
    }

    /// T279 / Task 4 — lazy-create the Project tab entity on first
    /// activation. Same pattern as `ensure_sessions`.
    fn ensure_project(&mut self, cx: &mut Context<Self>) -> Entity<tabs::ProjectTab> {
        if self.project.is_none() {
            let coordinator = cx.weak_entity();
            let entity = cx.new(|_| tabs::ProjectTab::new(coordinator));
            self._subs.push(cx.observe(&entity, |_, _, cx| cx.notify()));
            self.project = Some(entity);
        }
        self.project
            .as_ref()
            .expect("project tab created above")
            .clone()
    }

    /// T279 / Task 4 — lazy-create the shell tab (Plan/Tools/Skills/
    /// ContextFiles/Archive) on first activation. ShellTabs are stateless
    /// labels (Slice B/C bodies), keyed by `LeftTab` for reuse.
    fn ensure_shell(&mut self, tab: tabs::LeftTab, cx: &mut Context<Self>) -> Entity<tabs::ShellTab> {
        if !self.shells.contains_key(&tab) {
            let entity = cx.new(|_| tabs::ShellTab::new(tab));
            self._subs.push(cx.observe(&entity, |_, _, cx| cx.notify()));
            self.shells.insert(tab, entity);
        }
        self.shells
            .get(&tab)
            .expect("shell tab created above")
            .clone()
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
pub(crate) fn resizable_active(tab: crate::side_panel_left::tabs::LeftTab) -> Option<crate::side_panel_left::tabs::LeftTab> {
    if tab.is_resizable() { Some(tab) } else { None }
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // T278 architect round 2: the legacy child must mirror the
        // VISIBLE slice width (not the logical panel_width). Otherwise
        // the legacy sidebar strip bleeds 40 px past the visible slice
        // at every width, and paints a 40 px opaque band even when
        // visible_w == 0 (rail-only). Mirroring visible_w keeps the
        // child inside the input region exactly; the legacy `chat_open`
        // threshold (`width > sidebar + handle + 1`) still fires
        // correctly because visible_w > 1 ⇒ the threshold is met.
        //
        // We also wrap the child in a div sized to visible_w with
        // overflow_hidden, so any internal flex overflow (a 36 px
        // sidebar in a 0 px wrapper, etc.) cannot paint past the
        // visible slice. When visible_w == 0 we render an empty
        // wrapper — the legacy child stays alive (its ACP/Hermes
        // clients, composer state, chat history) but never paints.
        let so = cx.global::<crate::side_panel_left::SidePanelLeftState_>();
        let panel_w = so.panel_width;
        let dock = so.dock_content;
        let resizing = so.resizing;
        let active_tab = so.active_tab;
        let _ = so;

        let visible_w = geometry::visible_content_width(panel_w);

        // T278 architect round 2: the Chat child must mirror the VISIBLE
        // slice width (not logical panel_width) on every render — it
        // stays always-alive and is only the active body when
        // `active_tab == Chat`. Secondary tabs (Sessions/Project/Shell)
        // own their own layout and read panel_width from the global SoT
        // when they need it; they do not receive this width-mirror.
        self.chat.update(cx, |child, _cx| {
            child.state.width = visible_w.max(crate::side_panel_left::sessions_list::SIDEBAR_MIN_WIDTH);
            child.state.dock_chat = dock;
            child.state.remembered_chat_width = Some(child.state.width);
        });

        // T279 / Task 4 — lazy-create the active tab's entity and build
        // the clip wrapper as a concrete `AnyElement` per arm. `Render`
        // is not dyn-compatible (`Self: Sized`), so `Entity<dyn Render>`
        // is impossible — each arm builds the sized clip `div` with the
        // concrete entity clone and resolves to `AnyElement` before the
        // match ends. When the panel is closed (`visible_w == 0`) we
        // produce `None` so no tab entity is created on an invisible
        // surface (lazy creation waits for the first *visible* render).
        // Chat is always alive (created in `new`); secondary tabs are
        // created here on first visible activation and retained in the
        // `Option`/`HashMap` slots for reuse.
        let clip: Option<AnyElement> = if visible_w > 0.0 {
            Some(match active_tab {
                tabs::LeftTab::Chat => div()
                    .id("side-panel-left-product-clip")
                    .w(px(visible_w))
                    .h_full()
                    .overflow_hidden()
                    .flex_none()
                    .child(self.chat.clone())
                    .into_any_element(),
                tabs::LeftTab::Sessions => div()
                    .id("side-panel-left-product-clip")
                    .w(px(visible_w))
                    .h_full()
                    .overflow_hidden()
                    .flex_none()
                    .child(self.ensure_sessions(cx))
                    .into_any_element(),
                tabs::LeftTab::Project => div()
                    .id("side-panel-left-product-clip")
                    .w(px(visible_w))
                    .h_full()
                    .overflow_hidden()
                    .flex_none()
                    .child(self.ensure_project(cx))
                    .into_any_element(),
                tabs::LeftTab::Plan
                | tabs::LeftTab::Tools
                | tabs::LeftTab::Skills
                | tabs::LeftTab::ContextFiles
                | tabs::LeftTab::Archive => div()
                    .id("side-panel-left-product-clip")
                    .w(px(visible_w))
                    .h_full()
                    .overflow_hidden()
                    .flex_none()
                    .child(self.ensure_shell(active_tab, cx))
                    .into_any_element(),
            })
        } else {
            None
        };

        // Drain any pending focus request — only fires once per request.
        self.perform_focus_composer(window, cx);

        let interactive_w = geometry::content_interactive_width(visible_w, resizing);

        if self.last_visible_width != Some(interactive_w) {
            let canvas_h = f32::from(window.bounds().size.height);
            let regions = geometry::content_input_region(visible_w, canvas_h, resizing);
            window.set_input_region(Some(&regions));
            self.last_visible_width = Some(interactive_w);
        }

        let handle_x = geometry::resize_handle_x(visible_w);
        let show_handle = active_tab.is_resizable() && (visible_w > 1.0 || resizing);

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

        let theme = *Theme::global(cx);

        div()
            .id("side-panel-left-content-root")
            .window_font(&theme)
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
            // The active child renders inside the clip wrapper built
            // above (`Option<AnyElement>`). When the panel is closed
            // (`clip == None`) no child paints and no input region is
            // claimed. The Chat child stays alive between open/close
            // cycles (ACP/Hermes clients, composer state, chat history);
            // secondary tab entities also stay alive once created —
            // their list state persists across tab switches — but are
            // omitted from the paint when `visible_w == 0`.
            .children(clip)
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
        assert_eq!(
            tabs::CONTENT_CANVAS_WIDTH,
            tabs::MAX_PANEL_WIDTH - tabs::RAIL_WIDTH
        );
        assert!(tabs::RAIL_WIDTH > 0.0);
        assert!(RESIZE_HANDLE_WIDTH > 0.0);
        assert!(RESIZE_HANDLE_WIDTH < tabs::RAIL_WIDTH);
    }
}
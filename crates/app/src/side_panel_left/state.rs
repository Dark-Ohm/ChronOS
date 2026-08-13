/// T278 / Slice A1 — pure LEFT geometry helpers.
///
/// The rail sits at `x = 0`, the content canvas begins at `x = RAIL_WIDTH`.
/// Unlike the right panel (whose visible slice is right-aligned inside a
/// fixed canvas), the LEFT slice is **left-aligned**: cursor moves right
/// increase the panel, cursor moves left decrease it, and the resize
/// handle sits on the visible slice's outer (right) edge.
///
/// All helpers here are window-free — no `Window`/`App`/`Context` — so a
/// renderer can call them as pure functions and a unit test can assert
/// every formula directly.
pub mod geometry {
    use super::super::tabs::{
        CONTENT_CANVAS_WIDTH, MAX_PANEL_WIDTH, RAIL_WIDTH, RESIZE_HANDLE_WIDTH, SOFT_OPEN_MIN_WIDTH,
    };
    use gpui::{Bounds, Pixels, Point, Size, px};

    /// Hard drag clamp. The user can drag all the way down to rail-only
    /// (`RAIL_WIDTH`) so a panel collapsed to rail can be re-expanded with
    /// the same gesture — the in-flight drag keeps the 4 px handle alive at
    /// `visible_w == 0`. `MAX_PANEL_WIDTH` is the matching upper bound.
    pub const fn hard_min() -> f32 {
        RAIL_WIDTH
    }
    pub const fn hard_max() -> f32 {
        MAX_PANEL_WIDTH
    }

    /// Clamp a logical panel width into the hard drag range. NaN collapses
    /// to `hard_min()` so a corrupt value cannot poison downstream math.
    pub fn clamp_panel(w: f32) -> f32 {
        if !w.is_finite() {
            return hard_min();
        }
        w.clamp(hard_min(), hard_max())
    }

    /// Clamp a per-resizable-tab width into the resizable drag range
    /// `[SOFT_OPEN_MIN_WIDTH, MAX_PANEL_WIDTH]`. Fixed tabs use
    /// `clamp_panel` instead — a fixed tab can open below the soft floor
    /// (Sessions at 400) but never below rail-only.
    pub fn clamp_resizable(w: f32) -> f32 {
        if !w.is_finite() {
            return SOFT_OPEN_MIN_WIDTH;
        }
        w.clamp(SOFT_OPEN_MIN_WIDTH, hard_max())
    }

    /// Visible content slice width in pixels. `0` at rail-only,
    /// `CONTENT_CANVAS_WIDTH` at `MAX_PANEL_WIDTH`. Floored at 0 even when
    /// the input is slightly out of range (the renderer relies on this so
    /// a stale `width` from a prior session cannot leak a negative slice).
    pub fn visible_content_width(panel_width: f32) -> f32 {
        (panel_width - RAIL_WIDTH).clamp(0.0, CONTENT_CANVAS_WIDTH)
    }

    /// Input-region width during render.
    /// While resizing, the transparent handle must remain interactive even
    /// when the visible slice reaches zero — otherwise GPUI drops the drag
    /// target at the rail-only clamp and the pointer cannot pull the
    /// panel back in one gesture.
    pub fn content_interactive_width(visible_w: f32, resizing: bool) -> f32 {
        if resizing {
            visible_w.max(RESIZE_HANDLE_WIDTH)
        } else {
            visible_w
        }
    }

    /// Input region for the LEFT content canvas. The visible slice starts
    /// at `x = 0` (left-aligned, opposite to the right panel); empty when
    /// the panel is collapsed to rail-only and not currently being dragged.
    pub fn content_input_region(
        visible_w: f32,
        canvas_h: f32,
        resizing: bool,
    ) -> Vec<Bounds<Pixels>> {
        let interactive = content_interactive_width(visible_w, resizing);
        if interactive <= 0.0 {
            return Vec::new();
        }
        let w = interactive.min(CONTENT_CANVAS_WIDTH);
        let h = canvas_h.max(0.0);
        vec![Bounds::new(
            Point::new(px(0.0), px(0.0)),
            Size::new(px(w), px(h)),
        )]
    }

    /// X coordinate of the resize handle inside the fixed canvas. The
    /// handle sits flush against the visible slice's outer (right) edge:
    /// `clamp(visible_w - 4, 0, CONTENT_CANVAS_WIDTH - 4)`. The 916 upper
    /// bound keeps the handle inside the canvas even at full width.
    pub fn resize_handle_x(visible_w: f32) -> f32 {
        let max_x = (CONTENT_CANVAS_WIDTH - RESIZE_HANDLE_WIDTH).max(0.0);
        (visible_w - RESIZE_HANDLE_WIDTH).clamp(0.0, max_x)
    }

    /// Absolute-delta resize target. LEFT axis: cursor moves right increase
    /// the panel, so the delta is `current_x - start_x` (not the right
    /// panel's negated sign). The hard clamp anchors to the drag range.
    pub fn resize_target_width(start_width: f32, start_x: f32, current_x: f32) -> f32 {
        clamp_panel(start_width + (current_x - start_x))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn visible_width_at_rail_only_is_zero() {
            assert_eq!(visible_content_width(RAIL_WIDTH), 0.0);
        }

        #[test]
        fn visible_width_at_max_is_canvas_width() {
            assert_eq!(visible_content_width(MAX_PANEL_WIDTH), CONTENT_CANVAS_WIDTH);
        }

        #[test]
        fn visible_width_at_soft_floor_is_minus_forty() {
            // SOFT_OPEN_MIN_WIDTH is the panel width, not the content width.
            assert_eq!(
                visible_content_width(SOFT_OPEN_MIN_WIDTH),
                SOFT_OPEN_MIN_WIDTH - RAIL_WIDTH
            );
        }

        #[test]
        fn visible_width_clamped_above_max_and_below_zero() {
            assert_eq!(visible_content_width(1500.0), CONTENT_CANVAS_WIDTH);
            assert_eq!(visible_content_width(10.0), 0.0);
        }

        #[test]
        fn interactive_width_during_drag_survives_zero() {
            // Drag clamp keeps the 4px handle alive at visible=0 so the
            // user can pull the panel back from rail-only.
            assert_eq!(content_interactive_width(0.0, true), RESIZE_HANDLE_WIDTH);
            assert_eq!(content_interactive_width(0.0, false), 0.0);
        }

        #[test]
        fn interactive_width_passes_visible_through_when_not_resizing() {
            assert_eq!(content_interactive_width(320.0, false), 320.0);
            assert_eq!(content_interactive_width(920.0, false), 920.0);
        }

        #[test]
        fn input_region_empty_when_zero_and_not_dragging() {
            assert!(content_input_region(0.0, 100.0, false).is_empty());
        }

        #[test]
        fn input_region_nonempty_when_dragging_even_at_zero() {
            let r = content_input_region(0.0, 100.0, true);
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].origin.x.as_f32(), 0.0);
            assert_eq!(r[0].size.width.as_f32(), RESIZE_HANDLE_WIDTH);
        }

        #[test]
        fn input_region_left_aligned_starts_at_x_zero() {
            // LEFT axis: visible content begins at x=0 (NOT at
            // CONTENT_CANVAS_WIDTH - visible_w, which is the right
            // panel's right-aligned formula).
            let r = content_input_region(500.0, 800.0, false);
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].origin.x.as_f32(), 0.0);
            assert_eq!(r[0].size.width.as_f32(), 500.0);
            assert_eq!(r[0].size.height.as_f32(), 800.0);
        }

        #[test]
        fn input_region_clamped_to_canvas_width() {
            // visible_w must not exceed CONTENT_CANVAS_WIDTH even if a
            // stale state.width drifts above MAX_PANEL_WIDTH.
            let r = content_input_region(1500.0, 100.0, false);
            assert_eq!(r[0].size.width.as_f32(), CONTENT_CANVAS_WIDTH);
        }

        #[test]
        fn resize_handle_at_zero_visible() {
            assert_eq!(resize_handle_x(0.0), 0.0);
        }

        #[test]
        fn resize_handle_at_full_canvas() {
            assert_eq!(
                resize_handle_x(CONTENT_CANVAS_WIDTH),
                CONTENT_CANVAS_WIDTH - RESIZE_HANDLE_WIDTH
            );
        }

        #[test]
        fn resize_handle_at_intermediate() {
            // At visible=320, handle is at 316 (touching the outer edge).
            assert_eq!(resize_handle_x(320.0), 316.0);
        }

        #[test]
        fn resize_target_right_delta_grows_panel() {
            // Cursor moves right by 28 → panel grows by 28.
            assert_eq!(resize_target_width(500.0, 100.0, 128.0), 528.0);
        }

        #[test]
        fn resize_target_left_delta_shrinks_panel() {
            // Cursor moves left by 28 → panel shrinks by 28.
            assert_eq!(resize_target_width(500.0, 100.0, 72.0), 472.0);
        }

        #[test]
        fn resize_target_clamps_to_rail_only() {
            // Massive left delta collapses to rail-only, not below.
            assert_eq!(resize_target_width(40.0, 100.0, 0.0), 40.0);
            assert_eq!(resize_target_width(40.0, 100.0, -1000.0), 40.0);
        }

        #[test]
        fn resize_target_clamps_to_max() {
            assert_eq!(resize_target_width(900.0, 100.0, 200.0), MAX_PANEL_WIDTH);
        }

        #[test]
        fn resize_target_survives_nan() {
            // A NaN start_width must not crash; clamp_panel folds it to hard_min.
            assert_eq!(resize_target_width(f32::NAN, 100.0, 200.0), RAIL_WIDTH);
        }

        #[test]
        fn hard_min_and_max_anchored_to_rail_and_max_panel() {
            assert_eq!(hard_min(), RAIL_WIDTH);
            assert_eq!(hard_max(), MAX_PANEL_WIDTH);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelState {
    Peek,
    Pinned,
    Resizing,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AgentStatus {
    Connected,
    Disconnected,
    Thinking,
}

/// Streaming state for an in-progress ACP prompt turn.
pub struct StreamingState {
    /// Whether a streaming turn is in progress.
    pub active: bool,
    /// Join handle for the event receiver task (aborted on drop/cancel).
    pub receiver_task: Option<gpui::Task<()>>,
    /// Join handle for the ACP prompt task (aborted on drop/cancel).
    pub acp_task: Option<gpui::Task<()>>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            active: false,
            receiver_task: None,
            acp_task: None,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        // `gpui::Task` has no abort(); dropping the handle cancels the task.
        drop(self.receiver_task.take());
        drop(self.acp_task.take());
    }
}

pub struct SidePanelLeftState {
    pub state: PanelState,
    pub width: f32,
    pub height: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub session_id: Option<String>,
    pub agent_status: AgentStatus,
    pub sessions_collapsed: bool,
    pub active_session_id: Option<String>,
    /// When true, chat tiles alongside sidebar (exclusive = full width).
    /// When false (default), chat overlays — exclusive stays at sidebar width.
    pub dock_chat: bool,
    /// Last exclusive_zone value sent to compositor (avoids redundant Wayland round-trips).
    pub last_exclusive_zone: Option<f32>,
    /// Remembered expanded-chat width (N), restored on the next expand so a
    /// summon→expand→close→summon cycle returns the same N instead of the
    /// 352px default. Mirrors right panel `tab_resize_memory`. Survives panel
    /// close by being mirrored into the global `SidePanelLeftState_`.
    pub remembered_chat_width: Option<f32>,
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        // T220: summon rail-only — only the 36px strip + 4px handle show; the
        // chat column does NOT auto-open on Super+A/peek (that hid the composer
        // in the old T137 behaviour). Chat is revealed by the dock toggle or a
        // resize drag, which expand to `remembered_chat_width` (or default).
        let s = Self {
            state: PanelState::Peek,
            width: super::sessions_list::SIDEBAR_MIN_WIDTH,
            height: 1080.0,
            min_width: super::sessions_list::SIDEBAR_MIN_WIDTH,
            max_width: 960.0,
            session_id: None,
            agent_status: AgentStatus::Disconnected,
            sessions_collapsed: true,
            active_session_id: None,
            dock_chat: false,
            last_exclusive_zone: None,
            remembered_chat_width: None,
        };
        s
    }

    pub fn sidebar_width(&self) -> f32 {
        if self.sessions_collapsed {
            super::sessions_list::SIDEBAR_COLLAPSED_WIDTH
        } else {
            super::sessions_list::SIDEBAR_EXPANDED_WIDTH
        }
    }

    /// Exclusive zone px: full panel when docked; bar strip (sidebar + handle)
    /// when overlay. Handle is on the inner edge — without it in the zone the
    /// grab strip sits on top of tiled windows (live 2026-07-25: reserved 36
    /// vs window 40 = 36 rail + 4 handle). Mirrors right panel
    /// `RAIL_ONLY_WIDTH = rail + handle`.
    pub fn exclusive_px(&self) -> f32 {
        if self.dock_chat {
            self.width
        } else {
            self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH
        }
    }

    /// Recalculate min_width after collapse state changes.
    /// Expanded sessions need room for the 200px column + handle.
    pub fn recalc_min_width(&mut self) {
        self.min_width = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH;
        if self.width < self.min_width {
            self.width = self.min_width;
        }
    }

    pub fn resize(&mut self, new_width: f32) {
        self.width = new_width.clamp(self.min_width, self.max_width);
        // A manual resize (drag / dock toggle) sets the remembered chat width
        // so a later summon→expand returns here, not the 352px default.
        let rail = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH;
        if self.width - rail > 1.0 {
            self.remembered_chat_width = Some(self.width);
        }
    }

    /// Default width when opening chat / turning dock on from sidebar-only.
    pub const DEFAULT_CHAT_WIDTH: f32 = 352.;

    /// Rail-only summon width: collapsed sidebar strip + resize handle.
    /// Equal to the right panel's `RAIL_ONLY_WIDTH` (asserted by
    /// `rails_and_handles_match_right_panel`). T220: this is the width a
    /// summon opens at — only the rail shows; chat reveals separately.
    pub fn rail_only_width() -> f32 {
        super::sessions_list::SIDEBAR_MIN_WIDTH
    }

    /// Expand the chat column from rail-only to a usable width.
    /// Prefers the remembered width from a previous expand/resize; falls back
    /// to `DEFAULT_CHAT_WIDTH`. Never narrower than the sidebar strip + 120px
    /// thread column so the chat stays usable.
    pub fn ensure_chat_width(&mut self) {
        let need = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH + 120.0; // min thread column so chat is usable
        let target = self
            .remembered_chat_width
            .unwrap_or(Self::DEFAULT_CHAT_WIDTH)
            .max(need)
            .min(self.max_width);
        tracing::debug!(
            width = self.width,
            target,
            remembered = ?self.remembered_chat_width,
            need,
            "ensure_chat_width: before"
        );
        if self.width < target {
            self.width = target;
            tracing::debug!(new_width = self.width, "ensure_chat_width: expanded");
        } else {
            tracing::debug!(width = self.width, "ensure_chat_width: already wide enough (no-op)");
        }
        self.remembered_chat_width = Some(self.width);
    }
}

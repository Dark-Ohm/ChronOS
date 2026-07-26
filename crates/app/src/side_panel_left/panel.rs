use gpui::{AnimationExt, Context, IntoElement, Window, div, img, prelude::*, px};

use chronos_ui::{Theme, elevation_glow_bar};

use crate::motion;

use super::SidePanelLeft;
use super::sessions_list::{SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_EXPANDED_WIDTH, SIDEBAR_HANDLE_WIDTH};
use super::state::AgentStatus;

fn status_color(status: AgentStatus, theme: &Theme) -> gpui::Hsla {
    match status {
        AgentStatus::Connected => theme.status.success,
        AgentStatus::Disconnected => theme.status.error,
        AgentStatus::Thinking => theme.status.warning,
    }
}

pub fn render_panel(
    panel: &SidePanelLeft,
    _window: &mut Window,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let dot_color = status_color(panel.state.agent_status, &theme);
    let collapsed = panel.state.sessions_collapsed;
    let agent_menu_open = panel.agent_menu_open;
    // Chat visibility is width-driven (or forced by dock). Dock only changes
    // exclusive zone — it must NOT hide the thread (T126 accept errata).
    let sidebar_w = if collapsed {
        SIDEBAR_COLLAPSED_WIDTH
    } else {
        SIDEBAR_EXPANDED_WIDTH
    };
    let past_sidebar = panel.state.width > sidebar_w + SIDEBAR_HANDLE_WIDTH + 1.0;
    let chat_open = panel.state.dock_chat || past_sidebar;

    let agent_name = panel
        .agents
        .iter()
        .find(|a| a.id == panel.active_agent_id)
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| "Agent".to_string());

    // Thread title: active session's title, or a placeholder for a fresh
    // thread. Deliberately NOT `agent_name` — the outer header already
    // shows the agent (agent-cluster dot + name + switcher); repeating it
    // here reads as a duplicate label.
    let thread_title = panel
        .sessions
        .iter()
        .find(|s| s.active)
        .map(|s| s.title.clone())
        .unwrap_or_else(|| "New Agent Thread".to_string());

    // Resize handlers (borrows cx) — must be built before ANY other call
    // that returns `impl IntoElement` and stays alive past this point
    // (sidebar/composer/chat): their RPIT return captures `cx`'s lifetime
    // for as long as the resulting element is alive (Rust 2024 impl Trait
    // capture rules), so any `cx.listener(...)` call after them would
    // conflict (E0502).
    let resize_drag_handler = cx.listener(
        |this, ev: &gpui::DragMoveEvent<super::LeftPanelResize>, window, cx| {
            let current_x = f32::from(ev.event.position.x);
            this.update_resize(current_x, window, cx);
        },
    );

    let resize_mouse_handler = cx.listener(|this, ev: &gpui::MouseDownEvent, _window, _cx| {
        this.start_resize(f32::from(ev.position.x));
    });

    // Build sidebar (now borrows cx — click handlers on collapse/expand)
    let sidebar = build_sessions_sidebar(panel, collapsed, &theme, cx);

    // Thread header listener (built before any RPIT that captures cx)
    let thread_new_chat_handler = cx.listener(|this, _, _, cx| {
        this.create_new_session(cx);
    });

    // Build agent dropdown with click handlers — must be built BEFORE
    // chat/composer because those call cx.listener() internally and
    // Rust 2024 RPIT capture rules make the returned elements hold a
    // mutable borrow on cx for their entire lifetime.
    let active_id = panel.active_agent_id.clone();
    let dropdown = if agent_menu_open {
        Some(
            div()
                .id("agent-dropdown")
                .w(px(172.))
                .bg(theme.bg.primary)
                .border_1()
                .border_color(theme.border.subtle)
                .rounded(px(8.))
                .p(px(4.))
                .mx(px(8.))
                .mt(px(4.))
                .flex()
                .flex_col()
                .children(panel.agents.iter().map(|agent| {
                    let is_selected = agent.id == active_id.as_str();
                    let agent_id = agent.id.to_string();
                    div()
                        .id(format!("agent-option-{}", agent.id))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_size(px(11.5))
                        .text_color(theme.text.secondary)
                        .hover(|s| s.bg(theme.border.subtle))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.switch_agent(&agent_id, cx);
                        }))
                        .child(
                            div()
                                .text_color(if is_selected {
                                    theme.text.primary
                                } else {
                                    theme.text.secondary
                                })
                                .child(agent.display_name.clone()),
                        )
                        .when(is_selected, |el| {
                            el.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme.accent.primary)
                                    .child("✓"),
                            )
                        })
                })),
        )
    } else {
        None
    };

    // Build chat (borrows cx)
    let chat = div()
        .id("chat-area")
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(panel.chat.render(panel, _window, cx));

    // Build composer (borrows cx)
    let composer = super::composer::render_composer(panel, _window, cx);

    // Thread header (block A) — static chrome
    // Use builder for the header because rsx! with cx.listener listeners
    // hits RPIT capture issues (see rakes §Rust 2024 RPIT capture).
    let thread_header = div()
        .id("thread-header")
        .flex_none()
        .h(px(38.))
        .px(px(12.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.accent.primary)
                        .child("✦"),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text.primary)
                        .child(thread_title),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .id("thread-new-chat")
                        .w(px(20.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .on_click(thread_new_chat_handler)
                        .child("＋"),
                )
                .child(
                    div()
                        .id("thread-history")
                        .w(px(20.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .child("☰"),
                )
                .child(
                    div()
                        .id("thread-more")
                        .w(px(20.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(theme.text.muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                        .child("⋯"),
                ),
        );

    // Clipped content — sidebar beside the thread column, NOT stacked above
    // it. `flex_col` here (both were siblings of sidebar in one vertical
    // stack) made sidebar's `.h_full()` compete for vertical space against
    // thread_header/chat/composer instead of sitting in its own column —
    // sidebar came up short of the panel bottom and the thread column got
    // squeezed into a sliver near the bottom (live smoke, 2026-07-23).
    let thread_column = div()
        .id("thread-column")
        .flex_1()
        .min_w(px(0.))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(thread_header)
        .child(chat)
        .child(composer);

    let clipped_content = div()
        .id("clipped-content")
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_row()
        .overflow_hidden()
        .child(sidebar)
        .when(chat_open, |el| el.child(thread_column));

    // Header with listeners
    let header = div()
        .id("side-panel-header")
        .flex()
        .items_center()
        .justify_between()
        .flex_none()
        .px(px(14.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .child(
            div()
                .id("agent-cluster")
                .flex()
                .items_center()
                .gap(px(7.))
                .cursor_pointer()
                .rounded(px(6.))
                .px(px(6.))
                .py(px(3.))
                .mx(px(-6.))
                .my(px(-3.))
                .hover(|s| s.bg(theme.border.subtle))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.agent_menu_open = !this.agent_menu_open;
                    cx.notify();
                }))
                .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(dot_color))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text.secondary)
                        .child(agent_name),
                )
                .child({
                    let status_text = match panel.state.agent_status {
                        super::state::AgentStatus::Connected => "Connected",
                        super::state::AgentStatus::Disconnected => "Disconnected",
                        super::state::AgentStatus::Thinking => "Thinking…",
                    };
                    div()
                        .text_size(px(10.5))
                        .text_color(theme.text.muted)
                        .child(status_text)
                })
                .child(
                    div()
                        .text_size(px(9.))
                        .text_color(theme.text.muted)
                        .child("⌄"),
                ),
        )
        .child(
            div()
                .id("side-panel-left-close")
                .w(px(20.))
                .h(px(20.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.text.muted)
                .cursor_pointer()
                .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                .on_click(|_ev, window, cx| {
                    crate::side_panel_left::close_this(window, cx);
                })
                .child(img("icons/x.svg").w(px(12.)).h(px(12.))),
        );

    // Elevated chrome на content-колонке (только когда чат открыт, не
    // rail-only) — общий язык глубины из `theme.elevation_popup()` (T128).
    let elev = Theme::global(cx).elevation_popup();

    // Outer: sole window-level on_hover. Motion is native with_animation on the
    // shell row (T129) — not gpui_animation transition_when (silent no-op on
    // fresh layer-shell windows).
    div()
        .id("side-panel-left-root")
        .w(px(panel.state.width))
        .h_full()
        .flex()
        .flex_row()
        .on_hover(|hovered, _window, cx| {
            if *hovered {
                super::hold_peek(cx);
            } else {
                super::schedule_release_peek(cx);
            }
        })
        .child(
            div()
                .id("side-panel-left-motion")
                .relative()
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .flex()
                .flex_row()
                .child(
                    div()
                        .id("main-content")
                        .flex_1()
                        .min_w(px(0.))
                        .overflow_hidden()
                        .h_full()
                        .flex()
                        .flex_col()
                        .bg(theme.bg.primary)
                        .shadow(elev.shadows.to_vec())
                        .when(chat_open, |el| {
                            let el = el.child(header).children(dropdown);
                            match elev.glow {
                                Some(glow) => el.child(elevation_glow_bar(glow)),
                                None => el,
                            }
                        })
                        .child(clipped_content),
                )
                .child(
                    div()
                        .id("resize-handle")
                        .flex_none()
                        .w(px(SIDEBAR_HANDLE_WIDTH))
                        .h_full()
                        .cursor_col_resize()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(theme.bg.tertiary)
                        .border_l_1()
                        .border_color(theme.border.subtle)
                        .on_mouse_down(gpui::MouseButton::Left, resize_mouse_handler)
                        .on_drag(super::LeftPanelResize, |_, _, _, cx| {
                            cx.new(|_| gpui::EmptyView)
                        })
                        .on_drag_move(resize_drag_handler)
                        .child(div().w(px(1.)).h_full().bg(theme.text.disabled)),
                )
                .with_animation(
                    "side-panel-left-enter",
                    motion::enter_animation(),
                    motion::apply_enter_from_left,
                ),
        )
}

fn build_sessions_sidebar(
    panel: &SidePanelLeft,
    collapsed: bool,
    theme: &Theme,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement + use<> {
    let sessions = &panel.sessions;

    if collapsed {
        div()
            .id("sessions-sidebar-collapsed")
            .w(px(SIDEBAR_COLLAPSED_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .bg(theme.bg.tertiary)
            .border_r_1()
            .border_color(theme.border.subtle)
            .gap(px(4.))
            .p(px(4.))
            .child(
                div()
                    .id("sessions-expand")
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(theme.text.muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_collapse(cx);
                    }))
                    .child(">"),
            )
            .child(
                div()
                    .id("sessions-new-icon")
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(theme.text.muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle).text_color(theme.text.primary))
                    .child("+"),
            )
            .children(sessions.iter().map(|s| {
                let is_active = s.active;
                let sid = s.id.clone();
                div()
                    .id(format!("session-dot-{sid}"))
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .when(is_active, |el| el.bg(theme.text.disabled))
                    .when(!is_active, |el| el.cursor_pointer())
                    .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(if is_active {
                        theme.status.success
                    } else {
                        theme.interactive.active
                    }))
            }))
            .child(div().flex_1()) // spacer
            .child({
                let docked = panel.state.dock_chat;
                div()
                    .id("dock-toggle")
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.))
                    .text_color(if docked {
                        theme.accent.primary
                    } else {
                        theme.text.muted
                    })
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.dock_chat = !this.state.dock_chat;
                        if this.state.dock_chat {
                            this.state.ensure_chat_width();
                        }
                        // Force exclusive recompute next paint.
                        this.state.last_exclusive_zone = None;
                        cx.notify();
                    }))
                    .child(if docked { "⊞" } else { "⊟" })
            })
            .into_any()
    } else {
        div()
            .id("sessions-sidebar-expanded")
            .w(px(SIDEBAR_EXPANDED_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme.bg.tertiary)
            .border_r_1()
            .border_color(theme.border.subtle)
            .child(
                div()
                    .id("sessions-header")
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_none()
                    .px(px(10.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(theme.border.subtle)
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text.secondary)
                            .child("Sessions"),
                    )
                    .child({
                        let docked = panel.state.dock_chat;
                        div()
                            .id("sessions-header-buttons")
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(
                                div()
                                    .id("dock-toggle-expanded")
                                    .w(px(20.))
                                    .h(px(20.))
                                    .rounded(px(4.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.))
                                    .text_color(if docked {
                                        theme.accent.primary
                                    } else {
                                        theme.text.muted
                                    })
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.border.subtle))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.state.dock_chat = !this.state.dock_chat;
                                        if this.state.dock_chat {
                                            this.state.ensure_chat_width();
                                        }
                                        this.state.last_exclusive_zone = None;
                                        cx.notify();
                                    }))
                                    .child(if docked { "⊞" } else { "⊟" }),
                            )
                            .child(
                                div()
                                    .id("sessions-collapse")
                                    .w(px(20.))
                                    .h(px(20.))
                                    .rounded(px(4.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(11.))
                                    .text_color(theme.text.muted)
                                    .cursor_pointer()
                                    .hover(|s| {
                                        s.bg(theme.border.subtle).text_color(theme.text.primary)
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_collapse(cx);
                                    }))
                                    .child("<"),
                            )
                    }),
            )
            .child(
                div()
                    .id("sessions-new")
                    .flex_none()
                    .mx(px(8.))
                    .mt(px(8.))
                    .mb(px(4.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border.default)
                    .text_size(px(11.5))
                    .text_color(theme.text.secondary)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.border.subtle).border_color(theme.text.disabled))
                    .child("+ New session"),
            )
            .child(
                div()
                    .id("sessions-list-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .p(px(8.))
                    .children(sessions.iter().map(|s| {
                        let is_active = s.active;
                        let title = s.title.clone();
                        let sid = s.id.clone();
                        div()
                            .id(format!("session-item-{sid}"))
                            .w_full()
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .when(is_active, |el| el.bg(theme.border.default))
                            .when(!is_active, |el| el.hover(|s| s.bg(theme.border.subtle)))
                            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(if is_active {
                                theme.status.success
                            } else {
                                theme.interactive.active
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(if is_active {
                                        theme.text.primary
                                    } else {
                                        theme.text.secondary
                                    })
                                    .child(title),
                            )
                    })),
            )
            .into_any()
    }
}

//! Tray menu popup view — renders the live `MenuNode` tree from the
//! `TrayMenuState` global.
//!
//! Rendering rules (per the brief):
//!   * leaf item          → label (muted color if `!enabled`)
//!   * separator          → thin divider (label/children ignored)
//!   * toggle item        → `✓` / `○` prefix (Checkmark/Radio)
//!   * submenu (children) → unfolded inline with indentation (no nested
//!                          windows in MVP)
//!   * empty label        → rendered as `…` (the known OpenCode service bug
//!                          where child labels arrive empty — pending fix)
//!
//! ## T260 wave-2 (enter + accent)
//! The reference (`Chronos-Context-Menu.dc (1).html`) marks the selected /
//! hovered row with a 2px left accent-bar (`.ci::before`). The fork has no
//! `::before` pseudo-element, so the bar is an absolutely-positioned child
//! that is **always present** and morphs via `transition_when_else`
//! (opacity + top/bottom inset grow — a scaleY stand-in, since the fork has
//! no element `scale()`). The rest state lives in the base style chain so
//! the first frame is invisible (no flash). Same for the row hover wash —
//! `background` morphs instead of swapping.
//!
//! The fork reserves scrollbar space (`scrollbar_width`) but paints **no**
//! scrollbar, so a long menu would lose 6px of row width for nothing. We
//! draw our own 6px overlay thumb from the live `ScrollHandle` geometry:
//! the container's wheel handler `cx.notify`s this view on scroll, so
//! `render` re-runs with a fresh offset and the thumb tracks the wheel.
//! Rows never shrink (no layout reservation).

use std::time::Duration;

use gpui::{
    AnyElement, App, Context, InteractiveElement, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};
use gpui_animation::animation::TransitionExt;

use chronos_services::MenuNode;

use crate::motion;
use crate::tray_menu::TrayMenuState;
use crate::tray_menu::click_item;

use chronos_ui::{
    Theme, WindowRootExt, elevation_apply_light_chrome, elevation_blur_layer,
};

/// Fixed row height (px) — design `.ci { height: 34px }`. Fixed (not padding)
/// so the 2px accent-bar can be positioned predictably against a known box.
const ROW_H: f32 = 34.;
/// Row horizontal padding — design `.ci { padding: 0 10px }`.
const ROW_PAD_X: f32 = 10.;
/// Indentation per submenu nesting level (px).
const SUBMENU_INDENT: f32 = 16.;
/// Accent-bar geometry — design `.ci::before { top:7px; bottom:7px; width:2px;
/// border-radius:0 2px 2px 0 }`. Rest inset (12px → half-height bar) is the
/// scaleY(.5) stand-in.
const BAR_TOP: f32 = 7.;
const BAR_REST_INSET: f32 = 12.;
/// Custom overlay scrollbar — design `::-webkit-scrollbar { width:6px }`,
/// thumb `var(--border)` (= `border.subtle`), `border-radius:3px`.
const SCROLLBAR_W: f32 = 6.;
const SCROLLBAR_MARGIN: f32 = 1.;
const THUMB_MIN_H: f32 = 24.;

/// Tray menu popup view — renders the live `MenuNode` tree from the
/// `TrayMenuState` global.
pub struct TrayMenuView {
    /// Id of the currently hovered row (for accent-bar highlight).
    hovered_id: Option<i32>,
    /// View-driven enter progress 0..=1 (anchored popups — `with_animation`
    /// is invisible on live Hyprland; see `motion::arm_enter_progress`).
    enter_t: f32,
    /// Tracks the scroll column so `render` can place the custom overlay
    /// scrollbar thumb (the fork paints no scrollbars itself).
    scroll: ScrollHandle,
}

impl TrayMenuView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Menu enter follows the reference `ctx-in` curve, not the popups'
        // EaseOutBack: `cubic-bezier(.2,.8,.2,1)` over `.12s`.
        motion::arm_enter_progress_with(
            cx,
            Duration::from_millis(motion::MENU_ENTER_MS),
            motion::ease_menu_enter,
            |view, t| {
                view.enter_t = t;
            },
        );
        Self {
            hovered_id: None,
            enter_t: 0.0,
            scroll: ScrollHandle::new(),
        }
    }
}

impl TrayMenuView {
    /// Mark `row_id` as hovered (or `None` on hover-out) and repaint so the
    /// accent-bar can fade/grow in and out. Listener entry point.
    pub fn set_hovered(&mut self, row_id: Option<i32>, cx: &mut Context<Self>) {
        if self.hovered_id != row_id {
            self.hovered_id = row_id;
            cx.notify();
        }
    }
}

impl Render for TrayMenuView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let service = cx.global::<TrayMenuState>().open_service.clone();
        let nodes = cx.global::<TrayMenuState>().nodes.clone();

        let theme = Theme::global(cx);

        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let divider = theme.bg.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let hover = theme.interactive.hover;
        let accent = theme.accent.primary;
        let border_subtle = theme.border.subtle;
        let hovered_id = self.hovered_id;

        // Elevated-surface shell (T128 depth language): blur + drop shadow +
        // Light-C glow/watermark — the shared "one popup component" recipe that
        // ties the tray menu and the dock context menu together.
        let elev = theme.elevation_popup();
        let blur_layer = elevation_blur_layer(&elev, radius_lg);

        let Some(service) = service else {
            // No menu open — empty surface.
            return div().into_any_element();
        };

        if nodes.is_empty() {
            // Menu requested but not yet fetched (or empty). Show a tiny
            // placeholder so the surface isn't a zero-size transparent void.
            let mut card = div()
                .window_font(theme)
                .relative()
                .flex_col()
                .rounded(radius_lg)
                .bg(bg.alpha(0.94))
                .border_1()
                .border_color(border_subtle)
                .shadow(elev.shadows.to_vec())
                .overflow_hidden();
            card = elevation_apply_light_chrome(&elev, card);
            let card = card
                .p(px(ROW_PAD_X))
                .text_color(text_muted)
                .child("…".to_string());
            // Enter-animation (view-driven — anchored popups don't animate on map).
            return motion::apply_enter_menu(card, self.enter_t).into_any_element();
        }

        let rows: Vec<AnyElement> = nodes
            .iter()
            .filter(|n| n.visible)
            .map(|node| {
                render_node(
                    node,
                    cx,
                    service.clone(),
                    &bg,
                    &text_primary,
                    &text_muted,
                    &divider,
                    radius,
                    hover,
                    accent,
                    hovered_id,
                    0,
                )
            })
            .collect();

        // Custom overlay scrollbar thumb (6px, `--border`). Positioned from
        // the live `ScrollHandle` geometry; the wheel handler `cx.notify`s
        // this view, so `render` re-runs with a fresh offset while scrolling.
        // Only drawn when the column actually overflows.
        let scrollbar_thumb = self.scrollbar_thumb(border_subtle);

        // Menu root: elevated card shell (blur + shadow + Light-C chrome) with an
        // inner bounded scroll column. Design caps the menu at `viewport − 16px`
        // and scrolls rows past it; `mod.rs` already caps the window height, this
        // `.flex_1().min_h(0).overflow_y_scroll()` column takes that bounded
        // height and scrolls on overflow. `p(6)` + row `px(10)` ≈ design `.ctx-menu`
        // 6px padding + `.ci` `0 10px` content inset.
        let mut card = div()
            .window_font(theme)
            .relative()
            .flex_col()
            .rounded(radius_lg)
            .bg(bg.alpha(0.94))
            .border_1()
            .border_color(border_subtle)
            .shadow(elev.shadows.to_vec())
            .overflow_hidden();
        card = elevation_apply_light_chrome(&elev, card);
        card = card
            .child(blur_layer)
            .child(
                div()
                    .id("tray-menu-list")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    // NOTE: `scrollbar_width` would only *reserve* 6px (the
                    // fork paints no scrollbar) — the custom overlay thumb
                    // below keeps rows at their full width.
                    .track_scroll(&self.scroll)
                    .p(px(6.))
                    .children(rows),
            );
        let card = match scrollbar_thumb {
            Some(thumb) => card.child(thumb),
            None => card,
        };
        // Enter-animation (view-driven — anchored popups don't animate on map).
        motion::apply_enter_menu(card, self.enter_t).into_any_element()
    }
}

impl TrayMenuView {
    /// Geometry for the custom 6px overlay scrollbar, or `None` when the
    /// column fits (no thumb). See module docs for why we draw our own.
    fn scrollbar_thumb(&self, thumb_color: gpui::Hsla) -> Option<AnyElement> {
        let max = self.scroll.max_offset();
        let bounds = self.scroll.bounds();
        let viewport_h = f32::from(bounds.size.height);
        let max_y = f32::from(max.y);
        if max_y <= 0.0 || viewport_h <= 0.0 {
            return None;
        }
        let content_h = viewport_h + max_y;
        let thumb_h = (viewport_h * viewport_h / content_h).clamp(THUMB_MIN_H, viewport_h);
        let track_h = viewport_h - 2.0 * SCROLLBAR_MARGIN;
        let offset_y = (-f32::from(self.scroll.offset().y)).clamp(0.0, max_y);
        let thumb_top = if track_h > thumb_h {
            SCROLLBAR_MARGIN + offset_y * (track_h - thumb_h) / max_y
        } else {
            SCROLLBAR_MARGIN
        };
        Some(
            div()
                .absolute()
                .right(px(SCROLLBAR_MARGIN))
                .top(px(thumb_top))
                .w(px(SCROLLBAR_W))
                .h(px(thumb_h))
                .rounded(px(SCROLLBAR_W / 2.))
                .bg(thumb_color)
                .into_any_element(),
        )
    }
}

/// Render a single `MenuNode` (and, if it is a submenu, its children inline).
///
/// `cx` is threaded down so every enabled row gets its own `cx.listener`
/// hover callback (view state updates) — the fork's `on_hover` slot is a
/// plain `&mut App` closure, which can't touch `&mut TrayMenuView`.
#[allow(clippy::too_many_arguments)]
fn render_node(
    node: &MenuNode,
    cx: &Context<TrayMenuView>,
    service: String,
    bg: &gpui::Hsla,
    text_primary: &gpui::Hsla,
    text_muted: &gpui::Hsla,
    divider: &gpui::Hsla,
    radius: gpui::Pixels,
    hover: gpui::Hsla,
    accent: gpui::Hsla,
    hovered_id: Option<i32>,
    depth: u32,
) -> AnyElement {
    let indent = px(SUBMENU_INDENT * depth as f32);

    if node.separator {
        return div()
            .w_full()
            .h(px(1.))
            .my(px(4.))
            .ml(indent)
            .bg(*divider)
            .into_any_element();
    }

    let label = if node.label.is_empty() {
        "…".to_string()
    } else {
        node.label.clone()
    };

    // Toggle glyph. Design renders the check in the accent (`.ci-check`) and
    // only when checked; Radio keeps a persistent ○/◉ so the group state is
    // always visible. Rendered in a fixed 16px gutter below so all rows share
    // one left edge (design `.ci-check`/`.ci-ic` width).
    let (mark, mark_color) = match &node.toggle {
        Some((kind, checked)) => match kind {
            chronos_services::MenuToggleType::Radio => {
                if *checked {
                    (Some("◉".to_string()), accent)
                } else {
                    (Some("○".to_string()), *text_muted)
                }
            }
            chronos_services::MenuToggleType::Checkmark => {
                if *checked {
                    (Some("✓".to_string()), accent)
                } else {
                    (None, *text_muted)
                }
            }
        },
        None => (None, *text_muted),
    };

    let text_color = if node.enabled {
        *text_primary
    } else {
        *text_muted
    };

    let has_children = !node.children.is_empty();
    let is_hovered = hovered_id == Some(node.id);

    // Row chrome: 16px mark gutter + flex-1 label with ellipsis. Fixed height
    // (34px) so the 2px accent-bar can be positioned against a known box.
    // `hover` stays a no-op for disabled rows (design `.ci.disabled` has no
    // wash), and clicking only arms for enabled leaf items.
    //
    // Accent-bar: 2px left strip, inset ~7px top/bottom, rounded right edge —
    // an absolutely-positioned child (the fork has no `::before`). It is
    // ALWAYS present for enabled rows and morphs via `transition_when_else`
    // (opacity + inset grow ≈ the design's opacity+scaleY, `.12s ease`);
    // the hidden rest state lives in the base chain so nothing flashes.
    let row = div()
        .w_full()
        .h(px(ROW_H))
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(ROW_PAD_X))
        .rounded(radius)
        .ml(indent)
        .relative()
        // Explicit rest wash (transparent) in the base chain so the
        // `transition_when_else` else-branch equals it — otherwise every
        // row would start a wasted invisible 120ms animation on open
        // (base has no bg → else `bg(transparent)` differs → animates).
        .bg(gpui::transparent_black())
        .when(node.enabled, |el| {
            let bar_id = format!("tray-menu-bar-{}", node.id);
            el.child(
                div()
                    .id(bar_id.clone())
                    .absolute()
                    .left(px(0.))
                    .w(px(2.))
                    .rounded_tr(px(2.))
                    .rounded_br(px(2.))
                    .bg(accent)
                    .opacity(0.0)
                    .top(px(BAR_REST_INSET))
                    .bottom(px(BAR_REST_INSET))
                    .with_transition(bar_id)
                    .transition_when_else(
                        is_hovered,
                        Duration::from_millis(motion::MENU_ENTER_MS),
                        motion::MenuEase,
                        |s| s.opacity(1.0).top(px(BAR_TOP)).bottom(px(BAR_TOP)),
                        |s| s.opacity(0.0).top(px(BAR_REST_INSET)).bottom(px(BAR_REST_INSET)),
                    ),
            )
        })
        .child(
            div()
                .w(px(16.))
                .flex()
                .items_center()
                .children(mark.iter().map(|m| div().text_color(mark_color).child(m.clone()))),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_color(text_color)
                .child(label),
        );

    // Enabled rows: stable id (stateful handlers + transition key), hover
    // listener, and the animated wash. The hover listener is built with
    // `cx.listener` so it can update the view (`set_hovered`); the fork's
    // `on_hover` gives a plain `&mut App`, which can't. Wash + bar both
    // animate via `transition_when_else` on the *next* render (the listener
    // `cx.notify`s the view to flip the condition).
    //
    // The row type changes with `with_transition` (`Div` →
    // `AnimatedWrapper<Stateful<Div>>`), so the whole branch is typed per
    // arm — `row` is moved either into the animated chain or straight to
    // `AnyElement` for disabled rows. Clicking only arms for enabled leaf
    // items and goes through `AnimatedWrapper::on_click` (single
    // hover/click slot per element in the fork).
    let row_elem: AnyElement = if node.enabled {
        let id = node.id;
        let row_id = format!("tray-menu-item-{id}");
        let hover_listener = cx.listener(
            move |this: &mut TrayMenuView,
                  hovered: &bool,
                  _window: &mut Window,
                  cx: &mut Context<TrayMenuView>| {
                this.set_hovered(if *hovered { Some(id) } else { None }, cx);
            },
        );
        let row = row
            .cursor_pointer()
            .id(row_id.clone())
            .with_transition(row_id)
            .on_hover(hover_listener)
            .transition_when_else(
                is_hovered,
                Duration::from_millis(motion::MENU_ENTER_MS),
                motion::MenuEase,
                move |s| s.bg(hover),
                |s| s.bg(gpui::transparent_black()),
            );
        if has_children {
            row.into_any_element()
        } else {
            row.on_click(move |_event, window, cx: &mut App| {
                click_item(window, cx, id);
            })
            .into_any_element()
        }
    } else {
        row.into_any_element()
    };

    // Inline submenu expansion (no nested windows in MVP).
    if has_children {
        let child_rows: Vec<AnyElement> = node
            .children
            .iter()
            .filter(|n| n.visible)
            .map(|child| {
                render_node(
                    child,
                    cx,
                    service.clone(),
                    bg,
                    text_primary,
                    text_muted,
                    divider,
                    radius,
                    hover,
                    accent,
                    hovered_id,
                    depth + 1,
                )
            })
            .collect();
        div().w_full().children(child_rows).into_any_element()
    } else {
        row_elem
    }
}

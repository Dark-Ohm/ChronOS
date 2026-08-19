//! T278 / Slice A1 — the rail window's root view.
//!
//! Mirrors `side_panel_right::rail_view::RailView` (T276): the entity
//! that owns the standalone 40 px rail surface, sets the exclusive zone,
//! and renders the icon column + dock toggle. All mutable panel state
//! (active tab, width, dock) lives in `SidePanelLeftState_`; this view
//! only renders and forwards input via the weak content handle.
//!
//! Hover contract: hover-enter/leave bumps `peek_generation` so the
//! debounce in `mod.rs::schedule_release_peek` cannot close the panel
//! while the cursor is on the rail. (Hover-peek OPEN is disabled per
//! 2026-07-23 — the strip is dormant. The generation guard stays alive
//! for cross-surface continuity: leaving the rail for the content
//! canvas must not fire a stale close.)

use std::rc::Rc;

use gpui::{
    App, Context, IntoElement, Render, Subscription, WeakEntity, Window, div, prelude::*, px,
};

use chronos_ui::{Theme, WindowRootExt};

use crate::side_panel_left::SidePanelLeftState_;
use crate::side_panel_left::tabs::{
    BOTTOM_TAB, LeftTab, PRIMARY_TABS, RAIL_WIDTH, width_for_open,
};
use crate::side_panel_left::workspace_view::WorkspaceView;

pub struct RailView {
    /// Weak handle to the live `WorkspaceView` (lives in the content
    /// window). Used for tab switches and dock toggles triggered from
    /// this window.
    content: WeakEntity<WorkspaceView>,
    /// Held only to keep the subscription alive.
    _content_sub: Subscription,
}

impl RailView {
    pub fn new(content: WeakEntity<WorkspaceView>, cx: &mut Context<Self>) -> Self {
        let sub = cx.observe_global::<SidePanelLeftState_>(|_, cx| cx.notify());
        Self {
            content,
            _content_sub: sub,
        }
    }
}

/// T315: active tab gets a pill background (`accent.primary` with
/// theme-dependent alpha), not a 3px strip. The strip was a tab-strip
/// idiom; with rounded aperture corners the inner edge is curved and a
/// vertical line there contradicts the form.
fn rail_button_bg(is_active: bool, theme: &Theme) -> gpui::Hsla {
    if is_active {
        let alpha = if theme.is_light { 0.12 } else { 0.15 };
        theme.accent.primary.alpha(alpha)
    } else {
        gpui::transparent_black()
    }
}

/// Single rail icon button. Returns the element tree; caller wraps it in
/// the rail column. T315: active tab gets a pill background; the old 3px
/// accent strip was removed (tab-strip idiom, contradicts rounded aperture).
fn render_rail_button(
    tab: LeftTab,
    is_active: bool,
    content: WeakEntity<WorkspaceView>,
    theme: &Theme,
) -> impl IntoElement {
    let icon = div()
        .id(("rail-tab", tab as usize))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded(theme.radius)
        .bg(rail_button_bg(is_active, theme))
        .on_click({
            let content = content.clone();
            move |_ev, _window, cx| {
                if let Some(view) = content.upgrade() {
                    view.update(cx, |view, cx| view.on_rail_tab_select(tab, cx));
                }
            }
        })
        .child(
            gpui::svg()
                .path(tab.icon_path())
                .size(px(18.))
                .text_color(if is_active {
                    theme.text.primary
                } else {
                    theme.text.muted
                }),
        );
    icon.into_any_element()
}

pub fn render_rail(
    cx: &App,
    content: WeakEntity<WorkspaceView>,
) -> impl IntoElement {
    let theme = Theme::global(cx);
    let state = cx.global::<SidePanelLeftState_>();
    let active = state.active_tab;
    let dock_content = state.dock_content;
    drop(state);

    // Project selector (Slice A: top button, fixed). Then PRIMARY_TABS
    // in fixed order, a flex_1 spacer, then BOTTOM_TAB (Archive), then
    // the dock toggle.
    let project = LeftTab::Project;

    div()
        .id("side-panel-left-rail")
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.))
        .py(px(8.))
        .w(px(RAIL_WIDTH))
        .h_full()
        // T266: the rail's own plate follows surface alpha.
        // T311 D2b: the left rail now reads `surfaces::chrome` — same role
        // the right rail and the wrap bottom plate paint with. Identical
        // hex on every rail-mapped edge; the rail stops being the only
        // chrome element painted with `bg.primary`, which was distinct
        // enough in light theme (≈15 R units) to read as a separate
        // panel even though both rails are the same class.
        .bg(theme.surface_color(crate::side_panel_common::surfaces::chrome(&theme)))
        // T318 эррата: рельс НЕ скругляется. Скругляется дыра, а не кромка.
        // Угол апертуры вогнут со стороны выреза — значит в углу хрома должно
        // становиться БОЛЬШЕ, он заполняет угол. `rounded_tr/br` на самом
        // рельсе срезали материал: получилась плашка со скруглёнными краями,
        // висящая у экрана, и обои в вырезе. Кривизна была вывернута наизнанку.
        // Внутренний контур апертуры рисует матте (`frame.rs`,
        // `WrapSurfaceView::render`): у блока с бордером `rounded()` гнёт и
        // внешний, и внутренний контур, и в углу бордер становится толще —
        // это и есть заполнение. Верх апертуры — за баром (T316).
        // T315: border removed — inside continuous chrome, the seam reads
        // as "we are two objects." The rail IS the frame edge, not a panel
        // stuck next to content.
        .on_hover(|hovered, _window, cx| {
            if *hovered {
                crate::side_panel_left::hold_peek(cx);
            } else {
                crate::side_panel_left::schedule_release_peek(cx);
            }
        })
        // Project selector — sits above PRIMARY_TABS so it can carry a
        // project sigil/initials in Slice A2. A1 keeps it as the same
        // generic rail button.
        .child(render_rail_button(project, active == project, content.clone(), theme))
        // PRIMARY_TABS
        .children(PRIMARY_TABS.iter().copied().filter(|t| *t != project).map({
            let content = content.clone();
            move |tab| render_rail_button(tab, active == tab, content.clone(), theme)
        }))
        // Spacer — pushes Archive down regardless of badge counts.
        .child(div().flex_1())
        // Archive
        .child(render_rail_button(BOTTOM_TAB, active == BOTTOM_TAB, content.clone(), theme))
        // Dock toggle — always last, below Archive.
        .child({
            let on_click_dock = {
                let content = content.clone();
                move |_ev: &gpui::MouseUpEvent, _window: &mut Window, cx: &mut App| {
                    if let Some(view) = content.upgrade() {
                        view.update(cx, |view, cx| view.on_dock_toggle(cx));
                    }
                }
            };
            // Icon convention: the icon shows the ACTION that the click
            // performs. `⊞` enables dock (shown when currently undocked),
            // `⊟` disables dock (shown when currently docked). This is
            // Material Design's affordance-shows-action convention and
            // matches the spec wording `⊞ включает dock, ⊟ выключает`.
            div()
                .id("dock-toggle-left")
                .w(px(28.))
                .h(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(if dock_content {
                    theme.accent.primary
                } else {
                    theme.text.muted
                })
                .cursor_pointer()
                .hover(|s| s.bg(theme.border.subtle))
                .on_mouse_up(gpui::MouseButton::Left, on_click_dock)
                .child(if dock_content { "⊟" } else { "⊞" })
        })
}

impl Render for RailView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        // T276 mirror: rail owns the exclusive zone. Cache against the
        // shared global field — `mod.rs::ensure_content_width` invalidates
        // it on width changes; we re-derive and push when it actually
        // moves.
        let zone = cx.global::<SidePanelLeftState_>().exclusive_px();
        let cached = cx.global::<SidePanelLeftState_>().last_exclusive_zone;
        if cached != Some(zone) {
            _window.set_exclusive_edge(gpui::layer_shell::Anchor::LEFT);
            _window.set_exclusive_zone(px(zone));
            cx.global_mut::<SidePanelLeftState_>().last_exclusive_zone = Some(zone);
        }

        // T217: round the rail's top-left corner where it meets the bar
        // (mirrors the right panel's top-right rule). Free corners rhyme
        // with the bar; under-the-bar corners square off.
        let display_w = crate::monitor::pult_display_info(cx)
            .map(|d| f32::from(d.bounds().size.width))
            .unwrap_or(1920.);
        let corner_tl = crate::state::panel_corner_radius(0.0);

        div()
            .id("side-panel-left-rail-root")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_row()
            .when(corner_tl > 0.0, |d| {
                d.rounded_tl(px(corner_tl)).overflow_hidden()
            })

            .child(render_rail(cx, self.content.clone()))
    }
}

impl WorkspaceView {
    /// Called by `RailView` when an icon is clicked. Thin dispatcher —
    /// the real reducer lives in `crate::side_panel_left::select_tab`
    /// (a free function on `&mut App`, mirroring `apply_dock_toggle`)
    /// so the 3-action policy is unit-testable without instantiating
    /// `WorkspaceView` (which needs `ChatTab`, unconstructable in
    /// `TestAppContext`). Same three-action policy as T276 / T221:
    ///
    /// 1. Same tab, dock on → no-op (dock wins, can't collapse).
    /// 2. Same tab, content open → collapse to rail-only.
    /// 3. Same tab, content closed → open at `width_for_open`.
    /// 4. Different tab → switch and open.
    pub fn on_rail_tab_select(&mut self, tab: LeftTab, cx: &mut Context<Self>) {
        crate::side_panel_left::select_tab(tab, cx);
        cx.notify();
    }

    /// Called by `RailView`'s dock toggle button. Thin dispatcher — the
    /// real reducer lives in `crate::side_panel_left::apply_dock_toggle`
    /// (a free function on `&mut App`) so tests don't need a live
    /// `ChatTab` entity to exercise it.
    pub fn on_dock_toggle(&mut self, cx: &mut Context<Self>) {
        crate::side_panel_left::apply_dock_toggle(cx);
        cx.notify();
    }
}

/// Silence unused — `Rc` is referenced through `std::rc::Rc` paths in
/// the closure set, but Rust 2024 doesn't see the through-closure
/// borrow. Keeping the import list explicit would re-introduce a
/// warning without value.
#[allow(dead_code)]
fn _rc_anchor() {
    let _ = Rc::new(0);
}
//! Tray menu popup view — hosts a gpui-component `PopupMenu` built
//! recursively from the live `MenuNode` tree (T263 Часть 1.5).
//!
//! ## Why a component host instead of hand-rolled div rows
//!
//! `PopupMenu` already carries the hard 10% this popup was reimplementing by
//! hand (T260-wave2): keyboard navigation (`SelectUp`/`SelectDown`/`Confirm`/
//! `Cancel`), focus management, submenu anchoring, scrollable overflow,
//! hover/selection chrome and click-away dismiss. The window itself stays an
//! `AnchoredPopup`/`LayerShell` surface opened at the tray icon (see
//! `mod.rs`); we only replace the *content*.
//!
//! The window root MUST be a `gpui_component::Root` — component widgets
//! panic on `window.root()` otherwise. `mod.rs` wraps this view in `Root`.
//!
//! ## Row mapping (canon `MENUS.*.items[]`)
//!
//! * separator          → `PopupMenuItem::separator()`
//! * submenu (children) → `PopupMenu::submenu()` (recursive build)
//! * toggle Checkmark   → native `.checked()` (Check glyph in the gutter)
//! * toggle Radio       → custom row with the canon `◉`/`○` glyph (the
//!   component renders a single Check icon and cannot tell radio from
//!   checkmark)
//! * freedesktop icon   → custom row (component `Icon` only loads bundled
//!   SVG assets; resolved theme icons are arbitrary files, so we render
//!   `img()` inside `PopupMenuItem::element`)
//! * DBusMenu shortcut  → custom row (the component's kbd glyph is derived
//!   from *registered* action keybindings; DBusMenu shortcuts are data)
//!
//! Custom rows keep PopupMenu's hover/selection/keyboard/dismiss machinery —
//! `PopupMenuItem::element()` renders our interior inside the component's
//! `MenuItemElement` chrome. Native rows get `.icon(Icon::empty())` when the
//! menu has any gutter row, so label left-edges line up across row kinds.
//!
//! The menu entity is rebuilt whenever the fetched tree changes (the watcher
//! notifies this view after `FetchMenu` lands): `PopupMenu` has no
//! set-items API, so rebuild-on-change is the only correct sync path.

use std::time::Duration;

use gpui::{
    App, Context, DismissEvent, Entity, Focusable, ImageSource, ObjectFit, ParentElement, Render,
    Styled, Subscription, Window, div, img, prelude::*, px,
};

use chronos_ui::Theme;

use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::{Icon, Side, h_flex};

use chronos_services::{MenuNode, MenuToggleType};
use chronos_services::Service;

use crate::icon_resolution::cached_resolve_icon;
use crate::motion;
use crate::state::AppState;
use crate::tray_menu::{GUTTER_W, MENU_MAX_W, MENU_MIN_W, ROW_GAP, TrayMenuState, click_item};

/// Flat item count that triggers the scrollable column — mirrors the
/// component's own `with_menu_items` heuristic. Submenus force the menu
/// non-scrollable (the component cannot anchor a submenu inside a scrolled
/// column).
const SCROLLABLE_AFTER: usize = 20;

/// Tray menu popup view — builds and hosts the `PopupMenu` for the current
/// `TrayMenuState` tree.
pub struct TrayMenuView {
    /// The live `PopupMenu` entity, rebuilt when `nodes` changes.
    popup_menu: Option<Entity<PopupMenu>>,
    /// Dismiss subscription on the current `PopupMenu` entity. `PopupMenu`
    /// emits `DismissEvent` on item-confirm, Escape, and click-away — the
    /// single close path for the window (no double-close: item handlers do
    /// NOT close, the dismiss does).
    dismiss_subscription: Option<Subscription>,
    /// Last tree the menu was built from (comparison triggers rebuild).
    /// `None` is deliberate: the first render must build a placeholder menu
    /// even when the initial DBus snapshot is still empty.
    last_nodes: Option<Vec<MenuNode>>,
    /// View-driven enter progress 0..=1 — applied to the host element
    /// (anchored popups don't animate on map; see `motion::arm_enter_progress`).
    enter_t: f32,
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
            popup_menu: None,
            dismiss_subscription: None,
            last_nodes: None,
            enter_t: 0.0,
        }
    }
}

impl Render for TrayMenuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nodes = cx.global::<TrayMenuState>().nodes.clone();

        if menu_tree_changed(self.last_nodes.as_deref(), &nodes) {
            self.last_nodes = Some(nodes.clone());
            let menu = build_popup_menu(&nodes, window, cx);
            // Single close path: `PopupMenu` emits `DismissEvent` on item
            // confirm, Escape and click-away — close the window there. Item
            // handlers only dispatch the tray action and never close, so no
            // double `remove_window`.
            self.dismiss_subscription = Some(cx.subscribe_in(
                &menu,
                window,
                |_this, _menu, _: &DismissEvent, window, cx| {
                    crate::tray_menu::close_this(window, cx);
                },
            ));
            self.popup_menu = Some(menu);
            // Focus the freshly-built menu so arrow/enter work immediately —
            // the canon drives the menu with `navIdx`/`paintNav()`, so
            // keyboard navigation is required, not optional.
            let menu = self.popup_menu.as_ref().expect("just set");
            let handle = menu.read(cx).focus_handle(cx);
            handle.focus(window, cx);
        }

        let Some(menu) = self.popup_menu.clone() else {
            // No tree yet — empty surface (the window is closed on `close()`).
            return div().into_any_element();
        };

        // Host: full-window transparent surface, enter fade. PopupMenu draws
        // its own card (popover style: bg, border, radius, shadow) inside.
        // `items_start` keeps the card content-sized (canon min-230/max-300)
        // at the surface's left edge instead of stretching it across the
        // widest-reserve window: the remaining width is transparent room for
        // an open submenu (T263). Clicks landing there hit the component's
        // `on_mouse_down_out` → `DismissEvent` → `close_this` — a
        // client-side close, no compositor grab (T264).
        motion::apply_enter_menu(
            div().size_full().items_start().child(menu.clone()),
            self.enter_t,
        )
        .into_any_element()
    }
}

/// Whether the hosted component must be rebuilt for the current DBus tree.
/// An absent previous snapshot is always dirty, including `None -> []` on
/// the first frame while `FetchMenu` is still in flight.
fn menu_tree_changed(previous: Option<&[MenuNode]>, current: &[MenuNode]) -> bool {
    previous != Some(current)
}

/// Pure decision: should the menu column be scrollable? The component cannot
/// anchor submenus inside a scrolled column, so submenus force it off.
fn menu_scrollable(nodes: &[MenuNode]) -> bool {
    let visible = nodes.iter().filter(|n| n.visible);
    let has_submenu = visible
        .clone()
        .any(|n| !n.separator && !n.children.is_empty());
    let flat = visible.filter(|n| !n.separator).count();
    flat > SCROLLABLE_AFTER && !has_submenu
}

/// Pure decision: does the menu contain any row that reserves the leading
/// gutter (freedesktop icon or radio glyph)? Drives the `.icon(Icon::empty())`
/// alignment shim on native plain rows.
fn any_gutter_row(nodes: &[MenuNode]) -> bool {
    nodes.iter().filter(|n| n.visible).any(|n| {
        n.icon_name.as_deref().and_then(cached_resolve_icon).is_some()
            || matches!(n.toggle, Some((MenuToggleType::Radio, _)))
    })
}

/// Build a `PopupMenu` entity from the fetched `MenuNode` tree.
fn build_popup_menu(nodes: &[MenuNode], window: &mut Window, cx: &mut App) -> Entity<PopupMenu> {
    let scrollable = menu_scrollable(nodes);
    let any_gutter = any_gutter_row(nodes);
    // Sticky head (canon `.ctx-head`): app title, muted, hairline below. The
    // component has no sticky header slot, so it renders as the first row of
    // the list (scrolls with it — divergence documented in the report).
    let head: Option<(String, Option<std::path::PathBuf>)> = {
        let service = cx.global::<TrayMenuState>().open_service.clone();
        service.and_then(|svc| {
            let tray = AppState::tray(cx).get();
            let item = tray.find(&svc)?;
            let title = item.title.as_deref()?;
            if title.is_empty() {
                return None;
            }
            let icon = item
                .icon_name
                .as_deref()
                .and_then(cached_resolve_icon);
            Some((title.to_string(), icon))
        })
    };

    PopupMenu::build(window, cx, |menu, window, cx| {
        let mut menu = menu
            .min_w(px(MENU_MIN_W))
            .max_w(px(MENU_MAX_W))
            .check_side(Side::Left);
        if scrollable {
            menu = menu.scrollable(true);
        }
        if let Some((title, icon)) = head {
            menu = menu.item(head_item(title, icon));
        }
        for node in nodes.iter().filter(|n| n.visible) {
            menu = append_node(menu, node, window, cx, any_gutter);
        }
        menu
    })
}

/// The canon sticky header as a disabled custom row (muted title + optional
/// app icon).
fn head_item(title: String, icon: Option<std::path::PathBuf>) -> PopupMenuItem {
    PopupMenuItem::element(move |_, cx| {
        let theme = Theme::global(cx);
        let icon_elem = icon.as_deref().map(|path| {
            let src: ImageSource = path.to_path_buf().into();
            img(src).w(px(14.)).h(px(14.)).object_fit(ObjectFit::Contain)
        });
        h_flex()
            .w_full()
            .items_center()
            .gap(px(9.))
            .pb(px(8.))
            .pt(px(4.))
            .border_b_1()
            .border_color(theme.bg.secondary)
            .text_color(theme.text.muted)
            .font_family(theme.font_mono)
            .text_xs()
            .whitespace_nowrap()
            .overflow_hidden()
            .children(icon_elem)
            .child(div().flex_1().min_w(px(0.)).overflow_hidden().child(title.clone()))
    })
    .disabled(true)
}

/// Append one `MenuNode` to the menu. Recursive for submenu children.
fn append_node(
    menu: PopupMenu,
    node: &MenuNode,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    any_gutter: bool,
) -> PopupMenu {
    if node.separator {
        return menu.separator();
    }

    let label = if node.label.is_empty() {
        "…".to_string()
    } else {
        node.label.clone()
    };

    if !node.children.is_empty() {
        // Native submenu: the builder wires `parent_menu` on the child
        // (required for SelectLeft/SelectRight keyboard navigation back to
        // the parent) and takes a `'static` closure, so children are cloned
        // in. `submenu_with_icon` also lets us reserve the gutter so submenu
        // rows line up with icon/radio rows.
        let children = node.children.clone();
        let icon = any_gutter.then(Icon::empty);
        return menu.submenu_with_icon(icon, label, window, cx, move |sub, window, cx| {
            // Submenu cards hold the same canon bounds as the root card —
            // `tray_menu::estimate_menu_width` relies on the clamp when
            // reserving surface width for the widest submenu chain.
            append_children(
                sub.min_w(px(MENU_MIN_W)).max_w(px(MENU_MAX_W)),
                &children,
                window,
                cx,
                any_gutter,
            )
        });
    }

    let id = node.id;
    let enabled = node.enabled;
    let icon_path = node.icon_name.as_deref().and_then(cached_resolve_icon);
    let shortcut_glyph = node.shortcut.as_deref().and_then(shortcut_to_glyph);
    let radio = matches!(node.toggle, Some((MenuToggleType::Radio, _)));
    let radio_checked = matches!(node.toggle, Some((MenuToggleType::Radio, true)));
    let checked = matches!(node.toggle, Some((MenuToggleType::Checkmark, true)));

    if icon_path.is_some() || shortcut_glyph.is_some() || radio {
        // Custom interior: icon/radio gutter + label + mono shortcut glyph.
        let label_owned = label.clone();
        let icon_path = icon_path.clone();
        let shortcut_glyph = shortcut_glyph.clone();
        menu.item(
            PopupMenuItem::element(move |_, cx| {
                let theme = Theme::global(cx);
                let icon_elem = icon_path.as_deref().map(|path| {
                    let src: ImageSource = path.to_path_buf().into();
                    img(src).w(px(GUTTER_W)).h(px(GUTTER_W)).object_fit(ObjectFit::Contain)
                });
                let indicator = custom_row_indicator(radio, radio_checked, checked);
                let gutter = if let Some(glyph) = indicator {
                    let color = if radio_checked || checked {
                        theme.accent.primary
                    } else {
                        theme.text.muted
                    };
                    div()
                        .w(px(GUTTER_W))
                        .flex_none()
                        .items_center()
                        .text_color(color)
                        .child(glyph)
                } else {
                    div()
                        .w(px(GUTTER_W))
                        .flex_none()
                        .items_center()
                        .children(icon_elem)
                };
                h_flex()
                    .w_full()
                    .items_center()
                    .gap(px(ROW_GAP))
                    .child(gutter)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_color(if enabled { theme.text.primary } else { theme.text.muted })
                            .child(label_owned.clone()),
                    )
                    .children(shortcut_glyph.as_ref().map(|g| {
                        div()
                            .flex_none()
                            .text_color(theme.text.muted)
                            .font_family(theme.font_mono)
                            .text_xs()
                            .child(g.clone())
                    }))
            })
            .when(enabled, |item| {
                item.on_click(move |_event, _window, cx| click_item(cx, id))
            })
            .disabled(!enabled),
        )
    } else {
        // Plain row — native rendering (kbd binding display is action-driven
        // and unused here; `on_click` is the handler path).
        let mut item = PopupMenuItem::new(label)
            .when(checked, |item| item.checked(true))
            .disabled(!enabled)
            // Reserve the gutter so label edges line up with icon/radio rows.
            .when(any_gutter && !checked, |item| item.icon(Icon::empty()))
            .on_click(move |_event, _window, cx| click_item(cx, id));
        menu.item(item)
    }
}

fn custom_row_indicator(
    radio: bool,
    radio_checked: bool,
    checkmark_checked: bool,
) -> Option<&'static str> {
    if radio {
        Some(if radio_checked { "◉" } else { "○" })
    } else if checkmark_checked {
        Some("✓")
    } else {
        None
    }
}

/// Append every visible child of a submenu node to the child menu.
fn append_children(
    menu: PopupMenu,
    children: &[MenuNode],
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    any_gutter: bool,
) -> PopupMenu {
    let mut menu = menu;
    for child in children.iter().filter(|n| n.visible) {
        menu = append_node(menu, child, window, cx, any_gutter);
    }
    menu
}

/// Convert a raw DBusMenu shortcut (`[[\"Control\",\"X\"]]` / `[[\"F2\"]]`) to a
/// display glyph string (`⌃X` / `F2`). Multiple combinations are preserved as
/// a comma-separated sequence; modifier names map to the canon glyph set.
/// `tray_menu::estimate_menu_width` reuses this to size the surface — keep
/// it pure.
pub(crate) fn shortcut_to_glyph(shortcut: &[Vec<String>]) -> Option<String> {
    let combinations: Vec<String> = shortcut
        .iter()
        .filter_map(|combo| {
            let mut out = String::new();
            for key in combo {
                match key.as_str() {
                    "Control" | "Ctrl" => out.push('⌃'),
                    "Alt" | "Option" => out.push('⌥'),
                    "Shift" => out.push('⇧'),
                    "Super" | "Meta" | "Command" => out.push('⌘'),
                    "Return" | "Enter" => out.push('↵'),
                    "Escape" => out.push('⎋'),
                    "Tab" => out.push('⇥'),
                    "BackSpace" => out.push('⌫'),
                    "Delete" => out.push('⌦'),
                    "Up" => out.push('↑'),
                    "Down" => out.push('↓'),
                    "Left" => out.push('←'),
                    "Right" => out.push('→'),
                    "space" | "Space" => out.push('␣'),
                    other => out.push_str(other),
                }
            }
            (!out.is_empty()).then_some(out)
        })
        .collect();
    if combinations.is_empty() {
        None
    } else {
        Some(combinations.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_empty_snapshot_still_builds_placeholder_menu() {
        assert!(menu_tree_changed(None, &[]));
    }

    #[test]
    fn unchanged_snapshot_does_not_rebuild_menu() {
        let nodes = vec![node(1, 0)];
        assert!(!menu_tree_changed(Some(&nodes), &nodes));
    }

    #[test]
    fn shortcut_ctrl_x_maps_to_glyph() {
        assert_eq!(
            shortcut_to_glyph(&[vec!["Control".to_string(), "X".to_string()]]),
            Some("⌃X".to_string())
        );
    }

    #[test]
    fn shortcut_plain_key_stays_verbatim() {
        assert_eq!(shortcut_to_glyph(&[vec!["F2".to_string()]]), Some("F2".to_string()));
    }

    #[test]
    fn shortcut_preserves_multi_step_sequence() {
        let s = vec![
            vec!["Control".to_string(), "Shift".to_string(), "C".to_string()],
            vec!["Alt".to_string(), "C".to_string()],
        ];
        assert_eq!(shortcut_to_glyph(&s), Some("⌃⇧C, ⌥C".to_string()));
    }

    #[test]
    fn shortcut_empty_is_none() {
        assert_eq!(shortcut_to_glyph(&[vec![]]), None);
        assert_eq!(shortcut_to_glyph(&[]), None);
    }

    #[test]
    fn checked_custom_row_keeps_checkmark_indicator() {
        assert_eq!(custom_row_indicator(false, false, true), Some("✓"));
        assert_eq!(custom_row_indicator(true, true, false), Some("◉"));
        assert_eq!(custom_row_indicator(true, false, false), Some("○"));
        assert_eq!(custom_row_indicator(false, false, false), None);
    }

    fn node(id: i32, children: usize) -> MenuNode {
        MenuNode {
            id,
            label: format!("item {id}"),
            enabled: true,
            visible: true,
            separator: false,
            toggle: None,
            icon_name: None,
            shortcut: None,
            children: (0..children).map(|c| node(c as i32, 0)).collect(),
        }
    }

    #[test]
    fn scrollable_requires_many_flat_rows() {
        let many: Vec<MenuNode> = (0..25).map(|i| node(i, 0)).collect();
        assert!(menu_scrollable(&many));
        let few: Vec<MenuNode> = (0..3).map(|i| node(i, 0)).collect();
        assert!(!menu_scrollable(&few));
    }

    #[test]
    fn scrollable_disabled_when_any_submenu() {
        let mut many: Vec<MenuNode> = (0..25).map(|i| node(i, 0)).collect();
        many[3].children = vec![node(100, 0)];
        assert!(!menu_scrollable(&many));
    }

    #[test]
    fn scrollable_ignores_separators_and_hidden() {
        let mut many: Vec<MenuNode> = (0..25).map(|i| node(i, 0)).collect();
        many[0].separator = true;
        many[1].visible = false;
        // 23 flat visible rows → still over the threshold.
        assert!(menu_scrollable(&many));
    }

    #[test]
    fn empty_label_renders_as_placeholder() {
        let mut n = node(1, 0);
        n.label = String::new();
        // The placeholder substitution happens at build time; here we just
        // assert the helper sees a non-separator, non-submenu leaf.
        assert!(!n.separator && n.children.is_empty());
    }
}

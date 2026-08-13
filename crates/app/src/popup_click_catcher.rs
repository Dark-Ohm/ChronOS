//! Shared Wayland click-catcher for no-grab anchored popups.
//!
//! An xdg-popup with `grab: false` cannot receive a pointer event aimed at
//! another application. This small transparent layer-surface receives only
//! the area outside the popup host; the popup remains interactive through a
//! hole in the surface's input region. It never requests keyboard focus or a
//! compositor popup grab.

use std::rc::Rc;

use gpui::{
    AnyWindowHandle, App, Bounds, Context, DisplayId, InteractiveElement, MouseButton, Pixels,
    Render, Size, Window, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div,
    layer_shell::*, point, prelude::*, px,
};

pub(crate) type ClickAwayHandler = Rc<dyn Fn(&mut Window, &mut App)>;

pub(crate) struct ClickCatcherView {
    handler: ClickAwayHandler,
}

impl Render for ClickCatcherView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let handler = self.handler.clone();
        div()
            .id("popup-click-catcher")
            .size_full()
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                (handler)(window, cx);
            })
    }
}

/// Build a full-output layer surface with no exclusive zone and no keyboard
/// interactivity. The input region is installed immediately after creation.
pub(crate) fn open_for_popup(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
    popup_size: Size<Pixels>,
    handler: ClickAwayHandler,
) -> anyhow::Result<AnyWindowHandle> {
    let display = crate::monitor::pult_display_info(cx)
        .ok_or_else(|| anyhow::anyhow!("no display available for popup click-catcher"))?;
    let output_size = display.bounds().size;
    let popup_hole = protected_popup_bounds(anchor_rect, popup_size, output_size);
    open(
        cx,
        Some(display.id()),
        output_size,
        outside_input_regions(output_size, popup_hole),
        handler,
    )
}

pub(crate) fn open(
    cx: &mut App,
    display_id: Option<DisplayId>,
    output_size: Size<Pixels>,
    input_regions: Vec<Bounds<Pixels>>,
    handler: ClickAwayHandler,
) -> anyhow::Result<AnyWindowHandle> {
    let handle = cx.open_window(
        window_options(display_id, output_size),
        move |_window, cx| cx.new(|_| ClickCatcherView { handler }),
    )?;

    let any_handle: AnyWindowHandle = handle.into();
    let _ = any_handle.update(cx, |_, window, _| {
        window.set_input_region(Some(&input_regions));
    });
    Ok(any_handle)
}

fn window_options(display_id: Option<DisplayId>, output_size: Size<Pixels>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: output_size,
        })),
        app_id: Some("chronos-popup-click-catcher".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "chronos-popup-click-catcher".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Return a conservative pass-through hole for the anchored popup. The
/// compositor may slide/flip the popup, so include both sides of the anchor;
/// this trades a little extra pass-through area for never covering the menu.
pub(crate) fn protected_popup_bounds(
    anchor_rect: Bounds<Pixels>,
    popup_size: Size<Pixels>,
    output: Size<Pixels>,
) -> Bounds<Pixels> {
    let output_w = f32::from(output.width).max(0.0);
    let output_h = f32::from(output.height).max(0.0);
    let anchor_left = f32::from(anchor_rect.origin.x);
    let anchor_right = f32::from(anchor_rect.bottom_right().x);
    let anchor_bottom = f32::from(anchor_rect.bottom_right().y);
    let popup_w = f32::from(popup_size.width).max(0.0);
    let popup_h = f32::from(popup_size.height).max(0.0);
    let left = (anchor_left - popup_w).clamp(0.0, output_w);
    let right = (anchor_right + popup_w).clamp(left, output_w);
    let top = anchor_bottom.clamp(0.0, output_h);
    let bottom = (top + popup_h + 16.0).clamp(top, output_h);
    Bounds::from_corners(point(px(left), px(top)), point(px(right), px(bottom)))
}

/// Return four rectangles around `popup`, all in output-local coordinates.
/// Their union receives pointer input; the popup rectangle is a pass-through
/// hole for the anchored menu surface underneath the catcher.
pub(crate) fn outside_input_regions(
    output: Size<Pixels>,
    popup: Bounds<Pixels>,
) -> Vec<Bounds<Pixels>> {
    let output_w = f32::from(output.width).max(0.0);
    let output_h = f32::from(output.height).max(0.0);
    let x0 = f32::from(popup.origin.x).clamp(0.0, output_w);
    let y0 = f32::from(popup.origin.y).clamp(0.0, output_h);
    let x1 = f32::from(popup.bottom_right().x).clamp(0.0, output_w).max(x0);
    let y1 = f32::from(popup.bottom_right().y).clamp(0.0, output_h).max(y0);

    let mut regions = Vec::with_capacity(4);
    push_region(
        &mut regions,
        0.0,
        0.0,
        output_w,
        y0,
    );
    push_region(
        &mut regions,
        0.0,
        y1,
        output_w,
        output_h - y1,
    );
    push_region(&mut regions, 0.0, y0, x0, y1 - y0);
    push_region(&mut regions, x1, y0, output_w - x1, y1 - y0);
    regions
}

fn push_region(regions: &mut Vec<Bounds<Pixels>>, x: f32, y: f32, w: f32, h: f32) {
    if w > 0.0 && h > 0.0 {
        regions.push(Bounds::new(point(px(x), px(y)), Size::new(px(w), px(h))));
    }
}

#[cfg(test)]
mod tests {
    use super::{outside_input_regions, protected_popup_bounds};
    use gpui::{Bounds, Size, point, px};

    #[test]
    fn protected_popup_bounds_covers_both_possible_horizontal_placements() {
        let output = Size::new(px(1000.), px(800.));
        let anchor = Bounds::new(point(px(900.), px(0.)), Size::new(px(24.), px(32.)));
        let hole = protected_popup_bounds(anchor, Size::new(px(230.), px(180.)), output);

        assert_eq!(hole.origin, point(px(670.), px(32.)));
        assert_eq!(hole.bottom_right(), point(px(1000.), px(228.)));
    }

    #[test]
    fn outside_regions_cover_the_four_edges_and_leave_popup_hole() {
        let output = Size::new(px(1000.), px(800.));
        let popup = Bounds::new(point(px(400.), px(100.)), Size::new(px(200.), px(300.)));
        let regions = outside_input_regions(output, popup);

        assert_eq!(regions.len(), 4);
        assert_eq!(
            regions[0],
            Bounds::new(point(px(0.), px(0.)), Size::new(px(1000.), px(100.)))
        );
        assert_eq!(
            regions[1],
            Bounds::new(point(px(0.), px(400.)), Size::new(px(1000.), px(400.)))
        );
        assert_eq!(
            regions[2],
            Bounds::new(point(px(0.), px(100.)), Size::new(px(400.), px(300.)))
        );
        assert_eq!(
            regions[3],
            Bounds::new(point(px(600.), px(100.)), Size::new(px(400.), px(300.)))
        );
    }

    #[test]
    fn outside_regions_are_clamped_to_output_bounds() {
        let output = Size::new(px(1000.), px(800.));
        let popup = Bounds::new(point(px(-100.), px(-20.)), Size::new(px(1200.), px(900.)));
        let regions = outside_input_regions(output, popup);

        assert!(regions.iter().all(|region| {
            region.origin.x >= px(0.)
                && region.origin.y >= px(0.)
                && region.bottom_right().x <= output.width
                && region.bottom_right().y <= output.height
        }));
    }
}

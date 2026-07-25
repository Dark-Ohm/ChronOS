//! Elevated-surface + blur tokens — единый язык глубины для карточек шелла.
//!
//! T128 (visual depth wave 1/4) + errata helpers: views read
//! [`Theme::elevation_popup`] and build chrome via
//! [`elevation_blur_layer`] / [`elevation_glow_bar`] / [`elevation_watermark`].
//!
//! Style helpers do not need `&mut Window` except the blur paint path, which
//! runs inside a `canvas` closure (fork `paint_blur`).

use std::sync::OnceLock;

use gpui::{
    App, BoxShadow, Corners, Div, Hsla, ParentElement, Pixels, Styled, Window, canvas, div, px, svg,
};

use super::{Theme, parse_hex};

/// Blur parameters for frosted-glass (`window.paint_blur`).
///
/// Fork: `paint_blur(bounds, blur_radius, corner_radii, tint, saturation)`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BlurSpec {
    /// Gaussian radius. Canonical post-T121/T125: 18px.
    pub radius: Pixels,
    /// Tint over blur (white α≈0.06).
    pub tint: Hsla,
    /// Saturation boost (fork param). Canonical: 1.15.
    pub saturation: f32,
}

/// Elevated surface tokens for floating cards / panel content.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ElevationTokens {
    /// Card shadows. Empty in dark (blur carries depth).
    pub shadows: &'static [BoxShadow],
    /// Frosted blur spec (both schemes share the same numbers).
    pub blur: BlurSpec,
    /// Card corner radius (usually `theme.radius_lg`).
    pub radius: Pixels,
    /// Light-C top glow + watermark accent. `None` in dark.
    pub glow: Option<Hsla>,
    /// Whether to paint hexagon watermark (Light C only).
    pub watermark: bool,
}

impl Theme {
    /// Popup / elevated-card depth language.
    ///
    /// - **Dark:** blur-only (r18, tint α0.06, sat 1.15), no drop-shadow.
    /// - **Light (Light C):** same blur + indigo drop (y6/blur24) + inset
    ///   accent ring + glow + watermark.
    pub fn elevation_popup(&self) -> ElevationTokens {
        let blur = BlurSpec {
            radius: px(18.0),
            tint: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 0.06,
            },
            saturation: 1.15,
        };

        if self.is_light {
            ElevationTokens {
                shadows: light_popup_shadows(),
                blur,
                radius: self.radius_lg,
                glow: Some(self.accent.primary),
                watermark: true,
            }
        } else {
            ElevationTokens {
                shadows: &EMPTY_SHADOWS,
                blur,
                radius: self.radius_lg,
                glow: None,
                watermark: false,
            }
        }
    }
}

/// Empty shadow pool (dark).
pub static EMPTY_SHADOWS: [BoxShadow; 0] = [];

static LIGHT_SHADOWS_LOCK: OnceLock<Vec<BoxShadow>> = OnceLock::new();

fn light_popup_shadows() -> &'static [BoxShadow] {
    LIGHT_SHADOWS_LOCK
        .get_or_init(|| {
            // Fixed hexes — parse cannot fail on these literals.
            let indigo = parse_hex("3c406e").unwrap_or_else(|_| Hsla::default());
            let accent = parse_hex("007acc").unwrap_or_else(|_| Hsla::default());
            vec![
                BoxShadow::new(px(0.0), px(6.0), indigo).blur_radius(px(24.0)),
                BoxShadow::new(px(0.0), px(0.0), accent)
                    .spread_radius(px(1.0))
                    .inset(),
            ]
        })
        .as_slice()
}

// ── View helpers (shared chrome) ─────────────────────────────────────

/// Absolute frosted layer that paints `elev.blur` inside a `canvas`.
///
/// `corner_radius` is the card's corner (often `elev.radius` or a local
/// override like updates' 6px).
pub fn elevation_blur_layer(elev: &ElevationTokens, corner_radius: Pixels) -> Div {
    let radius = elev.blur.radius;
    let tint = elev.blur.tint;
    let sat = elev.blur.saturation;
    div().absolute().inset_0().child(canvas(
        |_bounds, _window, _cx| {},
        move |bounds, _state, window: &mut Window, _cx: &mut App| {
            window.paint_blur(bounds, radius, Corners::all(corner_radius), tint, sat);
        },
    ))
}

/// 1px top accent glow strip (Light C).
pub fn elevation_glow_bar(glow: Hsla) -> Div {
    div()
        .absolute()
        .top(px(0.))
        .left(px(0.))
        .right(px(0.))
        .h(px(1.))
        .bg(glow)
        .opacity(0.4)
}

/// Corner hexagon watermark (Light C).
pub fn elevation_watermark(glow: Hsla) -> Div {
    div()
        .absolute()
        .top(px(-30.))
        .right(px(-30.))
        .size(px(140.))
        .child(
            svg()
                .path("icons/hexagon-sigil.svg")
                .size(px(140.))
                .text_color(glow)
                .opacity(0.18),
        )
}

/// Attach Light-C glow (+ optional watermark) to a card `Div`.
///
/// Dark: returns `card` unchanged. Light: glow bar always when `glow` is
/// set; watermark only if `elev.watermark`.
pub fn elevation_apply_light_chrome(elev: &ElevationTokens, card: Div) -> Div {
    match elev.glow {
        Some(glow) => {
            let card = card.child(elevation_glow_bar(glow));
            if elev.watermark {
                card.child(elevation_watermark(glow))
            } else {
                card
            }
        }
        None => card,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_popup_is_blur_only() {
        let t = Theme::default();
        let e = t.elevation_popup();
        assert!(e.shadows.is_empty());
        assert!(e.glow.is_none());
        assert!(!e.watermark);
        assert_eq!(e.blur.radius, px(18.0));
        assert_eq!(e.blur.saturation, 1.15);
        assert_eq!(e.blur.tint.a, 0.06);
        assert_eq!(e.radius, t.radius_lg);
    }

    #[test]
    fn light_popup_has_shadows_glow_watermark() {
        let light = Theme::select_scheme(Some("Light".to_string()));
        assert!(light.is_light);
        let e = light.elevation_popup();
        assert_eq!(e.shadows.len(), 2);
        assert_eq!(e.shadows[0].offset.y, px(6.0));
        assert_eq!(e.shadows[0].blur_radius, px(24.0));
        assert!(!e.shadows[0].inset);
        assert!(e.shadows[1].inset);
        assert_eq!(e.shadows[1].spread_radius, px(1.0));
        assert_eq!(e.glow, Some(light.accent.primary));
        assert!(e.watermark);
        assert_eq!(e.blur.radius, px(18.0));
        assert_eq!(e.blur.tint.a, 0.06);
    }

    #[test]
    fn light_and_dark_differ_where_intended() {
        let dark = Theme::default().elevation_popup();
        let light = Theme::select_scheme(Some("Light".to_string())).elevation_popup();
        assert_ne!(dark.shadows.is_empty(), light.shadows.is_empty());
        assert_ne!(dark.glow.is_some(), light.glow.is_some());
        assert_ne!(dark.watermark, light.watermark);
        assert_eq!(dark.blur, light.blur);
    }

    #[test]
    fn light_shadow_pool_is_stable() {
        let a = light_popup_shadows();
        let b = light_popup_shadows();
        assert!(std::ptr::eq(a.as_ptr(), b.as_ptr()));
    }
}

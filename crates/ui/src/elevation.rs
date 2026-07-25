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
    /// - **Dark:** frosted blur + soft elevated drop-shadow (readable depth).
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
                shadows: dark_popup_shadows(),
                blur,
                radius: self.radius_lg,
                glow: None,
                watermark: false,
            }
        }
    }
}

/// Empty pool kept for tests / callers that want “no shadows”.
pub static EMPTY_SHADOWS: [BoxShadow; 0] = [];

static DARK_SHADOWS_LOCK: OnceLock<Vec<BoxShadow>> = OnceLock::new();
static LIGHT_SHADOWS_LOCK: OnceLock<Vec<BoxShadow>> = OnceLock::new();

/// Soft elevated shadow for dark cards (depth without Light-C ring).
fn dark_popup_shadows() -> &'static [BoxShadow] {
    DARK_SHADOWS_LOCK
        .get_or_init(|| {
            // Deep ink, high blur, moderate alpha — reads as lift on Mocha.
            let mut ink = parse_hex("11111b").unwrap_or_else(|_| Hsla::default());
            ink.a = 0.55;
            let mut soft = parse_hex("000000").unwrap_or_else(|_| Hsla::default());
            soft.a = 0.35;
            vec![
                BoxShadow::new(px(0.0), px(12.0), soft).blur_radius(px(32.0)),
                BoxShadow::new(px(0.0), px(4.0), ink).blur_radius(px(12.0)),
            ]
        })
        .as_slice()
}

fn light_popup_shadows() -> &'static [BoxShadow] {
    LIGHT_SHADOWS_LOCK
        .get_or_init(|| {
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
    fn dark_popup_has_soft_elevation_shadow() {
        let t = Theme::default();
        let e = t.elevation_popup();
        assert_eq!(e.shadows.len(), 2, "dark: soft double-layer drop");
        assert!(!e.shadows[0].inset);
        assert!(!e.shadows[1].inset);
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
        // Both have shadows now; light has inset ring, dark does not.
        assert!(dark.shadows.iter().all(|s| !s.inset));
        assert!(light.shadows.iter().any(|s| s.inset));
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

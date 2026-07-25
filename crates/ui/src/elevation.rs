//! Elevated-surface + blur tokens — единый язык глубины для карточек шелла.
//!
//! T128 (visual depth wave 1/4). Цель — убрать copy-paste
//! `BoxShadow::new(px(0.), px(6.), …)` / ad-hoc `paint_blur` из каждого
//! попапа и дать один продуктовый источник истинны для теней/blur/glow.
//!
//! Правило из задачи: хелперы обязаны работать БЕЗ `Window` — это только
//! стиль (тени/цвета/радиусы). `paint_blur` живёт в paint-фазе view
//! (`canvas`-замыкание) и читает радиус/тинт из [`BlurSpec`].

use std::sync::OnceLock;

use gpui::{BoxShadow, Hsla, Pixels, px};

use super::{Theme, parse_hex};

/// Blur-параметры frosted-glass слоя (читаются в `window.paint_blur`).
///
/// Сигнатура форка: `paint_blur(bounds, blur_radius, corner_radii, tint,
/// saturation)`. Радиус/углы задаются вызывающим (обычно `theme.radius_lg`);
/// здесь — только tint + saturation, общие для всех попапов.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BlurSpec {
    /// Радиус размытия (px). Сейчас 18 — эталон из post-T121 volume/system.
    pub radius: Pixels,
    /// Цветовой тинт поверх размытия (light tint, alpha ~0.06).
    pub tint: Hsla,
    /// Насыщенность (fork-параметр `paint_blur`). 1.15 — текущий рецепт.
    pub saturation: f32,
}

/// Токены приподнятой поверхности (popups / cards).
///
/// `shadows` пуст в тёмной схеме (там читаемость даёт blur+fill, а не
/// drop-shadow), и заполнен в светлой (Light C). `glow` — опциональный
/// Light-C premium-слой (1px акцентное ребро), поэтому `Option`, а не
/// пустой `Hsla`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ElevationTokens {
    /// Тени карточки. Пусто в тёмной схеме.
    pub shadows: &'static [BoxShadow],
    /// Blur-спецификация frosted-слоя (читается в paint-фазе).
    pub blur: BlurSpec,
    /// Радиус скругления карточки (обычно `theme.radius_lg`).
    pub radius: Pixels,
    /// Акцент для Light-C glow-ребра. `None` → слой не рисуем.
    pub glow: Option<Hsla>,
}

impl Theme {
    /// Токены приподнятой поверхности для всплывающих карточек.
    ///
    /// - **Тёмная:** blur-only (light tint alpha 0.06, radius 18, sat
    ///   1.15) — эталон post-T121/T125. Теней нет (на тёмном фоне drop-
    ///   shadow не читается и только замыливает).
    /// - **Светлая (Light C):** те же blur-параметры + мягкая indigo
    ///   drop-shadow (y=6, blur 24) + 1px inset accent ring (glow). Тени
    ///   измерены из канонического рецепта `volume_popup`/`updates_popup`
    ///   (`BoxShadow::new(0,6, 0x3c40_6e).blur(24)` + inset
    ///   `0x007a_cc.spread(1).inset()`).
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
            }
        } else {
            ElevationTokens {
                shadows: &EMPTY_SHADOWS,
                blur,
                radius: self.radius_lg,
                glow: None,
            }
        }
    }
}

/// Пустой пул теней (тёмная схема — blur-only).
pub static EMPTY_SHADOWS: [BoxShadow; 0] = [];

/// Light-C тени попапа: мягкая indigo drop-shadow + 1px inset accent ring.
///
/// `BoxShadow::new` не `const`, поэтому пул строится один раз лениво и
/// кэшируется в `OnceLock`, отдавая `&'static [BoxShadow]`. [`Theme`] —
/// `Copy`, а хелперы не должны аллоцировать на каждом render, поэтому
/// токены ссылаются на этот единственный статический пул.
static LIGHT_SHADOWS_LOCK: OnceLock<Vec<BoxShadow>> = OnceLock::new();

fn light_popup_shadows() -> &'static [BoxShadow] {
    LIGHT_SHADOWS_LOCK
        .get_or_init(|| {
            vec![
                // 0x3c40_6e — мягкая indigo drop-shadow (y=6, blur 24).
                BoxShadow::new(px(0.0), px(6.0), parse_hex("3c406e").unwrap())
                    .blur_radius(px(24.0)),
                // 0x007a_cc — 1px inset accent ring (Light C glow-ребро).
                BoxShadow::new(px(0.0), px(0.0), parse_hex("007acc").unwrap())
                    .spread_radius(px(1.0))
                    .inset(),
            ]
        })
        .as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_popup_is_blur_only() {
        let t = Theme::default(); // тёмная
        let e = t.elevation_popup();
        assert!(e.shadows.is_empty(), "тёмная схема — без drop-shadow");
        assert!(e.glow.is_none(), "тёмная схема — без glow-ребра");
        assert_eq!(e.blur.radius, px(18.0));
        assert_eq!(e.blur.saturation, 1.15);
        assert_eq!(e.blur.tint.a, 0.06);
        assert_eq!(e.radius, t.radius_lg);
    }

    #[test]
    fn light_popup_has_shadows_and_glow() {
        let light = Theme::select_scheme(Some("Light".to_string()));
        assert!(light.is_light);
        let e = light.elevation_popup();
        assert_eq!(e.shadows.len(), 2, "Light C: drop + inset ring");
        // drop-shadow: offset y=6, blur 24
        let drop = &e.shadows[0];
        assert_eq!(drop.offset.y, px(6.0));
        assert_eq!(drop.blur_radius, px(24.0));
        assert!(!drop.inset);
        // inset ring: spread 1, inset
        let ring = &e.shadows[1];
        assert!(ring.inset);
        assert_eq!(ring.spread_radius, px(1.0));
        // glow = accent.primary (#007acc), не переопределён в Light C
        assert_eq!(e.glow, Some(light.accent.primary));
        // blur-параметры идентичны тёмной (общий язык глубины)
        assert_eq!(e.blur.radius, px(18.0));
        assert_eq!(e.blur.tint.a, 0.06);
    }

    #[test]
    fn light_and_dark_differ_where_intended() {
        let dark = Theme::default().elevation_popup();
        let light = Theme::select_scheme(Some("Light".to_string())).elevation_popup();
        assert_ne!(dark.shadows.is_empty(), light.shadows.is_empty());
        assert_ne!(dark.glow.is_some(), light.glow.is_some());
        // но blur-язык общий
        assert_eq!(dark.blur, light.blur);
    }

    #[test]
    fn light_shadow_pool_is_stable() {
        // один и тот же 'static-срез между вызовами (без аллокаций на render)
        let a = light_popup_shadows();
        let b = light_popup_shadows();
        assert!(std::ptr::eq(a.as_ptr(), b.as_ptr()));
    }
}

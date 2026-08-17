//! Configurable surface transparency (T266).
//!
//! One user-facing alpha axis multiplies the OUTER surface plates of the
//! shell (bar, panels, popups, launcher, OSD, desktop-terminal) — never the
//! nested cards, hover washes, icons or progress tracks. The value lives on
//! [`Theme`] so every surface reads it from one place; `theme.toml` carries
//! the *requested* value and `theme_config` clamps it against the scheme's
//! measured readability floor (`min_alpha`).
//!
//! Default is opaque (`alpha = 1.0`) and blur off — a fresh install renders
//! pixel-identically to pre-T266. Transparency is an explicit user choice
//! (architect decision 2026-08-13: no pretty default alpha).

use gpui::Hsla;

use super::Theme;

/// Surface-effect tokens: effective alpha + per-scheme readability floor.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceTokens {
    /// Effective multiplier for outer surface plates (`1.0` = opaque).
    pub alpha: f32,
    /// Scheme floor: the lowest `alpha` at which text still reads (WCAG AA)
    /// on the covered surfaces. The slider maps its low end to this value,
    /// never below it (T266 spec: floor from readability measurement).
    pub min_alpha: f32,
    /// Whether compositor blur is enabled for the shell surfaces
    /// (Hyprland `layerrule` via the shipped module).
    pub blur_enabled: bool,
}

impl SurfaceTokens {
    /// Opaque, blur-off — the T266 default. `min_alpha` is scheme-specific
    /// (Dark and Light floors differ), so callers pass their measured floor.
    pub const fn opaque(min_alpha: f32) -> Self {
        Self {
            alpha: 1.0,
            min_alpha,
            blur_enabled: false,
        }
    }
}

impl Theme {
    /// Multiply a surface color's alpha by the effective surface alpha.
    /// `opacity()` multiplies, so an existing per-color alpha (e.g. the
    /// volume popup's `0.82`) is preserved, not replaced.
    pub fn surface_color(&self, color: Hsla) -> Hsla {
        color.opacity(self.surface.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_hex;

    #[test]
    fn surface_color_multiplies_existing_alpha() {
        let mut theme = Theme::default();
        theme.surface.alpha = 0.5;
        let color = parse_hex("1e1e2ecc").unwrap();
        assert!((theme.surface_color(color).a - 0.4).abs() < 1e-6);
    }

    #[test]
    fn opaque_keeps_existing_popup_alpha() {
        let theme = Theme::default();
        let original = theme.bg.primary.alpha(0.82);
        assert_eq!(theme.surface_color(original), original);
    }

    #[test]
    fn default_is_opaque_and_blurless() {
        let theme = Theme::default();
        assert_eq!(theme.surface.alpha, 1.0);
        assert!(!theme.surface.blur_enabled);
        assert_eq!(theme.surface_color(theme.bg.primary), theme.bg.primary);
    }
}

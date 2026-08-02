//! Theme surface roles for the right panel.
//!
//! Dark (Mocha-like) and Light C use different names for the same roles:
//! - **chrome** (panel/rail shell): dark `bg.tertiary`, light `bg.primary` (pageBg)
//! - **card** (raised content cards): dark `bg.primary`, light `bg.secondary` (cardBg)
//! - **well** (inset tray / meter track): dark `bg.elevated`/`border.default`, light `bg.elevated`
//!
//! Mapping tokens 1:1 without `is_light` made Light C look inverted (and dark
//! hierarchy flatter than the mockup).

use chronos_ui::Theme;
use gpui::Hsla;

/// Panel shell / rail / body fill.
pub fn chrome(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.bg.primary
    } else {
        theme.bg.tertiary
    }
}

/// Raised card surface (mpris, disks, wallpaper, permission).
pub fn card(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.bg.secondary
    } else {
        theme.bg.primary
    }
}

/// Inset control well (mpris tray, progress track).
pub fn well(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.bg.elevated
    } else {
        // Mockup tray `#15151f` — darker than primary; tertiary is the closest
        // shell dark. Elevated is lighter (surface pop), wrong for a well.
        theme.bg.tertiary
    }
}

/// Content column (main system tab surface).
pub fn content(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.bg.primary
    } else {
        theme.bg.primary
    }
}

/// Editor buffer surface (Edit mode) — T205. Dark: `bg.primary` (same as
/// panel body, so the buffer reads as a seamless sheet, not A4 white);
/// Light: `bg.secondary` (soft paper, not glare-white pageBg).
pub fn editor(theme: &Theme) -> Hsla {
    if theme.is_light {
        theme.bg.secondary
    } else {
        theme.bg.primary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_ui::Theme;

    #[test]
    fn dark_chrome_is_tertiary_card_is_primary() {
        let t = Theme::default();
        assert!(!t.is_light);
        assert_eq!(chrome(&t), t.bg.tertiary);
        assert_eq!(card(&t), t.bg.primary);
        assert_eq!(content(&t), t.bg.primary);
        // T205: editor buffer follows panel body in dark (seamless sheet).
        assert_eq!(editor(&t), t.bg.primary);
    }

    #[test]
    fn light_chrome_is_page_card_is_cardbg() {
        let t = Theme::select_scheme(Some("Light".into()));
        assert!(t.is_light);
        assert_eq!(chrome(&t), t.bg.primary);
        assert_eq!(card(&t), t.bg.secondary);
        assert_eq!(content(&t), t.bg.primary);
        // T205: light editor is soft paper (bg.secondary), not glare pageBg.
        assert_eq!(editor(&t), t.bg.secondary);
    }
}

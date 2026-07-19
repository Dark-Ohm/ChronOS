//! Встроенные схемы оформления ChronOS.

use anyhow::Result;

use super::{Base16Colors, Theme};

/// Описание одной схемы: имя, краткое описание и готовая [`Theme`].
pub struct ThemeScheme {
    pub name: &'static str,
    pub description: &'static str,
    pub theme: Theme,
}

impl ThemeScheme {
    fn new(name: &'static str, description: &'static str, theme: Theme) -> Self {
        Self {
            name,
            description,
            theme,
        }
    }
}

/// Базовая тёмная палитра ChronOS (используется как [`Theme::default`]).
///
/// Вынесена как константа Base16, чтобы `default_scheme` и `Theme::default`
/// опирались на один и тот же источник истинны, без ручного дублирования.
pub const DEFAULT_BASE16: [&str; 16] = [
    "1e1e2e", // base00 bg.primary
    "25253b", // base01 bg.secondary
    "181825", // base02 bg.tertiary
    "313244", // base03 bg.elevated
    "45475a", // base04 text.disabled / interactive.active
    "6c7086", // base05 text.muted
    "a6adc8", // base06 text.secondary
    "cdd6f4", // base07 text.primary
    "f38ba8", // base08 status.error (Catppuccin Mocha maroon/red)
    "f9e2af", // base09 status.warning (Catppuccin Mocha yellow)
    "94e2d5", // base0a status.warning/alt (Catppuccin Mocha teal)
    "a6e3a1", // base0b status.success (Catppuccin Mocha green)
    "89b4fa", // base0c status.info (Catppuccin Mocha blue)
    "007acc", // base0d accent.primary / border.focused
    "cba6f7", // base0e accent.hover / toggle_on_hover
    "f38ba8", // base0f active/emph
];

fn default_scheme() -> ThemeScheme {
    ThemeScheme::new(
        "Default",
        "Тёмная схема ChronOS (Mocha-подобная)",
        Theme::default(),
    )
}

fn light_scheme() -> ThemeScheme {
    // Инвертируем bg/text относительно дефолта: светлый фон, тёмный текст.
    let mut theme = Theme::default();
    theme.bg.primary = hex("eff1f5");
    theme.bg.secondary = hex("e6e9ef");
    theme.bg.tertiary = hex("f7f8fb");
    theme.bg.elevated = hex("dce0e8");
    theme.text.primary = hex("4c4f69");
    theme.text.secondary = hex("5c5f77");
    theme.text.muted = hex("9ca0b0");
    theme.text.disabled = hex("bcc0cc");
    theme.text.placeholder = hex("9ca0b0");
    theme.border.default = hex("ccd0da");
    theme.border.subtle = hex("e6e9ef");
    theme.interactive.default = hex("ccd0da");
    theme.interactive.hover = hex("bcc0cc");
    theme.interactive.active = hex("9ca0b0");
    ThemeScheme::new(
        "Light",
        "Светлая схема (инверсия bg/text дефолта)",
        theme,
    )
}

fn solarized_dark_scheme() -> Result<ThemeScheme> {
    // Реальная палитра Solarized Dark, маппится через Base16Colors.
    let colors = Base16Colors::from_hex(&[
        "002b36", // base00
        "073642", // base01
        "586e75", // base02
        "657b83", // base03
        "839496", // base04
        "93a1a1", // base05
        "eee8d5", // base06
        "fdf6e3", // base07
        "dc322f", // base08 error
        "cb4b16", // base09 warning
        "b58900", // base0a warning/alt
        "859900", // base0b success
        "2aa198", // base0c info
        "268bd2", // base0d accent
        "6c71c4", // base0e accent/hover
        "d33682", // base0f
    ])?;
    Ok(ThemeScheme::new(
        "Solarized Dark",
        "Solarized Dark через Base16",
        colors.to_theme(),
    ))
}

#[inline]
fn hex(s: &str) -> gpui::Hsla {
    super::parse_hex(s).expect("встроенный hex валиден")
}

/// Возвращает все встроенные схемы (дефолт, светлая, solarized dark).
pub fn builtin_schemes() -> Vec<ThemeScheme> {
    let mut out = vec![default_scheme(), light_scheme()];
    if let Ok(solarized) = solarized_dark_scheme() {
        out.push(solarized);
    }
    out
}

//! Ядро theme-API: семантические цветовые группы, глобальная [`Theme`],
//! утилита [`parse_hex`] и сетки spacing/radius.
//!
//! [`Theme`] регистрируется в gpui как глобальное состояние (см.
//! [`gpui::Global`]), поэтому доступна из любого контекста через
//! [`Theme::global`] / [`Theme::global_mut`] либо через трейт
//! [`ActiveTheme`].

use anyhow::{Context, Result};
use gpui::{App, Global, Hsla, Pixels, px, rgba};

pub mod base16;
pub mod schemes;

pub use base16::Base16Colors;
pub use schemes::{ThemeScheme, builtin_schemes};

/// Парсит hex-цвет в [`Hsla`].
///
/// Принимает строки вида `"#1e1e2e"`, `"1e1e2e"` (6 hex-цифр, alpha = 0xff)
/// или `"1e1e2eff"` (8 hex-цифр, задаёт alpha явно). Форк gpui здесь НЕ
/// предоставляет `Hsla::parse_hex`, поэтому парсер собственный: строка
/// переводится в `u32` в порядке BE-байт (0xRRGGBBAA) и прогоняется через
/// [`rgba`] -> [`Hsla::from`].
pub fn parse_hex(s: &str) -> Result<Hsla> {
    let raw = s.strip_prefix('#').unwrap_or(s);
    if raw.len() != 6 && raw.len() != 8 {
        anyhow::bail!(
            "ожидается 6 или 8 hex-цифр, получено {} в строке {:?}",
            raw.len(),
            s
        );
    }
    let value =
        u32::from_str_radix(raw, 16).with_context(|| format!("некорректный hex-цвет: {s:?}"))?;
    // rgba() ожидает 0xRRGGBBAA. Для 6 цифр alpha = 0xff.
    let packed = if raw.len() == 6 {
        (value << 8) | 0xff
    } else {
        value
    };
    Ok(Hsla::from(rgba(packed)))
}

/// Фоновые поверхности.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BgColors {
    pub primary: Hsla,
    pub secondary: Hsla,
    pub tertiary: Hsla,
    pub elevated: Hsla,
}

/// Текстовые цвета.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TextColors {
    pub primary: Hsla,
    pub secondary: Hsla,
    pub muted: Hsla,
    pub disabled: Hsla,
    pub placeholder: Hsla,
}

/// Границы/разделители.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BorderColors {
    pub default: Hsla,
    pub subtle: Hsla,
    pub focused: Hsla,
}

/// Акцентные (брендовые) цвета.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AccentColors {
    pub primary: Hsla,
    pub selection: Hsla,
    pub hover: Hsla,
}

/// Цвета статусов/ургентности (нотификации, OSD).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StatusColors {
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub info: Hsla,
}

/// Интерактивные состояния контролов.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct InteractiveColors {
    pub default: Hsla,
    pub hover: Hsla,
    pub active: Hsla,
    pub toggle_on: Hsla,
    pub toggle_on_hover: Hsla,
}

/// Относительные размеры шрифта (на базе [`FontSizes::base`]).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FontSizes {
    pub base: Pixels,
    pub xs: Pixels,
    pub sm: Pixels,
    pub md: Pixels,
    pub lg: Pixels,
    pub xl: Pixels,
}

impl FontSizes {
    pub fn new(base: f32) -> Self {
        Self {
            base: px(base),
            xs: px(base * 0.77),
            sm: px(base * 0.85),
            md: px(base * 1.08),
            lg: px(base * 1.23),
            xl: px(base * 1.38),
        }
    }
}

impl Default for FontSizes {
    fn default() -> Self {
        Self::new(13.0)
    }
}

/// Тема оформления ChronOS — глобальное состояние gpui.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Theme {
    pub bg: BgColors,
    pub text: TextColors,
    pub border: BorderColors,
    pub accent: AccentColors,
    pub status: StatusColors,
    pub interactive: InteractiveColors,
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub transparent: Hsla,
    pub font_sizes: FontSizes,
    pub font_mono: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        let text_secondary = parse_hex("a6adc8").unwrap();
        let text_muted = parse_hex("6c7086").unwrap();
        let border_default = parse_hex("313244").unwrap();
        Self {
            bg: BgColors {
                primary: parse_hex("1e1e2e").unwrap(),
                secondary: parse_hex("25253b").unwrap(),
                tertiary: parse_hex("181825").unwrap(),
                elevated: parse_hex("313244").unwrap(),
            },
            text: TextColors {
                primary: rgba(0xffffffff).into(),
                secondary: text_secondary,
                muted: text_muted,
                disabled: parse_hex("45475a").unwrap(),
                placeholder: text_muted,
            },
            border: BorderColors {
                default: border_default,
                subtle: parse_hex("45475a").unwrap(),
                focused: parse_hex("007acc").unwrap(),
            },
            accent: AccentColors {
                primary: parse_hex("007acc").unwrap(),
                selection: parse_hex("007acc").unwrap(),
                hover: parse_hex("1f9bdc").unwrap(),
            },
            status: StatusColors {
                success: parse_hex("a6e3a1").unwrap(),
                warning: parse_hex("f9e2af").unwrap(),
                error: parse_hex("f38ba8").unwrap(),
                info: parse_hex("89b4fa").unwrap(),
            },
            interactive: InteractiveColors {
                default: border_default,
                hover: parse_hex("45475a").unwrap(),
                active: parse_hex("585b70").unwrap(),
                toggle_on: parse_hex("007acc").unwrap(),
                toggle_on_hover: parse_hex("1f9bdc").unwrap(),
            },
            radius: px(6.0),
            radius_lg: px(12.0),
            transparent: rgba(0x00000000).into(),
            font_sizes: FontSizes::default(),
            font_mono: "JetBrains Mono",
        }
    }
}

impl Global for Theme {}

impl Theme {
    /// Регистрирует тему как глобальное состояние gpui.
    ///
    /// По умолчанию ставит [`Theme::default`] (тёмная схема). Если задана
    /// переменная окружения `CHRONOS_THEME` и её значение (case-insensitive)
    /// совпадает с именем одной из [`builtin_schemes`], ставится эта схема.
    /// Неверное имя → дефолт + `tracing::warn!` со списком доступных имён.
    /// Поведение без переменной не меняется (как до ввода механизма выбора).
    pub fn init(cx: &mut App) {
        cx.set_global(Self::select_scheme(std::env::var("CHRONOS_THEME").ok()));
    }

    /// Выбирает тему по опциональному имени схемы (case-insensitive).
    ///
    /// `None` или пустая строка → [`Theme::default`]. Валидное имя →
    /// соответствующая схема из [`builtin_schemes`]. Невалидное имя →
    /// [`Theme::default`] + предупреждение через `tracing::warn!`.
    pub fn select_scheme(env_value: Option<String>) -> Theme {
        let Some(raw) = env_value else {
            return Theme::default();
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Theme::default();
        }
        let wanted = trimmed.to_lowercase();
        let schemes = builtin_schemes();
        if let Some(scheme) = schemes.iter().find(|s| s.name.to_lowercase() == wanted) {
            return scheme.theme;
        }
        let available: Vec<&'static str> = schemes.iter().map(|s| s.name).collect();
        tracing::warn!(
            requested = %trimmed,
            available = ?available,
            "CHRONOS_THEME: неизвестная схема, fallback на Default"
        );
        Theme::default()
    }

    /// Возвращает заимствованную ссылку на активную тему.
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Возвращает изменяемую ссылку на активную тему.
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// Подменяет активную тему целиком.
    pub fn set(theme: Theme, cx: &mut App) {
        *cx.global_mut::<Theme>() = theme;
    }
}

/// Расширение контекстов, дающее удобный доступ к активной теме.
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

/// Сетка отступов (f32, применять через [`gpui::px`]).
pub mod spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// Радиусы скругления (f32, применять через [`gpui::px`]).
pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
    pub const PILL: f32 = 999.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base16_roundtrip() {
        let hex = [
            "#1e1e2e", "#25253b", "#181825", "#313244", "#45475a", "#a6adc8", "#cdd6f4", "#f8f8f2",
            "#f38ba8", "#f9e2af", "#89b4fa", "#a6e3a1", "#94e2d5", "#89b4fa", "#cba6f7", "#f38ba8",
        ];
        let colors = Base16Colors::from_hex(&hex).expect("16 валидных hex");
        let theme = colors.to_theme();
        assert_eq!(theme.bg.primary, parse_hex("#1e1e2e").unwrap());
    }

    #[test]
    fn default_theme_global() {
        let theme = Theme::default();
        assert_ne!(theme.status.error, theme.status.success);
        assert_eq!(theme.bg.primary, parse_hex("1e1e2e").unwrap());
    }

    #[test]
    fn parse_hex_handles_6_and_8_digits() {
        let six = parse_hex("#1e1e2e").unwrap();
        let eight = parse_hex("1e1e2eff").unwrap();
        assert_eq!(six, eight);
        assert_eq!(six.a, 1.0);
    }
}

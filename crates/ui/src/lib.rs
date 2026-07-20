//! chronos-ui — фундамент theme-API для всех UI ChronOS.
//!
//! Крейт предоставляет [`Theme`] (глобальное состояние gpui), набор
//! семантических цветовых групп, утилиту [`parse_hex`] для разбора
//! hex-цветов и конвертер [`Base16Colors`] <-> [`Theme`], а также
//! набор встроенных схем ([`builtin_schemes`]).

pub mod theme;

pub use theme::{on_fill, parse_hex};
pub use theme::{
    ActiveTheme, Base16Colors, FontSizes, Theme, ThemeScheme, builtin_schemes,
};

//! chronos-ui — фундамент theme-API для всех UI ChronOS.
//!
//! Крейт предоставляет [`Theme`] (глобальное состояние gpui), набор
//! семантических цветовых групп, утилиту [`parse_hex`] для разбора
//! hex-цветов и конвертер [`Base16Colors`] <-> [`Theme`], а также
//! набор встроенных схем ([`builtin_schemes`]).

pub mod elevation;
pub mod theme;
pub mod window_root;

pub use elevation::{
    BlurSpec, EMPTY_SHADOWS, ElevationTokens, elevation_apply_light_chrome, elevation_blur_layer,
    elevation_glow_bar, elevation_watermark,
};
pub use theme::{ActiveTheme, Base16Colors, FontSizes, Theme, ThemeScheme, builtin_schemes};
pub use theme::{on_fill, parse_hex};
pub use window_root::WindowRootExt;

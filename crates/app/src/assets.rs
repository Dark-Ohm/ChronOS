//! Embedded asset source for GPUI — bar/popup SVG icons.
//!
//! Icons are single-color line-art (Phosphor-style, viewBox 256, or sigil
//! hexagons, viewBox 32); the SVG renderer uses them as an alpha mask tinted
//! by the element's `text_color`, so `currentColor`/black both work.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

macro_rules! icons {
    ($($name:literal),+ $(,)?) => {
        fn load_icon(path: &str) -> Option<Cow<'static, [u8]>> {
            match path {
                $(concat!("icons/", $name) => Some(
                    include_bytes!(concat!("../assets/icons/", $name)).as_slice().into(),
                ),)+
                _ => None,
            }
        }
        const ICON_NAMES: &[&str] = &[$(concat!("icons/", $name)),+];
    };
}

icons!(
    "arrow-up.svg",
    "arrows-clockwise.svg",
    "battery.svg",
    "battery-charging.svg",
    "bell.svg",
    "bolt.svg",
    "chevron-down.svg",
    "folder.svg",
    "hexagon-core.svg",
    "hexagon-sigil.svg",
    "pause.svg",
    "play.svg",
    "power.svg",
    "sign-out.svg",
    "skip-back.svg",
    "skip-forward.svg",
    "speaker-high.svg",
    "speaker-low.svg",
    "speaker-mute.svg",
    "speaker-none.svg",
    "users.svg",
    "x.svg",
);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(load_icon(path))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICON_NAMES
            .iter()
            .filter(|name| name.starts_with(path))
            .map(|name| SharedString::from(*name))
            .collect())
    }
}

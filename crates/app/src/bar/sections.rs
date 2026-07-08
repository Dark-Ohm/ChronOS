// crates/app/src/bar/sections.rs
//! Bar layout sections and default geometry constants.

/// Which horizontal section a widget renders into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarSection {
    Left,
    Center,
    Right,
}

/// Bar thickness in logical pixels. Mirrors the previous hard-coded constant.
pub const BAR_HEIGHT: f32 = 32.0;

/// Bar background color (0xRRGGBB). Mirrors the previous hard-coded constant.
pub const BAR_COLOR: u32 = 0x1e1e2e;

//! Cava visualizer state.

/// One frame of bar heights from cava ascii raw output (0..=100).
///
/// Length is typically [`super::BAR_COUNT`] (24); may be empty when cava is
/// unavailable or has not produced a frame yet.
pub type CavaState = Vec<u8>;

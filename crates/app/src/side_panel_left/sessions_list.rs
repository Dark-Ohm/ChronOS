pub struct SessionItem {
    pub id: String,
    pub title: String,
    pub active: bool,
}

/// Total sidebar width when collapsed: icon strip + padding.
/// Target ~36: icon buttons ~28 + ~4px padding each side.
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 36.;

/// Total sidebar width when expanded: session list + header chrome.
pub const SIDEBAR_EXPANDED_WIDTH: f32 = 200.;

/// Width of the resize-handle grab strip (must match `HANDLE_WIDTH` in panel.rs).
pub const SIDEBAR_HANDLE_WIDTH: f32 = 10.;

/// Minimum window width = collapsed sidebar + resize handle.
pub const SIDEBAR_MIN_WIDTH: f32 = SIDEBAR_COLLAPSED_WIDTH + SIDEBAR_HANDLE_WIDTH;

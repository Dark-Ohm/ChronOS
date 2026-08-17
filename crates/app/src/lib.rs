//! Chronos app library - public API for examples and tests.

pub mod agent_follow;
pub mod bar_settings;
// Lib-side twin of bin `mod calendar_popup` (main.rs).
pub(crate) mod calendar_popup;
// Desktop-terminal widget module — needed in the lib crate too: the System
// tab (lib) references `desktop_terminal::add_widget` (T259) and the widget
// tests run against the lib (`cargo test --lib`). The bin (main.rs) has its
// own `mod desktop_terminal;` copy; this is the lib-side twin.
pub(crate) mod desktop_terminal;
pub mod edit_mode;
// Lib-side twin of bin `mod frame` (main.rs): the side panels and the bar
// settings tab (lib) read `frame::wrap_inset` / `FrameStyle` and write the
// style key; this mirror lets `cargo test --lib` compile the same path
// `crate::frame` that main.rs resolves. In the release binary only the
// bin's copy is linked (the bin does not link `chronos_app`), so the
// frame's static state lives once. Same twin pattern as `dock` / `start_menu`.
pub(crate) mod frame;
pub mod games_config;
pub mod icon_resolution;
pub mod launcher;
pub mod monitor;
pub mod motion;
// Lib-side twin of bin `mod dock` (main.rs): the launcher (lib) opens a
// right-click pin/unpin PopupMenu that reads/writes the dock config and uses
// the shared click-catcher. Same twin pattern as `desktop_terminal` /
// `project_switcher` (see comments above) — declared in both the lib and the
// bin so `crate::dock` / `crate::popup_click_catcher` resolve in either crate.
pub(crate) mod dock;
// Lib-side twin of bin `mod popup_click_catcher` (main.rs).
pub(crate) mod popup_click_catcher;
pub mod notifications;
// Lib-side twin of bin `mod project_switcher` (main.rs) — the left
// workspace tabs (lib) read `ProjectsConfig` and call domain actions from
// `project_switcher`; this mirror lets `cargo test --lib` compile the
// same path `crate::project_switcher` that main.rs resolves. Same twin
// pattern as `desktop_terminal` (see comment above).
pub(crate) mod project_switcher;
pub mod scene;
mod surface_effects;
pub mod side_panel_left;
pub mod side_panel_right;
// Lib-side twin of bin `mod start_menu` (main.rs): the bar's dock widget
// (lib) toggles it via `crate::start_menu`. Same twin pattern as
// `dock` / `calendar_popup` so the path resolves in either crate.
pub(crate) mod start_menu;
pub mod state;
mod gaming_mode;
pub mod theme_config;
pub mod wallpaper_ctl;
pub mod workspace_mode;
mod power;
mod power_controls;
// Lib-side twin of bin `mod updates_list` (main.rs): the Updates tab
// (side_panel_right/tab/updates.rs, lib) renders its list through the shared
// geometry/cell helpers here. Same twin pattern as `desktop_terminal` /
// `project_switcher` so `crate::updates_list` resolves in either crate.
pub(crate) mod updates_list;

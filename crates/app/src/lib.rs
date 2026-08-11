//! Chronos app library - public API for examples and tests.

pub mod agent_follow;
pub mod bar_settings;
// Desktop-terminal widget module — needed in the lib crate too: the System
// tab (lib) references `desktop_terminal::add_widget` (T259) and the widget
// tests run against the lib (`cargo test --lib`). The bin (main.rs) has its
// own `mod desktop_terminal;` copy; this is the lib-side twin.
pub(crate) mod desktop_terminal;
pub mod edit_mode;
pub mod games_config;
pub mod icon_resolution;
pub mod launcher;
pub mod monitor;
pub mod motion;
pub mod notifications;
pub mod scene;
pub mod side_panel_right;
pub mod state;
pub mod system_popup;
pub mod theme_config;
pub mod wallpaper_ctl;
pub mod workspace_mode;

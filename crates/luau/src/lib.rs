pub mod manager;
pub mod sandbox;
pub mod dsl;
pub mod capabilities;
pub mod api;
pub mod bar;

pub use manager::PluginManager;
pub use dsl::{Element, TextStyle, Alignment};
pub use capabilities::Manifest;
pub use bar::{BarWidget, BarWidgetRegistry, BarSection, BAR_HEIGHT, BAR_COLOR};
pub use mlua;

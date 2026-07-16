//! Compositor data types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositorBackend {
    #[default]
    Hyprland,
    Niri,
}

impl CompositorBackend {
    /// Human-readable name for logging.
    pub fn name(&self) -> &'static str {
        match self {
            CompositorBackend::Hyprland => "Hyprland",
            CompositorBackend::Niri => "Niri",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub active: bool,
    /// Monitor the workspace is currently on (Hyprland `monitorID`).
    /// `None` in some cases (e.g. special workspaces). Typed `i128` to match
    /// the `hyprland` crate's `MonitorId = i128` exactly — no truncation.
    pub monitor_id: Option<i128>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveWindow {
    pub title: String,
    pub class: String,
    /// Unique window address (Hyprland `Address`). `title`/`class` are not
    /// unique across windows; this is the stable identifier for focus/close
    /// operations. Derived via `Address`'s `Display` impl.
    pub address: String,
}

// NOTE: `Monitor` intentionally does NOT derive `Eq` — `scale: f32` is not
// `Eq`. `Service::Data` only requires `Clone`, so dropping `Eq` is safe.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Monitor {
    pub name: String,
    pub active_workspace: i32,
    /// Monitor id (Hyprland `id`). Typed `i128` to match the `hyprland`
    /// crate's `MonitorId = i128` exactly — no truncation.
    pub id: i128,
    /// Absolute position on the virtual desktop (pre-scale pixels). Used as
    /// the global origin for standalone desktop-widget plugins.
    pub x: i32,
    pub y: i32,
    /// Display scale factor (e.g. 1.0, 1.5, 2.0).
    pub scale: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompositorState {
    pub backend: CompositorBackend,
    pub workspaces: Vec<Workspace>,
    pub active_window: Option<ActiveWindow>,
    pub monitors: Vec<Monitor>,
    pub keyboard_layout: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositorCommand {
    /// Focus a specific workspace by ID.
    FocusWorkspace(i32),
    /// Focus next workspace.
    NextWorkspace,
    /// Focus previous workspace.
    PrevWorkspace,
    /// Move active window to a workspace.
    MoveToWorkspace(i32),
}

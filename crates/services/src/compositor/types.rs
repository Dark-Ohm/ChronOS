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
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveWindow {
    pub title: String,
    pub class: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monitor {
    pub name: String,
    pub active_workspace: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

//! Dock: pinned launcher panel — now a bar widget (left cluster).
//!
//! The standalone dock window was removed in #8. Dock icons are rendered
//! as a `BarWidget` in `bar/widgets/dock.rs`. This module provides:
//! - `config` — persistent pinned list (`~/.config/chronos/dock.toml`)
//! - `context_menu` — right-click "Unpin" popup
//! - `signal` — `DockConfigSignal` for cache invalidation

pub mod config;
pub mod context_menu;
pub mod signal;

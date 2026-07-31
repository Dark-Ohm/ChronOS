//! Shared target for the Preview tab — which file the user wants to see.
//!
//! `FilesTab::open_entry` bumps this global on a regular file click.
//! `PreviewTab` observes it via `cx.observe_global` and reacts. No
//! autotoggle: clicking a file in Files does not switch the active tab —
//! the user opens Preview themselves. The pair `(path, generation)` lets
//! observers drop stale `background_spawn` results when a second click
//! arrives before the first finishes loading.

use std::path::PathBuf;

use gpui::Global;

/// Single source of truth for "what should Preview show right now".
///
/// `path` is `None` until the user clicks a regular file in Files; the
/// `PreviewTab` initial state then reads the current value once (the
/// observer only fires on subsequent changes).
///
/// `generation` is bumped on every distinct file selection. Stale
/// background reads compare against it and discard themselves if it has
/// advanced.
#[derive(Debug, Clone, Default)]
pub struct PreviewTarget {
    pub path: Option<PathBuf>,
    pub generation: u64,
}

impl PreviewTarget {
    pub fn file(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            generation: 1,
        }
    }
}

impl Global for PreviewTarget {}

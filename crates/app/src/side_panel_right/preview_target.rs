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

/// What the caller wants to do with the file — view it (rendered/scrolled
/// read) or edit it (raw source in an editable buffer). Only meaningful
/// for markdown-like files (T194c) — `PreviewTab` forces `View` for any
/// kind that isn't editable, regardless of what was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewIntent {
    #[default]
    View,
    Edit,
}

/// Single source of truth for "what should Preview show right now".
///
/// `path` is `None` until the user clicks a regular file in Files; the
/// `PreviewTab` initial state then reads the current value once (the
/// observer only fires on subsequent changes).
///
/// `generation` is bumped on every distinct file selection **or** intent
/// change for the same file. Stale background reads compare against it
/// and discard themselves if it has advanced.
#[derive(Debug, Clone, Default)]
pub struct PreviewTarget {
    pub path: Option<PathBuf>,
    pub generation: u64,
    pub intent: PreviewIntent,
}

impl PreviewTarget {
    pub fn file(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            generation: 1,
            intent: PreviewIntent::View,
        }
    }
}

impl Global for PreviewTarget {}

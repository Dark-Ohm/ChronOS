//! File entry DTO — ported from Chronos-FM (`chronos-fm-models/src/file_entry.rs`).

use serde::Serialize;

/// A filesystem entry as produced by the listing layer and consumed by the UI.
///
/// Field types are intentionally primitive so this type stays free of any
/// toolkit dependency.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct FileEntryDto {
    /// File or directory name (final path component).
    pub name: String,
    /// Full path to the entry.
    pub path: String,
    /// Entry kind as a string (`"file"`, `"dir"`, or `"symlink"`).
    pub kind: String,
    /// Size in bytes.
    pub size: u64,
    /// Last modification time as a Unix timestamp in seconds.
    pub modified: u64,
}

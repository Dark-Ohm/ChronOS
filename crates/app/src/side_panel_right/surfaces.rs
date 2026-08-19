//! Backward-compatible re-export shim for the shared surface-role mapper.
//!
//! T311 D2b lifted the role helpers (T239 chrome / T205 editor) to
//! `crate::side_panel_common::surfaces` so the left rail can read the same
//! chrome role. This module is kept as a thin re-export so the existing
//! callers in the right panel and friends do not have to change in the
//! same patch. New code should prefer the common path.
pub use crate::side_panel_common::surfaces::*;

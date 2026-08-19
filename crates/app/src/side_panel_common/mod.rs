//! Cross-panel surfaces — currently the role mapper from `Theme` to
//! `chrome / card / well / content / editor`. Owned by both side panels,
//! historically kept under `side_panel_right::surfaces` and lifted here
//! in T311 D2b so the left rail can read the same chrome role.
//!
//! Future shared chrome (per-side rounding decisions, top-bar offset
//! dialogue with the wrap frame, …) belongs alongside `surfaces`.

pub mod surfaces;

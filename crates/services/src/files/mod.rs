//! Filesystem listing for the shell Files tab.
//!
//! Ported from Chronos-FM (`chronos-fm-services/src/fs/listing.rs`,
//! `chronos-fm-models` `FileEntryDto`, explorer `entries` sort). Read-only;
//! mutations (`ops.rs`) are intentionally out of scope (spec §4.1).

mod entry;
mod listing;
mod sort;

pub use entry::FileEntryDto;
pub use listing::{DIR_LISTING_LIMIT, ListParams, ListResult, list_dir_sync};
pub use sort::{SortKey, get_extension, sort_entries};

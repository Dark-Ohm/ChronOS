use chronos_services::threads::ThreadRecord;

/// A thread in the sidebar list, backed by the SQLite store (T150).
/// `active` tracks whether this thread is currently open in the chat column.
pub struct ThreadListItem {
    pub record: ThreadRecord,
    pub active: bool,
}

impl ThreadListItem {
    /// Display title: `title_override` wins over auto-generated `title`.
    pub fn display_title(&self) -> &str {
        self.record
            .title_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.record.title)
    }

    /// Short display title for the sidebar row (~30 chars, no newlines).
    pub fn short_title(&self) -> String {
        let title = self.display_title();
        let first_line = title.lines().next().unwrap_or("");
        if first_line.chars().count() > 30 {
            format!("{}…", first_line.chars().take(29).collect::<String>())
        } else {
            first_line.to_string()
        }
    }

    /// Whether this thread has a cached transcript in the store.
    pub fn has_cache(&self) -> bool {
        self.record.transcript_json.is_some()
    }
}

/// Generate an auto-title from the first user message: first line, ~60 chars.
pub fn auto_title_from_text(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let trimmed = first_line.trim();
    if trimmed.chars().count() > 60 {
        format!("{}…", trimmed.chars().take(59).collect::<String>())
    } else {
        trimmed.to_string()
    }
}

/// Format an RFC3339 timestamp for display (e.g. "2:30 PM" or "Yesterday").
pub fn format_timestamp(ts: &str) -> String {
    if ts.is_empty() {
        return String::new();
    }
    // Parse RFC3339 and format as HH:MM.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let now = chrono::Local::now();
        let local = dt.with_timezone(&chrono::Local);
        let diff = now.signed_duration_since(local);
        if diff.num_days() == 0 {
            local.format("%-I:%M %p").to_string()
        } else if diff.num_days() == 1 {
            "Yesterday".to_string()
        } else if diff.num_days() < 7 {
            local.format("%a %-I:%M %p").to_string()
        } else {
            local.format("%m/%d %-I:%M %p").to_string()
        }
    } else {
        // Fallback: show raw string truncated.
        ts.chars().take(16).collect()
    }
}

/// Total sidebar width when collapsed: icon strip + padding.
/// Target ~36: icon buttons ~28 + ~4px padding each side.
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 36.;

/// Total sidebar width when expanded: session list + header chrome.
pub const SIDEBAR_EXPANDED_WIDTH: f32 = 200.;

/// Width of the resize-handle grab strip (must match `HANDLE_WIDTH` in panel.rs).
pub const SIDEBAR_HANDLE_WIDTH: f32 = 10.;

/// Minimum window width = collapsed sidebar + resize handle.
pub const SIDEBAR_MIN_WIDTH: f32 = SIDEBAR_COLLAPSED_WIDTH + SIDEBAR_HANDLE_WIDTH;

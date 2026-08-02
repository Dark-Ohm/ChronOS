//! Agent Follow mode — live activity strip on the right panel (T195).
//!
//! The left panel's thread header has a Follow toggle. When Follow is ON
//! and the agent performs a tool call, this global receives the tool info.
//! The right panel observes it and shows a lightweight activity strip with
//! the last tool name + status. If the tool result or args mention a file
//! path, the right panel opens it in the Editor via `PreviewTarget`.

use gpui::Global;

/// Tool call preview — standalone (not imported from the bin-only
/// `side_panel_left::chat_view`) so the lib tree can use it.
#[derive(Debug, Clone, Default)]
pub struct ToolCallPreview {
    pub id: String,
    pub name: String,
    pub status: String,
    pub args: Option<String>,
    pub result: Option<String>,
}

/// Shared global: agent follow state + last activity.
///
/// Set by the left panel (Follow toggle + streaming handler on ToolCall
/// events). Read by the right panel to render the activity strip and
/// auto-open files.
#[derive(Debug, Clone, Default)]
pub struct AgentFollowState {
    /// Whether the Follow toggle is active.
    pub enabled: bool,
    /// The most recent tool call from the agent (streaming channel).
    /// `None` before any tool call in the current session.
    pub last_tool: Option<ToolCallPreview>,
}

impl Global for AgentFollowState {}

impl AgentFollowState {
    /// Update `last_tool` from a streaming event. Idempotent — callers
    /// (the streaming handler in `composer.rs` and `select_session` in
    /// `mod.rs`) push each ToolCall as it arrives; the last one wins.
    pub fn push_tool(&mut self, tool: ToolCallPreview) {
        self.last_tool = Some(tool);
    }

    /// Try to extract a file path from tool call info. Heuristic:
    /// - `edit_file` / `write_file` / `read_file` tools: args IS the path
    /// - Other tools: scan args/result for an absolute path or ~/
    /// Returns the extracted path if found, `None` otherwise.
    pub fn extract_file_path(tool: &ToolCallPreview) -> Option<String> {
        // File-oriented tools: args is the path.
        if matches!(
            tool.name.as_str(),
            "edit_file" | "write_file" | "read_file" | "file" | "open_file"
        ) {
            if let Some(ref args) = tool.args {
                let trimmed = args.trim();
                if !trimmed.is_empty() && (trimmed.starts_with('/') || trimmed.starts_with("~/")) {
                    return Some(trimmed.to_string());
                }
            }
        }

        // Scan args and result for any absolute path or ~/ path
        for source in [tool.args.as_deref(), tool.result.as_deref()] {
            if let Some(text) = source {
                for word in text.split_whitespace() {
                    let w = word.trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == '`');
                    if w.starts_with('/') || w.starts_with("~/") {
                        return Some(w.to_string());
                    }
                }
            }
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, args: Option<&str>, result: Option<&str>) -> ToolCallPreview {
        ToolCallPreview {
            id: "t1".into(),
            name: name.into(),
            status: "done".into(),
            args: args.map(str::to_string),
            result: result.map(str::to_string),
        }
    }

    #[test]
    fn edit_file_args_is_path() {
        let t = tool("edit_file", Some("/tmp/foo.rs"), None);
        assert_eq!(
            AgentFollowState::extract_file_path(&t).as_deref(),
            Some("/tmp/foo.rs")
        );
    }

    #[test]
    fn tilde_path_accepted() {
        let t = tool("read_file", Some("~/config.toml"), None);
        assert_eq!(
            AgentFollowState::extract_file_path(&t).as_deref(),
            Some("~/config.toml")
        );
    }

    #[test]
    fn relative_path_rejected_for_file_tools() {
        let t = tool("edit_file", Some("src/main.rs"), None);
        assert_eq!(AgentFollowState::extract_file_path(&t), None);
    }

    #[test]
    fn scan_result_for_other_tools() {
        let t = tool("bash", Some("cat"), Some("wrote /home/neo/a.txt ok"));
        assert_eq!(
            AgentFollowState::extract_file_path(&t).as_deref(),
            Some("/home/neo/a.txt")
        );
    }

    #[test]
    fn push_tool_last_wins() {
        let mut s = AgentFollowState::default();
        s.push_tool(tool("a", None, None));
        s.push_tool(tool("b", None, None));
        assert_eq!(s.last_tool.as_ref().map(|t| t.name.as_str()), Some("b"));
    }
}

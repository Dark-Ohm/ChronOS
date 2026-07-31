//! Tab identity, full catalog, and mode-driven rail composition.
//!
//! `ALL` is the complete catalog of every tab the shell knows about — icon
//! and label coverage tests iterate it. Mode composition uses `for_mode` /
//! `resolve_for_mode`; never replace `ALL` with a mode subset.

use crate::workspace_mode::WorkspaceMode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_has_ten_tabs_in_fixed_order() {
        assert_eq!(PanelTab::ALL.len(), 10);
        assert_eq!(PanelTab::ALL[0], PanelTab::System);
        assert_eq!(PanelTab::ALL[1], PanelTab::Files);
        assert_eq!(PanelTab::ALL[9], PanelTab::HyprlandBinds);
    }

    #[test]
    fn every_tab_has_a_non_empty_label() {
        for tab in PanelTab::ALL {
            assert!(!tab.label().is_empty(), "{tab:?} has an empty label");
        }
    }

    #[test]
    fn every_tab_has_a_distinct_icon_path() {
        let paths: Vec<&str> = PanelTab::ALL.iter().map(|t| t.icon_path()).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), paths.len(), "two tabs share an icon path");
    }

    #[test]
    fn system_is_the_default_active_tab() {
        assert_eq!(PanelTab::default(), PanelTab::System);
    }

    #[test]
    fn developer_and_gamer_sets_are_nonempty_with_system_first() {
        let dev = PanelTab::for_mode(WorkspaceMode::Developer);
        let gamer = PanelTab::for_mode(WorkspaceMode::Gamer);
        assert!(!dev.is_empty());
        assert!(!gamer.is_empty());
        assert_eq!(dev[0], PanelTab::System);
        assert_eq!(gamer[0], PanelTab::System);
    }

    #[test]
    fn shared_tabs_keep_relative_order_across_modes() {
        let dev = PanelTab::for_mode(WorkspaceMode::Developer);
        let gamer = PanelTab::for_mode(WorkspaceMode::Gamer);
        let shared: Vec<PanelTab> = dev
            .iter()
            .copied()
            .filter(|t| gamer.contains(t))
            .collect();
        let shared_in_gamer: Vec<PanelTab> = gamer
            .iter()
            .copied()
            .filter(|t| dev.contains(t))
            .collect();
        assert_eq!(
            shared, shared_in_gamer,
            "relative order of shared rail tabs must be stable across modes"
        );
        assert!(shared.contains(&PanelTab::System));
    }

    #[test]
    fn scene_override_wins_over_mode_default() {
        let tabs = PanelTab::resolve_for_mode(
            WorkspaceMode::Developer,
            Some(&["system".into(), "terminal".into()]),
        );
        assert_eq!(tabs, vec![PanelTab::System, PanelTab::Terminal]);
    }

    #[test]
    fn unknown_override_names_are_skipped() {
        let tabs = PanelTab::resolve_for_mode(
            WorkspaceMode::Developer,
            Some(&["system".into(), "nope".into(), "files".into()]),
        );
        assert_eq!(tabs, vec![PanelTab::System, PanelTab::Files]);
    }

    #[test]
    fn all_unknown_override_falls_back_to_mode() {
        let tabs = PanelTab::resolve_for_mode(
            WorkspaceMode::Gamer,
            Some(&["garbage".into(), "also-bad".into()]),
        );
        assert_eq!(tabs, PanelTab::for_mode(WorkspaceMode::Gamer));
    }

    #[test]
    fn parse_id_is_case_insensitive() {
        assert_eq!(PanelTab::parse_id("System"), Some(PanelTab::System));
        assert_eq!(PanelTab::parse_id("ACPSETTINGS"), Some(PanelTab::AcpSettings));
        assert_eq!(PanelTab::parse_id("acp_settings"), Some(PanelTab::AcpSettings));
        assert_eq!(PanelTab::parse_id("???"), None);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PanelTab {
    #[default]
    System,
    Files,
    Editor,
    Terminal,
    AcpSettings,
    McpSettings,
    LspSettings,
    ApiProviders,
    EditorSettings,
    HyprlandBinds,
}

impl PanelTab {
    /// Full catalog — every tab that exists. Coverage tests iterate this.
    pub const ALL: [PanelTab; 10] = [
        PanelTab::System,
        PanelTab::Files,
        PanelTab::Editor,
        PanelTab::Terminal,
        PanelTab::AcpSettings,
        PanelTab::McpSettings,
        PanelTab::LspSettings,
        PanelTab::ApiProviders,
        PanelTab::EditorSettings,
        PanelTab::HyprlandBinds,
    ];

    /// Stable id for scene overrides (`scenes.toml` `rail_tabs`).
    pub fn id(self) -> &'static str {
        match self {
            PanelTab::System => "system",
            PanelTab::Files => "files",
            PanelTab::Editor => "editor",
            PanelTab::Terminal => "terminal",
            PanelTab::AcpSettings => "acp_settings",
            PanelTab::McpSettings => "mcp_settings",
            PanelTab::LspSettings => "lsp_settings",
            PanelTab::ApiProviders => "api_providers",
            PanelTab::EditorSettings => "editor_settings",
            PanelTab::HyprlandBinds => "hyprland_binds",
        }
    }

    /// Parse a scene/config id into a tab. Unknown → `None`.
    pub fn parse_id(s: &str) -> Option<Self> {
        let key = s.trim().to_ascii_lowercase().replace('-', "_");
        match key.as_str() {
            "system" => Some(PanelTab::System),
            "files" => Some(PanelTab::Files),
            "editor" => Some(PanelTab::Editor),
            "terminal" => Some(PanelTab::Terminal),
            "acp_settings" | "acpsettings" => Some(PanelTab::AcpSettings),
            "mcp_settings" | "mcpsettings" => Some(PanelTab::McpSettings),
            "lsp_settings" | "lspsettings" => Some(PanelTab::LspSettings),
            "api_providers" | "apiproviders" => Some(PanelTab::ApiProviders),
            "editor_settings" | "editorsettings" => Some(PanelTab::EditorSettings),
            "hyprland_binds" | "hyprlandbinds" => Some(PanelTab::HyprlandBinds),
            _ => None,
        }
    }

    /// Default rail composition for a workspace mode.
    ///
    /// Developer — full workbench (all tabs). Gamer — System + settings group;
    /// work tools (Files/Editor/Terminal) leave so the deck is not a second IDE.
    /// Shared tabs keep the same relative order in both sets (§5).
    pub fn for_mode(mode: WorkspaceMode) -> Vec<PanelTab> {
        match mode {
            WorkspaceMode::Developer => PanelTab::ALL.to_vec(),
            WorkspaceMode::Gamer => vec![
                PanelTab::System,
                PanelTab::AcpSettings,
                PanelTab::McpSettings,
                PanelTab::LspSettings,
                PanelTab::ApiProviders,
                PanelTab::EditorSettings,
                PanelTab::HyprlandBinds,
            ],
        }
    }

    /// Resolve the rail set: scene override (if any) beats mode default.
    /// Unknown names are skipped with a warn; if nothing remains, fall back
    /// to the mode default so the rail never goes empty.
    pub fn resolve_for_mode(
        mode: WorkspaceMode,
        scene_override: Option<&[String]>,
    ) -> Vec<PanelTab> {
        let Some(names) = scene_override else {
            return Self::for_mode(mode);
        };
        let mut tabs = Vec::with_capacity(names.len());
        for name in names {
            match Self::parse_id(name) {
                Some(tab) => {
                    if !tabs.contains(&tab) {
                        tabs.push(tab);
                    }
                }
                None => {
                    tracing::warn!(
                        tab = %name,
                        "rail: unknown tab id in scene override, ignoring"
                    );
                }
            }
        }
        if tabs.is_empty() {
            Self::for_mode(mode)
        } else {
            tabs
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PanelTab::System => "System",
            PanelTab::Files => "Files",
            PanelTab::Editor => "Editor",
            PanelTab::Terminal => "Terminal",
            PanelTab::AcpSettings => "ACP settings",
            PanelTab::McpSettings => "MCP settings",
            PanelTab::LspSettings => "LSP settings",
            PanelTab::ApiProviders => "API providers",
            PanelTab::EditorSettings => "Editor settings",
            PanelTab::HyprlandBinds => "Hyprland binds",
        }
    }

    pub fn icon_path(self) -> &'static str {
        match self {
            PanelTab::System => "icons/rail-system.svg",
            PanelTab::Files => "icons/folder.svg",
            PanelTab::Editor => "icons/rail-editor.svg",
            PanelTab::Terminal => "icons/rail-terminal.svg",
            PanelTab::AcpSettings => "icons/rail-acp.svg",
            PanelTab::McpSettings => "icons/rail-mcp.svg",
            PanelTab::LspSettings => "icons/rail-lsp.svg",
            PanelTab::ApiProviders => "icons/rail-api.svg",
            PanelTab::EditorSettings => "icons/rail-editor-settings.svg",
            PanelTab::HyprlandBinds => "icons/rail-binds.svg",
        }
    }
}

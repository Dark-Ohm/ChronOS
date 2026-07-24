//! Tab identity and fixed ordering for the IDE tab-container.
//!
//! Order is fixed by the design brief (`design.md` §"Shell-IDE правая
//! панель (таб-контейнер)") — do not reorder `ALL` without updating the
//! brief and the accepted mockup.

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
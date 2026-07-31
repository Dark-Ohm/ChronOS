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
    fn all_has_fourteen_tabs_in_fixed_order() {
        // §4.1 spec: Developer sees System + 7 work tools (Files/Editor/
        // Terminal/Preview/Inspector/Build/SourceControl) + 6 settings
        // (AcpSettings/McpSettings/LspSettings/ApiProviders/EditorSettings/
        // HyprlandBinds). Settings group sits at the tail, unchanged.
        assert_eq!(PanelTab::ALL.len(), 14);
        assert_eq!(PanelTab::ALL[0], PanelTab::System);
        assert_eq!(PanelTab::ALL[1], PanelTab::Files);
        assert_eq!(PanelTab::ALL[2], PanelTab::Editor);
        assert_eq!(PanelTab::ALL[3], PanelTab::Terminal);
        assert_eq!(PanelTab::ALL[4], PanelTab::Preview);
        assert_eq!(PanelTab::ALL[5], PanelTab::Inspector);
        assert_eq!(PanelTab::ALL[6], PanelTab::Build);
        assert_eq!(PanelTab::ALL[7], PanelTab::SourceControl);
        assert_eq!(PanelTab::ALL[8], PanelTab::AcpSettings);
        assert_eq!(PanelTab::ALL[9], PanelTab::McpSettings);
        assert_eq!(PanelTab::ALL[10], PanelTab::LspSettings);
        assert_eq!(PanelTab::ALL[11], PanelTab::ApiProviders);
        assert_eq!(PanelTab::ALL[12], PanelTab::EditorSettings);
        assert_eq!(PanelTab::ALL[13], PanelTab::HyprlandBinds);
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

    // --- T169: new four work-tool tabs round-trip parse_id ↔ id (§4.1) ---

    #[test]
    fn parse_id_round_trip_for_new_work_tools() {
        for tab in [
            PanelTab::Preview,
            PanelTab::Inspector,
            PanelTab::Build,
            PanelTab::SourceControl,
        ] {
            assert_eq!(
                PanelTab::parse_id(tab.id()),
                Some(tab),
                "{tab:?} round-trip via id() failed"
            );
        }
    }

    #[test]
    fn parse_id_accepts_underscore_and_camel_for_new_tabs() {
        // Spec gives ids with underscore; parse_id normalises hyphens to
        // underscores and lowercases, so caller-style variants must work
        // too — same lenience rule as the original ten.
        assert_eq!(PanelTab::parse_id("preview"), Some(PanelTab::Preview));
        assert_eq!(PanelTab::parse_id("PREVIEW"), Some(PanelTab::Preview));
        assert_eq!(PanelTab::parse_id("Preview"), Some(PanelTab::Preview));
        assert_eq!(PanelTab::parse_id("inspector"), Some(PanelTab::Inspector));
        assert_eq!(PanelTab::parse_id("INSPECTOR"), Some(PanelTab::Inspector));
        assert_eq!(PanelTab::parse_id("build"), Some(PanelTab::Build));
        assert_eq!(PanelTab::parse_id("BUILD"), Some(PanelTab::Build));
        assert_eq!(
            PanelTab::parse_id("source_control"),
            Some(PanelTab::SourceControl)
        );
        assert_eq!(
            PanelTab::parse_id("source-control"),
            Some(PanelTab::SourceControl)
        );
        assert_eq!(
            PanelTab::parse_id("sourcecontrol"),
            Some(PanelTab::SourceControl)
        );
        assert_eq!(
            PanelTab::parse_id("SOURCECONTROL"),
            Some(PanelTab::SourceControl)
        );
    }

    #[test]
    fn parse_id_rejects_unknown_names_including_new_ones() {
        for bogus in ["previewz", "inspectorrr", "buildit", "git", "scm", ""] {
            assert_eq!(
                PanelTab::parse_id(bogus),
                None,
                "{bogus:?} unexpectedly parsed as a known tab"
            );
        }
    }

    // --- T169: composition rules per §4.1 + §5 ---

    #[test]
    fn developer_rail_is_full_catalog_of_fourteen() {
        let dev = PanelTab::for_mode(WorkspaceMode::Developer);
        assert_eq!(dev.len(), 14);
        assert_eq!(dev, PanelTab::ALL.to_vec());
        // The four new work tools must be present.
        assert!(dev.contains(&PanelTab::Preview));
        assert!(dev.contains(&PanelTab::Inspector));
        assert!(dev.contains(&PanelTab::Build));
        assert!(dev.contains(&PanelTab::SourceControl));
    }

    #[test]
    fn gamer_rail_stays_seven_tabs_without_new_work_tools() {
        // §4.1 line 149: "Gamer mode replaces the work-tool group with its
        // own tools and keeps the settings group intact". Work tools
        // (Files/Editor/Terminal + the four new ones) are absent; settings
        // group is intact; System stays first.
        let gamer = PanelTab::for_mode(WorkspaceMode::Gamer);
        assert_eq!(gamer.len(), 7);
        assert_eq!(gamer[0], PanelTab::System);
        assert_eq!(gamer[1], PanelTab::AcpSettings);
        assert_eq!(gamer[2], PanelTab::McpSettings);
        assert_eq!(gamer[3], PanelTab::LspSettings);
        assert_eq!(gamer[4], PanelTab::ApiProviders);
        assert_eq!(gamer[5], PanelTab::EditorSettings);
        assert_eq!(gamer[6], PanelTab::HyprlandBinds);
        for absent in [
            PanelTab::Files,
            PanelTab::Editor,
            PanelTab::Terminal,
            PanelTab::Preview,
            PanelTab::Inspector,
            PanelTab::Build,
            PanelTab::SourceControl,
        ] {
            assert!(
                !gamer.contains(&absent),
                "Gamer must not include work-tool tab {absent:?}"
            );
        }
    }

    #[test]
    fn developer_settings_group_matches_gamer_settings_group_order() {
        // §5: shared rail tabs keep relative order across modes. Settings
        // is shared between Developer and Gamer; the four new work tools
        // are not in Gamer, so they must not affect the shared order.
        let dev = PanelTab::for_mode(WorkspaceMode::Developer);
        let gamer = PanelTab::for_mode(WorkspaceMode::Gamer);
        let dev_settings: Vec<PanelTab> = dev
            .iter()
            .copied()
            .filter(|t| matches!(
                t,
                PanelTab::AcpSettings
                    | PanelTab::McpSettings
                    | PanelTab::LspSettings
                    | PanelTab::ApiProviders
                    | PanelTab::EditorSettings
                    | PanelTab::HyprlandBinds
            ))
            .collect();
        assert_eq!(dev_settings, gamer[1..].to_vec());
    }

    #[test]
    fn every_new_tab_has_a_distinct_icon_path() {
        // Defensive copy of `every_tab_has_a_distinct_icon_path` narrowed
        // to the four new entries, so a missing icon file surfaces as a
        // descriptive failure rather than a 14-way collision report.
        let paths = [
            PanelTab::Preview.icon_path(),
            PanelTab::Inspector.icon_path(),
            PanelTab::Build.icon_path(),
            PanelTab::SourceControl.icon_path(),
        ];
        let mut sorted: Vec<&str> = paths.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), paths.len(), "new tabs share an icon path");
        // And the paths all live under icons/.
        for p in paths {
            assert!(
                p.starts_with("icons/") && p.ends_with(".svg"),
                "icon path does not look like icons/rail-*.svg: {p}"
            );
        }
    }

    // --- T171: per-tab preferred content width ---

    #[test]
    fn every_preferred_width_in_valid_range() {
        use crate::side_panel_right::{MAX_WIDTH, RAIL_ONLY_WIDTH};
        for tab in PanelTab::ALL {
            let w = tab.preferred_content_width();
            assert!(
                w >= RAIL_ONLY_WIDTH && w <= MAX_WIDTH,
                "{tab:?} preferred width {w} outside [{RAIL_ONLY_WIDTH}, {MAX_WIDTH}]"
            );
        }
    }

    #[test]
    fn system_preferred_width_is_400() {
        assert_eq!(PanelTab::System.preferred_content_width(), 400.);
    }

    #[test]
    fn editor_and_terminal_preferred_width_is_default() {
        use crate::side_panel_right::DEFAULT_CONTENT_WIDTH;
        assert_eq!(PanelTab::Editor.preferred_content_width(), DEFAULT_CONTENT_WIDTH);
        assert_eq!(PanelTab::Terminal.preferred_content_width(), DEFAULT_CONTENT_WIDTH);
    }

    #[test]
    fn files_and_source_control_preferred_width_is_440() {
        assert_eq!(PanelTab::Files.preferred_content_width(), 440.);
        assert_eq!(PanelTab::SourceControl.preferred_content_width(), 440.);
    }

    #[test]
    fn empty_state_tabs_preferred_width_is_320() {
        for tab in [
            PanelTab::Preview,
            PanelTab::Inspector,
            PanelTab::AcpSettings,
            PanelTab::McpSettings,
            PanelTab::LspSettings,
            PanelTab::ApiProviders,
            PanelTab::EditorSettings,
            PanelTab::HyprlandBinds,
        ] {
            assert_eq!(
                tab.preferred_content_width(),
                320.,
                "{tab:?} empty-state preferred width must be 320"
            );
        }
    }

    #[test]
    fn build_preferred_width_is_640() {
        assert_eq!(PanelTab::Build.preferred_content_width(), 640.);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PanelTab {
    #[default]
    System,
    // --- Work tools (§4.1) ---
    Files,
    Editor,
    Terminal,
    Preview,
    Inspector,
    Build,
    SourceControl,
    // --- Settings group ---
    AcpSettings,
    McpSettings,
    LspSettings,
    ApiProviders,
    EditorSettings,
    HyprlandBinds,
}

impl PanelTab {
    /// Full catalog — every tab that exists. Coverage tests iterate this.
    pub const ALL: [PanelTab; 14] = [
        PanelTab::System,
        PanelTab::Files,
        PanelTab::Editor,
        PanelTab::Terminal,
        PanelTab::Preview,
        PanelTab::Inspector,
        PanelTab::Build,
        PanelTab::SourceControl,
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
            PanelTab::Preview => "preview",
            PanelTab::Inspector => "inspector",
            PanelTab::Build => "build",
            PanelTab::SourceControl => "source_control",
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
            "preview" => Some(PanelTab::Preview),
            "inspector" => Some(PanelTab::Inspector),
            "build" => Some(PanelTab::Build),
            "source_control" | "sourcecontrol" => Some(PanelTab::SourceControl),
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
    /// Developer — full workbench (all 14 tabs per §4.1). Gamer — System +
    /// settings group; work tools (the original three plus
    /// Preview/Inspector/Build/SourceControl) leave so the deck is not a
    /// second IDE. Shared tabs keep the same relative order in both sets
    /// (§5).
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
            PanelTab::Preview => "Preview",
            PanelTab::Inspector => "Inspector",
            PanelTab::Build => "Build",
            PanelTab::SourceControl => "Source control",
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
            PanelTab::Preview => "icons/rail-preview.svg",
            PanelTab::Inspector => "icons/rail-inspector.svg",
            PanelTab::Build => "icons/rail-build.svg",
            PanelTab::SourceControl => "icons/rail-source-control.svg",
            PanelTab::AcpSettings => "icons/rail-acp.svg",
            PanelTab::McpSettings => "icons/rail-mcp.svg",
            PanelTab::LspSettings => "icons/rail-lsp.svg",
            PanelTab::ApiProviders => "icons/rail-api.svg",
            PanelTab::EditorSettings => "icons/rail-editor-settings.svg",
            PanelTab::HyprlandBinds => "icons/rail-binds.svg",
        }
    }

    /// Preferred content width for this tab (px). Used by the right panel
    /// to resize when switching tabs. `DEFAULT_CONTENT_WIDTH` is the fallback
    /// for any tab that does not override this.
    pub fn preferred_content_width(self) -> f32 {
        match self {
            PanelTab::System => 400.,
            PanelTab::Editor | PanelTab::Terminal => super::DEFAULT_CONTENT_WIDTH,
            PanelTab::Files | PanelTab::SourceControl => 440.,
            // Build/Logs: cargo diagnostics need ~82 mono cols (640/7.8).
            // 560≈72 cols truncates long ` --> path:line` lines; 640 keeps them.
            PanelTab::Build => 640.,
            // Empty-state tabs: icon + label + one-line description.
            PanelTab::Preview
            | PanelTab::Inspector
            | PanelTab::AcpSettings
            | PanelTab::McpSettings
            | PanelTab::LspSettings
            | PanelTab::ApiProviders
            | PanelTab::EditorSettings
            | PanelTab::HyprlandBinds => 320.,
        }
    }
}

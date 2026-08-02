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
    fn all_has_seventeen_tabs_in_fixed_order() {
        // §4.1 spec: Developer sees System + 7 work tools (Files/Editor/
        // Terminal/Preview/Inspector/Build/SourceControl) + 6 settings
        // (AcpSettings/McpSettings/LspSettings/ApiProviders/EditorSettings/
        // HyprlandBinds). §4.2 adds three Gamer at-rest hub tools
        // (Library/Scenes/Captures) to the full catalog, slotted between the
        // work tools and the settings group. `for_mode(Developer)` excludes
        // them — they live in `ALL` for icon/label/coverage, not the dev rail.
        assert_eq!(PanelTab::ALL.len(), 17);
        assert_eq!(PanelTab::ALL[0], PanelTab::System);
        assert_eq!(PanelTab::ALL[1], PanelTab::Files);
        assert_eq!(PanelTab::ALL[2], PanelTab::Editor);
        assert_eq!(PanelTab::ALL[3], PanelTab::Terminal);
        assert_eq!(PanelTab::ALL[4], PanelTab::Preview);
        assert_eq!(PanelTab::ALL[5], PanelTab::Inspector);
        assert_eq!(PanelTab::ALL[6], PanelTab::Build);
        assert_eq!(PanelTab::ALL[7], PanelTab::SourceControl);
        assert_eq!(PanelTab::ALL[8], PanelTab::Library);
        assert_eq!(PanelTab::ALL[9], PanelTab::Scenes);
        assert_eq!(PanelTab::ALL[10], PanelTab::Captures);
        assert_eq!(PanelTab::ALL[11], PanelTab::AcpSettings);
        assert_eq!(PanelTab::ALL[12], PanelTab::McpSettings);
        assert_eq!(PanelTab::ALL[13], PanelTab::LspSettings);
        assert_eq!(PanelTab::ALL[14], PanelTab::ApiProviders);
        assert_eq!(PanelTab::ALL[15], PanelTab::EditorSettings);
        assert_eq!(PanelTab::ALL[16], PanelTab::HyprlandBinds);
    }

    #[test]
    fn product_cut_labels_are_renamed() {
        // T192: Preview surfaces as "Editor" (real edit lands T194),
        // AcpSettings as "ACP agents", EditorSettings as "System settings"
        // (docs/PRODUCT.md §2/§4). HyprlandBinds label is unchanged.
        assert_eq!(PanelTab::Preview.label(), "Editor");
        assert_eq!(PanelTab::AcpSettings.label(), "ACP agents");
        assert_eq!(PanelTab::EditorSettings.label(), "System settings");
        assert_eq!(PanelTab::HyprlandBinds.label(), "Hyprland binds");
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
    fn acp_settings_precedes_system_settings_in_both_modes() {
        // T192 product cut: HyprlandBinds intentionally sits in a different
        // relative slot per mode (ahead of settings in Developer, trailing
        // in Gamer — see `for_mode` doc comment), so the old "all shared
        // tabs keep identical relative order" invariant no longer holds.
        // What *does* still hold: System is first, and within the settings
        // pair, ACP agents precedes System settings, in both modes.
        for mode in [WorkspaceMode::Developer, WorkspaceMode::Gamer] {
            let tabs = PanelTab::for_mode(mode);
            assert_eq!(tabs[0], PanelTab::System, "{mode:?}: System must be first");
            let acp_idx = tabs.iter().position(|t| *t == PanelTab::AcpSettings);
            let sys_settings_idx = tabs.iter().position(|t| *t == PanelTab::EditorSettings);
            match (acp_idx, sys_settings_idx) {
                (Some(a), Some(s)) => assert!(
                    a < s,
                    "{mode:?}: ACP agents must precede System settings, got acp={a} sys={s}"
                ),
                _ => panic!("{mode:?}: both AcpSettings and EditorSettings must be present"),
            }
        }
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

    // --- T186: three Gamer hub tabs round-trip parse_id ↔ id (§4.2) ---

    #[test]
    fn parse_id_round_trip_for_gamer_hub_tools() {
        for tab in [PanelTab::Library, PanelTab::Scenes, PanelTab::Captures] {
            assert_eq!(
                PanelTab::parse_id(tab.id()),
                Some(tab),
                "{tab:?} round-trip via id() failed"
            );
        }
    }

    #[test]
    fn parse_id_accepts_case_and_hyphen_variants_for_gamer_hub_tools() {
        // Single-word ids: case-insensitive + hyphen→underscore lenience,
        // same rule as the original ten and the four work tools.
        assert_eq!(PanelTab::parse_id("library"), Some(PanelTab::Library));
        assert_eq!(PanelTab::parse_id("LIBRARY"), Some(PanelTab::Library));
        assert_eq!(PanelTab::parse_id("Library"), Some(PanelTab::Library));
        assert_eq!(PanelTab::parse_id("scenes"), Some(PanelTab::Scenes));
        assert_eq!(PanelTab::parse_id("SCENES"), Some(PanelTab::Scenes));
        assert_eq!(PanelTab::parse_id("Scenes"), Some(PanelTab::Scenes));
        assert_eq!(PanelTab::parse_id("captures"), Some(PanelTab::Captures));
        assert_eq!(PanelTab::parse_id("CAPTURES"), Some(PanelTab::Captures));
        assert_eq!(PanelTab::parse_id("Captures"), Some(PanelTab::Captures));
    }

    #[test]
    fn parse_id_rejects_unknown_names_including_new_ones() {
        for bogus in [
            "previewz",
            "inspectorrr",
            "buildit",
            "git",
            "scm",
            "",
            "lib",
            "scene",
            "capture",
            "games",
            "librarys",
        ] {
            assert_eq!(
                PanelTab::parse_id(bogus),
                None,
                "{bogus:?} unexpectedly parsed as a known tab"
            );
        }
    }

    // --- T169: composition rules per §4.1 + §5 ---

    #[test]
    fn developer_rail_is_six_product_tabs() {
        // T192 product cut (docs/PRODUCT.md §2/§4): Developer default rail
        // ships System, Files, Editor (Preview relabeled, real edit is
        // T194), Hyprland binds, ACP agents, System settings. Everything
        // else (empty IDE tabs, LSP/MCP/API-providers settings, Gamer hub
        // tools) stays in `ALL` for parse/scene-override/icon coverage but
        // is not in the default rail.
        let dev = PanelTab::for_mode(WorkspaceMode::Developer);
        assert_eq!(
            dev,
            vec![
                PanelTab::System,
                PanelTab::Files,
                PanelTab::Preview,
                PanelTab::HyprlandBinds,
                PanelTab::AcpSettings,
                PanelTab::EditorSettings,
            ]
        );
        for absent in [
            PanelTab::Editor,
            PanelTab::Terminal,
            PanelTab::Inspector,
            PanelTab::Build,
            PanelTab::SourceControl,
            PanelTab::McpSettings,
            PanelTab::LspSettings,
            PanelTab::ApiProviders,
            PanelTab::Library,
            PanelTab::Scenes,
            PanelTab::Captures,
        ] {
            assert!(
                !dev.contains(&absent),
                "Developer rail must not include cut tab {absent:?}"
            );
        }
    }

    #[test]
    fn gamer_rail_is_six_product_tabs() {
        // T192 product cut: Gamer default rail ships System, Library,
        // Captures (honest empty — no capture backend), then the same
        // three settings tabs as Developer. Scenes is a full product kill
        // (docs/PRODUCT.md §4 — "сцены нахуй не нужны") and does not
        // appear in any default rail, even though `scene.rs`/seed code may
        // stay dormant.
        let gamer = PanelTab::for_mode(WorkspaceMode::Gamer);
        assert_eq!(
            gamer,
            vec![
                PanelTab::System,
                PanelTab::Library,
                PanelTab::Captures,
                PanelTab::AcpSettings,
                PanelTab::EditorSettings,
                PanelTab::HyprlandBinds,
            ]
        );
        for absent in [
            PanelTab::Files,
            PanelTab::Editor,
            PanelTab::Terminal,
            PanelTab::Preview,
            PanelTab::Inspector,
            PanelTab::Build,
            PanelTab::SourceControl,
            PanelTab::Scenes,
            PanelTab::McpSettings,
            PanelTab::LspSettings,
            PanelTab::ApiProviders,
        ] {
            assert!(
                !gamer.contains(&absent),
                "Gamer rail must not include cut tab {absent:?}"
            );
        }
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

    #[test]
    fn gamer_hub_tabs_have_distinct_icon_paths() {
        // T186: the three Gamer at-rest hub tabs each need their own icon
        // file, registered in `assets.rs` (T169 lesson — missing registration
        // renders an empty slot).
        let paths = [
            PanelTab::Library.icon_path(),
            PanelTab::Scenes.icon_path(),
            PanelTab::Captures.icon_path(),
        ];
        let mut sorted: Vec<&str> = paths.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), paths.len(), "gamer hub tabs share an icon path");
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
            PanelTab::Inspector,
            PanelTab::Captures,
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
    fn preview_preferred_width_is_560() {
        // T179 §3: image and markdown need more than 320; 560 aligns with
        // Editor/Terminal and gives a comfortable markdown line length.
        assert_eq!(PanelTab::Preview.preferred_content_width(), 560.);
    }

    #[test]
    fn build_preferred_width_is_640() {
        assert_eq!(PanelTab::Build.preferred_content_width(), 640.);
    }

    #[test]
    fn library_preferred_width_is_480() {
        // §4.2 at-rest hub: game grid + artwork needs more than the 320
        // empty-state width (T188 lands the real content).
        assert_eq!(PanelTab::Library.preferred_content_width(), 480.);
    }

    #[test]
    fn scenes_preferred_width_is_400() {
        // §4.2 at-rest hub: per-game scene cards (T189).
        assert_eq!(PanelTab::Scenes.preferred_content_width(), 400.);
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
    // --- Gamer at-rest hub tools (§4.2) ---
    Library,
    Scenes,
    Captures,
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
    pub const ALL: [PanelTab; 17] = [
        PanelTab::System,
        PanelTab::Files,
        PanelTab::Editor,
        PanelTab::Terminal,
        PanelTab::Preview,
        PanelTab::Inspector,
        PanelTab::Build,
        PanelTab::SourceControl,
        PanelTab::Library,
        PanelTab::Scenes,
        PanelTab::Captures,
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
            PanelTab::Library => "library",
            PanelTab::Scenes => "scenes",
            PanelTab::Captures => "captures",
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
            "library" => Some(PanelTab::Library),
            "scenes" => Some(PanelTab::Scenes),
            "captures" => Some(PanelTab::Captures),
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

    /// Default rail composition for a workspace mode (product cut, T192 —
    /// `docs/PRODUCT.md` §2/§4). The full 17-tab catalog (`ALL`) still
    /// exists for `parse_id`/scene overrides/icon coverage, but the default
    /// rail only shows what the product actually ships: no empty IDE
    /// tabs (Terminal/Inspector/Build/SourceControl/empty Editor), no
    /// LSP/MCP/API-providers settings, no Scenes (killed per §4 — "сцены
    /// нахуй не нужны").
    ///
    /// Developer: System, Files, Editor (`PanelTab::Preview` relabeled —
    /// real view+edit lands T194), Hyprland binds, ACP agents, System
    /// settings (former Editor settings).
    ///
    /// Gamer: System, Library, Captures (honest empty — no capture backend
    /// yet), then the same three settings tabs as Developer.
    ///
    /// Hyprland binds sits ahead of the settings pair in Developer (it's a
    /// primary work surface once binds RO ships, T193) but trails them in
    /// Gamer (it's a secondary settings-group entry there) — this is a
    /// deliberate per-mode placement, not a shared-order invariant; only
    /// System-first and Acp-before-System-settings hold across both modes
    /// (see `tests::acp_settings_precedes_system_settings_in_both_modes`).
    pub fn for_mode(mode: WorkspaceMode) -> Vec<PanelTab> {
        match mode {
            WorkspaceMode::Developer => vec![
                PanelTab::System,
                PanelTab::Files,
                PanelTab::Preview,
                PanelTab::HyprlandBinds,
                PanelTab::AcpSettings,
                PanelTab::EditorSettings,
            ],
            WorkspaceMode::Gamer => vec![
                PanelTab::System,
                PanelTab::Library,
                PanelTab::Captures,
                PanelTab::AcpSettings,
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
            // T192 product cut: Preview is the default rail's "Editor" tab
            // until T194 lands the real view+edit path and the enum can be
            // renamed/merged with the (now rail-hidden) empty `Editor`
            // variant. Two variants sharing the label "Editor" is a known,
            // temporary duplication — not a bug.
            PanelTab::Preview => "Editor",
            PanelTab::Inspector => "Inspector",
            PanelTab::Build => "Build",
            PanelTab::SourceControl => "Source control",
            PanelTab::Library => "Library",
            PanelTab::Scenes => "Scenes",
            PanelTab::Captures => "Captures",
            PanelTab::AcpSettings => "ACP agents",
            PanelTab::McpSettings => "MCP settings",
            PanelTab::LspSettings => "LSP settings",
            PanelTab::ApiProviders => "API providers",
            PanelTab::EditorSettings => "System settings",
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
            PanelTab::Library => "icons/rail-library.svg",
            PanelTab::Scenes => "icons/rail-scenes.svg",
            PanelTab::Captures => "icons/rail-captures.svg",
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
            // Preview: 560 matches Editor/Terminal and gives a comfortable
            // markdown line length (~80 chars at ~7 px each). Image pixels
            // render at native size via `object_fit: Contain`, so width is
            // only a ceiling, not a target. Lower than Build because no
            // mono diagnostics demand wider.
            PanelTab::Preview => super::DEFAULT_CONTENT_WIDTH,
            // Gamer at-rest hub (§4.2): Library hosts a game grid + artwork
            // (T188), Scenes lists per-game scene cards (T189). Widths are
            // set now so the panel does not jump when real content lands.
            PanelTab::Library => 480.,
            PanelTab::Scenes => 400.,
            // Empty-state tabs: icon + label + one-line description.
            // Captures is honestly unavailable this slice (no capture
            // backend — slice 6), so it stays an empty-state 320.
            PanelTab::Inspector
            | PanelTab::Captures
            | PanelTab::AcpSettings
            | PanelTab::McpSettings
            | PanelTab::LspSettings
            | PanelTab::ApiProviders
            | PanelTab::EditorSettings
            | PanelTab::HyprlandBinds => 320.,
        }
    }
}

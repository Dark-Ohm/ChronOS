pub const PING_PAYLOAD: &str = "ping";
pub const TOGGLE_LAUNCHER_PAYLOAD: &str = "toggle-launcher";
pub const TOGGLE_SIDE_PANEL_LEFT_PAYLOAD: &str = "toggle-side-panel-left";
pub const WALLPAPER_NEXT_PAYLOAD: &str = "wallpaper-next";
pub const WALLPAPER_GALLERY_PAYLOAD: &str = "wallpaper-gallery";
pub const WALLPAPER_REFRESH_PAYLOAD: &str = "wallpaper-refresh";
const WALLPAPER_SET_PREFIX: &str = "wallpaper-set:";

pub fn encode_ping() -> String {
    PING_PAYLOAD.to_string()
}

pub fn is_ping(payload: &str) -> bool {
    payload.trim() == PING_PAYLOAD
}

// `encode_toggle_launcher` is part of the public IPC protocol surface: external
// keybind daemons (Hyprland/niri) call it to trigger the launcher. It is not
// used inside this crate, only by out-of-tree clients.
#[allow(dead_code)]
pub fn encode_toggle_launcher() -> String {
    TOGGLE_LAUNCHER_PAYLOAD.to_string()
}

pub fn is_toggle_launcher(payload: &str) -> bool {
    payload.trim() == TOGGLE_LAUNCHER_PAYLOAD
}

// Same contract as `encode_toggle_launcher` above — external keybind
// daemons trigger the left agent panel (pinned-only, no hover-peek).
#[allow(dead_code)]
pub fn encode_toggle_side_panel_left() -> String {
    TOGGLE_SIDE_PANEL_LEFT_PAYLOAD.to_string()
}

pub fn is_toggle_side_panel_left(payload: &str) -> bool {
    payload.trim() == TOGGLE_SIDE_PANEL_LEFT_PAYLOAD
}

pub const TOGGLE_SIDE_PANEL_RIGHT_PAYLOAD: &str = "toggle-side-panel-right";
pub const TOGGLE_THEME_PAYLOAD: &str = "toggle-theme";
pub const TOGGLE_EDIT_MODE_PAYLOAD: &str = "toggle-edit-mode";
pub const TOGGLE_WORKSPACE_MODE_PAYLOAD: &str = "toggle-workspace-mode";
const SET_WORKSPACE_MODE_PREFIX: &str = "set-workspace-mode:";
/// T230 task B: switch the right panel to a tab by id (e.g. `terminal`).
/// `PanelTab::parse_id` normalizes case/hyphens, so `select-tab:system` and
/// `select-tab:System` are the same command.
pub const SELECT_TAB_PREFIX: &str = "select-tab:";
/// T226 tooling: point the right panel's Preview (Editor) tab at a file
/// path — same `PreviewTarget` global a Files click sets.
pub const PREVIEW_TARGET_PREFIX: &str = "preview-target:";
/// T226 tooling: open the left agent panel docked and focus the composer.
pub const EXPAND_LEFT_PAYLOAD: &str = "expand-left";
/// T241 tooling: send text directly to the left panel composer and dispatch
/// to the agent — bypasses Wayland seat focus for automated captures/tests.
pub const COMPOSE_AND_SEND_PREFIX: &str = "compose-and-send:";

// Same contract as `encode_toggle_launcher` above — external keybind
// daemons trigger the right agent panel (pinned-only, no hover-peek).
#[allow(dead_code)]
pub fn encode_toggle_side_panel_right() -> String {
    TOGGLE_SIDE_PANEL_RIGHT_PAYLOAD.to_string()
}

pub fn is_toggle_side_panel_right(payload: &str) -> bool {
    payload.trim() == TOGGLE_SIDE_PANEL_RIGHT_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_toggle_theme() -> String {
    TOGGLE_THEME_PAYLOAD.to_string()
}

pub fn is_toggle_theme(payload: &str) -> bool {
    payload.trim() == TOGGLE_THEME_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_toggle_edit_mode() -> String {
    TOGGLE_EDIT_MODE_PAYLOAD.to_string()
}

pub fn is_toggle_edit_mode(payload: &str) -> bool {
    payload.trim() == TOGGLE_EDIT_MODE_PAYLOAD
}

// Тот же контракт, что и `encode_toggle_launcher` выше — внешние keybind-демоны
// переключают режим рабочего пространства.
#[allow(dead_code)]
pub fn encode_toggle_workspace_mode() -> String {
    TOGGLE_WORKSPACE_MODE_PAYLOAD.to_string()
}

pub fn is_toggle_workspace_mode(payload: &str) -> bool {
    payload.trim() == TOGGLE_WORKSPACE_MODE_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_set_workspace_mode(mode: crate::workspace_mode::WorkspaceMode) -> String {
    format!("{SET_WORKSPACE_MODE_PREFIX}{}", mode.label().to_ascii_lowercase())
}

/// Разбирает `set-workspace-mode:<mode>`. Неизвестный режим → `None`
/// (команда игнорируется, режим не меняется).
pub fn classify_set_workspace_mode(
    payload: &str,
) -> Option<crate::workspace_mode::WorkspaceMode> {
    let rest = payload.trim().strip_prefix(SET_WORKSPACE_MODE_PREFIX)?;
    crate::workspace_mode::WorkspaceMode::parse(rest)
}

/// Encode a `select-tab:<id>` payload for an out-of-tree client.
#[allow(dead_code)]
pub fn encode_select_tab(tab: crate::side_panel_right::tabs::PanelTab) -> String {
    format!("{SELECT_TAB_PREFIX}{}", tab.id())
}

/// Parse `select-tab:<alias>` into a panel tab. Unknown alias → `None`
/// (command ignored — matches how an unknown widget name is dropped).
pub fn classify_select_tab(
    payload: &str,
) -> Option<crate::side_panel_right::tabs::PanelTab> {
    let rest = payload.trim().strip_prefix(SELECT_TAB_PREFIX)?;
    crate::side_panel_right::tabs::PanelTab::parse_id(rest)
}

/// Encode a `preview-target:<abs-path>` payload for an out-of-tree client.
#[allow(dead_code)]
pub fn encode_preview_target(path: &std::path::Path) -> String {
    format!("{PREVIEW_TARGET_PREFIX}{}", path.display())
}

/// Parse `preview-target:<abs-path>` into an absolute path.
pub fn parse_preview_target(payload: &str) -> Option<std::path::PathBuf> {
    let trimmed = payload.trim();
    let rest = trimmed.strip_prefix(PREVIEW_TARGET_PREFIX)?;
    if rest.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(rest);
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

pub fn is_expand_left(payload: &str) -> bool {
    payload.trim() == EXPAND_LEFT_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_expand_left() -> String {
    EXPAND_LEFT_PAYLOAD.to_string()
}

/// Encode a `compose-and-send:<text>` payload for an out-of-tree client.
#[allow(dead_code)]
pub fn encode_compose_and_send(text: &str) -> String {
    format!("{COMPOSE_AND_SEND_PREFIX}{text}")
}

/// Parse `compose-and-send:<text>` — everything after the first `:` is the
/// message text. Empty text → `None` (don't send empty prompts).
pub fn parse_compose_and_send(payload: &str) -> Option<String> {
    let trimmed = payload.trim();
    let rest = trimmed.strip_prefix(COMPOSE_AND_SEND_PREFIX)?;
    let text = rest.trim();
    if text.is_empty() { None } else { Some(text.to_string()) }
}

pub fn is_wallpaper_next(payload: &str) -> bool {
    payload.trim() == WALLPAPER_NEXT_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_wallpaper_next() -> String {
    WALLPAPER_NEXT_PAYLOAD.to_string()
}

#[allow(dead_code)]
pub fn encode_wallpaper_set(path: &std::path::Path) -> String {
    format!("{}{}", WALLPAPER_SET_PREFIX, path.display())
}

/// Parse a `wallpaper-set:/abs/path` payload into an absolute path.
pub fn parse_wallpaper_set(payload: &str) -> Option<std::path::PathBuf> {
    let trimmed = payload.trim();
    let rest = trimmed.strip_prefix(WALLPAPER_SET_PREFIX)?;
    if rest.is_empty() {
        return None;
    }
    let path = std::path::PathBuf::from(rest);
    if path.is_absolute() { Some(path) } else { None }
}

/// Parsed wallpaper IPC command.
pub enum WallpaperIpcCmd {
    Next,
    Set(std::path::PathBuf),
    Gallery,
    Refresh,
}

/// Workspace-mode IPC command sent across the tokio channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceModeIpcCmd {
    Toggle,
    Set(crate::workspace_mode::WorkspaceMode),
}

/// Classify a raw IPC payload into a wallpaper command, if applicable.
pub fn classify_wallpaper(payload: &str) -> Option<WallpaperIpcCmd> {
    let trimmed = payload.trim();
    if is_wallpaper_next(trimmed) {
        Some(WallpaperIpcCmd::Next)
    } else if is_wallpaper_gallery(trimmed) {
        Some(WallpaperIpcCmd::Gallery)
    } else if is_wallpaper_refresh(trimmed) {
        Some(WallpaperIpcCmd::Refresh)
    } else {
        parse_wallpaper_set(trimmed).map(WallpaperIpcCmd::Set)
    }
}

pub fn is_wallpaper_gallery(payload: &str) -> bool {
    payload.trim() == WALLPAPER_GALLERY_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_wallpaper_gallery() -> String {
    WALLPAPER_GALLERY_PAYLOAD.to_string()
}

pub fn is_wallpaper_refresh(payload: &str) -> bool {
    payload.trim() == WALLPAPER_REFRESH_PAYLOAD
}

#[allow(dead_code)]
pub fn encode_wallpaper_refresh() -> String {
    WALLPAPER_REFRESH_PAYLOAD.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_recognizes_ping() {
        let payload = encode_ping();
        assert!(is_ping(&payload));
    }

    #[test]
    fn rejects_non_ping_payload() {
        assert!(!is_ping("not-a-ping"));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert!(is_ping("  ping\n"));
    }

    #[test]
    fn encodes_and_recognizes_toggle_launcher() {
        let payload = encode_toggle_launcher();
        assert!(is_toggle_launcher(&payload));
    }

    #[test]
    fn rejects_non_toggle_launcher_payload() {
        assert!(!is_toggle_launcher("ping"));
    }

    #[test]
    fn encodes_and_recognizes_toggle_side_panel_left() {
        let payload = encode_toggle_side_panel_left();
        assert!(is_toggle_side_panel_left(&payload));
    }

    #[test]
    fn rejects_non_toggle_side_panel_left_payload() {
        assert!(!is_toggle_side_panel_left("toggle-launcher"));
    }

    #[test]
    fn encodes_and_recognizes_toggle_side_panel_right() {
        let payload = encode_toggle_side_panel_right();
        assert!(is_toggle_side_panel_right(&payload));
    }

    #[test]
    fn rejects_non_toggle_side_panel_right_payload() {
        assert!(!is_toggle_side_panel_right("toggle-launcher"));
    }

    #[test]
    fn encodes_and_recognizes_wallpaper_next() {
        let payload = encode_wallpaper_next();
        assert!(is_wallpaper_next(&payload));
    }

    #[test]
    fn rejects_non_wallpaper_next_payload() {
        assert!(!is_wallpaper_next("ping"));
        assert!(!is_wallpaper_next("wallpaper-set:/tmp/a.png"));
    }

    #[test]
    fn encodes_wallpaper_set_payload() {
        let path = std::path::Path::new("/home/user/Pictures/Wallpapers/a.jpg");
        let payload = encode_wallpaper_set(path);
        assert_eq!(
            payload,
            "wallpaper-set:/home/user/Pictures/Wallpapers/a.jpg"
        );
    }

    #[test]
    fn parse_wallpaper_set_extracts_path() {
        let parsed = parse_wallpaper_set("wallpaper-set:/home/user/pics/test.png");
        assert_eq!(
            parsed,
            Some(std::path::PathBuf::from("/home/user/pics/test.png"))
        );
    }

    #[test]
    fn parse_wallpaper_set_rejects_empty_path() {
        assert!(parse_wallpaper_set("wallpaper-set:").is_none());
    }

    #[test]
    fn parse_wallpaper_set_rejects_relative_path() {
        assert!(parse_wallpaper_set("wallpaper-set:relative/path.png").is_none());
    }

    #[test]
    fn parse_wallpaper_set_trims_whitespace() {
        let parsed = parse_wallpaper_set("  wallpaper-set:/tmp/wall.png\n");
        assert_eq!(parsed, Some(std::path::PathBuf::from("/tmp/wall.png")));
    }

    #[test]
    fn encodes_and_recognizes_wallpaper_gallery() {
        let payload = encode_wallpaper_gallery();
        assert!(is_wallpaper_gallery(&payload));
    }

    #[test]
    fn rejects_non_wallpaper_gallery_payload() {
        assert!(!is_wallpaper_gallery("ping"));
        assert!(!is_wallpaper_gallery("wallpaper-next"));
    }

    #[test]
    fn encodes_and_recognizes_wallpaper_refresh() {
        let payload = encode_wallpaper_refresh();
        assert!(is_wallpaper_refresh(&payload));
    }

    #[test]
    fn rejects_non_wallpaper_refresh_payload() {
        assert!(!is_wallpaper_refresh("ping"));
        assert!(!is_wallpaper_refresh("wallpaper-gallery"));
    }

    #[test]
    fn classify_wallpaper_gallery() {
        assert!(matches!(
            classify_wallpaper("wallpaper-gallery"),
            Some(WallpaperIpcCmd::Gallery)
        ));
    }

    #[test]
    fn classify_wallpaper_refresh() {
        assert!(matches!(
            classify_wallpaper("wallpaper-refresh"),
            Some(WallpaperIpcCmd::Refresh)
        ));
    }

    #[test]
    fn encodes_and_recognizes_toggle_theme() {
        let payload = encode_toggle_theme();
        assert!(is_toggle_theme(&payload));
        assert!(!is_toggle_theme("toggle-launcher"));
    }

    #[test]
    fn encodes_and_recognizes_toggle_edit_mode() {
        let payload = encode_toggle_edit_mode();
        assert!(is_toggle_edit_mode(&payload));
        assert!(!is_toggle_edit_mode("toggle-theme"));
    }

    #[test]
    fn encodes_and_recognizes_toggle_workspace_mode() {
        let payload = encode_toggle_workspace_mode();
        assert!(is_toggle_workspace_mode(&payload));
        assert!(!is_toggle_workspace_mode("toggle-edit-mode"));
    }

    #[test]
    fn classifies_set_workspace_mode() {
        use crate::workspace_mode::WorkspaceMode;
        assert_eq!(
            classify_set_workspace_mode(&encode_set_workspace_mode(WorkspaceMode::Gamer)),
            Some(WorkspaceMode::Gamer)
        );
        assert_eq!(
            classify_set_workspace_mode("set-workspace-mode:developer"),
            Some(WorkspaceMode::Developer)
        );
        assert_eq!(classify_set_workspace_mode("set-workspace-mode:nonsense"), None);
        assert_eq!(classify_set_workspace_mode("set-workspace-mode:"), None);
        assert_eq!(classify_set_workspace_mode("toggle-workspace-mode"), None);
    }

    #[test]
    fn classifies_select_tab() {
        use crate::side_panel_right::tabs::PanelTab;
        assert_eq!(
            classify_select_tab(&encode_select_tab(PanelTab::Terminal)),
            Some(PanelTab::Terminal)
        );
        assert_eq!(classify_select_tab("select-tab:system"), Some(PanelTab::System));
        assert_eq!(
            classify_select_tab("select-tab:Terminal"),
            Some(PanelTab::Terminal)
        );
        assert_eq!(classify_select_tab("select-tab:preview"), Some(PanelTab::Preview));
    }

    #[test]
    fn select_tab_unknown_alias_is_none() {
        assert_eq!(classify_select_tab("select-tab:nonsense"), None);
        assert_eq!(classify_select_tab("select-tab:"), None);
        assert_eq!(classify_select_tab("toggle-side-panel-right"), None);
    }

    #[test]
    fn parse_preview_target_extracts_absolute_path() {
        let parsed = parse_preview_target("preview-target:/home/user/foo.md");
        assert_eq!(parsed, Some(std::path::PathBuf::from("/home/user/foo.md")));
    }

    #[test]
    fn parse_preview_target_rejects_empty_and_relative() {
        assert!(parse_preview_target("preview-target:").is_none());
        assert!(parse_preview_target("preview-target:relative/foo.md").is_none());
        assert!(parse_preview_target("  preview-target:/x\n").is_some());
    }

    #[test]
    fn encodes_and_recognizes_expand_left() {
        let payload = encode_expand_left();
        assert!(is_expand_left(&payload));
        assert!(!is_expand_left("toggle-side-panel-left"));
    }

    #[test]
    fn parse_compose_and_send_extracts_text() {
        let parsed = parse_compose_and_send("compose-and-send:hello world");
        assert_eq!(parsed, Some("hello world".to_string()));
    }

    #[test]
    fn parse_compose_and_send_rejects_empty() {
        assert!(parse_compose_and_send("compose-and-send:").is_none());
        assert!(parse_compose_and_send("compose-and-send:   ").is_none());
        assert!(parse_compose_and_send("compose-and-send").is_none());
    }

    #[test]
    fn parse_compose_and_send_trims_whitespace() {
        let parsed = parse_compose_and_send("  compose-and-send:  hello \n");
        assert_eq!(parsed, Some("hello".to_string()));
    }

    #[test]
    fn encodes_compose_and_send_roundtrips() {
        let payload = encode_compose_and_send("test 123");
        assert_eq!(parse_compose_and_send(&payload), Some("test 123".to_string()));
    }
}

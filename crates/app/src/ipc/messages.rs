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
}

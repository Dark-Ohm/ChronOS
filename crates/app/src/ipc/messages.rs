pub const PING_PAYLOAD: &str = "ping";
pub const TOGGLE_LAUNCHER_PAYLOAD: &str = "toggle-launcher";
pub const TOGGLE_SIDE_PANEL_LEFT_PAYLOAD: &str = "toggle-side-panel-left";
pub const WALLPAPER_NEXT_PAYLOAD: &str = "wallpaper-next";
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
}

/// Classify a raw IPC payload into a wallpaper command, if applicable.
pub fn classify_wallpaper(payload: &str) -> Option<WallpaperIpcCmd> {
    if is_wallpaper_next(payload) {
        Some(WallpaperIpcCmd::Next)
    } else {
        parse_wallpaper_set(payload).map(WallpaperIpcCmd::Set)
    }
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
}

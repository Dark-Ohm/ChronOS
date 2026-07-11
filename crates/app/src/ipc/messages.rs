pub const PING_PAYLOAD: &str = "ping";
pub const TOGGLE_LAUNCHER_PAYLOAD: &str = "toggle-launcher";

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
}

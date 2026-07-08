pub const PING_PAYLOAD: &str = "ping";

pub fn encode_ping() -> String {
    PING_PAYLOAD.to_string()
}

pub fn is_ping(payload: &str) -> bool {
    payload.trim() == PING_PAYLOAD
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
}

//! Pure parsers + thin subprocess wrappers for WirePlumber's `wpctl`.
//!
//! Kept separate so unit tests never need a live PipeWire session.

use std::process::Command;

/// Parse `wpctl get-volume` stdout.
///
/// Examples:
/// - `Volume: 0.40`
/// - `Volume: 0.40 [MUTED]`
/// - trailing whitespace / extra lines are tolerated
pub fn parse_get_volume(stdout: &str) -> Option<(f64, bool)> {
    for line in stdout.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("Volume:") else {
            continue;
        };
        let rest = rest.trim();
        let muted = rest.contains("[MUTED]");
        let token = rest.split_whitespace().next()?;
        let volume: f64 = token.parse().ok()?;
        return Some((volume, muted));
    }
    None
}

/// Parse `node.description` from `wpctl inspect` stdout.
///
/// Lines look like:
/// `  * node.description = "Built-in Audio Analog Stereo"`
pub fn parse_node_description(inspect_stdout: &str) -> Option<String> {
    for line in inspect_stdout.lines() {
        let line = line.trim().trim_start_matches('*').trim();
        let Some(rest) = line.strip_prefix("node.description") else {
            continue;
        };
        let rest = rest.trim().trim_start_matches('=').trim();
        if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
            return Some(rest[1..rest.len() - 1].to_string());
        }
        // unquoted fallback
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    None
}

/// Clamp volume into a sane wireplumber range before `set-volume`.
///
/// Allows mild boost (>1.0) up to 1.5; rejects NaN/inf/negatives.
pub fn clamp_volume(v: f64) -> f64 {
    if !v.is_finite() {
        return 0.0;
    }
    v.clamp(0.0, 1.5)
}

/// Run `wpctl` with the given args; return stdout on success.
pub fn run_wpctl(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("wpctl")
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to spawn wpctl: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "wpctl {} failed ({}): {}",
            args.join(" "),
            output.status,
            stderr.trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_volume_plain() {
        assert_eq!(parse_get_volume("Volume: 0.40\n"), Some((0.40, false)));
    }

    #[test]
    fn parse_volume_muted() {
        assert_eq!(
            parse_get_volume("Volume: 0.55 [MUTED]\n"),
            Some((0.55, true))
        );
    }

    #[test]
    fn parse_volume_boosted() {
        assert_eq!(parse_get_volume("Volume: 1.25\n"), Some((1.25, false)));
    }

    #[test]
    fn parse_volume_garbage() {
        assert_eq!(parse_get_volume("not a volume line\n"), None);
        assert_eq!(parse_get_volume(""), None);
    }

    #[test]
    fn parse_description_star_line() {
        let sample = r#"
id 53, type PipeWire:Interface:Node
  * media.class = "Audio/Sink"
  * node.description = "Built-in Audio Analog Stereo"
  * node.name = "alsa_output.pci-0000_00_1f.3.analog-stereo"
"#;
        assert_eq!(
            parse_node_description(sample).as_deref(),
            Some("Built-in Audio Analog Stereo")
        );
    }

    #[test]
    fn parse_description_missing() {
        assert_eq!(parse_node_description("id 1\n"), None);
    }

    #[test]
    fn clamp_rejects_nan_and_caps_boost() {
        assert_eq!(clamp_volume(f64::NAN), 0.0);
        assert_eq!(clamp_volume(-1.0), 0.0);
        assert_eq!(clamp_volume(2.0), 1.5);
        assert_eq!(clamp_volume(0.35), 0.35);
    }
}

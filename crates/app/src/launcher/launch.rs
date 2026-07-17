//! Process launch with session detachment so spawned apps survive chronos.

use anyhow::{Context, Result};
use std::process::{Command, Stdio};

use chronos_services::strip_field_codes;

/// Launch a desktop entry's `Exec=` value as a detached process.
///
/// The child is reparented into a new session via `setsid` and has all three
/// stdio streams redirected to `/dev/null`, so it keeps running after chronos
/// is killed or crashes (no SIGHUP from a shared controlling terminal, no
/// broken-pipe writes to closed fds).
pub fn launch(exec: &str) -> Result<()> {
    // Defensive: field codes should already be stripped at parse time, but the
    // launcher never passes file arguments, so strip again before spawning.
    let clean = strip_field_codes(exec);

    Command::new("setsid")
        .arg("sh")
        .arg("-c")
        .arg(&clean)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .context("failed to launch application")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_field_codes_before_launch() {
        // launch() must not embed raw field codes in the spawned command.
        let clean = strip_field_codes("firefox %U --profile %f");
        assert!(
            !clean.contains('%'),
            "field codes must be stripped: {clean}"
        );
    }

    #[test]
    fn setsid_is_available() {
        // The detach strategy depends on setsid existing on the host.
        assert!(
            Command::new("setsid")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok(),
            "setsid must be present on the system"
        );
    }
}

//! Shell provider (`>`): run the typed command in `$SHELL -lc` from `$HOME`.

use anyhow::{Context, Result};
use std::process::{Command, Stdio};

use super::{ProviderAction, ProviderResult};

/// One row: the command itself. Enter runs it. No shell-history suggestions in
/// this wave (T265-E marks them optional — "может предлагать").
pub fn results(cmd: &str) -> Vec<ProviderResult> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return vec![ProviderResult {
            id: "shell-hint".into(),
            label: "type a command to run".into(),
            detail: Some("$SHELL -lc · cwd $HOME".into()),
            glyph: '>',
            action: ProviderAction::None,
        }];
    }
    vec![ProviderResult {
        id: "shell-run".into(),
        label: cmd.to_string(),
        detail: Some("run in $SHELL".into()),
        glyph: '>',
        action: ProviderAction::RunCommand(cmd.to_string()),
    }]
}

/// Run a command detached in `$SHELL -lc` with cwd `$HOME` (T265-E).
///
/// Detached (setsid + `/dev/null`) exactly like `launch::launch`, so the
/// process survives chronos. The launcher is a launcher, not a terminal:
/// output is not captured.
pub fn run(cmd: &str) -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    Command::new("setsid")
        .arg(&shell)
        .arg("-lc")
        .arg(cmd)
        .current_dir(&home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .context("failed to run shell command")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_is_a_hint() {
        let rows = results("   ");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, ProviderAction::None);
    }

    #[test]
    fn command_row_runs_on_enter() {
        let rows = results("echo hi");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, ProviderAction::RunCommand("echo hi".into()));
    }
}

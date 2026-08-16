//! AUR/pacman update-check service data types.
//!
//! No floats here (unlike audio/upower) — plain strings/enum, `Eq` is safe.

/// Which package source an update came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UpdateSource {
    #[default]
    Official,
    Aur,
}

/// One pending package update (name + old→new version).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageUpdate {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    pub source: UpdateSource,
}

/// State of a running "Upgrade all" operation — drives button
/// enable/disable, progress bar, live output, and footer status text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum UpgradeState {
    #[default]
    Idle,
    Running(UpgradeProgress),
    Done,
    Failed,
}

/// Live progress of a running upgrade.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpgradeProgress {
    /// Current step (1-indexed).
    pub current: usize,
    /// Total steps to perform.
    pub total: usize,
    /// Last line of output from the upgrade process.
    pub last_line: String,
    /// Package names that have been fully processed (for staircase removal).
    pub completed_names: Vec<String>,
}

impl UpgradeProgress {
    /// Percentage as 0-100.
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            0
        } else {
            ((self.current as f64 / self.total as f64) * 100.0).min(100.0) as u8
        }
    }
}

/// Reactive snapshot of all pending updates (official + AUR, if `yay` is
/// present). Empty `updates` means "no pending updates" — the same value the
/// service reports while genuinely up to date and (briefly) while
/// `Initializing`; consult `Service::status()` to tell those apart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdatesState {
    pub updates: Vec<PackageUpdate>,
    pub upgrade_state: UpgradeState,
    /// True while an explicit `AurCommand::Refresh` is in flight (popup
    /// "Check updates" / open-time re-fetch). Distinct from poll ticks so
    /// the UI can show a busy affordance instead of a dead-looking button.
    pub checking: bool,
}

impl UpdatesState {
    /// Number of pending updates — what the bar badge shows.
    pub fn count(&self) -> usize {
        self.updates.len()
    }
}

/// Commands issued by the bar widget / popup.
#[derive(Clone, Debug)]
pub enum AurCommand {
    /// Force an immediate re-check instead of waiting for the next poll
    /// tick (bar click / popup open — mirrors `TrayCommand::FetchMenu`).
    Refresh,
    /// Run the real system upgrade (`pkexec pacman -Syu --noconfirm`).
    /// The ONLY privileged operation in this service — never invoked by the
    /// poll loop itself, only from the Updates tab's "Upgrade all" button.
    /// T294: apply is always pacman (official repos); AUR is display-only.
    UpgradeAll,
    /// Upgrade only the named (Official) packages — `pkexec pacman -Sy
    /// --noconfirm -- <pkgs>` (`-y` refreshes DBs so versions match what
    /// `checkupdates` reported; no `-u` → not a full sysupgrade). Same
    /// streaming path as `UpgradeAll`. Callers must send ONLY Official names
    /// (AUR is display-only); an AUR-only selection dispatches `[]`, which is
    /// a no-op here (UI + service both guard).
    UpgradeSelected { packages: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_state_default_is_idle() {
        assert_eq!(UpgradeState::default(), UpgradeState::Idle);
    }

    #[test]
    fn upgrade_state_roundtrip() {
        for s in [
            UpgradeState::Idle,
            UpgradeState::Running(UpgradeProgress::default()),
            UpgradeState::Done,
            UpgradeState::Failed,
        ] {
            let clone = s.clone();
            assert_eq!(s, clone);
        }
    }

    #[test]
    fn upgrade_progress_percent() {
        let p = UpgradeProgress { current: 3, total: 10, ..Default::default() };
        assert_eq!(p.percent(), 30);
        let zero = UpgradeProgress::default();
        assert_eq!(zero.percent(), 0);
    }

    #[test]
    fn updates_state_default_has_idle_upgrade() {
        let state = UpdatesState::default();
        assert_eq!(state.upgrade_state, UpgradeState::Idle);
        assert!(state.updates.is_empty());
    }
}

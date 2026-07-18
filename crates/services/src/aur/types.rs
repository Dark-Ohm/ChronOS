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

/// Reactive snapshot of all pending updates (official + AUR, if `yay` is
/// present). Empty `updates` means "no pending updates" — the same value the
/// service reports while genuinely up to date and (briefly) while
/// `Initializing`; consult `Service::status()` to tell those apart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdatesState {
    pub updates: Vec<PackageUpdate>,
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
    /// Run the real system upgrade (`pkexec yay -Syu`/`pkexec pacman -Syu`).
    /// The ONLY privileged operation in this service — never invoked by the
    /// poll loop itself, only from the popup's "Upgrade all" button.
    UpgradeAll,
}

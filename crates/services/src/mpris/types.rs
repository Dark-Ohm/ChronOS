//! MPRIS media-player state and commands.

/// Snapshot of the active MPRIS player (flat, widget-friendly).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MprisState {
    pub title: String,
    pub artist: String,
    /// True when `PlaybackStatus == "Playing"`.
    pub playing: bool,
    /// At least one `org.mpris.MediaPlayer2.*` name is on the session bus.
    pub has_player: bool,
    /// Number of live MPRIS players on the session bus.
    pub player_count: usize,
    /// 1-based index of the active player in the live list (0 if none).
    pub player_index: usize,
    /// Short id of the active player (suffix after `org.mpris.MediaPlayer2.`).
    pub player_id: String,
    /// `mpris:artUrl` from Metadata (often `file://…`; http(s) left as-is for UI).
    pub art_url: Option<String>,
    /// Player `Position` property, microseconds. `None` if Get failed.
    pub position_us: Option<i64>,
    /// `mpris:length` from Metadata, microseconds. `None` if missing or ≤0.
    pub length_us: Option<i64>,
}

/// Direction for cycling the sticky user-selected player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleDirection {
    Next,
    Prev,
}

/// Commands issued by the bar widget against the active player.
#[derive(Clone, Debug)]
pub enum MprisCommand {
    PlayPause,
    Next,
    Previous,
    /// Advance sticky selection to next/prev live player (wrap-around).
    CyclePlayer(CycleDirection),
}

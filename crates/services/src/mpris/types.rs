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
}

/// Commands issued by the bar widget against the active player.
#[derive(Clone, Debug)]
pub enum MprisCommand {
    PlayPause,
    Next,
    Previous,
}

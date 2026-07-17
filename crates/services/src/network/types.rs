//! Network data types.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectivityState {
    #[default]
    Unknown,
    None,
    Portal,
    Limited,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkData {
    pub connectivity: ConnectivityState,
    pub wifi_ssid: Option<String>,
    pub wifi_strength: Option<u8>,
    /// Honest wired-ethernet flag, derived from NetworkManager
    /// (an active connection of type "802-3-ethernet"). Replaces the old
    /// heuristic `Full && ssid.is_none()` in the bar widget.
    pub wired: bool,
}

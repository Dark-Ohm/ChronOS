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
}

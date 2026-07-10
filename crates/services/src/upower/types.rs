//! Power data types.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BatteryState {
    #[default]
    Unknown,
    Charging,
    Discharging,
    Full,
    Empty,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PowerProfile {
    #[default]
    Performance,
    Balanced,
    PowerSaver,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UPowerData {
    pub battery_percent: f64,
    pub state: BatteryState,
    pub power_profile: PowerProfile,
}

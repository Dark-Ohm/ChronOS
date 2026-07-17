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
    /// Honest "a battery is present" flag, derived from UPower
    /// `EnumerateDevices` (any device of Type=Battery) OR the
    /// DisplayDevice's `IsPresent`. Replaces the old heuristic
    /// (`Unknown` state + 0%) in the bar widget.
    pub has_battery: bool,
}

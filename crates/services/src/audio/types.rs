//! Audio service data types (PipeWire via WirePlumber `wpctl` MVP backend).
//!
//! Float volumes → **no `Eq`** (HANDOFF / MEMORY: float-in-Data trap, third hit
//! was UPower; do not reintroduce).

/// One selectable PipeWire endpoint (sink or source) from `pw-dump`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioDevice {
    /// PipeWire object id — argument to `wpctl set-default <id>`.
    pub id: u32,
    /// Human-readable `node.description`.
    pub name: String,
    /// Technical `node.name` — matched against metadata default.
    pub node_name: String,
    /// Whether this device is the current session default.
    pub is_default: bool,
}

/// One PipeWire endpoint (default sink or default source).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EndpointState {
    /// Linear volume. WirePlumber reports 0.0–1.0 for 0–100%; values >1.0 are
    /// valid when boosted above 100%.
    pub volume: f64,
    pub muted: bool,
    /// Human-readable name (`node.description`), empty when unknown.
    pub name: String,
    /// All sinks or sources of this kind, from `pw-dump` (refreshed on poll).
    pub available: Vec<AudioDevice>,
}

/// Reactive snapshot of the default sink + default source.
///
/// `PartialEq` only — contains `f64` volumes (must not derive `Eq`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioState {
    pub sink: EndpointState,
    pub source: EndpointState,
}

/// Commands issued by UI (OSD / bar / volume popup) against devices.
#[derive(Clone, Debug)]
pub enum AudioCommand {
    SetSinkVolume(f64),
    SetSourceVolume(f64),
    ToggleSinkMute,
    ToggleSourceMute,
    /// `wpctl set-default <id>` for a sink node id from `pw-dump`.
    SetDefaultSink(u32),
    /// `wpctl set-default <id>` for a source node id from `pw-dump`.
    SetDefaultSource(u32),
}

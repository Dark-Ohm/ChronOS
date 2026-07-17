//! Audio service data types (PipeWire via WirePlumber `wpctl` MVP backend).
//!
//! Float volumes → **no `Eq`** (HANDOFF / MEMORY: float-in-Data trap, third hit
//! was UPower; do not reintroduce).

/// One PipeWire endpoint (default sink or default source).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EndpointState {
    /// Linear volume. WirePlumber reports 0.0–1.0 for 0–100%; values >1.0 are
    /// valid when boosted above 100%.
    pub volume: f64,
    pub muted: bool,
    /// Human-readable name (`node.description`), empty when unknown.
    pub name: String,
}

/// Reactive snapshot of the default sink + default source.
///
/// `PartialEq` only — contains `f64` volumes (must not derive `Eq`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioState {
    pub sink: EndpointState,
    pub source: EndpointState,
}

/// Commands issued by UI (OSD / bar sliders) against the default devices.
#[derive(Clone, Debug)]
pub enum AudioCommand {
    SetSinkVolume(f64),
    SetSourceVolume(f64),
    ToggleSinkMute,
    ToggleSourceMute,
}

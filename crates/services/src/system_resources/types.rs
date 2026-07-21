//! System resources service data types (CPU/RAM/GPU).

/// Snapshot of current system resource utilization, 0.0–100.0 per field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemResourcesState {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    /// `None` when no supported GPU backend is available (non-Nvidia, or
    /// NVML failed to initialize) — the panel hides the GPU row in that case.
    pub gpu_percent: Option<f32>,
}

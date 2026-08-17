//! T266 blur bridge state — the shell's view of the compositor blur module.
//!
//! The Hyprland module (`packaging/hyprland/45-surface-effects-chronos.lua`)
//! is OPT-IN: the user imports it, we never touch their config. This global
//! is what Bar settings renders against: probe once at startup in the
//! background, apply the persisted `blur_enabled` only when the bridge is
//! actually available, and surface a disabled toggle with a reason when it
//! is not (T246 bar: a control without a backend is a lie; a disabled
//! control with an explanation is an honest interface).

use gpui::{App, Global};

use chronos_services::compositor::BlurCapability;

/// Snapshot of the blur bridge state, updated by the background probe task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceEffectsState {
    /// Last probed bridge availability.
    pub capability: BlurCapability,
    /// Persisted `blur_enabled` from `theme.toml` — the state the toggle
    /// renders while the probe is still in flight.
    pub persisted_blur: bool,
    /// Whether the probe has finished (toggle renders disabled until then).
    pub probed: bool,
}

impl Default for SurfaceEffectsState {
    fn default() -> Self {
        Self {
            capability: BlurCapability::Available,
            persisted_blur: false,
            probed: false,
        }
    }
}

impl Global for SurfaceEffectsState {}

/// Read the current bridge state (any context, no App borrow).
pub fn current(cx: &App) -> SurfaceEffectsState {
    cx.try_global::<SurfaceEffectsState>().copied().unwrap_or_default()
}

/// Initialize the bridge: seed with persisted config, probe in the
/// background, then — only when `Available` — apply the persisted blur.
/// Runs after `theme_config::init` (cold-start contract: a persisted
/// `blur_enabled = true` must work before the settings page is ever opened).
pub fn init(cx: &mut App) {
    let persisted = chronos_ui::Theme::global(cx).surface.blur_enabled;
    cx.set_global(SurfaceEffectsState {
        capability: BlurCapability::Available,
        persisted_blur: persisted,
        probed: false,
    });
    cx.spawn(async move |mut cx| {
        let capability = chronos_services::compositor::probe_shell_blur();
        // Apply the persisted flag only when the bridge is genuinely there —
        // never push a no-op set to a session without the module. Run the
        // compositor I/O off the main thread (the probe is a subprocess).
        let apply_result: Result<(), String> = cx.update(|cx| {
            let mut state = current(cx);
            state.capability = capability;
            state.probed = true;
            if capability == BlurCapability::Available {
                let wanted = state.persisted_blur;
                chronos_services::compositor::set_shell_blur_enabled(wanted)
                    .map_err(|e| e.to_string())
                    .map(|_| {
                        tracing::info!(
                            capability = ?capability,
                            blur = wanted,
                            "surface_effects: persisted blur applied"
                        );
                    })
            } else {
                tracing::info!(
                    capability = ?capability,
                    "surface_effects: blur bridge not available, persisted state untouched"
                );
                Ok(())
            };
            cx.set_global(state);
            cx.refresh_windows();
            Ok(())
        });
        if let Err(e) = apply_result {
            tracing::warn!("surface_effects: probe/apply failed: {e}");
        }
    })
    .detach();
}

/// Toggle handler used by Bar settings: call the bridge first, persist only
/// on success, update the global, refresh windows. Never renders an enabled
/// control whose action cannot reach the bridge.
pub fn set_blur_enabled(enabled: bool, cx: &mut App) -> Result<(), String> {
    let state = current(cx);
    if !state.probed || state.capability != BlurCapability::Available {
        return Err(match state.capability {
            BlurCapability::ModuleMissing => {
                "Hyprland blur module not imported — add \
                 `dofile(os.getenv(\"HOME\") .. \"/.config/hypr/chronos/45-surface-effects-chronos.lua\")` \
                 to hyprland.lua"
                    .to_string()
            }
            _ => "Compositor does not support blur".to_string(),
        });
    }
    chronos_services::compositor::set_shell_blur_enabled(enabled).map_err(|e| e.to_string())?;
    crate::theme_config::persist_blur_enabled(enabled)?;
    let mut state = current(cx);
    state.persisted_blur = enabled;
    cx.set_global(state);
    cx.refresh_windows();
    Ok(())
}

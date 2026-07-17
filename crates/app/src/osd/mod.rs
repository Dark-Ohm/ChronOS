//! OSD (on-screen display) for audio volume / mute.
//!
//! Architecture mirrors `notifications/`:
//!   * `OsdPopupState` — GPUI global: last audio snapshot, what is shown,
//!     open window handle, hide-timer generation.
//!   * `OsdWatcher` — tiny entity hosting `state::watch()` on
//!     `AppState::audio(cx).subscribe()`.
//!   * Baseline: first **non-default** `AudioState` is seeded only (no show).
//!     Only subsequent sink/source diffs open/refresh the overlay.
//!   * Hide is soft: clear display + empty input region, **no** `remove_window`
//!     (destroy races Wayland frame callbacks → pair of `ERROR : window not
//!     found` from gpui `.log_err()`). Surface is reused on the next show.

pub mod view;

use std::time::Duration;

use gpui::{
    App, Bounds, Context, DisplayId, Entity, Global, Size, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::{AudioState, EndpointState, Service};

use crate::osd::view::OsdView;
use crate::state::{self, AppState};

/// OSD surface size (px). Compact volume strip, GNOME/macOS-like.
const OSD_WIDTH: f32 = 320.;
const OSD_HEIGHT: f32 = 80.;
/// Bottom margin so the strip sits above the bottom edge.
const OSD_MARGIN_BOTTOM: f32 = 48.;
/// Auto-hide delay after the last audio change.
const HIDE_AFTER: Duration = Duration::from_millis(1500);

/// What the open OSD window is currently presenting.
#[derive(Clone, Debug)]
pub struct OsdDisplay {
    pub volume: f64,
    pub muted: bool,
    /// `true` → default source (microphone); `false` → default sink.
    pub is_source: bool,
    pub name: String,
}

impl OsdDisplay {
    fn from_endpoint(ep: &EndpointState, is_source: bool) -> Self {
        Self {
            volume: ep.volume,
            muted: ep.muted,
            is_source,
            name: ep.name.clone(),
        }
    }

    /// Fill width of the progress bar in 0.0–1.0 (boosts above 100% clamp).
    pub fn bar_fraction(&self) -> f32 {
        self.volume.clamp(0.0, 1.0) as f32
    }

    pub fn percent_label(&self) -> u32 {
        (self.volume * 100.0).round().clamp(0.0, 999.0) as u32
    }
}

/// Global OSD state driven by the audio service.
#[derive(Default)]
pub struct OsdPopupState {
    handle: Option<WindowHandle<OsdView>>,
    watcher: Option<Entity<OsdWatcher>>,
    /// `None` until the first non-default snapshot is seeded (still suppressed).
    last_audio: Option<AudioState>,
    /// Content of the open window; `None` when hidden.
    display: Option<OsdDisplay>,
    /// Bumped on every show/refresh so stale hide timers no-op.
    hide_generation: u64,
}

impl Global for OsdPopupState {}

impl OsdPopupState {
    /// Live display for the view (read-only).
    pub fn display(&self) -> Option<&OsdDisplay> {
        self.display.as_ref()
    }
}

/// Hosts `state::watch()` — no state of its own.
pub struct OsdWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}

/// Bottom-centre overlay. `KeyboardInteractivity::None` — Exclusive is
/// permanently forbidden (freezes Hyprland input stack).
fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(OSD_WIDTH), px(OSD_HEIGHT)),
        })),
        app_id: Some("chronos-osd".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "osd".to_string(),
            layer: Layer::Overlay,
            // Bottom + left + right → full-width row at bottom; content
            // centres itself inside the view. Exclusive zone must stay None.
            anchor: Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(0.), px(0.), px(OSD_MARGIN_BOTTOM), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Soft-hide: clear content + empty input region. Do **not** `remove_window`.
///
/// Destroying the layer-shell surface races Wayland frame callbacks in gpui
/// (`handle.update(...).log_err()`), which emit a pair of
/// `ERROR : window not found` on every hide. Reusing the surface avoids that.
fn hide_window(cx: &mut App) {
    cx.global_mut::<OsdPopupState>().display = None;
    let handle = cx.global::<OsdPopupState>().handle.clone();
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, view_cx| {
            // Empty region → no input at all (pass-through while surface lives).
            window.set_input_region(Some(&[]));
            view_cx.notify();
        });
    }
}

/// Open or repaint the OSD with `display`, then (re)start the hide timer.
fn show(display: OsdDisplay, cx: &mut Context<OsdWatcher>) {
    cx.global_mut::<OsdPopupState>().display = Some(display);

    let handle = cx.global::<OsdPopupState>().handle.clone();
    match handle {
        Some(existing) => {
            let _ = existing.update(cx, |_, window, view_cx| {
                // OSD is display-only; keep input empty so the strip never
                // steals clicks under the card.
                window.set_input_region(Some(&[]));
                view_cx.notify();
            });
        }
        None => {
            let display_id = pick_display(cx);
            match cx.open_window(window_options(display_id), |_, app_cx| {
                app_cx.new(|view_cx| OsdView::new(view_cx))
            }) {
                Ok(new_handle) => {
                    let _ = new_handle.update(cx, |_, window, _| {
                        window.set_input_region(Some(&[]));
                    });
                    cx.global_mut::<OsdPopupState>().handle = Some(new_handle);
                }
                Err(err) => tracing::warn!("Failed to open OSD window: {err}"),
            }
        }
    }

    schedule_hide(cx);
}

fn schedule_hide(cx: &mut Context<OsdWatcher>) {
    let hide_token = {
        let state = cx.global_mut::<OsdPopupState>();
        state.hide_generation = state.hide_generation.wrapping_add(1);
        state.hide_generation
    };

    cx.spawn(async move |this, cx| {
        cx.background_executor().timer(HIDE_AFTER).await;
        let _ = this.update(cx, |_, cx| {
            if cx.global::<OsdPopupState>().hide_generation != hide_token {
                return;
            }
            hide_window(cx);
        });
    })
    .detach();
}

fn on_audio(state: AudioState, cx: &mut Context<OsdWatcher>) {
    // --- Baseline gate (erratum #2): ignore default until first real poll ---
    // AudioSubscriber starts as AudioState::default(); if OSD subscribes
    // before the first wpctl poll, that default is the first signal emission.
    // Treating it as "seed" made the *second* (real) snapshot look like a
    // user change → flash on shell start. Wait for non-default, seed only.
    if cx.global::<OsdPopupState>().last_audio.is_none() {
        if state == AudioState::default() {
            tracing::debug!("OSD: waiting for first non-default audio snapshot");
            return;
        }
        cx.global_mut::<OsdPopupState>().last_audio = Some(state);
        tracing::debug!("OSD: baseline audio snapshot seeded (suppressed)");
        return;
    }

    let prev = cx
        .global_mut::<OsdPopupState>()
        .last_audio
        .replace(state.clone())
        .expect("last_audio set above");

    let sink_changed = prev.sink != state.sink;
    let source_changed = prev.source != state.source;
    if !sink_changed && !source_changed {
        return;
    }

    // Prefer the endpoint that actually changed; if both, sink wins
    // (playback volume is the common case).
    let shown = if sink_changed {
        OsdDisplay::from_endpoint(&state.sink, false)
    } else {
        OsdDisplay::from_endpoint(&state.source, true)
    };

    tracing::info!(
        is_source = shown.is_source,
        volume = shown.volume,
        muted = shown.muted,
        "OSD: audio change"
    );
    show(shown, cx);
}

/// Wire OSD to the live audio stream. Called once from `main.rs`.
pub fn init(cx: &mut App) {
    let mut state = OsdPopupState::default();
    // If audio already has a real snapshot (poll won the race), seed now so
    // the first watch emission is treated as a no-op or a real user change.
    let current = AppState::audio(cx).get();
    if current != AudioState::default() {
        state.last_audio = Some(current);
        tracing::debug!("OSD: seeded last_audio from service.get() at init");
    }
    cx.set_global(state);

    let signal = AppState::audio(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut OsdWatcher, state: AudioState, cx: &mut Context<OsdWatcher>| {
                on_audio(state, cx);
            },
        );
        OsdWatcher {}
    });

    cx.global_mut::<OsdPopupState>().watcher = Some(watcher);
    tracing::info!("OSD: subscribed to audio service");
}

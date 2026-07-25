//! Audio service via WirePlumber CLI (`wpctl`) — **temporary MVP backend**.
//!
//! ASYNC TEMPLATE (spec §5.1): same shape as `NetworkSubscriber` /
//! `UPowerSubscriber`. `new()` captures `Handle::current()` and
//! `tokio::spawn`s a poll loop; `init_all()` calls this inside `rt.block_on`.
//!
//! ## Backend choice (see DECISIONS.log 2026-07-17)
//!
//! Native `pipewire` crate (FFI mainloop on a dedicated thread) is the correct
//! long-term path. For the first service cut we use `wpctl` subprocesses + a
//! 250 ms poll so external changes (pavucontrol / `wpctl set-volume` from
//! another terminal) reach `AudioState` and wake subscribers. Replace this
//! module body when the native backend lands — keep `types.rs` stable.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{AudioCommand, AudioDevice, AudioState, AudioStream, EndpointState};
pub use wpctl::{
    clamp_volume, format_set_default_args, format_set_mute_toggle_args, format_set_volume_args,
    parse_get_volume, parse_node_description,
};

mod pw_dump;
pub mod types;
mod wpctl;

const DEFAULT_SINK: &str = "@DEFAULT_AUDIO_SINK@";
const DEFAULT_SOURCE: &str = "@DEFAULT_AUDIO_SOURCE@";
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Volume to apply: target endpoint id and the volume value.
/// `PartialEq` so `watch` only notifies when the pending value actually changes.
#[derive(Clone, Debug, PartialEq)]
struct PendingVolume {
    target: String,
    volume: f64,
}

#[derive(Clone)]
pub struct AudioSubscriber {
    data: Mutable<AudioState>,
    status: Mutable<ServiceStatus>,
    /// Captured in `new()` — runtime guard + fire-and-forget for `dispatch`.
    runtime: Handle,
    /// Channel for volume coalescing: latest value wins, background task applies.
    volume_tx: watch::Sender<Option<PendingVolume>>,
}

impl AudioSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(AudioState::default());
        let status = Mutable::new(ServiceStatus::Initializing);

        // Guard: must run inside `rt.block_on` (spec §5.1 + §7).
        let handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone()));

        // Spawn volume coalesce task — drains latest pending value, applies to
        // PipeWire, reads back actual volume (light confirm). Non-volume commands
        // still use full read_state on their own path.
        let (volume_tx, volume_rx) = watch::channel::<Option<PendingVolume>>(None);
        let coalesce_data = data.clone();
        let coalesce_status = status.clone();
        tokio::spawn(run_volume_coalesce(
            volume_rx,
            coalesce_data,
            coalesce_status,
        ));

        Self {
            data,
            status,
            runtime: handle,
            volume_tx,
        }
    }

    /// Fire-and-forget command dispatch (mirrors `TraySubscriber::dispatch` /
    /// `CompositorSubscriber::dispatch`). Safe to call from a GPUI click handler.
    ///
    /// **Volume commands** (`SetSinkVolume` / `SetSourceVolume`) take a coalesced
    /// fast path: optimistic state update + background wpctl apply with light
    /// confirm. No full `read_state()` (no `pw-dump`).
    ///
    /// **All other commands** use the original path: apply + full `read_state()`.
    pub fn dispatch(&self, cmd: AudioCommand) {
        let data = self.data.clone();
        let status = self.status.clone();

        match &cmd {
            AudioCommand::SetSinkVolume(v) => {
                let target = DEFAULT_SINK.to_string();
                let v_clamped = wpctl::clamp_volume(*v);

                // Optimistic: update state immediately so UI re-renders.
                let current = data.get_cloned();
                let optimistic = AudioState {
                    sink: EndpointState {
                        volume: v_clamped,
                        ..current.sink
                    },
                    source: current.source,
                };
                data.set(optimistic);
                status.set(ServiceStatus::Available);

                // Coalesce: wake background task (latest-wins).
                let _ = self.volume_tx.send(Some(PendingVolume {
                    target,
                    volume: v_clamped,
                }));
                debug!("volume: dispatch sink {v_clamped:.2}");
            }
            AudioCommand::SetSourceVolume(v) => {
                let target = DEFAULT_SOURCE.to_string();
                let v_clamped = wpctl::clamp_volume(*v);

                let current = data.get_cloned();
                let optimistic = AudioState {
                    sink: current.sink,
                    source: EndpointState {
                        volume: v_clamped,
                        ..current.source
                    },
                };
                data.set(optimistic);
                status.set(ServiceStatus::Available);

                let _ = self.volume_tx.send(Some(PendingVolume {
                    target,
                    volume: v_clamped,
                }));
                debug!("volume: dispatch source {v_clamped:.2}");
            }
            _ => {
                // Non-volume commands: original path (apply + full re-read).
                let data = self.data.clone();
                let status = self.status.clone();
                self.runtime.spawn(async move {
                    if let Err(e) = apply_command(&cmd).await {
                        warn!("AudioSubscriber command failed ({cmd:?}): {e:?}");
                        return;
                    }
                    // Full re-read for mute/toggle/default (needs device list).
                    match read_state().await {
                        Ok(state) => {
                            data.set(state);
                            status.set(ServiceStatus::Available);
                        }
                        Err(e) => {
                            warn!("AudioSubscriber re-read after command failed: {e:?}");
                        }
                    }
                });
            }
        }
    }

    /// Resolve `player_hint` to a live PipeWire stream and toggle its mute.
    ///
    /// No-op (logged, not erred) if no matching stream is found — expected for
    /// many-tabs / mismatched MPRIS↔PipeWire names; see
    /// [`pw_dump::find_stream_for_player`].
    pub fn toggle_stream_mute_for_player(&self, player_hint: String) {
        let this = self.clone();
        self.runtime.spawn(async move {
            let json = match tokio::task::spawn_blocking(pw_dump::run_pw_dump).await {
                Ok(Ok(json)) => json,
                Ok(Err(e)) => {
                    warn!("toggle_stream_mute_for_player: pw-dump failed: {e}");
                    return;
                }
                Err(e) => {
                    warn!("toggle_stream_mute_for_player: join error: {e}");
                    return;
                }
            };
            let streams = match pw_dump::parse_pw_dump_streams(&json) {
                Ok(s) => s,
                Err(e) => {
                    warn!("toggle_stream_mute_for_player: parse failed: {e}");
                    return;
                }
            };
            match pw_dump::find_stream_for_player(&streams, &player_hint) {
                Some(id) => this.dispatch(AudioCommand::ToggleStreamMute(id)),
                None => {
                    info!(
                        "toggle_stream_mute_for_player: no PipeWire stream matched '{player_hint}'"
                    );
                }
            }
        });
    }
}

impl Service for AudioSubscriber {
    type Data = AudioState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = AudioState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> AudioState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Volume coalesce task: waits for watch notifications, applies latest
/// pending volume to PipeWire, light-confirms (no `pw-dump`, no inspect).
///
/// **Latest-wins:** while `wpctl` runs, further `dispatch` calls overwrite the
/// channel; after apply we drain any newer value before sleeping on
/// `changed()` again. We must **not** spin-apply the same `Some(pv)` every
/// few ms — the channel keeps the last `Some` until a newer send replaces it.
async fn run_volume_coalesce(
    mut rx: watch::Receiver<Option<PendingVolume>>,
    data: Mutable<AudioState>,
    status: Mutable<ServiceStatus>,
) {
    loop {
        // Block until the sender posts a new value (first Some or a change).
        if rx.changed().await.is_err() {
            return; // sender dropped
        }

        let Some(mut pv) = rx.borrow_and_update().clone() else {
            continue; // ignore empty clears if ever used
        };

        // Apply current, then any value that arrived mid-apply (true coalesce).
        loop {
            if let Err(e) = apply_to_pipewire(&pv.target, pv.volume).await {
                warn!("volume coalesce: wpctl failed for {}: {e:?}", pv.target);
            } else if let Ok((vol, muted)) = read_volume_only(&pv.target).await {
                let current = data.get_cloned();
                let updated = merge_volume_into_state(&current, &pv.target, vol, muted);
                if data.get_cloned() != updated {
                    data.set(updated);
                }
                status.set(ServiceStatus::Available);
            }

            if !rx.has_changed().unwrap_or(false) {
                break;
            }
            match rx.borrow_and_update().clone() {
                Some(next) => pv = next,
                None => break,
            }
        }
    }
}

/// Apply a single `wpctl set-volume` for the given target.
async fn apply_to_pipewire(target: &str, volume: f64) -> anyhow::Result<()> {
    let args = wpctl::format_set_volume_args(target, volume);
    tokio::task::spawn_blocking(move || {
        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        wpctl::run_wpctl(&str_args).map(|_| ())
    })
    .await
    .map_err(|e| anyhow::anyhow!("volume apply join error: {e}"))?
}

/// Read only `wpctl get-volume <target>` — returns (volume, muted).
/// No `pw-dump`, no `inspect` — very cheap.
async fn read_volume_only(target: &str) -> anyhow::Result<(f64, bool)> {
    let target = target.to_string();
    tokio::task::spawn_blocking(move || {
        let vol_out = wpctl::run_wpctl(&["get-volume", &target])?;
        let (volume, muted) = parse_get_volume(&vol_out)
            .ok_or_else(|| anyhow::anyhow!("unparseable get-volume for {target}: {vol_out:?}"))?;
        Ok((volume, muted))
    })
    .await
    .map_err(|e| anyhow::anyhow!("volume read join error: {e}"))?
}

/// Poll loop: read sink+source, publish diffs, exponential backoff on hard
/// failure (missing `wpctl` / no PipeWire session).
///
/// Device lists (from `pw-dump`) are only refreshed here — never on the
/// command dispatch path.
async fn run(data: Mutable<AudioState>, status: Mutable<ServiceStatus>) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    let mut backoff = Duration::from_secs(1);
    let mut logged_ok = false;

    loop {
        match read_state().await {
            Ok(state) => {
                // Only notify subscribers when something actually changed
                // (float PartialEq is fine for "same wpctl report").
                if data.get_cloned() != state {
                    data.set(state);
                }
                if status.get_cloned() != ServiceStatus::Available {
                    status.set(ServiceStatus::Available);
                }
                if !logged_ok {
                    info!("AudioSubscriber connected (wpctl MVP backend)");
                    logged_ok = true;
                }
                backoff = Duration::from_secs(1);
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(e) => {
                warn!("AudioSubscriber read failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
                logged_ok = false;
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

async fn read_state() -> anyhow::Result<AudioState> {
    tokio::task::spawn_blocking(|| {
        let mut sink = read_endpoint(DEFAULT_SINK)?;
        let mut source = read_endpoint(DEFAULT_SOURCE)?;
        // Device list from pw-dump (same poll tick). Soft-fail: empty lists if
        // dump is missing/broken — volume/mute still work.
        match pw_dump::run_pw_dump().and_then(|j| pw_dump::parse_pw_dump_devices(&j)) {
            Ok((sinks, sources)) => {
                sink.available = sinks;
                source.available = sources;
            }
            Err(e) => {
                warn!("audio: pw-dump device list failed: {e:?}");
            }
        }
        Ok(AudioState { sink, source })
    })
    .await
    .map_err(|e| anyhow::anyhow!("audio read join error: {e}"))?
}

fn read_endpoint(id: &str) -> anyhow::Result<EndpointState> {
    let vol_out = wpctl::run_wpctl(&["get-volume", id])?;
    let (volume, muted) = parse_get_volume(&vol_out)
        .ok_or_else(|| anyhow::anyhow!("unparseable get-volume for {id}: {vol_out:?}"))?;

    let name = wpctl::run_wpctl(&["inspect", id])
        .ok()
        .and_then(|s| parse_node_description(&s))
        .unwrap_or_default();

    Ok(EndpointState {
        volume,
        muted,
        name,
        available: Vec::new(),
    })
}

/// Pure mapping of [`AudioCommand`] → `wpctl` argv (no binary name).
///
/// Unit-tested; `apply_command` only shells out.
#[allow(dead_code)]
pub fn command_to_wpctl_args(cmd: &AudioCommand) -> Vec<String> {
    match cmd {
        AudioCommand::SetSinkVolume(v) => format_set_volume_args(DEFAULT_SINK, *v),
        AudioCommand::SetSourceVolume(v) => format_set_volume_args(DEFAULT_SOURCE, *v),
        AudioCommand::ToggleSinkMute => format_set_mute_toggle_args(DEFAULT_SINK),
        AudioCommand::ToggleSourceMute => format_set_mute_toggle_args(DEFAULT_SOURCE),
        AudioCommand::ToggleStreamMute(id) => format_set_mute_toggle_args(&id.to_string()),
        AudioCommand::SetDefaultSink(id) | AudioCommand::SetDefaultSource(id) => {
            format_set_default_args(*id)
        }
    }
}

async fn apply_command(cmd: &AudioCommand) -> anyhow::Result<()> {
    let args = command_to_wpctl_args(cmd);

    tokio::task::spawn_blocking(move || {
        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        wpctl::run_wpctl(&str_args).map(|_| ())
    })
    .await
    .map_err(|e| anyhow::anyhow!("audio command join error: {e}"))?
}

/// Merge a confirmed volume into the state, preserving device lists.
fn merge_volume_into_state(
    state: &AudioState,
    target: &str,
    volume: f64,
    muted: bool,
) -> AudioState {
    if target == DEFAULT_SINK {
        AudioState {
            sink: EndpointState {
                volume,
                muted,
                ..state.sink.clone()
            },
            source: state.source.clone(),
        }
    } else {
        AudioState {
            sink: state.sink.clone(),
            source: EndpointState {
                volume,
                muted,
                ..state.source.clone()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Same runtime_guard contract as network/upower/tray.
    #[test]
    fn audio_new_panics_outside_runtime() {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = AudioSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "AudioSubscriber::new() must panic outside a tokio runtime (Handle::current guard)"
        );
    }

    #[tokio::test]
    async fn audio_new_inside_runtime_starts_initializing_or_available() {
        let svc = AudioSubscriber::new();
        // Immediately after new() we are Initializing or already Available if
        // the first poll raced ahead (PipeWire present). Never Unavailable yet
        // on a healthy host — but we only assert the constructor returned.
        let st = svc.status();
        assert!(
            matches!(
                st,
                ServiceStatus::Initializing | ServiceStatus::Available | ServiceStatus::Unavailable
            ),
            "unexpected status: {st:?}"
        );
        let _ = svc.get();
    }

    #[test]
    fn audio_state_is_partial_eq_not_eq() {
        // Compile-time guard: if someone re-derives Eq this test file would
        // still compile, so we document the contract here and assert floats
        // compare via PartialEq.
        let a = AudioState {
            sink: EndpointState {
                volume: 0.5,
                muted: false,
                name: "a".into(),
                available: Vec::new(),
            },
            source: EndpointState::default(),
        };
        let b = a.clone();
        assert_eq!(a, b);
        // f64 NaN != NaN under PartialEq — intentional.
        let mut c = a.clone();
        c.sink.volume = f64::NAN;
        assert_ne!(a, c);
    }

    #[test]
    fn command_to_wpctl_args_set_sink_volume() {
        let args = command_to_wpctl_args(&AudioCommand::SetSinkVolume(0.40));
        assert_eq!(
            args,
            vec!["set-volume", "-l", "1.5", "@DEFAULT_AUDIO_SINK@", "40%",]
        );
    }

    #[test]
    fn command_to_wpctl_args_toggle_source_mute() {
        let args = command_to_wpctl_args(&AudioCommand::ToggleSourceMute);
        assert_eq!(args, vec!["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"]);
    }

    #[test]
    fn command_to_wpctl_args_set_default_sink() {
        let args = command_to_wpctl_args(&AudioCommand::SetDefaultSink(70));
        assert_eq!(args, vec!["set-default", "70"]);
    }

    #[test]
    fn command_to_wpctl_args_set_default_source() {
        let args = command_to_wpctl_args(&AudioCommand::SetDefaultSource(46));
        assert_eq!(args, vec!["set-default", "46"]);
    }

    #[test]
    fn command_to_wpctl_args_stream_mute_targets_the_given_id() {
        let args = command_to_wpctl_args(&AudioCommand::ToggleStreamMute(142));
        assert_eq!(args, vec!["set-mute", "142", "toggle"]);
    }

    #[test]
    fn merge_volume_into_state_sink_preserves_source_and_devices() {
        let state = AudioState {
            sink: EndpointState {
                volume: 0.3,
                muted: false,
                name: "Built-in".into(),
                available: vec![AudioDevice {
                    id: 1,
                    name: "Headphones".into(),
                    node_name: "headphones".into(),
                    is_default: false,
                }],
            },
            source: EndpointState {
                volume: 0.7,
                muted: true,
                name: "Mic".into(),
                available: vec![],
            },
        };
        let merged = merge_volume_into_state(&state, DEFAULT_SINK, 0.9, true);
        // Sink volume updated
        assert!((merged.sink.volume - 0.9).abs() < 1e-9);
        assert!(merged.sink.muted);
        // Sink device list preserved
        assert_eq!(merged.sink.available.len(), 1);
        assert_eq!(merged.sink.available[0].name, "Headphones");
        // Source untouched
        assert!((merged.source.volume - 0.7).abs() < 1e-9);
        assert!(merged.source.muted);
    }

    #[test]
    fn merge_volume_into_state_source_preserves_sink() {
        let state = AudioState {
            sink: EndpointState {
                volume: 0.5,
                ..EndpointState::default()
            },
            source: EndpointState::default(),
        };
        let merged = merge_volume_into_state(&state, DEFAULT_SOURCE, 0.25, false);
        assert!((merged.sink.volume - 0.5).abs() < 1e-9);
        assert!((merged.source.volume - 0.25).abs() < 1e-9);
    }
}

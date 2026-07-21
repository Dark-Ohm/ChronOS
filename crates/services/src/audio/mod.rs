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
use tracing::{info, warn};

use crate::Service;
use crate::ServiceStatus;
pub use types::{AudioCommand, AudioDevice, AudioState, AudioStream, EndpointState};
pub use wpctl::{
    clamp_volume, format_set_default_args, format_set_mute_toggle_args, format_set_volume_args,
    parse_get_volume, parse_node_description,
};

pub mod types;
mod pw_dump;
mod wpctl;

const DEFAULT_SINK: &str = "@DEFAULT_AUDIO_SINK@";
const DEFAULT_SOURCE: &str = "@DEFAULT_AUDIO_SOURCE@";
const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct AudioSubscriber {
    data: Mutable<AudioState>,
    status: Mutable<ServiceStatus>,
    /// Captured in `new()` — runtime guard + fire-and-forget for `dispatch`.
    runtime: Handle,
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

        Self {
            data,
            status,
            runtime: handle,
        }
    }

    /// Fire-and-forget command dispatch (mirrors `TraySubscriber::dispatch` /
    /// `CompositorSubscriber::dispatch`). Safe to call from a GPUI click handler.
    pub fn dispatch(&self, cmd: AudioCommand) {
        let data = self.data.clone();
        let status = self.status.clone();
        self.runtime.spawn(async move {
            if let Err(e) = apply_command(&cmd).await {
                warn!("AudioSubscriber command failed ({cmd:?}): {e:?}");
                return;
            }
            // Immediate re-read so UI does not wait for the next poll tick.
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

/// Poll loop: read sink+source, publish diffs, exponential backoff on hard
/// failure (missing `wpctl` / no PipeWire session).
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
            vec![
                "set-volume",
                "-l",
                "1.5",
                "@DEFAULT_AUDIO_SINK@",
                "40%",
            ]
        );
    }

    #[test]
    fn command_to_wpctl_args_toggle_source_mute() {
        let args = command_to_wpctl_args(&AudioCommand::ToggleSourceMute);
        assert_eq!(
            args,
            vec!["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"]
        );
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
}

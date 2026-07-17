//! Live smoke for `AudioSubscriber::dispatch` (SetSinkVolume + ToggleSourceMute).
//!
//! Intended to run **while** release `chronos` is up so the OSD reacts to
//! dispatch (immediate re-read, not the 250 ms poll). Does **not** call raw
//! `wpctl` for the volume step under test — only `dispatch`.
//!
//! ```sh
//! cargo run -p chronos-services --example audio-dispatch-smoke
//! ```

use std::time::{Duration, Instant};

use chronos_services::{AudioCommand, AudioSubscriber, Service, ServiceStatus};
use tokio::runtime::Runtime;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rt = Runtime::new()?;
    rt.block_on(async {
        let audio = AudioSubscriber::new();

        let mut ready = false;
        for _ in 0..40 {
            if audio.status() == ServiceStatus::Available {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            ready,
            "AudioSubscriber never became Available (is wpctl/PipeWire up?)"
        );

        let before = audio.get();
        println!(
            "[dispatch-smoke] before sink={:.2} mute={} source mute={}",
            before.sink.volume, before.sink.muted, before.source.muted
        );
        let restore_sink = before.sink.volume;
        let restore_source_mute = before.source.muted;

        // --- SetSinkVolume(0.40) via dispatch; expect <300ms to land in Data ---
        println!("[dispatch-smoke] SetSinkVolume(0.40)…");
        let t0 = Instant::now();
        audio.dispatch(AudioCommand::SetSinkVolume(0.40));

        let mut hit = false;
        let mut elapsed_ms = 0u128;
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let s = audio.get();
            if (s.sink.volume - 0.40).abs() < 0.02 {
                elapsed_ms = t0.elapsed().as_millis();
                println!(
                    "[dispatch-smoke] sink now {:.2} after {}ms (want <300ms)",
                    s.sink.volume, elapsed_ms
                );
                hit = true;
                break;
            }
        }
        assert!(hit, "sink never reached ~0.40 after SetSinkVolume dispatch");
        assert!(
            elapsed_ms < 300,
            "dispatch re-read too slow: {elapsed_ms}ms (poll path?)"
        );

        // Cross-check with raw wpctl get-volume (read-only).
        let out = std::process::Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        println!("[dispatch-smoke] wpctl get-volume sink: {}", stdout.trim());
        assert!(
            stdout.contains("0.40") || stdout.contains("0.4"),
            "wpctl get-volume did not report ~0.40: {stdout:?}"
        );

        // Hold long enough for a concurrent ChronOS shell (250 ms audio poll
        // + OSD) to observe the change before we restore. Without this, set→
        // restore nets to zero between shell poll ticks.
        println!("[dispatch-smoke] holding 0.40 for 600ms (shell/OSD observe)…");
        tokio::time::sleep(Duration::from_millis(600)).await;

        // --- ToggleSourceMute ---
        println!("[dispatch-smoke] ToggleSourceMute…");
        let muted_before = audio.get().source.muted;
        audio.dispatch(AudioCommand::ToggleSourceMute);
        let mut toggled = false;
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            if audio.get().source.muted != muted_before {
                println!(
                    "[dispatch-smoke] source muted {} → {}",
                    muted_before,
                    audio.get().source.muted
                );
                toggled = true;
                break;
            }
        }
        assert!(toggled, "ToggleSourceMute did not flip source.muted");

        // Hold mute state so shell OSD can show microphone mute.
        println!("[dispatch-smoke] holding mute for 600ms…");
        tokio::time::sleep(Duration::from_millis(600)).await;

        // Restore
        println!("[dispatch-smoke] restoring sink={restore_sink:.2} source_mute={restore_source_mute}");
        audio.dispatch(AudioCommand::SetSinkVolume(restore_sink));
        if audio.get().source.muted != restore_source_mute {
            audio.dispatch(AudioCommand::ToggleSourceMute);
        }
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let s = audio.get();
            if (s.sink.volume - restore_sink).abs() < 0.03
                && s.source.muted == restore_source_mute
            {
                break;
            }
        }
        // Brief settle so final restore is not raced by process exit.
        tokio::time::sleep(Duration::from_millis(300)).await;

        println!("\n✅ audio-dispatch-smoke PASSED");
        Ok(())
    })
}

//! Live PipeWire smoke test for the audio service (wpctl MVP backend).
//!
//! Prints current default sink/source state, sets source volume to 0.35 and
//! back, then waits for an external change (run `wpctl set-volume …` from a
//! second terminal). Requires a session with WirePlumber/`wpctl`.
//!
//! ```sh
//! cargo run -p chronos-services --example audio-smoke
//! ```

use std::time::Duration;

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

        // Wait until the first successful poll.
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
            "[smoke] initial sink: vol={:.2} mute={} name={:?}",
            before.sink.volume, before.sink.muted, before.sink.name
        );
        println!(
            "[smoke] initial source: vol={:.2} mute={} name={:?}",
            before.source.volume, before.source.muted, before.source.name
        );

        let original_source = before.source.volume;

        // Set source volume to 0.35, confirm Data updates.
        println!("[smoke] SetSourceVolume(0.35)…");
        audio.dispatch(AudioCommand::SetSourceVolume(0.35));
        let mut hit = false;
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let s = audio.get();
            if (s.source.volume - 0.35).abs() < 0.02 {
                println!(
                    "[smoke] source now vol={:.2} mute={}",
                    s.source.volume, s.source.muted
                );
                hit = true;
                break;
            }
        }
        assert!(hit, "source volume never reached ~0.35 after SetSourceVolume");

        // Restore.
        println!("[smoke] restoring source volume to {original_source:.2}…");
        audio.dispatch(AudioCommand::SetSourceVolume(original_source));
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let s = audio.get();
            if (s.source.volume - original_source).abs() < 0.02 {
                println!("[smoke] source restored to {:.2}", s.source.volume);
                break;
            }
        }

        // External change: raw `wpctl` subprocess (NOT AudioCommand::dispatch) so
        // the poll loop must pick it up — same path as pavucontrol / second terminal.
        let baseline_sink = audio.get().sink.volume;
        let target_sink = if (baseline_sink - 0.33).abs() < 0.02 {
            0.37
        } else {
            0.33
        };
        println!(
            "[smoke] external wpctl set-volume sink → {target_sink:.2} \
             (baseline={baseline_sink:.2}; bypasses dispatch)"
        );
        let status = std::process::Command::new("wpctl")
            .args([
                "set-volume",
                "@DEFAULT_AUDIO_SINK@",
                &format!("{target_sink:.2}"),
            ])
            .status()?;
        assert!(status.success(), "external wpctl set-volume failed");

        let mut external = false;
        for i in 0..40 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let v = audio.get().sink.volume;
            if (v - target_sink).abs() < 0.02 {
                println!(
                    "[smoke] external change detected at t={}ms: sink {baseline_sink:.2} → {v:.2}",
                    (i + 1) * 100
                );
                external = true;
                break;
            }
        }
        assert!(
            external,
            "poll loop never saw external sink volume change to ~{target_sink}"
        );

        // Restore sink.
        let _ = std::process::Command::new("wpctl")
            .args([
                "set-volume",
                "@DEFAULT_AUDIO_SINK@",
                &format!("{baseline_sink:.4}"),
            ])
            .status();
        // brief settle
        tokio::time::sleep(Duration::from_millis(300)).await;

        println!("\n✅ audio-smoke PASSED");
        Ok(())
    })
}

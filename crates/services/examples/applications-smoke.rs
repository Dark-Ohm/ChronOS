//! Desktop entries smoke test for the applications service.
//!
//! Prints total count + first 5 entries, then verifies inotify hot-reload:
//! creates a temporary .desktop file, waits for the service to detect it,
//! then removes it and verifies the entry disappears.
//!
//! ```sh
//! cargo run -p chronos-services --example applications-smoke
//! ```

use std::time::Duration;

use chronos_services::{ApplicationsSubscriber, Service, ServiceStatus};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let svc = ApplicationsSubscriber::new();

        // Wait for initial scan to complete.
        let mut ready = false;
        for _ in 0..40 {
            if svc.status() == ServiceStatus::Available {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ready, "ApplicationsSubscriber never became Available");

        // Give the inotify watcher thread time to start and register watches.
        tokio::time::sleep(Duration::from_millis(500)).await;

        let state = svc.get();
        println!("[smoke] total entries: {}", state.entries.len());
        for (i, entry) in state.entries.iter().take(5).enumerate() {
            println!(
                "  {}. {} (exec={}, icon={:?}, terminal={})",
                i + 1,
                entry.name,
                entry.exec,
                entry.icon,
                entry.terminal
            );
        }

        // Hot-reload test: create a temp .desktop file in ~/.local/share/applications/.
        let user_dir = std::env::var("XDG_DATA_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join(".local/share")
            })
            .join("applications");
        std::fs::create_dir_all(&user_dir).expect("failed to create applications dir");

        let test_file = user_dir.join("zzz-test-smoke.desktop");
        let minimal = "[Desktop Entry]\nType=Application\nName=ZZZTest\nExec=true\n";
        std::fs::write(&test_file, minimal).expect("failed to write test file");
        println!("[smoke] created {}", test_file.display());

        // Wait for inotify to pick up the new file.
        let mut found = false;
        for i in 0..40 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let s = svc.get();
            if s.entries.iter().any(|e| e.id == "zzz-test-smoke") {
                println!(
                    "[smoke] ZZZTest detected after ~{}ms",
                    (i + 1) * 200
                );
                found = true;
                break;
            }
        }
        assert!(found, "inotify never detected the new .desktop file");

        // Remove the test file.
        std::fs::remove_file(&test_file).expect("failed to remove test file");
        println!("[smoke] removed {}", test_file.display());

        // Wait for inotify to pick up the removal.
        let mut gone = false;
        for i in 0..40 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let s = svc.get();
            if !s.entries.iter().any(|e| e.id == "zzz-test-smoke") {
                println!(
                    "[smoke] ZZZTest removed after ~{}ms",
                    (i + 1) * 200
                );
                gone = true;
                break;
            }
        }
        assert!(gone, "inotify never detected the file removal");

        println!("\n✅ applications-smoke PASSED");
        Ok(())
    })
}

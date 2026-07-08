# Chronos Workspace + Bar MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the Chronos Cargo workspace, prove the gpui-ce path-dependency builds and runs, add single-instance IPC, and get a real layer-shell bar window rendering on every monitor under Hyprland.

**Architecture:** One binary crate, `crates/app`, path-depending directly on the local `gpui-ce-main` checkout (mirrors the working pattern already used by `hermes-gpui-ide/hermes-gpui/Cargo.toml`). No `crates/ui`, `crates/services`, `crates/luau`, or `crates/plugins` yet — those are separate subsystems with their own plans (see `ARCHITECTURE.md` §3). The bar in this plan is visual-only: hardcoded position/size/color, no config system, no widgets, no D-Bus. It exists to prove the windowing + multi-monitor + hot path works before anything is built on top of it.

**Tech Stack:** Rust (edition 2024), `gpui` + `gpui_platform` (gpui-ce, path dependency), `tokio` (single-instance socket only — no services runtime yet), `tracing`.

## Global Constraints

- gpui-ce source: local checkout at `/home/neo/Projects/SOURCE/gpui/gpui-ce-main`, pinned content corresponds to rev `20340e14874a3b55122e5cb2aa0d023874e08b2d` (2026-07-06). Path-dep now; migrate to `git = "...", rev = "..."` once `gpui-ce-main` is a git repo (ARCHITECTURE.md §2). Do not add `crates.io` or `zed/main` as a gpui source.
- `panic = "unwind"` in `[profile.release]` — never `"abort"` (DECISIONS.log, 2026-07-08, "Panic strategy").
- Multi-monitor bar: open one layer-shell window per `cx.displays()` entry, passing `display_id` into `WindowOptions` (ARCHITECTURE.md §4). This is what closes Zed issue #48501 — do not special-case "primary display only".
- Single-instance via Unix socket at `$XDG_RUNTIME_DIR/chronos.sock`, falling back to `/tmp` (ARCHITECTURE.md §4, adapted from `gpui-shell`'s `ipc/service.rs`).
- Out of scope for this plan (separate future plans): `crates/ui` fork, `crates/services` D-Bus/compositor integrations, `crates/luau` + `crates/plugins`, config file + hot-reload, runtime widget registry, launcher/dock/notifications/osd modules. Do not build any of these here even if it looks convenient — YAGNI.

---

## File Structure

```
Cargo.toml                    # workspace root, gpui-ce path-deps, release profile
crates/
  app/
    Cargo.toml
    src/
      main.rs                 # tokio entrypoint, wires IPC + GPUI app + bar
      ipc/
        mod.rs                 # gpui-aware glue: IpcSubscriber::start(cx)
        service.rs              # gpui-agnostic: socket lifecycle, accept loop
        messages.rs              # gpui-agnostic: ping payload encode/decode
      bar.rs                   # layer-shell window_options() + init()
```

Each file's responsibility:
- `messages.rs` — pure payload (de)serialization, zero I/O, fully unit-testable.
- `service.rs` — socket path resolution + accept/bind logic, testable against a temp path without touching the real runtime socket.
- `ipc/mod.rs` — the only file that touches `gpui::App`; bridges the tokio receiver into a GPUI-spawned future.
- `bar.rs` — window geometry + GPUI `Render` impl, no IPC/service knowledge.

---

### Task 1: Workspace scaffold + gpui-ce smoke test

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/app/Cargo.toml`
- Create: `crates/app/src/main.rs`

**Interfaces:**
- Produces: a runnable `chronos` binary that starts and stops cleanly. Later tasks add `mod ipc;` and `mod bar;` to this file and extend the `app.run(...)` closure.

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
members = ["crates/app"]
resolver = "3"

[workspace.dependencies]
gpui = { path = "/home/neo/Projects/SOURCE/gpui/gpui-ce-main/crates/gpui" }
gpui_platform = { path = "/home/neo/Projects/SOURCE/gpui/gpui-ce-main/crates/gpui_platform" }
anyhow = "1.0.100"
tokio = { version = "1.44.1", features = ["rt-multi-thread", "macros", "time", "net"] }
tracing = { version = "0.1.41", features = ["log"] }
tracing-subscriber = { version = "0.3.19", features = ["env-filter"] }

[profile.release]
codegen-units = 1
lto = true
panic = "unwind"
opt-level = "z"
strip = true
debug = false
debug-assertions = false
overflow-checks = false
```

- [ ] **Step 2: Create `crates/app/Cargo.toml`**

```toml
[package]
name = "chronos"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "chronos"
path = "src/main.rs"

[dependencies]
gpui.workspace = true
gpui_platform.workspace = true
anyhow.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

- [ ] **Step 3: Create `crates/app/src/main.rs`**

```rust
use gpui_platform::application;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Chronos starting");

    let app = application();
    app.run(|cx| {
        tracing::info!("GPUI application context ready");
        cx.quit();
    });

    tracing::info!("Chronos exited");
}
```

- [ ] **Step 4: Build and verify the gpui-ce path-dependency resolves**

Run: `cargo build --manifest-path Cargo.toml`
Expected: compiles successfully (first build pulls in `gpui`/`gpui_platform` from the local `SOURCE/gpui/gpui-ce-main` checkout — this will take a while, it's a large crate graph). No "believes it's in a workspace" or path-resolution errors.

If the path is wrong, `cargo build` fails immediately with `failed to load source for dependency 'gpui'` — the path is absolute (`/home/neo/Projects/SOURCE/gpui/gpui-ce-main/crates/gpui`) specifically so it resolves the same whether the workspace root is `/home/neo/Projects/chronos` or a worktree checkout under `.claude/worktrees/...`. Do not change it to a relative path.

- [ ] **Step 5: Run and verify clean startup/shutdown**

Run: `RUST_LOG=info cargo run --manifest-path Cargo.toml`
Expected output (in order), exit code 0:
```
Chronos starting
GPUI application context ready
Chronos exited
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/app/Cargo.toml crates/app/src/main.rs
git commit -m "feat: scaffold Chronos workspace with gpui-ce path-dependency"
```

---

### Task 2: IPC payload + socket-path logic (unit tested)

**Files:**
- Create: `crates/app/src/ipc/messages.rs`
- Create: `crates/app/src/ipc/service.rs`
- Modify: `crates/app/src/main.rs` (add `mod ipc;` at the bottom of this task, not before — the module isn't wired into `main()` until Task 3)

**Interfaces:**
- Produces (used by Task 3): `service::IpcSubscriber::init() -> Option<IpcSubscriber>`, `IpcSubscriber::start_listener(&mut self) -> mpsc::UnboundedReceiver<()>`, `messages::encode_ping() -> String`, `messages::is_ping(&str) -> bool`.

- [ ] **Step 1: Write the failing tests for `messages.rs`**

```rust
// crates/app/src/ipc/messages.rs
pub const PING_PAYLOAD: &str = "ping";

pub fn encode_ping() -> String {
    PING_PAYLOAD.to_string()
}

pub fn is_ping(payload: &str) -> bool {
    payload.trim() == PING_PAYLOAD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_recognizes_ping() {
        let payload = encode_ping();
        assert!(is_ping(&payload));
    }

    #[test]
    fn rejects_non_ping_payload() {
        assert!(!is_ping("not-a-ping"));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert!(is_ping("  ping\n"));
    }
}
```

This file has no `mod ipc;` wiring yet, so it can't be reached by `cargo test` until `main.rs` declares the module. Do this step and Step 2 together, then wire the module in Step 3.

- [ ] **Step 2: Write `service.rs` with its own unit tests**

```rust
// crates/app/src/ipc/service.rs
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use tokio::net::UnixListener as TokioUnixListener;
use tokio::sync::mpsc;

use super::messages::{encode_ping, is_ping};

pub type IpcReceiver = mpsc::UnboundedReceiver<()>;

pub enum AcquireResult {
    Primary(IpcSubscriber),
    Secondary,
    Error(String),
}

/// Owns the bound-but-not-yet-accepting Unix socket for the primary instance.
pub struct IpcSubscriber {
    listener: Option<TokioUnixListener>,
    socket_path: PathBuf,
}

impl IpcSubscriber {
    /// Returns `Some` when this process should continue as the primary
    /// instance, `None` when an existing instance was signaled instead.
    pub fn init() -> Option<IpcSubscriber> {
        match acquire_at(&socket_path(), &encode_ping()) {
            AcquireResult::Primary(subscriber) => Some(subscriber),
            AcquireResult::Secondary => None,
            AcquireResult::Error(err) => {
                tracing::error!("IPC service error: {}", err);
                None
            }
        }
    }

    /// Starts the accept loop. Must be called from within a tokio runtime.
    /// Returns a receiver that yields `()` once per received ping.
    pub fn start_listener(&mut self) -> IpcReceiver {
        let (sender, receiver) = mpsc::unbounded_channel();

        if let Some(listener) = self.listener.take() {
            tokio::spawn(async move {
                accept_loop(listener, sender).await;
            });
        }

        receiver
    }
}

impl Drop for IpcSubscriber {
    fn drop(&mut self) {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

/// Try to become the primary instance at `path`, or signal an existing one.
///
/// Synchronous on the secondary path (no runtime needed to signal). When
/// becoming primary, the returned `IpcSubscriber` still needs a tokio
/// runtime active before `start_listener` is called.
pub fn acquire_at(path: &Path, payload: &str) -> AcquireResult {
    if let Ok(mut stream) = UnixStream::connect(path) {
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));

        if let Err(e) = stream.write_all(payload.as_bytes()) {
            return AcquireResult::Error(format!("Failed to signal existing instance: {}", e));
        }
        let _ = stream.flush();
        let _ = stream.shutdown(std::net::Shutdown::Write);

        return AcquireResult::Secondary;
    }

    if path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            return AcquireResult::Error(format!("Failed to remove stale socket: {}", e));
        }
    }

    let listener = match UnixListener::bind(path) {
        Ok(l) => l,
        Err(e) => return AcquireResult::Error(format!("Failed to create socket: {}", e)),
    };

    if let Err(e) = listener.set_nonblocking(true) {
        return AcquireResult::Error(format!("Failed to configure socket: {}", e));
    }

    let tokio_listener = match TokioUnixListener::from_std(listener) {
        Ok(l) => l,
        Err(e) => return AcquireResult::Error(format!("Failed to create async listener: {}", e)),
    };

    AcquireResult::Primary(IpcSubscriber {
        listener: Some(tokio_listener),
        socket_path: path.to_path_buf(),
    })
}

pub fn socket_path_in(runtime_dir: Option<&str>) -> PathBuf {
    match runtime_dir {
        Some(dir) => PathBuf::from(dir).join("chronos.sock"),
        None => PathBuf::from("/tmp").join(format!("chronos-{}.sock", std::process::id())),
    }
}

pub fn socket_path() -> PathBuf {
    socket_path_in(std::env::var("XDG_RUNTIME_DIR").ok().as_deref())
}

async fn accept_loop(listener: TokioUnixListener, sender: mpsc::UnboundedSender<()>) {
    use tokio::io::AsyncReadExt;

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let sender = sender.clone();
                tokio::spawn(async move {
                    let mut buffer = Vec::with_capacity(16);
                    let read = tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        stream.read_to_end(&mut buffer),
                    )
                    .await;

                    if let Ok(Ok(_)) = read {
                        let payload = String::from_utf8_lossy(&buffer).to_string();
                        if is_ping(&payload) {
                            let _ = sender.send(());
                        }
                    }
                });
            }
            Err(e) => {
                tracing::error!("Failed to accept connection: {}", e);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_xdg_runtime_dir_when_set() {
        let path = socket_path_in(Some("/run/user/1000"));
        assert_eq!(path, PathBuf::from("/run/user/1000/chronos.sock"));
    }

    #[test]
    fn falls_back_to_tmp_when_unset() {
        let path = socket_path_in(None);
        assert!(path.starts_with("/tmp"));
        assert!(path.to_string_lossy().contains("chronos-"));
    }

    #[tokio::test]
    async fn second_acquire_on_same_path_becomes_secondary() {
        let dir = std::env::temp_dir().join(format!("chronos-ipc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.sock");
        let _ = std::fs::remove_file(&path);

        let first = acquire_at(&path, "ping");
        assert!(matches!(&first, AcquireResult::Primary(_)));

        let second = acquire_at(&path, "ping");
        assert!(matches!(&second, AcquireResult::Secondary));

        drop(first);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
```

- [ ] **Step 3: Wire the module and run the tests**

Add to `crates/app/src/main.rs`, right after the imports:

```rust
mod ipc;
```

Create `crates/app/src/ipc/mod.rs` with just the two submodules for now (Task 3 adds the `App`-aware glue):

```rust
mod messages;
mod service;
```

Run: `cargo test --manifest-path crates/app/Cargo.toml`
Expected: 6 tests pass —
```
test ipc::messages::tests::encodes_and_recognizes_ping ... ok
test ipc::messages::tests::rejects_non_ping_payload ... ok
test ipc::messages::tests::trims_surrounding_whitespace ... ok
test ipc::service::tests::prefers_xdg_runtime_dir_when_set ... ok
test ipc::service::tests::falls_back_to_tmp_when_unset ... ok
test ipc::service::tests::second_acquire_on_same_path_becomes_secondary ... ok
```

`main.rs` now has an unused `mod ipc;` (nothing calls `IpcSubscriber::init()` yet) — expect a `dead_code` warning from `cargo build`, not an error. That's resolved in Task 3.

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/ipc crates/app/src/main.rs
git commit -m "feat: add single-instance IPC socket logic with unit tests"
```

---

### Task 3: Wire IPC into main, verify single-instance behavior

**Files:**
- Create: `crates/app/src/ipc/mod.rs` → replace content from Task 3 Step 3 (adds `App`-aware `start`)
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `ipc::IpcSubscriber` (from Task 2), `gpui::App` (from `gpui` crate).
- Produces: a running process that (a) as primary, logs each ping it receives, (b) as secondary, signals the primary and exits within ~1s without opening any window.

- [ ] **Step 1: Add the `App`-aware glue to `crates/app/src/ipc/mod.rs`**

```rust
mod messages;
mod service;

use gpui::App;

pub use service::IpcSubscriber;

impl IpcSubscriber {
    /// Starts listening for pings and logs each one. Keeps `self` alive for
    /// the lifetime of the listener so the socket file isn't removed early.
    pub fn start(mut self, cx: &mut App) {
        let mut receiver = self.start_listener();

        cx.spawn(async move |cx| {
            let _ipc_guard = self;
            tracing::info!("IPC listener started");

            while receiver.recv().await.is_some() {
                let _ = cx.update(|_cx| {
                    tracing::info!("Received ping from a secondary instance");
                });
            }

            tracing::warn!("IPC listener ended unexpectedly");
        })
        .detach();
    }
}
```

- [ ] **Step 2: Wire it into `main.rs`**

Replace `crates/app/src/main.rs` with:

```rust
mod ipc;

use gpui_platform::application;
use ipc::IpcSubscriber;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Chronos starting");

    let Some(subscriber) = IpcSubscriber::init() else {
        tracing::info!("Another Chronos instance is running, signaled it and exiting");
        return;
    };

    let app = application();
    app.run(move |cx| {
        tracing::info!("GPUI application context ready");
        subscriber.start(cx);
    });

    tracing::info!("Chronos exited");
}
```

Note this removes the `cx.quit()` smoke test from Task 1 — from here on the process is meant to stay running like a real shell, so it needs to be killed explicitly in verification.

- [ ] **Step 3: Build**

Run: `cargo build --manifest-path Cargo.toml`
Expected: compiles with no warnings about unused `ipc` items.

- [ ] **Step 4: Manually verify single-instance behavior**

```bash
RUST_LOG=info cargo run --manifest-path Cargo.toml > /tmp/chronos-primary.log 2>&1 &
PRIMARY_PID=$!
sleep 2
grep -q "GPUI application context ready" /tmp/chronos-primary.log && echo "PRIMARY_OK"

RUST_LOG=info timeout 3 cargo run --manifest-path Cargo.toml > /tmp/chronos-secondary.log 2>&1
grep -q "Another Chronos instance is running" /tmp/chronos-secondary.log && echo "SECONDARY_OK"

sleep 1
grep -q "Received ping from a secondary instance" /tmp/chronos-primary.log && echo "PING_RECEIVED_OK"

kill "$PRIMARY_PID"
```

Expected: `PRIMARY_OK`, `SECONDARY_OK`, and `PING_RECEIVED_OK` all printed. The secondary `cargo run` must return well under the 3s timeout (it exits as soon as it signals, it doesn't wait for `timeout` to kill it).

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/ipc/mod.rs crates/app/src/main.rs
git commit -m "feat: wire single-instance IPC into Chronos entrypoint"
```

---

### Task 4: Bar layer-shell window

**Files:**
- Create: `crates/app/src/bar.rs`

**Interfaces:**
- Produces (used by Task 5): `bar::init(cx: &mut App)` — spawns bar windows on every currently-known display, 100ms after startup (mirrors `gpui-shell`'s `bar::init`, ARCHITECTURE.md §4).
- Consumes: nothing from earlier tasks — this module is IPC-agnostic.

- [ ] **Step 1: Write `crates/app/src/bar.rs`**

```rust
use std::time::Duration;

use gpui::{
    App, Bounds, Context, DisplayId, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rgb,
};

const BAR_HEIGHT: f32 = 32.0;
const BAR_COLOR: u32 = 0x1e1e2e;

struct Bar;

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(BAR_COLOR))
    }
}

/// Returns window options for a top-anchored bar on the given display.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(display_size.width, px(BAR_HEIGHT)),
        })),
        app_id: Some("chronos-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
            layer: Layer::Top,
            anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
            exclusive_zone: Some(px(BAR_HEIGHT)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn open_on_display(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |_, cx| cx.new(|_| Bar)) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open bar window: {}", err);
            false
        }
    }
}

/// Opens one bar window per display. Called once at startup.
pub fn init(cx: &mut App) {
    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        let _ = cx.update(|cx: &mut App| {
            let displays = cx.displays();
            if displays.is_empty() {
                tracing::info!("No displays found, opening bar on default display");
                open_on_display(None, cx);
            } else {
                tracing::info!("Opening bar on {} displays", displays.len());
                for d in displays {
                    open_on_display(Some(d.id()), cx);
                }
            }
        });
    })
    .detach();
}
```

- [ ] **Step 2: Build**

Run: `cargo build --manifest-path Cargo.toml`
Expected: compiles. `bar` is unused from `main.rs`'s perspective until Task 5 — expect a `dead_code` warning, not an error.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/bar.rs
git commit -m "feat: add layer-shell bar window module"
```

---

### Task 5: Wire the bar into main, verify multi-monitor rendering

**Files:**
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `bar::init` (Task 4), `ipc::IpcSubscriber` (Task 3).

- [ ] **Step 1: Wire `bar::init` into `main.rs`**

```rust
mod bar;
mod ipc;

use gpui_platform::application;
use ipc::IpcSubscriber;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Chronos starting");

    let Some(subscriber) = IpcSubscriber::init() else {
        tracing::info!("Another Chronos instance is running, signaled it and exiting");
        return;
    };

    let app = application();
    app.run(move |cx| {
        tracing::info!("GPUI application context ready");
        subscriber.start(cx);
        bar::init(cx);
    });

    tracing::info!("Chronos exited");
}
```

- [ ] **Step 2: Build**

Run: `cargo build --manifest-path Cargo.toml`
Expected: compiles with no `dead_code` warnings left.

- [ ] **Step 3: Manually verify the bar renders on every monitor**

```bash
RUST_LOG=info cargo run --manifest-path Cargo.toml > /tmp/chronos-bar.log 2>&1 &
CHRONOS_PID=$!
sleep 2

grep -c "Opening bar on display" /tmp/chronos-bar.log
hyprctl monitors -j | jq 'length'
hyprctl layers -j | jq '[.[] | .levels."2"[]? | select(.namespace == "bar")] | length'
```

Expected: the `hyprctl layers` count of `bar`-namespace layer surfaces equals the `hyprctl monitors` count (one bar per monitor). Visually: a solid dark bar (`#1e1e2e`) 32px tall pinned to the top edge of every monitor, with desktop windows/content shifted down by 32px (proves `exclusive_zone` is honored, not just an overlay).

- [ ] **Step 4: Clean up and commit**

```bash
kill "$CHRONOS_PID"
git add crates/app/src/main.rs
git commit -m "feat: render Chronos bar on every monitor at startup"
```

---

## What This Plan Does Not Cover

Deliberately deferred to their own plans, per `ARCHITECTURE.md`:
- `crates/services` (D-Bus, Hyprland/Niri IPC, `trait Service`, tokio services runtime thread) — §7, §10.
- `crates/ui` fork of `gpui-component` + theme system — §2 decision, §11.
- `crates/luau` + `crates/plugins` sandboxed LuaU runtime and capability gating — §5.
- Runtime module registry (`HashMap<String, Box<dyn BarWidget>>`, `chronos.bar:register(...)`) — §6.
- Config file (TOML) + hot-reload (`inotify`, `cx.refresh_windows()`) — §9. The bar's position/size/color are hardcoded constants in this plan on purpose.
- `dock` / `launcher` / `notifications` / `osd` modules.

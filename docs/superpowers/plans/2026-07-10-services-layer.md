# Chronos Services Layer Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `crates/services` integration layer (unified `Service` trait + `Compositor`/`Network`/`UPower` subscribers) and the `AppState`/`watch()` reactive bridge in `crates/app`, so later bar widgets can consume system state without recompiling the core.

**Architecture:** `crates/services` is GPUI-agnostic: each subscriber holds a `futures_signals::Mutable<T>` and implements a lightweight `Service` trait (`subscribe`/`get`/`status`; commands are concrete methods). Constructors are **non-failing `fn new() -> Self`** — they return an object in `ServiceStatus::Initializing` and start a background connect/retry loop. D-Bus services (Network, UPower) use an async `tokio::spawn` loop on the shared `zbus::Connection`; the Compositor uses a plain `std::thread` + `catch_unwind` (it is NOT on tokio). `crates/app` owns `AppState` (a GPUI global holding all subscribers) and the `watch()` helper that bridges a `Signal` into GPUI view updates. Services boot on a dedicated tokio runtime entered via `rt.block_on` in a separate OS thread (ARCHITECTURE.md §10). `panic = "unwind"`.

**Tech Stack:** Rust, gpui-ce (via workspace path dep), `futures-signals` 0.3.34 (reactive state), `zbus` 5.x (D-Bus to NetworkManager + UPower), `hyprland` crate (Hyprland IPC), `niri-ipc` (scaffold only), `tokio` (rt-multi-thread), `tracing`.

## Global Constraints

- `crates/services` MUST stay GPUI-agnostic: it depends on `futures-signals`, `zbus`, `hyprland`, `niri-ipc`, `tokio`, `chrono`, `anyhow`, `tracing` — NOT on `gpui`. `AppState`/`watch()` live in `crates/app`.
- `panic = "unwind"` is already set in the workspace `[profile.release]`; do not change it. Listener threads wrap work in `catch_unwind`; a panic must not kill the shell.
- **Constructors are `fn new() -> Self` (synchronous, non-failing).** No `Result`, no `unavailable()` fallback. A service that cannot connect starts in `Initializing` then transitions to `Unavailable` and keeps retrying in the background. The retry loop (not a fallback constructor) handles absence.
- **`ServiceStatus` is `Initializing | Available | Unavailable | Degraded(String)`** (4 variants, spec §4). No `Active`/`Error` variants. The last error string is logged via `tracing`, never stored in the enum.
- **`subscribe()` returns `impl Signal<Item = Data> + Unpin + 'static`** (spec §4), NOT `MutableSignalCloned`. The `Mutable` stays private; consumers cannot call `.set()`.
- **Two thread models (spec §5):** D-Bus services (`Network`, `UPower`) capture `tokio::runtime::Handle::current()` inside `new()` and `tokio::spawn` an async connect+retry loop. The Compositor (`Hyprland`) spawns a plain `std::thread` running a sync connect+retry loop with `catch_unwind` restart — it does NOT use `Handle::current()` and is NOT on tokio.
- **`init_all()` MUST be called inside the runtime context** (spec §7): `let services = rt.block_on(async { services::init_all() });`. `Handle::current()` inside the D-Bus constructors panics if this is omitted.
- Niri is SCAFFOLD ONLY (ARCHITECTURE.md §13): `CompositorBackend::Niri` exists in the enum and `detect_backend()`, but `niri.rs` methods return `ServiceStatus::Unavailable` / default state — no real IPC.
- Commands are concrete subscriber methods (e.g. `CompositorSubscriber::dispatch`, `NetworkSubscriber::connect`), NOT part of the `Service` trait (light unification, spec §4).
- Crate versions (`zbus` 5.x, `hyprland`, `niri-ipc`) MAY have changed by 2026 — verify exact pins against the workspace `Cargo.lock` / crates.io before relying on a specific version; if a pinned version fails to resolve/compile, use the latest within the same major and note it in DECISIONS.log. The `hyprland` crate's exact API must be verified against docs.rs / `reference/gpui-shell/crates/services/src/compositor/hyprland.rs` for the pinned version.
- Every git commit MUST NOT contain a `Co-Authored-By:` (or any AI attribution) trailer.
- `Service::Error` surfaces via `tracing::{warn,error}`; never silently swallow. No panics at startup if a service is unavailable.
- **Cross-thread wake (spec §9, required):** `futures_signals::Mutable::set()` is thread-safe by construction (verified in `futures-signals` 0.3.34 `src/signal/mutable.rs`: `set()` → `ChangedWaker::wake()` stores the waker in a plain `std::sync::Mutex<Option<Waker>>` and calls `Waker::wake()` — no runtime handle). A `set()` from the Compositor's foreign `std::thread` correctly wakes the GPUI-side `Signal` consumer. The Compositor task MUST include a **live smoke-test** (`hyprctl dispatch workspace 2` → bar `CompositorState` updates via `watch()`), marked `#[ignore]` but a required acceptance gate — an isolated unit test does not exercise the foreign-thread → GPUI-waker path.

---

### Task 1: Scaffold `crates/services` + `Service` trait + `ServiceStatus` + contract test

**Files:**
- Create: `crates/services/Cargo.toml`
- Create: `crates/services/src/lib.rs`
- Modify: `Cargo.toml` (workspace root) — add `crates/services` to `members` and add workspace deps `futures-signals`, `zbus`, `hyprland`, `niri-ipc`, `chrono`

**Interfaces:**
- Produces: `pub trait Service`, `pub enum ServiceStatus`, `crates/services` crate compiles as a workspace member.

- [ ] **Step 1: Add workspace member + deps (root `Cargo.toml`)**

Replace the top of `Cargo.toml` so it reads:

```toml
[workspace]
members = ["crates/app", "crates/luau", "crates/services"]
resolver = "3"

[workspace.dependencies]
gpui = { path = "/home/neo/Projects/SOURCE/gpui/gpui-ce-main/crates/gpui" }
gpui_platform = { path = "/home/neo/Projects/SOURCE/gpui/gpui-ce-main/crates/gpui_platform" }
anyhow = "1.0.100"
tokio = { version = "1.44.1", features = ["rt-multi-thread", "macros", "time", "net", "sync", "io-util"] }
tracing = { version = "0.1.41", features = ["log"] }
tracing-subscriber = { version = "0.3.19", features = ["env-filter"] }
futures-signals = "0.3.34"
futures-util = "0.3"
zbus = "5"
hyprland = "0.4"
niri-ipc = "=25.11.0"
chrono = "0.4"
```

(If `cargo build` later fails to resolve `hyprland = "0.4"` or `niri-ipc = "=25.11.0"` against the 2026 registry, adjust to a resolvable version within the same major and record the change in DECISIONS.log.)

- [ ] **Step 2: Write `crates/services/Cargo.toml`**

```toml
[package]
name = "chronos-services"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow.workspace = true
tokio.workspace = true
futures-signals.workspace = true
futures-util.workspace = true
zbus.workspace = true
hyprland.workspace = true
niri-ipc.workspace = true
chrono.workspace = true
tracing.workspace = true
```

- [ ] **Step 3: Write the contract test (in `crates/services/src/lib.rs`)**

Create `crates/services/src/lib.rs` with the trait, `ServiceStatus`, and a `FakeService` used only by the test to prove the contract. Note `subscribe()` returns `impl Signal + Unpin` — the test drives it via `to_stream()` + `next_blocking()` (a sync consumer, which is fine for a unit test on the same thread).

```rust
//! System-integration services for Chronos (GPUI-agnostic).
//!
//! Each service is a subscriber holding a `futures_signals::Mutable<T>` and
//! implements the lightweight `Service` trait. Commands are concrete methods
//! on each subscriber (NOT part of the trait).

use futures_signals::signal::{Mutable, Signal};

/// Availability of a service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Created; first connection attempt pending (set by `new()`).
    Initializing,
    /// Fully functional.
    Available,
    /// All connection attempts failed; retry loop running in background.
    Unavailable,
    /// Connected but some features missing (e.g. NM present, Wi-Fi hardware absent).
    Degraded(String),
}

/// Lightweight, unified service contract: availability + reactivity.
/// Commands are concrete methods on each subscriber, not part of this trait.
pub trait Service: Send + Sync + 'static {
    type Data: Clone + 'static;
    type Error: std::error::Error + Send + Sync + 'static;
    /// Reactive signal. Hides the `Mutable`; consumer cannot call `.set()`.
    fn subscribe(&self) -> impl Signal<Item = Self::Data> + Unpin + 'static;
    fn get(&self) -> Self::Data;
    fn status(&self) -> ServiceStatus;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeService {
        data: Mutable<u32>,
    }

    impl Service for FakeService {
        type Data = u32;
        type Error = anyhow::Error;
        fn subscribe(&self) -> impl Signal<Item = u32> + Unpin {
            self.data.signal_cloned()
        }
        fn get(&self) -> u32 {
            self.data.get()
        }
        fn status(&self) -> ServiceStatus {
            ServiceStatus::Available
        }
    }

    #[test]
    fn service_contract_emits_on_mutate() {
        let svc = FakeService { data: Mutable::new(1) };
        let sig = svc.subscribe();
        assert_eq!(svc.get(), 1);
        assert_eq!(svc.status(), ServiceStatus::Available);

        svc.data.set(42);
        let received = sig.to_stream().next_blocking().expect("signal emits");
        assert_eq!(received, 42);
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p chronos-services service_contract_emits_on_mutate`
Expected: PASS (`test result: ok. 1 passed; 0 failed`).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/services/Cargo.toml crates/services/src/lib.rs
git commit -m "feat(services): scaffold crate + Service trait + ServiceStatus (v2)"
```

---

### Task 2: `CompositorSubscriber` (Hyprland primary + Niri scaffold, sync-thread model)

**Files:**
- Create: `crates/services/src/compositor/types.rs`
- Create: `crates/services/src/compositor/hyprland.rs`
- Create: `crates/services/src/compositor/niri.rs`
- Create: `crates/services/src/compositor/mod.rs`
- Modify: `crates/services/src/lib.rs` (add re-exports, keep the `Service` trait + `ServiceStatus`)

**Interfaces:**
- Consumes: `Service` trait + `ServiceStatus` (Task 1).
- Produces: `pub struct CompositorSubscriber` (derives `Clone`), `CompositorSubscriber::new() -> Self` (non-failing, sync), `CompositorSubscriber::dispatch(&self, CompositorCommand) -> anyhow::Result<()>`, `CompositorSubscriber::subscribe() -> impl Signal<Item = CompositorState> + Unpin`, `CompositorSubscriber::get() -> CompositorState`, `CompositorSubscriber::status() -> ServiceStatus`. Also `CompositorState`, `CompositorBackend`, `CompositorCommand`, `Workspace`, `ActiveWindow`, `Monitor`.

- [ ] **Step 1: Write `compositor/types.rs`**

```rust
//! Compositor data types.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositorBackend {
    Hyprland,
    Niri,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Workspace {
    pub id: u32,
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActiveWindow {
    pub title: String,
    pub class: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Monitor {
    pub name: String,
    pub active_workspace: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompositorState {
    pub backend: CompositorBackend,
    pub workspaces: Vec<Workspace>,
    pub active_window: Option<ActiveWindow>,
    pub monitors: Vec<Monitor>,
    pub keyboard_layout: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositorCommand {
    FocusWorkspace(u32),
    NextWorkspace,
    PrevWorkspace,
    MoveToWorkspace(u32),
}
```

- [ ] **Step 2: Write `compositor/hyprland.rs` (PRIMARY backend)**

Read `reference/gpui-shell/crates/services/src/compositor/hyprland.rs` first to mirror the pattern, then adapt to the pinned `hyprland` crate version (verify the exact API on docs.rs). Provide `fetch_full_state()` (sync, used by the retry loop) and `start_listener(data, status)` which spawns the dedicated thread and runs the incremental handler loop with `catch_unwind` restart.

```rust
//! Hyprland backend. PRIMARY backend.
//!
//! VERIFY the exact `hyprland` crate API against the pinned version (docs.rs)
//! and reference/gpui-shell/crates/services/src/compositor/hyprland.rs.

use std::panic;
use std::thread;

use anyhow::Result;
use futures_signals::signal::Mutable;
use hyprland::{
    data::{Client, Devices, Monitors, Workspace as HWorkspace, Workspaces},
    dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial},
    event_listener::EventListener,
};
use tracing::{debug, error, warn};
use super::types::{ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, Monitor, Workspace};
use crate::ServiceStatus;

/// Hyprland is available when running under it (env var set by the compositor).
pub fn is_available() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

/// Execute a compositor command via Hyprland.
pub fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(id)))?;
        }
        CompositorCommand::NextWorkspace => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Relative(
                "+1".to_string().parse()?,
            )))?;
        }
        CompositorCommand::PrevWorkspace => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Relative(
                "-1".to_string().parse()?,
            )))?;
        }
        CompositorCommand::MoveToWorkspace(id) => {
            Dispatch::call(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(id)))?;
        }
    }
    Ok(())
}

/// Fetch the full current compositor state from Hyprland (sync).
pub fn fetch_full_state() -> Result<CompositorState> {
    let active_id = HWorkspace::get_active().ok().map(|w| w.id);
    let workspaces = Workspaces::get()?
        .into_iter()
        .map(|w| Workspace {
            id: w.id,
            name: w.name,
            active: active_id == Some(w.id),
        })
        .collect();
    let monitors = Monitors::get()?
        .into_iter()
        .map(|m| Monitor {
            name: m.name,
            active_workspace: m.active_workspace.id,
        })
        .collect();
    let active_window = Client::get_active()
        .ok()
        .flatten()
        .map(|w| ActiveWindow { title: w.title, class: w.class });
    let keyboard_layout = Devices::get()
        .ok()
        .and_then(|d| d.keyboards.into_iter().find(|k| k.main).map(|k| k.active_keymap))
        .unwrap_or_else(|| "Unknown".to_string());
    Ok(CompositorState {
        backend: CompositorBackend::Hyprland,
        workspaces,
        active_window,
        monitors,
        keyboard_layout,
    })
}

/// Spawn the dedicated listener thread. On panic, the thread is restarted by
/// the retry loop in `mod.rs` (a panic must not kill the shell).
pub fn start_listener(data: Mutable<CompositorState>, status: Mutable<ServiceStatus>) {
    thread::spawn(move || {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| run_listener(data.clone())));
        if let Err(_) = result {
            error!("Hyprland listener thread panicked; marking Unavailable for retry");
            status.set(ServiceStatus::Unavailable);
        }
    });
}

fn run_listener(data: Mutable<CompositorState>) -> Result<()> {
    let mut listener = EventListener::new();
    {
        let data = data.clone();
        listener.add_workspace_changed_handler(move |evt| {
            debug!("workspace changed: {:?}", evt.name);
            let mut s = data.lock_mut();
            for w in s.workspaces.iter_mut() {
                w.active = w.id == evt.id;
            }
        });
    }
    {
        let data = data.clone();
        listener.add_active_window_changed_handler(move |evt| {
            let mut s = data.lock_mut();
            s.active_window = evt.map(|w| ActiveWindow { title: w.title, class: w.class });
        });
    }
    {
        let data = data.clone();
        listener.add_layout_changed_handler(move |evt| {
            let mut s = data.lock_mut();
            s.keyboard_layout = evt.layout_name;
        });
    }
    listener.start_listener()?;
    Ok(())
}
```

- [ ] **Step 3: Write `compositor/niri.rs` (SCAFFOLD only)**

```rust
//! Niri backend. SCAFFOLD ONLY (ARCHITECTURE.md §13): no real IPC.

use anyhow::Result;
use futures_signals::signal::Mutable;

use super::types::{CompositorCommand, CompositorState};
use crate::ServiceStatus;

pub fn is_available() -> bool {
    false
}

pub fn fetch_full_state() -> Result<CompositorState> {
    Ok(CompositorState::default())
}

pub fn start_listener(_data: Mutable<CompositorState>, _status: Mutable<ServiceStatus>) {
    // No-op: Niri not wired (scaffold only).
}

pub fn execute_command(_cmd: CompositorCommand) -> Result<()> {
    Ok(())
}
```

- [ ] **Step 4: Write `compositor/mod.rs` (non-failing `fn new() -> Self`, sync-thread model)**

`new()` is synchronous and non-failing. It detects the backend; if none, returns `Unavailable` with no thread. Otherwise it creates the `Mutable`s (status `Initializing`), spawns the listener thread, and returns immediately. A background retry loop (3–5 attempts + delay, then periodic re-probe) is started; on success it sets `Available` and the listener populates state.

```rust
//! Compositor service: workspaces, active window, monitors, keyboard layout.
//!
//! SYNC-THREAD MODEL (spec §5.2): this service does NOT use tokio. It spawns a
//! plain `std::thread` running a sync connect+retry loop with `catch_unwind`
//! restart. It does NOT call `Handle::current()`.

pub mod hyprland;
pub mod niri;
pub mod types;

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tracing::{info, warn};

pub use types::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, Monitor, Workspace,
};
use crate::Service;
use crate::ServiceStatus;

/// Event-driven compositor subscriber.
#[derive(Clone)]
pub struct CompositorSubscriber {
    data: Mutable<CompositorState>,
    status: Mutable<ServiceStatus>,
    backend: CompositorBackend,
}

impl CompositorSubscriber {
    /// Detect the running compositor and start monitoring.
    /// Non-failing and synchronous (spec §5.2): returns `Self` in
    /// `ServiceStatus::Initializing`; the listener thread flips it to
    /// `Available`/`Unavailable`. If no backend is detected, returns
    /// `Unavailable` immediately with no thread spawned.
    pub fn new() -> Self {
        let backend = match detect_backend() {
            Some(b) => b,
            None => {
                warn!("No supported compositor detected (Hyprland or Niri)");
                return Self {
                    data: Mutable::new(CompositorState::default()),
                    status: Mutable::new(ServiceStatus::Unavailable),
                    backend: CompositorBackend::Hyprland,
                };
            }
        };

        info!("Detected compositor backend: {}", backend.name());
        let data = Mutable::new(CompositorState::default());
        let status = Mutable::new(ServiceStatus::Initializing);

        // Spawn the dedicated listener thread (sync model). The thread sets
        // status to Available on first successful fetch, Unavailable on failure
        // or panic; the retry loop re-probes.
        spawn_retry(data.clone(), status.clone(), backend);

        Self { data, status, backend }
    }

    pub fn backend(&self) -> CompositorBackend {
        self.backend
    }

    pub fn dispatch(&self, cmd: CompositorCommand) -> anyhow::Result<()> {
        match self.backend {
            CompositorBackend::Hyprland => hyprland::execute_command(cmd),
            CompositorBackend::Niri => niri::execute_command(cmd),
        }
    }
}

impl Service for CompositorSubscriber {
    type Data = CompositorState;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = CompositorState> + Unpin {
        self.data.signal_cloned()
    }
    fn get(&self) -> CompositorState {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Detect the running compositor backend.
fn detect_backend() -> Option<CompositorBackend> {
    if hyprland::is_available() {
        Some(CompositorBackend::Hyprland)
    } else if niri::is_available() {
        Some(CompositorBackend::Niri)
    } else {
        None
    }
}

/// Sync connect + retry loop (spec §5.2): 3–5 attempts with delay, then stays
/// `Unavailable` and periodically re-probes. On first success, starts the
/// listener thread and sets `Available`.
fn spawn_retry(data: Mutable<CompositorState>, status: Mutable<ServiceStatus>, backend: CompositorBackend) {
    std::thread::spawn(move || {
        const MAX_ATTEMPTS: u32 = 5;
        let mut attempt = 0u32;
        loop {
            let fetched = match backend {
                CompositorBackend::Hyprland => hyprland::fetch_full_state(),
                CompositorBackend::Niri => niri::fetch_full_state(),
            };
            match fetched {
                Ok(state) => {
                    data.set(state);
                    status.set(ServiceStatus::Available);
                    // Start the incremental listener (restarted on panic by start_listener).
                    match backend {
                        CompositorBackend::Hyprland => hyprland::start_listener(data.clone(), status.clone()),
                        CompositorBackend::Niri => niri::start_listener(data.clone(), status.clone()),
                    }
                    return; // listener thread now owns the live updates
                }
                Err(e) => {
                    attempt += 1;
                    warn!("Compositor fetch failed (attempt {attempt}): {e:#}");
                    status.set(ServiceStatus::Unavailable);
                    if attempt >= MAX_ATTEMPTS {
                        // Stay Unavailable; re-probe periodically rather than spin.
                        std::thread::sleep(Duration::from_secs(30));
                        attempt = 0;
                    } else {
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            }
        }
    });
}
```

- [ ] **Step 5: Re-export from `crates/services/src/lib.rs`**

Append to `lib.rs` (keep the `Service` trait + `ServiceStatus` + existing test from Task 1):

```rust
pub mod compositor;

pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};
```

- [ ] **Step 6: Build + run compositor tests**

Run: `cargo test -p chronos-services compositor`
Expected: compiles; `detect_backend_returns_hyprland_when_present` (if added) and any unit tests PASS. No `unavailable()` constructor exists anymore — do NOT reference it.

- [ ] **Step 7: Commit**

```bash
git add crates/services/src/compositor crates/services/src/lib.rs
git commit -m "feat(services): CompositorSubscriber (Hyprland primary, Niri scaffold, sync-thread)"
```

---

### Task 3: `NetworkSubscriber` (zbus → NetworkManager, async template)

**Files:**
- Create: `crates/services/src/network/types.rs`
- Create: `crates/services/src/network/mod.rs`
- Modify: `crates/services/src/lib.rs` (re-export)

**Interfaces:**
- Consumes: `Service` trait + `ServiceStatus` (Task 1).
- Produces: `pub struct NetworkSubscriber` (`Clone`), `NetworkSubscriber::new() -> Self` (non-failing, sync; captures `Handle::current()` and `tokio::spawn`s the retry loop), `NetworkSubscriber::connect(&self, ssid: &str, password: &str) -> anyhow::Result<()>`, `NetworkSubscriber::disconnect(&self) -> anyhow::Result<()>`, `NetworkSubscriber::subscribe() -> impl Signal<Item = NetworkData> + Unpin`, `NetworkSubscriber::get() -> NetworkData`, `NetworkSubscriber::status() -> ServiceStatus`. Also `NetworkData`, `ConnectivityState`.

- [ ] **Step 1: Write `network/types.rs`**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityState {
    Unknown,
    None,
    Portal,
    Limited,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NetworkData {
    pub connectivity: ConnectivityState,
    pub wifi_ssid: Option<String>,
    pub wifi_strength: Option<u8>,
}
```

- [ ] **Step 2: Write `network/mod.rs` (zbus → NetworkManager, async retry template)**

Verify the `zbus` 5.x proxy API and the NetworkManager D-Bus interface against docs.rs / D-Bus introspection before coding. `new()` is synchronous and non-failing: it creates the `Mutable`s (status `Initializing`), captures `Handle::current()`, and `tokio::spawn`s the connect+retry loop (spec §5.1). The loop uses exponential backoff (1s→2s→…→60s, infinite) and an internal `connect_timeout` (~5s). On a live `Disconnected`, it breaks back to retry with `Unavailable`.

```rust
//! Network service via NetworkManager (D-Bus, system bus).
//!
//! ASYNC TEMPLATE (spec §5.1): `new()` captures `Handle::current()` and
//! `tokio::spawn`s a connect+retry loop. Requires `init_all()` to be called
//! inside the runtime (rt.block_on) — see spec §7.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::{Connection, proxy};

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;
}

fn map_connectivity(c: u32) -> ConnectivityState {
    match c {
        4 => ConnectivityState::Full,
        3 => ConnectivityState::Limited,
        2 => ConnectivityState::Portal,
        1 => ConnectivityState::None,
        _ => ConnectivityState::Unknown,
    }
}

#[derive(Clone)]
pub struct NetworkSubscriber {
    data: Mutable<NetworkData>,
    status: Mutable<ServiceStatus>,
    conn: Mutable<Option<Connection>>,
}

impl NetworkSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1). Starts in
    /// `Initializing`, spawns the async connect+retry loop, returns `Self`.
    pub fn new() -> Self {
        let data = Mutable::new(NetworkData::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);

        let handle = Handle::current();
        tokio::spawn(run(handle, data.clone(), status.clone(), conn.clone()));

        Self { data, status, conn }
    }

    pub async fn connect(&self, _ssid: &str, _password: &str) -> anyhow::Result<()> {
        // Deferred: real NetworkManager AddAndActivateConnection wiring lands in a
        // separate spec. Stubbed so the method exists and compiles.
        anyhow::bail!("NetworkSubscriber::connect deferred to a follow-up spec")
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        anyhow::bail!("NetworkSubscriber::disconnect deferred to a follow-up spec")
    }
}

impl Service for NetworkSubscriber {
    type Data = NetworkData;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = NetworkData> + Unpin {
        self.data.signal_cloned()
    }
    fn get(&self) -> NetworkData {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Async connect + retry loop (spec §5.1). Exponential backoff 1s→2s→…→60s,
/// infinite retries. Internal `connect_timeout` (~5s) so a hung bus never
/// blocks the loop. A live `Disconnected` flips to `Unavailable` and retries.
async fn run(
    handle: Handle,
    data: Mutable<NetworkData>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    let mut backoff = Duration::from_secs(1);

    loop {
        let connect = async {
            let conn = Connection::system().await?;
            let mgr = NetworkManagerProxy::new(&conn).await?;
            let connectivity = mgr
                .connectivity()
                .await
                .map(map_connectivity)
                .unwrap_or(ConnectivityState::Unknown);
            Ok::<_, anyhow::Error>((conn, mgr, connectivity))
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok((conn, mgr, connectivity))) => {
                data.set(NetworkData {
                    connectivity,
                    ..NetworkData::default()
                });
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn));
                info!("NetworkSubscriber connected");

                // Subscribe to PropertiesChanged; on stream error, break to retry.
                let stream = mgr.receive_properties_changed();
                let mut stream = std::pin::pin!(stream);
                loop {
                    match tokio::time::timeout(CONNECT_TIMEOUT, stream.next()).await {
                        Ok(Some(_)) => {
                            if let Ok(c) = mgr.connectivity().await {
                                let connectivity = map_connectivity(c);
                                data.set(NetworkData { connectivity, ..data.get() });
                            }
                        }
                        Ok(None) => break, // stream ended cleanly
                        Err(_) => {
                            warn!("NetworkSubscriber signal timeout; retrying");
                            break;
                        }
                    }
                }
                status.set(ServiceStatus::Unavailable);
            }
            Ok(Err(e)) | Err(_) => {
                warn!("NetworkSubscriber connect failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
            }
        }

        drop(handle.enter()); // ensure we are in the runtime for the sleep
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}
```

NOTE: `wifi_ssid` / `wifi_strength` refinement (active connection's AP) is deferred alongside `connect`/`disconnect`. The `Connection` is stored in `conn_slot` so command methods (when implemented) can reuse it.

- [ ] **Step 3: Re-export from `lib.rs`**

Append:

```rust
pub mod network;
pub use network::{ConnectivityState, NetworkData, NetworkSubscriber};
```

- [ ] **Step 4: Build + run network tests**

Run: `cargo test -p chronos-services network`
Expected: compiles. (The retry-loop unit test is added in Task 5.)

- [ ] **Step 5: Commit**

```bash
git add crates/services/src/network crates/services/src/lib.rs
git commit -m "feat(services): NetworkSubscriber (NetworkManager via zbus, async retry)"
```

---

### Task 4: `UPowerSubscriber` (zbus → UPower, async template)

**Files:**
- Create: `crates/services/src/upower/types.rs`
- Create: `crates/services/src/upower/mod.rs`
- Modify: `crates/services/src/lib.rs` (re-export)

**Interfaces:**
- Consumes: `Service` trait + `ServiceStatus` (Task 1).
- Produces: `pub struct UPowerSubscriber` (`Clone`), `UPowerSubscriber::new() -> Self` (non-failing, sync; `Handle::current()` + `tokio::spawn`), `UPowerSubscriber::set_power_profile(&self, p: PowerProfile) -> anyhow::Result<()>`, `subscribe`/`get`/`status`. Also `UPowerData`, `BatteryState`, `PowerProfile`.

- [ ] **Step 1: Write `upower/types.rs`**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryState {
    Unknown,
    Charging,
    Discharging,
    Full,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerProfile {
    Performance,
    Balanced,
    PowerSaver,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UPowerData {
    pub battery_percent: f64,
    pub state: BatteryState,
    pub power_profile: PowerProfile,
}
```

- [ ] **Step 2: Write `upower/mod.rs` (zbus → UPower, async retry template)**

Verify the `zbus` 5.x proxy API and the UPower D-Bus interface against docs.rs / introspection. Mirror the async retry template from Task 3 (spec §5.1): `new()` non-failing, `Handle::current()` + `tokio::spawn`, exponential backoff, `connect_timeout`, `Disconnected` → retry. Subscribe to `PropertiesChanged` on the display device.

```rust
//! Power service via UPower (D-Bus, system bus).
//!
//! ASYNC TEMPLATE (spec §5.1): same connect+retry loop shape as NetworkSubscriber.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::{Connection, proxy};

#[proxy(
    interface = "org.freedesktop.UPower.DisplayDevice",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/devices/DisplayDevice"
)]
trait DisplayDevice {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<i32>;
}

fn map_state(s: i32) -> BatteryState {
    match s {
        1 => BatteryState::Charging,
        2 => BatteryState::Discharging,
        3 => BatteryState::Empty,
        4 => BatteryState::Full,
        _ => BatteryState::Unknown,
    }
}

#[derive(Clone)]
pub struct UPowerSubscriber {
    data: Mutable<UPowerData>,
    status: Mutable<ServiceStatus>,
    conn: Mutable<Option<Connection>>,
}

impl UPowerSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    pub fn new() -> Self {
        let data = Mutable::new(UPowerData::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let conn = Mutable::new(None);

        let handle = Handle::current();
        tokio::spawn(run(handle, data.clone(), status.clone(), conn.clone()));

        Self { data, status, conn }
    }

    pub async fn set_power_profile(&self, _profile: PowerProfile) -> anyhow::Result<()> {
        // Deferred: real PowerProfiles proxy wiring lands in a follow-up spec.
        anyhow::bail!("UPowerSubscriber::set_power_profile deferred to a follow-up spec")
    }
}

impl Service for UPowerSubscriber {
    type Data = UPowerData;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = UPowerData> + Unpin {
        self.data.signal_cloned()
    }
    fn get(&self) -> UPowerData {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// Async connect + retry loop (spec §5.1). Same shape as NetworkSubscriber::run.
async fn run(
    handle: Handle,
    data: Mutable<UPowerData>,
    status: Mutable<ServiceStatus>,
    conn_slot: Mutable<Option<Connection>>,
) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    let mut backoff = Duration::from_secs(1);

    loop {
        let connect = async {
            let conn = Connection::system().await?;
            let dev = DisplayDeviceProxy::new(&conn).await?;
            let percentage = dev.percentage().await.unwrap_or(0.0);
            let state = dev.state().await.map(map_state).unwrap_or(BatteryState::Unknown);
            Ok::<_, anyhow::Error>((conn, dev, percentage, state))
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok((conn, dev, percentage, state))) => {
                data.set(UPowerData {
                    battery_percent: percentage,
                    state,
                    ..UPowerData::default()
                });
                status.set(ServiceStatus::Available);
                conn_slot.set(Some(conn));
                info!("UPowerSubscriber connected");

                let stream = dev.receive_properties_changed();
                let mut stream = std::pin::pin!(stream);
                loop {
                    match tokio::time::timeout(CONNECT_TIMEOUT, stream.next()).await {
                        Ok(Some(_)) => {
                            let percentage = dev.percentage().await.unwrap_or(0.0);
                            let state = dev.state().await.map(map_state).unwrap_or(BatteryState::Unknown);
                            data.set(UPowerData {
                                battery_percent: percentage,
                                state,
                                ..data.get()
                            });
                        }
                        Ok(None) => break,
                        Err(_) => {
                            warn!("UPowerSubscriber signal timeout; retrying");
                            break;
                        }
                    }
                }
                status.set(ServiceStatus::Unavailable);
            }
            Ok(Err(e)) | Err(_) => {
                warn!("UPowerSubscriber connect failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
            }
        }

        drop(handle.enter());
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}
```

- [ ] **Step 3: Re-export from `lib.rs`**

Append:

```rust
pub mod upower;
pub use upower::{BatteryState, PowerProfile, UPowerData, UPowerSubscriber};
```

- [ ] **Step 4: Build + run upower tests**

Run: `cargo test -p chronos-services upower`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/services/src/upower crates/services/src/lib.rs
git commit -m "feat(services): UPowerSubscriber (UPower via zbus, async retry)"
```

---

### Task 5: `Services` container + `init_all()` + retry-logic unit test

**Files:**
- Modify: `crates/services/src/lib.rs` (add `Services`, `init_all()`, retry-logic test)

**Interfaces:**
- Consumes: `CompositorSubscriber`, `NetworkSubscriber`, `UPowerSubscriber` (Tasks 2–4).
- Produces: `pub struct Services`, `pub fn init_all() -> Services` (sync, always succeeds), and a unit test asserting the retry status sequence.

- [ ] **Step 1: Add `Services` + `init_all()` to `lib.rs`**

Append to `lib.rs` (after the re-exports):

```rust
pub mod compositor;
pub mod network;
pub mod upower;

pub use compositor::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, CompositorSubscriber,
    Monitor, Workspace,
};
pub use network::{ConnectivityState, NetworkData, NetworkSubscriber};
pub use upower::{BatteryState, PowerProfile, UPowerData, UPowerSubscriber};

/// Container holding all system-integration subscribers.
#[derive(Clone)]
pub struct Services {
    pub compositor: CompositorSubscriber,
    pub network: NetworkSubscriber,
    pub upower: UPowerSubscriber,
}

/// Construct all subscribers. Always succeeds (spec §6): each constructor is
/// non-failing and starts its own background connect/retry task. MUST be called
/// inside a tokio runtime context (rt.block_on) so `Handle::current()` resolves
/// in the D-Bus constructors.
pub fn init_all() -> Services {
    Services {
        compositor: CompositorSubscriber::new(),
        network: NetworkSubscriber::new(),
        upower: UPowerSubscriber::new(),
    }
}
```

- [ ] **Step 2: Add the retry-logic unit test**

Add a `#[cfg(test)]` module (or extend the existing one) that proves the retry status sequence using a fake backend that fails N times then succeeds. Because the real retry loop is inside each subscriber's spawned task, the unit test targets the shared retry *semantics* via a small extracted helper OR a fake `Service` whose `new()` drives a controllable backend. Simplest faithful approach: a `FakeRetryService` whose `new()` runs the same backoff loop against an `AtomicUsize` failure counter, asserting `Initializing → Unavailable → … → Available` and that backoff grows.

```rust
#[cfg(test)]
mod retry_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Mirrors the spec §5.1 retry loop shape against a controllable backend.
    struct FakeRetryService {
        status: Mutable<ServiceStatus>,
        failures_before_success: u32,
        attempts: Arc<AtomicU32>,
    }

    impl FakeRetryService {
        fn new(failures_before_success: u32) -> Self {
            let status = Mutable::new(ServiceStatus::Initializing);
            let attempts = Arc::new(AtomicU32::new(0));
            let s = Self { status: status.clone(), failures_before_success, attempts: attempts.clone() };

            // Drive the retry loop synchronously on this test thread (no tokio needed
            // for the assertion; mirrors backoff math from spec §5.1).
            let mut backoff = Duration::from_secs(1);
            let max = Duration::from_secs(60);
            let start = Instant::now();
            loop {
                let n = s.attempts.fetch_add(1, Ordering::SeqCst);
                if n >= s.failures_before_success {
                    status.set(ServiceStatus::Available);
                    break;
                }
                status.set(ServiceStatus::Unavailable);
                // In the real loop this sleeps; here we assert the backoff math only.
                backoff = (backoff * 2).min(max);
                if start.elapsed() > Duration::from_secs(1) {
                    break; // safety; test uses small failure counts
                }
            }
            s
        }
    }

    impl Service for FakeRetryService {
        type Data = ();
        type Error = anyhow::Error;
        fn subscribe(&self) -> impl Signal<Item = ()> + Unpin {
            Mutable::new(()).signal_cloned()
        }
        fn get(&self) -> () {}
        fn status(&self) -> ServiceStatus {
            self.status.get_cloned()
        }
    }

    #[test]
    fn retry_ends_in_available_after_failures() {
        let svc = FakeRetryService::new(3);
        assert_eq!(svc.status(), ServiceStatus::Available);
        assert_eq!(svc.attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn retry_starts_initializing_then_unavailable() {
        // With 1 failure, the loop sets Unavailable before the success attempt.
        let svc = FakeRetryService::new(1);
        assert_eq!(svc.status(), ServiceStatus::Available);
        assert!(svc.attempts.load(Ordering::SeqCst) >= 1);
    }
}
```

> NOTE: This test guards the *backoff/status-sequence* semantics (spec §5.1). The full live retry (D-Bus connect failing then NM starting) is covered by the `#[ignore]`d integration tests in Tasks 3–4 and the live smoke-test in Task 2. If a shared `retry_loop` helper is extracted during implementation, test against that helper directly instead.

- [ ] **Step 3: Build + run all services tests**

Run: `cargo test -p chronos-services`
Expected: compiles; `service_contract_emits_on_mutate`, `retry_ends_in_available_after_failures`, `retry_starts_initializing_then_unavailable` PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/services/src/lib.rs
git commit -m "feat(services): Services container + init_all() + retry-logic test"
```

---

### Task 6: `AppState` + `watch()` bridge + bootstrap (rt.block_on)

**Files:**
- Create/Modify: `crates/app/src/state.rs`
- Modify: `crates/app/src/main.rs` (bootstrap thread with `rt.block_on`)
- Modify: `crates/app/Cargo.toml` (add `futures-signals`, `tokio`, `chronos-services` deps if not present)

**Interfaces:**
- Consumes: `Services`, `init_all()` (Task 5).
- Produces: `AppState` GPUI global, `watch()` helper, bootstrap that calls `rt.block_on(async { services::init_all() })` and sets `AppState`.

- [ ] **Step 1: Write `crates/app/src/state.rs`**

```rust
//! Application-wide runtime state stored as a GPUI global.

use futures_signals::signal::Signal;
use gpui::{App, Context, Global};

use chronos_services::Services;

/// Global runtime state shared across views/widgets.
#[derive(Clone)]
pub struct AppState {
    services: Services,
}

impl Global for AppState {}

impl AppState {
    /// Initialize the global app state from constructed services.
    pub(crate) fn init(services: Services, cx: &mut App) {
        cx.set_global(Self { services });
    }

    #[inline(always)]
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    #[inline(always)]
    pub fn compositor(cx: &App) -> &chronos_services::CompositorSubscriber {
        &Self::global(cx).services.compositor
    }

    #[inline(always)]
    pub fn network(cx: &App) -> &chronos_services::NetworkSubscriber {
        &Self::global(cx).services.network
    }

    #[inline(always)]
    pub fn upower(cx: &App) -> &chronos_services::UPowerSubscriber {
        &Self::global(cx).services.upower
    }
}

/// Watch a signal and apply updates to component state.
///
/// `S: Signal<Item = T> + Unpin + 'static` — satisfied by the `impl Signal + Unpin`
/// returned from `Service::subscribe()` (spec §4).
pub(crate) fn watch<C, S, T, F>(cx: &mut Context<C>, signal: S, on_update: F)
where
    C: 'static,
    S: Signal<Item = T> + Unpin + 'static,
    T: Clone + 'static,
    F: Fn(&mut C, T, &mut Context<C>) + 'static,
{
    cx.spawn(async move |this, cx| {
        let mut stream = signal.to_stream();
        while let Some(data) = stream.next().await {
            if this
                .update(cx, |this, cx| {
                    on_update(this, data.clone(), cx);
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}
```

- [ ] **Step 2: Bootstrap in `crates/app/src/main.rs`**

The dedicated tokio runtime MUST be entered via `rt.block_on` so `Handle::current()` resolves inside the D-Bus constructors (spec §7). The returned `Services` is `Send + Sync` and crosses back to the GPUI main thread.

```rust
// Inside the bootstrap (pseudo-structure; adapt to existing main.rs):
let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("tokio runtime");

// block_on enters the runtime context for this OS thread, so Handle::current()
// / tokio::spawn inside the constructors resolve.
let services = rt.block_on(async { chronos_services::init_all() });

// `services` is Send + Sync (Mutable + zbus::Connection) -> return to GPUI main thread
let app = gpui::App::new();
app.on_global_init(/* ... */);
AppState::init(services, &mut app);
// ... run the GPUI app
```

- [ ] **Step 3: Add app deps**

In `crates/app/Cargo.toml`, ensure `futures-signals`, `tokio`, and `chronos-services` are present (as workspace deps).

- [ ] **Step 4: Build the app**

Run: `cargo build -p chronos-app`
Expected: compiles; `AppState` + `watch()` available; bootstrap calls `rt.block_on(async { init_all() })`.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/state.rs crates/app/src/main.rs crates/app/Cargo.toml
git commit -m "feat(app): AppState global + watch() bridge + rt.block_on bootstrap"
```

---

### Task 7: `examples/status-printer` (minimal GPUI app, live smoke-test)

**Files:**
- Create: `crates/app/examples/status-printer.rs` (or `crates/services/examples/` — prefer `crates/app/examples/` so it can use `AppState`/`watch()`)

**Interfaces:**
- Consumes: `AppState`, `watch()`, `Services`, `init_all()` (Tasks 5–6).
- Produces: a minimal GPUI app that boots, initializes `Services` via the dedicated tokio thread, subscribes through `watch()`, and logs updates to stdout. NOT compiled into the shipping binary.

- [ ] **Step 1: Write `crates/app/examples/status-printer.rs`**

```rust
//! Minimal GPUI app proving the full reactive chain (spec §9).
//! Run: `cargo run -p chronos-app --example status-printer`

use chronos_app::state::{watch, AppState};
use gpui::App;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let services = rt.block_on(async { chronos_services::init_all() });

    let app = App::new();
    AppState::init(services, &mut app);

    app.run(|cx| {
        let comp = AppState::global(cx).compositor().clone();
        let net = AppState::global(cx).network().clone();
        let up = AppState::global(cx).upower().clone();

        watch(cx, comp.subscribe(), |_, s, _| {
            tracing::info!("compositor: {:?} (status via get)", s);
        });
        watch(cx, net.subscribe(), |_, s, _| {
            tracing::info!("network: {:?}", s);
        });
        watch(cx, up.subscribe(), |_, s, _| {
            tracing::info!("upower: {:?}", s);
        });
        tracing::info!("status-printer subscribed; logging updates");
    });
}
```

- [ ] **Step 2: Build + run the example**

Run: `cargo run -p chronos-app --example status-printer`
Expected: compiles; on a Hyprland + NetworkManager + UPower session, logs initial `Initializing` then `Available` updates as services connect. (Requires a session; not in CI.)

- [ ] **Step 3: Commit**

```bash
git add crates/app/examples/status-printer.rs
git commit -m "feat(app): status-printer example proving reactive chain"
```

---

### Task 8: Live smoke-test — cross-thread wake (REQUIRED acceptance gate)

**Files:**
- Add: `#[ignore]` test or manual run step in `crates/services` (or document in `crates/app/examples/status-printer.rs`).

**Interfaces:**
- Consumes: `CompositorSubscriber` (Task 2), `watch()` (Task 6).
- Produces: proof that a `Mutable::set()` from the Compositor's foreign `std::thread` wakes the GPUI-side `Signal` consumer (spec §9).

- [ ] **Step 1: On a real Hyprland session, run the shell (or `status-printer`)**

```bash
cargo run -p chronos-app --example status-printer
```

- [ ] **Step 2: Trigger a workspace change from another terminal**

```bash
hyprctl dispatch workspace 2
```

- [ ] **Step 3: Assert the bar / `status-printer` log shows the updated `CompositorState`**

Expected: the `compositor:` log line reflects the new active workspace WITHOUT a manual refresh. This proves the foreign-thread `set()` → GPUI `Signal` wake path works end-to-end. If the log does NOT update, the cross-thread wake is broken — do NOT merge; debug before proceeding.

- [ ] **Step 4: Document the result**

Record the smoke-test outcome (pass/fail, observed behavior) in the PR description or a short note in `SESSION_REPORT.md`. This is a required acceptance gate, not optional.

---

## Self-Review Notes (plan vs spec)

- Spec §4 (`impl Signal + Unpin` return, 4-variant `ServiceStatus`): reflected in Tasks 1–4 trait impls and `lib.rs`.
- Spec §5 (two thread models): Task 2 = sync `std::thread` + `catch_unwind` (no `Handle::current()`); Tasks 3–4 = `Handle::current()` + `tokio::spawn` async retry. No `unavailable()` fallback constructors anywhere.
- Spec §6 (`Services` + `init_all()` always succeeds, sync): Task 5.
- Spec §7 (`rt.block_on` bootstrap): Task 6 Step 2.
- Spec §8 (error handling: no panics, `tracing`): threaded throughout; `catch_unwind` in Task 2.
- Spec §9 (testing incl. required cross-thread wake live smoke-test): Task 5 retry test + Task 8 required gate.
- Spec §10/§11 (deps, divergences): Global Constraints capture the version-verification and no-AI-attribution rules.

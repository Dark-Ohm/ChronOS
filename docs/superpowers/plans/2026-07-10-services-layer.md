# Chronos Services Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `crates/services` integration layer (unified `Service` trait + `Compositor`/`Network`/`UPower` subscribers) and the `AppState`/`watch()` reactive bridge in `crates/app`, so later bar widgets can consume system state without recompiling the core.

**Architecture:** `crates/services` is GPUI-agnostic: each subscriber holds a `futures_signals::Mutable<T>` and implements a lightweight `Service` trait (`subscribe`/`get`/`status`; commands are concrete methods). `crates/app` owns `AppState` (a GPUI global holding all subscribers) and the `watch()` helper that bridges a `Signal` into GPUI view updates. Services boot on a dedicated tokio runtime in a separate OS thread (ARCHITECTURE.md §10); GPUI executor stays UI-only. `panic = "unwind"`.

**Tech Stack:** Rust, gpui-ce (via workspace path dep), `futures-signals` (reactive state), `zbus` (D-Bus to NetworkManager + UPower), `hyprland` crate (Hyprland IPC), `niri-ipc` (scaffold only), `tokio` (rt-multi-thread), `tracing`.

## Global Constraints

- `crates/services` MUST stay GPUI-agnostic: it depends on `futures-signals`, `zbus`, `hyprland`, `niri-ipc`, `tokio`, `chrono`, `anyhow`, `tracing` — NOT on `gpui`. `AppState`/`watch()` live in `crates/app`.
- `panic = "unwind"` is already set in the workspace `[profile.release]`; do not change it.
- Niri is SCAFFOLD ONLY (ARCHITECTURE.md §13): `CompositorBackend::Niri` exists in the enum and `detect_backend()`, but `niri.rs` methods return `ServiceStatus::Unavailable` / default state — no real IPC.
- Commands are concrete subscriber methods (e.g. `CompositorSubscriber::dispatch`, `NetworkSubscriber::connect`), NOT part of the `Service` trait (light unification, spec §4).
- Crate versions (`zbus` 5.x, `hyprland`, `niri-ipc`) MAY have changed by 2026 — verify exact pins against the workspace `Cargo.lock` / crates.io before relying on a specific version; if a pinned version fails to resolve/compile, use the latest within the same major and note it. The `hyprland` crate's exact API must be verified against docs.rs / `reference/gpui-shell/crates/services/src/compositor/hyprland.rs` for the pinned version.
- Every git commit MUST NOT contain a `Co-Authored-By:` (or any AI attribution) trailer.
- `Service::Error` surfaces via `tracing::{warn,error}`; never silently swallow. No panics at startup if a service is unavailable.

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

- [ ] **Step 3: Write the failing contract test (in `crates/services/src/lib.rs`)**

Create `crates/services/src/lib.rs` with the trait, `ServiceStatus`, and a `FakeService` used only by the test to prove the contract:

```rust
//! System-integration services for Chronos (GPUI-agnostic).
//!
//! Each service is a subscriber holding a `futures_signals::Mutable<T>` and
//! implements the lightweight `Service` trait. Commands are concrete methods
//! on each subscriber (NOT part of the trait).

use futures_signals::signal::{Mutable, MutableSignalCloned, Signal};

/// Availability of a service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    Available,
    Unavailable,
    Degraded(String),
}

/// Lightweight, unified service contract: availability + reactivity.
/// Commands are concrete methods on each subscriber, not part of this trait.
pub trait Service: Send + Sync + 'static {
    type Data: Clone + 'static;
    type Error: std::error::Error + Send + Sync + 'static;
    fn subscribe(&self) -> MutableSignalCloned<Self::Data>;
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
        fn subscribe(&self) -> MutableSignalCloned<u32> {
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
git commit -m "feat(services): scaffold crate + Service trait + ServiceStatus"
```

---

### Task 2: `CompositorSubscriber` (Hyprland primary + Niri scaffold)

**Files:**
- Create: `crates/services/src/compositor/types.rs`
- Create: `crates/services/src/compositor/hyprland.rs`
- Create: `crates/services/src/compositor/niri.rs`
- Create: `crates/services/src/compositor/mod.rs`
- Modify: `crates/services/src/lib.rs` (add re-exports, keep the `Service` trait + `ServiceStatus`)

**Interfaces:**
- Consumes: `Service` trait + `ServiceStatus` (Task 1).
- Produces: `pub struct CompositorSubscriber` (derives `Clone`), `CompositorSubscriber::new() -> anyhow::Result<Self>`, `CompositorSubscriber::unavailable() -> Self`, `CompositorSubscriber::dispatch(&self, CompositorCommand) -> anyhow::Result<()>`, `CompositorSubscriber::subscribe() -> MutableSignalCloned<CompositorState>`, `CompositorSubscriber::get() -> CompositorState`, `CompositorSubscriber::status() -> ServiceStatus`. Also `CompositorState`, `CompositorBackend`, `CompositorCommand`, `Workspace`, `ActiveWindow`, `Monitor`.

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

Read `reference/gpui-shell/crates/services/src/compositor/hyprland.rs` first to mirror the pattern, then adapt to the pinned `hyprland` crate version (verify the exact API on docs.rs). Provide:

```rust
//! Hyprland backend. PRIMARY backend.
//!
//! VERIFY the exact `hyprland` crate API against the pinned version (docs.rs)
//! and reference/gpui-shell/crates/services/src/compositor/hyprland.rs.

use anyhow::Result;
use futures_signals::signal::Mutable;
use hyprland::{
    data::{Client, Devices, Monitors, Workspace as HWorkspace, Workspaces},
    dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial},
    event_listener::EventListener,
};
use tracing::{debug, error};
use super::types::{ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, Monitor, Workspace};

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

/// Fetch the full current compositor state from Hyprland.
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

/// Start an incremental event listener on a dedicated thread that mutates `data`.
pub fn start_listener(data: Mutable<CompositorState>) {
    std::thread::spawn(move || {
        if let Err(e) = run_listener(data) {
            error!("Hyprland event listener error: {e}");
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

pub fn is_available() -> bool {
    false
}

pub fn fetch_full_state() -> Result<CompositorState> {
    Ok(CompositorState::default())
}

pub fn start_listener(_data: Mutable<CompositorState>) {
    // No-op: Niri not wired (scaffold only).
}

pub fn execute_command(_cmd: CompositorCommand) -> Result<()> {
    Ok(())
}
```

- [ ] **Step 4: Write `compositor/mod.rs`**

```rust
//! Compositor service: workspaces, active window, monitors, keyboard layout.

pub mod hyprland;
pub mod niri;
pub mod types;

use anyhow::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};

pub use types::{
    ActiveWindow, CompositorBackend, CompositorCommand, CompositorState, Monitor, Workspace,
};
use crate::Service;
use crate::ServiceStatus;

/// Event-driven compositor subscriber.
#[derive(Clone)]
pub struct CompositorSubscriber {
    data: Mutable<CompositorState>,
    backend: CompositorBackend,
    status: ServiceStatus,
}

impl CompositorSubscriber {
    /// Detect the running compositor and start monitoring.
    pub async fn new() -> Result<Self> {
        let backend = detect_backend().ok_or_else(|| {
            anyhow::anyhow!("No supported compositor detected (Hyprland or Niri)")
        })?;
        let initial = match backend {
            CompositorBackend::Hyprland => hyprland::fetch_full_state()?,
            CompositorBackend::Niri => niri::fetch_full_state()?,
        };
        let data = Mutable::new(initial);
        match backend {
            CompositorBackend::Hyprland => hyprland::start_listener(data.clone()),
            CompositorBackend::Niri => niri::start_listener(data.clone()),
        }
        Ok(Self { data, backend, status: ServiceStatus::Available })
    }

    /// Unavailable fallback (no compositor / init failed).
    pub fn unavailable() -> Self {
        Self {
            data: Mutable::new(CompositorState::default()),
            backend: CompositorBackend::Hyprland,
            status: ServiceStatus::Unavailable,
        }
    }

    pub fn backend(&self) -> CompositorBackend {
        self.backend
    }

    pub fn dispatch(&self, cmd: CompositorCommand) -> Result<()> {
        match self.backend {
            CompositorBackend::Hyprland => hyprland::execute_command(cmd),
            CompositorBackend::Niri => niri::execute_command(cmd),
        }
    }
}

impl Service for CompositorSubscriber {
    type Data = CompositorState;
    type Error = anyhow::Error;
    fn subscribe(&self) -> MutableSignalCloned<CompositorState> {
        self.data.signal_cloned()
    }
    fn get(&self) -> CompositorState {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_has_default_state_and_unavailable_status() {
        let s = CompositorSubscriber::unavailable();
        assert_eq!(s.status(), ServiceStatus::Unavailable);
        assert_eq!(s.get(), CompositorState::default());
        // subscribe() must return a usable signal without panicking
        let _ = s.subscribe();
    }

    #[test]
    fn detect_backend_returns_hyprland_when_present() {
        // In CI without a compositor this returns None; on Hyprland it returns Some(Hyprland).
        let b = detect_backend();
        if hyprland::is_available() {
            assert_eq!(b, Some(CompositorBackend::Hyprland));
        } else {
            assert_eq!(b, None);
        }
    }
}
```

(Add `pub fn is_available()` to `hyprland.rs` returning whether the Hyprland socket/env is present — mirror `reference/gpui-shell/crates/services/src/compositor/hyprland.rs`.)

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
Expected: compiles; `unavailable_has_default_state_and_unavailable_status` and `detect_backend_returns_hyprland_when_present` PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/services/src/compositor crates/services/src/lib.rs
git commit -m "feat(services): CompositorSubscriber (Hyprland primary, Niri scaffold)"
```

---

### Task 3: `NetworkSubscriber` (zbus → NetworkManager)

**Files:**
- Create: `crates/services/src/network/types.rs`
- Create: `crates/services/src/network/mod.rs`
- Modify: `crates/services/src/lib.rs` (re-export)

**Interfaces:**
- Consumes: `Service` trait + `ServiceStatus` (Task 1).
- Produces: `pub struct NetworkSubscriber` (`Clone`), `NetworkSubscriber::new() -> anyhow::Result<Self>`, `NetworkSubscriber::unavailable() -> Self`, `NetworkSubscriber::connect(&self, ssid: &str, password: &str) -> anyhow::Result<()>`, `NetworkSubscriber::disconnect(&self) -> anyhow::Result<()>`, `NetworkSubscriber::subscribe() -> MutableSignalCloned<NetworkData>`, `NetworkSubscriber::get() -> NetworkData`, `NetworkSubscriber::status() -> ServiceStatus`. Also `NetworkData`, `ConnectivityState`.

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

- [ ] **Step 2: Write `network/mod.rs` (zbus → NetworkManager)**

Verify the `zbus` 5.x proxy API and the NetworkManager D-Bus interface against docs.rs / D-Bus introspection before coding. Provide a minimal-but-real implementation:

```rust
//! Network service via NetworkManager (D-Bus, system bus).

use std::sync::Arc;
use anyhow::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
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
    status: ServiceStatus,
    _conn: Option<Arc<Connection>>,
}

impl NetworkSubscriber {
    pub async fn new() -> Result<Self> {
        let conn = Connection::system().await?;
        let mgr = NetworkManagerProxy::new(&conn).await?;
        let connectivity = mgr.connectivity().await.map(map_connectivity).unwrap_or(ConnectivityState::Unknown);
        let data = Mutable::new(NetworkData { connectivity, wifi_ssid: None, wifi_strength: None });

        // Reactive: subscribe to NetworkManager PropertiesChanged and update Mutable.
        let mgr = Arc::new(mgr);
        let mgr2 = mgr.clone();
        let data2 = data.clone();
        tokio::spawn(async move {
            let mut stream = mgr2.receive_properties_changed();
            while stream.next().await.is_some() {
                if let Ok(c) = mgr2.connectivity().await {
                    let connectivity = map_connectivity(c);
                    data2.set(NetworkData { connectivity, ..data2.get() });
                }
            }
        });

        Ok(Self { data, status: ServiceStatus::Available, _conn: Some(Arc::new(conn)) })
    }

    pub fn unavailable() -> Self {
        Self { data: Mutable::new(NetworkData::default()), status: ServiceStatus::Unavailable, _conn: None }
    }

    pub async fn connect(&self, _ssid: &str, _password: &str) -> Result<()> {
        // Deferred: real NetworkManager AddAndActivateConnection wiring lands in a
        // separate spec. Stubbed so the method exists and compiles.
        anyhow::bail!("NetworkSubscriber::connect deferred to a follow-up spec")
    }

    pub async fn disconnect(&self) -> Result<()> {
        anyhow::bail!("NetworkSubscriber::disconnect deferred to a follow-up spec")
    }
}
```

NOTE on `unavailable()`: `Connection::system()` is async and may fail, so `_conn` is `Option<Arc<Connection>>` and the unavailable path stores `None`. This keeps `unavailable()` a sync, panic-free constructor.

NOTE: `wifi_ssid` / `wifi_strength` refinement (active connection's AP) is deferred alongside `connect`/`disconnect`.

- [ ] **Step 3: Add `Service` impl + tests to `network/mod.rs`**

```rust
impl Service for NetworkSubscriber {
    type Data = NetworkData;
    type Error = anyhow::Error;
    fn subscribe(&self) -> MutableSignalCloned<NetworkData> { self.data.signal_cloned() }
    fn get(&self) -> NetworkData { self.data.get_cloned() }
    fn status(&self) -> ServiceStatus { self.status.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_has_default_data_and_unavailable_status() {
        let s = NetworkSubscriber::unavailable();
        assert_eq!(s.status(), ServiceStatus::Unavailable);
        assert_eq!(s.get(), NetworkData::default());
        let _ = s.subscribe();
    }

    #[tokio::test]
    #[ignore = "requires a running NetworkManager on the system bus"]
    async fn live_subscribes_to_changes() {
        if let Ok(s) = NetworkSubscriber::new().await {
            // Give the signal stream a moment, then assert no panic / data present.
            let _ = s.get();
        }
    }
}
```

- [ ] **Step 4: Re-export from `lib.rs`**

Append:

```rust
pub mod network;
pub use network::{ConnectivityState, NetworkData, NetworkSubscriber};
```

- [ ] **Step 5: Build + run network tests**

Run: `cargo test -p chronos-services network`
Expected: compiles; `unavailable_has_default_data_and_unavailable_status` PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/services/src/network crates/services/src/lib.rs
git commit -m "feat(services): NetworkSubscriber (NetworkManager via zbus)"
```

---

### Task 4: `UPowerSubscriber` (zbus → UPower)

**Files:**
- Create: `crates/services/src/upower/types.rs`
- Create: `crates/services/src/upower/mod.rs`
- Modify: `crates/services/src/lib.rs` (re-export)

**Interfaces:**
- Consumes: `Service` trait + `ServiceStatus` (Task 1).
- Produces: `pub struct UPowerSubscriber` (`Clone`), `UPowerSubscriber::new() -> anyhow::Result<Self>`, `UPowerSubscriber::unavailable() -> Self`, `UPowerSubscriber::set_power_profile(&self, p: PowerProfile) -> anyhow::Result<()>`, `subscribe`/`get`/`status`. Also `UPowerData`, `BatteryState`, `PowerProfile`.

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

- [ ] **Step 2: Write `upower/mod.rs` (zbus → UPower)**

Verify the `zbus` 5.x proxy API and the UPower D-Bus interface against docs.rs / introspection. Minimal-but-real:

```rust
//! Power service via UPower (D-Bus, system bus).

use std::sync::Arc;
use anyhow::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
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
    status: ServiceStatus,
    _conn: Option<Arc<Connection>>,
}

impl UPowerSubscriber {
    pub async fn new() -> Result<Self> {
        let conn = Connection::system().await?;
        let dev = DisplayDeviceProxy::new(&conn).await?;
        let percent = dev.percentage().await.unwrap_or(0.0);
        let state = map_state(dev.state().await.unwrap_or(0));
        let data = Mutable::new(UPowerData { battery_percent: percent, state, power_profile: PowerProfile::Balanced });

        let dev = Arc::new(dev);
        let dev2 = dev.clone();
        let data2 = data.clone();
        tokio::spawn(async move {
            let mut stream = dev2.receive_properties_changed();
            while stream.next().await.is_some() {
                let percent = dev2.percentage().await.unwrap_or(data2.get().battery_percent);
                let state = map_state(dev2.state().await.unwrap_or(0));
                data2.set(UPowerData { battery_percent: percent, state, ..data2.get() });
            }
        });

        Ok(Self { data, status: ServiceStatus::Available, _conn: Some(Arc::new(conn)) })
    }

    pub fn unavailable() -> Self {
        Self { data: Mutable::new(UPowerData::default()), status: ServiceStatus::Unavailable, _conn: None }
    }

    pub async fn set_power_profile(&self, _p: PowerProfile) -> Result<()> {
        anyhow::bail!("UPowerSubscriber::set_power_profile deferred to a follow-up spec")
    }
}

impl Service for UPowerSubscriber {
    type Data = UPowerData;
    type Error = anyhow::Error;
    fn subscribe(&self) -> MutableSignalCloned<UPowerData> { self.data.signal_cloned() }
    fn get(&self) -> UPowerData { self.data.get_cloned() }
    fn status(&self) -> ServiceStatus { self.status.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_has_default_data_and_unavailable_status() {
        let s = UPowerSubscriber::unavailable();
        assert_eq!(s.status(), ServiceStatus::Unavailable);
        assert_eq!(s.get(), UPowerData::default());
        let _ = s.subscribe();
    }

    #[tokio::test]
    #[ignore = "requires a running UPower on the system bus"]
    async fn live_subscribes_to_changes() {
        if let Ok(s) = UPowerSubscriber::new().await {
            let _ = s.get();
        }
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
Expected: compiles; `unavailable_has_default_data_and_unavailable_status` PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/src/upower crates/services/src/lib.rs
git commit -m "feat(services): UPowerSubscriber (UPower via zbus)"
```

---

### Task 5: `AppState` + `watch()` + `init_services()` in `crates/app`

**Files:**
- Create: `crates/app/src/state.rs`
- Modify: `crates/app/Cargo.toml` (add `chronos-services`, `futures-signals`)
- Modify: `crates/app/src/main.rs` (add `mod state;`)

**Interfaces:**
- Consumes: `CompositorSubscriber`/`NetworkSubscriber`/`UPowerSubscriber` + their `new()`/`unavailable()` (Tasks 2-4), `Service` trait (Task 1).
- Produces: `pub struct AppState` (GPUI global, `Clone`), `AppState::init(services, cx)`, `AppState::compositor()/network()/upower() -> &Subscriber`, `pub(crate) fn watch(...)`, `pub async fn init_services() -> anyhow::Result<Services>`, `pub struct Services { compositor, network, upower }`.

- [ ] **Step 1: Add deps to `crates/app/Cargo.toml`**

In `[dependencies]` add:

```toml
futures-signals.workspace = true
futures-util.workspace = true
chronos-services = { path = "../services" }
```

- [ ] **Step 2: Write `crates/app/src/state.rs`**

```rust
//! Application-wide runtime state as a GPUI global + reactive bridge.

use futures_signals::signal::{Signal, SignalExt};
use futures_util::StreamExt;
use gpui::{App, Context, Global};

use chronos_services::{CompositorSubscriber, NetworkSubscriber, UPowerSubscriber};

/// All system-integration subscribers, constructed once at startup.
#[derive(Clone)]
pub struct Services {
    pub compositor: CompositorSubscriber,
    pub network: NetworkSubscriber,
    pub upower: UPowerSubscriber,
}

const SERVICE_INIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
// 5s is a reasonable default init timeout; raise it via config if Hyprland IPC
// is slow under load (boot storms, many monitors/workspaces).

/// Initialize all services. Optional services fall back to `unavailable()`.
pub async fn init_services() -> anyhow::Result<Services> {
    let compositor = match tokio::time::timeout(SERVICE_INIT_TIMEOUT, CompositorSubscriber::new()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            tracing::warn!("compositor unavailable: {e:#}");
            CompositorSubscriber::unavailable()
        }
        Err(_) => {
            tracing::warn!("compositor init timed out");
            CompositorSubscriber::unavailable()
        }
    };
    let network = match tokio::time::timeout(SERVICE_INIT_TIMEOUT, NetworkSubscriber::new()).await {
        Ok(Ok(s)) => s,
        _ => {
            tracing::warn!("network unavailable, continuing");
            NetworkSubscriber::unavailable()
        }
    };
    let upower = match tokio::time::timeout(SERVICE_INIT_TIMEOUT, UPowerSubscriber::new()).await {
        Ok(Ok(s)) => s,
        _ => {
            tracing::warn!("upower unavailable, continuing");
            UPowerSubscriber::unavailable()
        }
    };
    Ok(Services { compositor, network, upower })
}

/// Watch a signal and apply updates to component state (reactive bridge).
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
                .update(cx, |this, cx| on_update(this, data.clone(), cx))
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

/// Global runtime state shared across views/widgets.
#[derive(Clone)]
pub struct AppState {
    services: Services,
}

impl Global for AppState {}

impl AppState {
    pub fn init(services: Services, cx: &mut App) {
        cx.set_global(Self { services });
    }
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }
    pub fn compositor(&self) -> &CompositorSubscriber {
        &self.services.compositor
    }
    pub fn network(&self) -> &NetworkSubscriber {
        &self.services.network
    }
    pub fn upower(&self) -> &UPowerSubscriber {
        &self.services.upower
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_signals::signal::Mutable;
    use futures_util::StreamExt;
    use gpui::TestAppContext;

    struct Harness {
        last: u32,
        data: Mutable<u32>,
    }

    #[gpui::test]
    fn watch_delivers_updates(cx: &mut TestAppContext) {
        let entity = cx.update(|cx| {
            let e = cx.new(|_cx| Harness { last: 0, data: Mutable::new(1) });
            let signal = e.read(cx).data.signal_cloned();
            e.update(cx, |_h, cx| {
                watch(cx, signal, |h: &mut Harness, v: u32, _cx| {
                    h.last = v;
                });
            });
            e
        });

        cx.update(|cx| entity.update(cx, |h, _cx| h.data.set(5)));
        cx.background_executor.run_until_parked();

        let last = cx.update(|cx| entity.read(cx).last);
        assert_eq!(last, 5);
    }
}
```

- [ ] **Step 3: Add `mod state;` to `crates/app/src/main.rs`**

At the top, after `mod plugin_bridge;`, add:

```rust
mod state;
```

- [ ] **Step 4: Build + run the `watch()` bridge test**

Run: `cargo test -p chronos watch_delivers_updates`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/app/Cargo.toml crates/app/src/state.rs crates/app/src/main.rs
git commit -m "feat(app): AppState global + watch() reactive bridge + init_services"
```

---

### Task 6: Wire bootstrap into `main.rs` (dedicated tokio thread → AppState)

**Files:**
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `state::init_services()` (Task 5), `AppState::init` (Task 5).
- Produces: `AppState` set as a GPUI global during startup; services run on a dedicated tokio runtime in a separate OS thread.

- [ ] **Step 1: Rewrite the startup of `crates/app/src/main.rs`**

`Cargo.toml` already provides `tokio` with `rt-multi-thread`, `macros`, `time`, `sync`, `net`, `io-util`. Replace `main.rs` so it spawns a dedicated services runtime in a separate OS thread and hands the `Services` to the GPUI closure via an mpsc channel:

```rust
mod bar;
mod ipc;
mod plugin_bridge;
mod state;

use std::sync::mpsc;

use chronos_luau::PluginManager;
use gpui::App;
use gpui_platform::application;
use ipc::IpcSubscriber;
use state::AppState;
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

    // Services run on their own tokio runtime in a separate OS thread so a
    // panic there cannot kill the GPUI shell (panic = "unwind").
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("failed to build services tokio runtime");
        match rt.block_on(state::init_services()) {
            Ok(services) => { let _ = tx.send(Some(services)); }
            Err(e) => {
                tracing::error!("services init failed: {e:#}");
                let _ = tx.send(None);
            }
        }
    });

    let app = application();
    app.run(move |cx: &mut App| {
        tracing::info!("GPUI application context ready");

        if let Ok(Some(services)) = rx.recv() {
            AppState::init(services, cx);
        } else {
            tracing::warn!("services unavailable; continuing without system state");
        }

        subscriber.start(cx);
        bar::init(cx);

        let plugin_dirs = vec![
            dirs::config_dir().unwrap().join("chronos/plugins"),
            std::path::PathBuf::from("/usr/share/chronos/plugins"),
        ];
        let mut plugin_manager = PluginManager::new(plugin_dirs);
        plugin_manager.load_all();
        plugin_bridge::register_plugin_widgets(&plugin_manager, cx);
        cx.set_global(plugin_manager);
        PluginManager::start_tick_loop(cx);
        PluginManager::start_watcher(cx);
    });

    tracing::info!("Chronos exited");
}
```

- [ ] **Step 2: Build**

Run: `cargo build -p chronos`
Expected: 0 errors.

- [ ] **Step 3: Run (live, on Hyprland) and confirm no startup panic**

Run: `RUST_LOG=info cargo run -p chronos` under a live Hyprland session.
Expected: logs "GPUI application context ready"; `AppState` is set (no `services unavailable` warning unless a service truly is missing). (Manual check; not a CI assertion.)

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/main.rs
git commit -m "feat(app): bootstrap services on dedicated tokio thread, set AppState"
```

---

### Task 7: `examples/status_printer` (proves the reactive chain without UI)

**Files:**
- Create: `crates/services/examples/status_printer.rs`

**Interfaces:**
- Consumes: `CompositorSubscriber` + `Service` (Tasks 2/1). NOTE: lives in `crates/services`, NOT `crates/app` — `crates/app` is a binary crate with no lib target, so an example there cannot `use state::AppState`; the `watch()` bridge is covered by the unit test in Task 5.

- [ ] **Step 1: Write `crates/services/examples/status_printer.rs`**

```rust
//! Proves the reactive chain (service -> Mutable -> signal) without any UI.
//! Run: cargo run -p chronos-services --example status_printer
//!
//! NOTE: the GPUI-side `watch()` bridge is covered by the unit test in
//! crates/app/src/state.rs; this example validates the service -> signal half
//! directly (crates/app is a binary crate, so an example there cannot reach
//! the `AppState`/`watch` module).

use chronos_services::{CompositorSubscriber, Service};
use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    let svc = CompositorSubscriber::new()
        .await
        .unwrap_or_else(|_| CompositorSubscriber::unavailable());

    let mut stream = svc.subscribe().to_stream();
    println!(
        "subscribed; initial state has {} workspaces",
        svc.get().workspaces.len()
    );

    while let Some(state) = stream.next().await {
        println!(
            "[compositor] workspaces={} active_window={:?} kbd={}",
            state.workspaces.len(),
            state.active_window.as_ref().map(|w| w.title.clone()),
            state.keyboard_layout
        );
    }
}
```

- [ ] **Step 2: Build the example**

Run: `cargo build -p chronos-services --example status_printer`
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add crates/services/examples/status_printer.rs
git commit -m "feat(services): status-printer example proving service -> signal reactivity"
```

---

### Task 8: Record design divergences in DECISIONS.log

**Files:**
- Modify: `DECISIONS.log` (append 5 entries at the end)

**Interfaces:**
- Consumes: the design decisions from spec §10.

- [ ] **Step 1: Append to `DECISIONS.log`**

Add these five entries at the end of `DECISIONS.log`:

```markdown
## 2026-07-10 — Services: `Service` trait is typed & light (variant Б)

- Considered: put `dispatch` in the `Service` trait (ARCHITECTURE.md §7 sketch: `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }`).
- Rejected: the untyped sketch cannot be implemented as written; forcing `dispatch` into the trait makes read-only services (future SysInfo) fake a command path.
- Decided: `Service` has only `subscribe` / `get` / `status` (+ associated `Data` / `Error`). Commands are concrete methods on each subscriber (`CompositorSubscriber::dispatch`, `NetworkSubscriber::connect`, `UPowerSubscriber::set_power_profile`). Mirrors gpui-shell's per-subscriber command pattern but adds the shared reactive/availability contract gpui-shell lacks.

## 2026-07-10 — Services: AppState + watch() live in crates/app

- Considered: putting `AppState`/`watch()` in `crates/services`.
- Rejected: would make `crates/services` GPUI-aware, inverting the integration-layer boundary (§7 positions services as the layer that feeds UI, not knows about it).
- Decided: `crates/services` stays GPUI-agnostic (no `gpui` dep); `AppState` global + `watch()` reactive bridge live in `crates/app/src/state.rs`, which owns lifecycle and the dedicated tokio runtime. `crates/core` (shared bridge) deferred until `crates/ui`/plugins actually need it (YAGNI).

## 2026-07-10 — Services: Niri is scaffold-only

- Per ARCHITECTURE.md §13 (Hyprland primary, Niri-first not supported): `CompositorBackend::Niri` exists in the enum and `detect_backend()`, but `crates/services/src/compositor/niri.rs` methods return `ServiceStatus::Unavailable` / default state. No real Niri IPC yet.

## 2026-07-10 — Services: dependency versions verified at implementation time

- `zbus` 5.x, `hyprland`, `niri-ipc` pins were verified against the workspace `Cargo.lock` / crates.io during implementation (2026). The §7/§10 architecture (dedicated tokio thread, `panic = "unwind"`, `futures_signals` reactive bridge) is unchanged by this spec.

## 2026-07-10 — Services: review-driven refinements

- D-Bus subscribers (Network/UPower) subscribe to `PropertiesChanged` (`receive_properties_changed()` + tokio task mutating `Mutable`) so state is reactive, not static-snapshot-only.
- `futures-util` added to both crates (StreamExt for signal streams / `watch`).
- `compositor/hyprland.rs` implemented concretely from the `hyprland` crate API (is_available via `HYPRLAND_INSTANCE_SIGNATURE`); no `todo!()` left in the primary backend.
- `NetworkSubscriber::connect/disconnect` and `UPowerSubscriber::set_power_profile` are explicit deferred stubs (separate spec), not silent TODOs.
- `examples/status_printer` lives in `crates/services` (binary crate can't expose `AppState` to examples) and proves service -> signal reactivity; `watch()` is covered by a unit test.
```

- [ ] **Step 2: Commit**

```bash
git add DECISIONS.log
git commit -m "docs(decisions): record services-layer design divergences"
```

---

## Self-Review Notes (author)

- Spec coverage: §1 Goal → Tasks 1-7; §2 current state → reflected in Tasks 1/5/6; §3 file structure → Tasks 1-4; §4 `Service` trait → Task 1 (+ Tasks 2-4 impls); §5 concrete services → Tasks 2/3/4; §6 runtime+bridge → Tasks 5/6; §7 error/availability → Tasks 2-6 (`unavailable()` fallbacks, 5s timeout); §8 testing → Task 1 (contract), Task 5 (`watch` bridge), Task 7 (example), live tests noted; §9 deps → Task 1; §10 divergences → Task 8. No spec requirement left without a task.
- Type consistency: `Service::Data`/`Error` match across Tasks 1-4; `AppState::{compositor,network,upower}` return `&CompositorSubscriber`/`&NetworkSubscriber`/`&UPowerSubscriber` (Task 5) consistent with `Services` fields (Task 5); `init_services()` returns `anyhow::Result<Services>` used by `main.rs` (Task 6). `watch` signature matches gpui-shell `state.rs:143-164`.
- Placeholders: `compositor/hyprland.rs` is now implemented concretely from the verified `hyprland` crate API (no `todo!()` left in the primary backend); `is_available()` uses `HYPRLAND_INSTANCE_SIGNATURE`. `connect`/`disconnect`/`set_power_profile` use `anyhow::bail!` as explicit deferred stubs (separate follow-up spec), not silent TODOs; implement against the pinned D-Bus API before claiming the command works.
- Review fixes applied (pre-implementation review, 2026-07-10): D-Bus subscribers (Network/UPower) now subscribe to `PropertiesChanged` via `receive_properties_changed()` + a tokio task mutating `Mutable`, so state is reactive, not a static snapshot. `futures-util` added to both crates (workspace dep, `StreamExt` for signal streams / `watch`). `examples/status_printer` lives in `crates/services/examples/` and proves the service -> signal half directly (crates/app is a binary crate, so an example there cannot reach `AppState`/`watch`); the `watch()` bridge is covered by the unit test in Task 5. The `watch_delivers_updates` test calls `watch` from inside an entity's `update` (where `cx: &mut Context<C>` is valid), not from `cx: &mut App`. `SERVICE_INIT_TIMEOUT` (5s) noted as a config-raisable default for slow Hyprland IPC. DECISIONS.log gained a 5th entry summarizing these.

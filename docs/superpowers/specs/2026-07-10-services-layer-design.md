# Chronos Services Layer — Design Spec

> Status: approved design (brainstorming complete)
> Date: 2026-07-10
> Scope: `crates/services` integration layer — unified `Service` trait plus `Compositor` / `Network` / `UPower` subscribers — and the `AppState` / `watch()` reactive bridge in `crates/app`. No bar widgets are built in this spec (separate spec, depends on `crates/ui`). Tray, notifications, bluetooth, audio, mpris, sysinfo are deferred to their own specs.
> Branch: `feat/services`
> Relation: implements ARCHITECTURE.md §7 (unified `Service` trait) and §10 (dedicated tokio runtime + `futures_signals` reactive bridge). This spec supersedes the untyped `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }` sketch in §7 — see §10.

## 1. Goal

Build the foundational system-integration layer that later bar widgets (workspaces, network, battery) consume without recompiling the core:

- A unified, lightweight `Service` contract shared by all subscribers.
- Three concrete, GPUI-agnostic subscribers: `CompositorSubscriber` (Hyprland IPC), `NetworkSubscriber` (NetworkManager via D-Bus), `UPowerSubscriber` (UPower via D-Bus).
- A reactive bridge (`watch()`) owned by `crates/app` that pipes `futures_signals` state into GPUI views. No `Mutex` in view code.

This layer is the dependency foundation for `dock` / `osd` / `launcher` / `notifications`. It deliberately stops at the integration + bridge layer; UI widgets are a separate spec.

## 2. Current state (verified in source)

- `crates/services` does NOT exist yet (ARCHITECTURE.md §3 lists it as "not yet created").
- `crates/app` already has: `PluginManager` as a GPUI global (`crates/luau/src/manager.rs:10`), a bar scaffold (`crates/app/src/bar/`), and a single-instance IPC socket (`crates/app/src/ipc/`). None conflict with this design.
- `crates/luau/src/api/mod_net.rs` and `mod_ipc.rs` are **plugin-facing** APIs (`chronos.net`, `chronos.ipc`) — a separate concern from the internal services layer. They must not share code with `crates/services`; document the boundary, do not couple them.
- Reference `reference/gpui-shell/crates/services/` was studied: each service is a `{X}Subscriber` holding `Mutable<{X}Data>`; `AppState` global lives in `crates/app/src/state.rs`; a `watch()` helper bridges `Signal -> GPUI Context`. We mirror that structure but ADD the unified `Service` trait (gpui-shell lacks it — ARCHITECTURE.md §7 notes gpui-shell "lacks this").

## 3. File structure (Layout A — per-service submodules, mirror gpui-shell)

```
crates/services/
  Cargo.toml                 # zbus, futures-signals, tokio(rt-multi-thread), hyprland, niri-ipc, chrono
  src/
    lib.rs                   # pub trait Service, ServiceStatus, re-export subscribers
    compositor/
      mod.rs                 # CompositorSubscriber + Service impl + detect_backend()
      hyprland.rs            # fetch_full_state / start_listener / execute_command (PRIMARY)
      niri.rs                # scaffold: detect + stubbed methods (§13: Niri not primary)
      types.rs               # CompositorState, Workspace, ActiveWindow, Monitor, CompositorBackend, CompositorCommand
    network/
      mod.rs                 # NetworkSubscriber + Service impl (zbus -> NetworkManager)
      types.rs               # NetworkData, ConnectivityState, NetworkCommand
    upower/
      mod.rs                 # UPowerSubscriber + Service impl (zbus -> UPower)
      types.rs               # UPowerData, BatteryState, PowerProfile, UPowerCommand
crates/app/src/state.rs      # AppState global + watch() (mirrors gpui-shell; see §6)
```

D-Bus specifics (Connection, proxies) live INSIDE `network/mod.rs` and `upower/mod.rs` — no separate `dbus/` module (YAGNI: only two D-Bus services today, different interfaces).

## 4. The `Service` trait (light unification, variant Б)

```rust
use futures_signals::signal::{MutableSignalCloned, Signal};

pub trait Service: Send + Sync + 'static {
    /// Snapshot type. Must be cheaply clonable.
    type Data: Clone + 'static;
    /// Service-specific error type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Reactive signal that emits on every state change.
    fn subscribe(&self) -> MutableSignalCloned<Self::Data>;

    /// Current snapshot.
    fn get(&self) -> Self::Data;

    /// Availability of the service (see `ServiceStatus`).
    fn status(&self) -> ServiceStatus;
}
```

Commands are NOT part of the trait (this is variant Б — light unification). Commands are concrete methods on each subscriber: `CompositorSubscriber::dispatch(cmd)`, `NetworkSubscriber::connect(ssid, pw)`, `UPowerSubscriber::set_power_profile(p)`. Read-only subscribers simply omit command methods. This avoids forcing read-only services (e.g. future `SysInfo`) to fake a `dispatch`.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceStatus {
    Available,
    Unavailable,
    Degraded(String), // reason, e.g. "NetworkManager present but wifi unavailable"
}
```

Rationale: a single, compiler-checked contract for *availability + reactivity* across all services, without over-uniforming command dispatch (which the bar never needs polymorphically). This diverges from the §7 sketch, which put `dispatch` in the trait — see §10.

## 5. Concrete services

### 5.1 CompositorSubscriber

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositorBackend { Hyprland, Niri }

pub struct CompositorState {
    pub backend: CompositorBackend,
    pub workspaces: Vec<Workspace>,
    pub active_window: Option<ActiveWindow>,
    pub monitors: Vec<Monitor>,
    pub keyboard_layout: String,
}

pub enum CompositorCommand {
    FocusWorkspace(u32),
    NextWorkspace,
    PrevWorkspace,
    MoveToWorkspace(u32),
}
```

- `CompositorSubscriber::new() -> Result<Self>`: `detect_backend()` picks Hyprland or Niri. Hyprland path uses the `hyprland` crate: `fetch_full_state()` for initial `CompositorState`, then `start_listener(data.clone())` runs a **dedicated sync thread** with incremental handlers that mutate the `Mutable<CompositorState>` (mirrors `reference/gpui-shell/crates/services/src/compositor/mod.rs`). `dispatch(cmd)` calls `hyprland::execute_command(cmd)`.
- **Niri = scaffold only** (ARCHITECTURE.md §13: Hyprland is primary, Niri-first not supported). `niri.rs` provides `is_available()` / `fetch_full_state()` / `start_listener()` / `execute_command()` with `todo!()`-free stubs returning `ServiceStatus::Unavailable` / empty state, so the `enum CompositorBackend` and `detect_backend()` exist but Niri is not wired.
- Implements `Service` with `Data = CompositorState`, `Error = anyhow::Error`.

### 5.2 NetworkSubscriber

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityState { Unknown, None, Portal, Limited, Full }

pub struct NetworkData {
    pub connectivity: ConnectivityState,
    pub wifi_ssid: Option<String>,
    pub wifi_strength: Option<u8>, // 0..=100
}
```

- Built on `zbus` against NetworkManager on the **system bus**. `new()` connects, fetches initial `NetworkData`, subscribes to state/AP signals that mutate `Mutable<NetworkData>`.
- Command methods (concrete, not in trait): `connect(&self, ssid: &str, password: &str) -> Result<(), Error>`, `disconnect(&self) -> Result<(), Error>`.
- Implements `Service` with `Data = NetworkData`.

### 5.3 UPowerSubscriber

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryState { Unknown, Charging, Discharging, Full, Empty }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerProfile { Performance, Balanced, PowerSaver }

pub struct UPowerData {
    pub battery_percent: f64,
    pub state: BatteryState,
    pub power_profile: PowerProfile,
}
```

- Built on `zbus` against UPower on the **system bus**. `new()` connects, reads initial `UPowerData`, subscribes to property changes.
- Command method (concrete): `set_power_profile(&self, profile: PowerProfile) -> Result<(), Error>`.
- Implements `Service` with `Data = UPowerData`.

## 6. Runtime & reactive bridge (ARCHITECTURE.md §10)

- Bootstrap lives in `crates/app`. `crates/app` spawns a **dedicated `tokio::runtime::Runtime` (multi-thread) inside a `std::thread::spawn`** (separate OS thread). Inside that thread it calls `services::init_all()` which constructs the three subscribers (their `new()` may be async — zbus connect runs on the tokio runtime). The constructed `Services` container is returned to the GPUI main thread, where `AppState::init(services, cx)` sets it as a GPUI global.
- Each subscriber holds `Mutable<Data>`. D-Bus signal callbacks (tokio tasks) or the compositor listener thread call `.set()` — `Mutable` is `Send + Sync`, so writing from a non-GPUI thread is safe.
- `watch()` helper (in `crates/app/src/state.rs`, mirrors `reference/gpui-shell/crates/app/src/state.rs:143-164`):

```rust
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
            if this.update(cx, |this, cx| on_update(this, data.clone(), cx)).is_err() {
                break;
            }
        }
    })
    .detach();
}
```

- `AppState` (GPUI global) holds the `Services` container and exposes `compositor(cx)`, `network(cx)`, `upower(cx)` returning `&{X}Subscriber`. View code pattern: `let sig = cx.global::<AppState>().compositor().subscribe(); watch(cx, sig, |view, state, cx| { ... });`. **No `Mutex` in view code.**
- `panic = "unwind"` (§7/§10): a panic in a service listener thread must not kill the shell; services use `Result`/`expect` rigorously.

## 7. Error handling & availability

- A service that fails to initialize (no D-Bus / UPower absent / NetworkManager absent) returns `ServiceStatus::Unavailable` and its `Mutable` holds a default `Data`. The bar simply omits the corresponding widget. **No panics at startup.**
- `services::init_all()` uses a ~5s timeout per async init (mirror gpui-shell `SERVICE_INIT_TIMEOUT`); optional services fall back to `unavailable()`.
- `CompositorSubscriber::new()` errors only if no compositor is detected; on Hyprland this is expected to succeed, otherwise `Unavailable`.
- `Service::Error` is surfaced via `tracing::{warn,error}`; never silently swallowed.

## 8. Testing

- **Unit — `Service` contract:** a fake subscriber implements `Service`; mutate its `Mutable` and assert `subscribe()` emits the new value (clone-equality).
- **Unit — `watch()` bridge:** under `#[gpui::test]` / `TestAppContext`, build `AppState` with a subscriber, mutate `Mutable`, assert `on_update` fired with the new snapshot.
- **Integration (live, gated):** under a real Hyprland session, switch workspace → `CompositorState` updates → `watch()` delivers. Under running NetworkManager/UPower, assert initial state is fetched. These are NOT run in CI by default (require a session/bus) — mark with a `#[ignore]` or a feature flag.
- **`examples/status-printer`:** a minimal binary (under `crates/services/examples/` or `crates/app/examples/`) that constructs `AppState`, subscribes via `watch()`, and prints status updates to stdout. Proves the full reactive chain without pulling UI dependencies into the main binary. Not compiled into the shipping binary.

## 9. Dependencies

`crates/services/Cargo.toml` adds (as workspace deps where available): `zbus` (D-Bus, version 5.x family — verify exact pin against the workspace lockfile), `futures-signals`, `tokio` with `rt-multi-thread` + `time` + `macros`, `hyprland` (Hyprland IPC), `niri-ipc` (scaffold only), `chrono`. `crates/app` gains `futures-signals` if not already present (it already depends on `gpui`).

> NOTE: version/protocol facts may have changed by 2026. Verify exact crate versions against the workspace `Cargo.lock` / network before implementation; do not hard-pin to memory.

## 10. Divergences from ARCHITECTURE.md — decisions to append to DECISIONS.log

1. **`Service` trait is typed and light (variant Б).** Only `subscribe` / `get` / `status` (+ associated `Data` / `Error`). Commands are concrete subscriber methods, NOT in the trait. This supersedes the untyped §7 sketch `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }`, which cannot be implemented as written.
2. **`AppState` + `watch()` live in `crates/app`** (Approach 1); `crates/services` stays GPUI-agnostic. Services never depend on `gpui`.
3. **Niri = scaffold-only** (ARCHITECTURE.md §13 confirmed): `CompositorBackend::Niri` exists in the enum and `detect_backend()`, but `niri.rs` methods are stubs returning `Unavailable` / empty state.
4. **Deps verified at implementation time** (2026): `zbus` 5.x, `hyprland`, `niri-ipc` — confirm pins against the lockfile/network; the §7/§10 architecture (dedicated tokio thread, `panic = "unwind"`, `futures_signals` bridge) is unchanged.

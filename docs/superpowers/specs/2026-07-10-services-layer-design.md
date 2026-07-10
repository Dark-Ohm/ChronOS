# Chronos Services Layer — Design Spec (v2)

> Status: revised design (brainstorming complete; v2 amendments applied 2026-07-10)
> Date: 2026-07-10
> Scope: `crates/services` integration layer — unified `Service` trait plus `Compositor` / `Network` / `UPower` subscribers — and the `AppState` / `watch()` reactive bridge in `crates/app`. No bar widgets are built in this spec (separate spec, depends on `crates/ui`). Tray, notifications, bluetooth, audio, mpris, sysinfo are deferred to their own specs.
> Branch: `feat/services`
> Relation: implements ARCHITECTURE.md §7 (unified `Service` trait) and §10 (dedicated tokio runtime + `futures_signals` reactive bridge). This spec supersedes the untyped `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }` sketch in §7 — see §10.
> v2 amendments over the original 2026-07-10 spec: (1) all constructors are non-failing `fn new() -> Self`; (2) `ServiceStatus` is now `Initializing | Available | Unavailable | Degraded(String)`; (3) `subscribe()` returns `impl Signal<Item = Data> + Unpin + 'static`; (4) two explicit thread models — async `tokio::spawn` + `Handle::current()` for D-Bus services, plain `std::thread` + `catch_unwind` for the compositor; (5) `init_all()` must be called inside the runtime via `rt.block_on`; (6) per-service `connect_timeout` replaces the global init timeout.

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
use futures_signals::signal::Signal;

pub trait Service: Send + Sync + 'static {
    /// Snapshot type. Must be cheaply clonable.
    type Data: Clone + 'static;
    /// Service-specific error type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Reactive signal that emits on every state change.
    /// Returns `impl Signal` (not `MutableSignalCloned`) so consumers cannot call `.set()`.
    fn subscribe(&self) -> impl Signal<Item = Self::Data> + Unpin + 'static;

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
    /// Created; first connection attempt pending (set by `new()`).
    Initializing,
    /// Fully functional.
    Available,
    /// All connection attempts failed; retry loop running in background.
    Unavailable,
    /// Connected but some features missing (e.g. NM present, Wi-Fi hardware absent).
    Degraded(String),
}
```

Rationale: a single, compiler-checked contract for *availability + reactivity* across all services, without over-uniforming command dispatch (which the bar never needs polymorphically). This diverges from the §7 sketch, which put `dispatch` in the trait — see §10.

## 5. Constructor contract (non-failing, two thread models)

All three constructors are **`fn new() -> Self`** (synchronous, non-failing). A constructor never returns `Result` and never blocks on a real connection — it returns an object in `ServiceStatus::Initializing` and starts background work. There are **two distinct thread models**; which one a service uses is NOT uniform across the three.

### 5.1 D-Bus services (NetworkSubscriber, UPowerSubscriber) — async template

Inside `new()`:
1. Create `Mutable<Data>` with `Data::default()` and `Mutable<ServiceStatus>` set to `Initializing`.
2. Capture `let handle = tokio::runtime::Handle::current();` — this is valid **only because `init_all()` is called inside the runtime context** (see §7; `Handle::current()` panics if called outside a `block_on`/`enter` guard).
3. `tokio::spawn(run(handle, data, status))` and return `Self` immediately.

`run` is the shared async connect + retry loop:

```rust
async fn run(handle: Handle, data: Mutable<Data>, status: Mutable<ServiceStatus>) {
    let mut backoff = Duration::from_secs(1);
    const MAX: Duration = Duration::from_secs(60);
    loop {
        match connect_once(&handle, &data).await {
            Ok(()) => {
                status.set(ServiceStatus::Available); // or Degraded(reason) if partial
                // Subscribe to signals on the single shared zbus::Connection.
                // On stream error (incl. zbus::Error::Disconnected) -> break to retry.
                match stream_signals(&handle, &data, &status).await {
                    Ok(()) => return, // stream ended cleanly (shutdown)
                    Err(e) => {
                        tracing::warn!("signal stream ended, retrying: {e:#}");
                        status.set(ServiceStatus::Unavailable);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("connect failed, retrying: {e:#}");
                status.set(ServiceStatus::Unavailable);
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX);
    }
}
```

- Exponential backoff 1s → 2s → 4s … capped at 60s, **infinite** retries.
- `connect_once` has an internal `connect_timeout` (~5s) so a hung bus never blocks the loop.
- A live connection dropping (`zbus::Error::Disconnected` or any stream error) breaks back into the retry loop with status `Unavailable`.

### 5.2 CompositorSubscriber (Hyprland) — sync-thread model (EXCEPTION)

The Compositor does **NOT** use the async template and does **NOT** call `Handle::current()`. Hyprland IPC is a blocking, socket-event-stream protocol; it runs on a **dedicated plain `std::thread`**, not a tokio task. Inside `new()`:
1. `detect_backend()`; if no backend → `Unavailable` immediately, no thread spawned.
2. Create `Mutable<CompositorState>` with default state, `Mutable<ServiceStatus>` = `Initializing`.
3. `std::thread::spawn(move || run_sync(data, status))` and return `Self` immediately. `run_sync` is a **sync** connect + retry loop: 3–5 attempts with delay, then stays `Unavailable` and periodically re-probes. The listener body is wrapped in `std::panic::catch_unwind`; on panic the thread is **restarted** from the same retry mechanism (a panic must not kill the shell).

Niri is a scaffold stub: `detect_backend()` may return `CompositorBackend::Niri`, but `niri::*` methods return `Unavailable` / empty state and spawn no thread.

> The shared retry *semantics* (start `Initializing`, backoff, `Unavailable` while retrying, `Available`/`Degraded` on success) apply to BOTH models. Only the *mechanism* differs: async `tokio::spawn` + `Handle::current()` for D-Bus services, plain `std::thread` + `catch_unwind` for the compositor.

## 6. Concrete services

### 6.1 CompositorSubscriber

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

- `CompositorSubscriber::new() -> Self` (non-failing, sync — see §4.2): `detect_backend()` picks Hyprland or Niri. Hyprland path spawns a **dedicated sync thread** running `run_sync` (§4.2) which calls `fetch_full_state()` for initial `CompositorState` then starts incremental handlers that mutate the `Mutable<CompositorState>` (mirrors `reference/gpui-shell/crates/services/src/compositor/mod.rs`). `dispatch(cmd)` calls `hyprland::execute_command(cmd)`. If no backend detected → `Unavailable`, no thread.
- **Niri = scaffold only** (ARCHITECTURE.md §13: Hyprland is primary, Niri-first not supported). `niri.rs` provides `is_available()` / `fetch_full_state()` / `start_listener()` / `execute_command()` with `todo!()`-free stubs returning `ServiceStatus::Unavailable` / empty state, so the `enum CompositorBackend` and `detect_backend()` exist but Niri is not wired.
- Implements `Service` with `Data = CompositorState`, `Error = anyhow::Error`.

### 6.2 NetworkSubscriber

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityState { Unknown, None, Portal, Limited, Full }

pub struct NetworkData {
    pub connectivity: ConnectivityState,
    pub wifi_ssid: Option<String>,
    pub wifi_strength: Option<u8>, // 0..=100
}
```

- Built on `zbus` against NetworkManager on the **system bus**. `new() -> Self` (non-failing, sync — see §4.1) starts in `Initializing`, spawns the async connect + retry loop; on first successful connect it fetches initial `NetworkData` and subscribes to state/AP signals that mutate `Mutable<NetworkData>`.
- Command methods (concrete, not in trait): `connect(&self, ssid: &str, password: &str) -> Result<(), Error>`, `disconnect(&self) -> Result<(), Error>`.
- Implements `Service` with `Data = NetworkData`.

### 6.3 UPowerSubscriber

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

- Built on `zbus` against UPower on the **system bus**. `new() -> Self` (non-failing, sync — see §4.1) starts in `Initializing`, spawns the async connect + retry loop; on first successful connect it reads initial `UPowerData` and subscribes to property changes.
- Command method (concrete): `set_power_profile(&self, profile: PowerProfile) -> Result<(), Error>`.
- Implements `Service` with `Data = UPowerData`.

## 7. Runtime & reactive bridge (ARCHITECTURE.md §10)

- Bootstrap lives in `crates/app`. `crates/app` spawns a **dedicated `tokio::runtime::Runtime` (multi-thread) inside a `std::thread::spawn`** (separate OS thread). **`init_all()` MUST be called inside the runtime context** — otherwise `Handle::current()` inside the D-Bus constructors panics. Use `rt.block_on`:

```rust
// crates/app — bootstrap thread
let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("tokio runtime");
// block_on enters the runtime context for this OS thread,
// so Handle::current() / tokio::spawn inside the constructors resolve.
let services = rt.block_on(async { services::init_all() });
// `services` is Send + Sync (Mutable + zbus::Connection) -> return to GPUI main thread
```

  (`let _guard = rt.enter();` before a sync `init_all()` call is an acceptable alternative; `block_on` is preferred.)
- The constructed `Services` container is `Send + Sync` (`Mutable` and `zbus::Connection` are both `Send + Sync`), so it crosses back to the GPUI main thread where `AppState::init(services, cx)` sets it as a GPUI global.
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

## 8. Error handling & availability

- A service that fails to initialize (no D-Bus / UPower absent / NetworkManager absent) starts in `ServiceStatus::Initializing`, then transitions to `Unavailable` and keeps its `Mutable` at default `Data` while the retry loop runs in the background. The bar simply omits the corresponding widget. **No panics at startup.**
- The v1 "~5s timeout per async init" is **removed** — constructors are instant and non-failing. Each D-Bus service applies an internal `connect_timeout` (~5s) inside `connect_once` (§4.1) so a hung bus never blocks the retry loop.
- `CompositorSubscriber::new()` returns `Unavailable` immediately (no thread) only if no compositor backend is detected; on Hyprland it spawns the listener thread and starts in `Initializing`.
- `Service::Error` is surfaced via `tracing::{warn,error}`; never silently swallowed. The last error string is logged, NOT stored in `ServiceStatus` (see §3).

## 9. Testing

- **Unit — `Service` contract:** a fake subscriber implements `Service`; mutate its `Mutable`, assert `subscribe()` (now `impl Signal`) emits the new value. Drive the signal via `futures_signals::signal::SignalExt::to_stream()` + `block_on` a single `.next().await`, or poll with a short `futures::executor`/`tokio::time::timeout` guard.
- **Unit — retry logic (NEW, required):** a fake D-Bus backend that fails N times then succeeds; assert the status sequence `Initializing → Unavailable → … → Available` and that backoff delay grows between attempts (1s, 2s, 4s … capped 60s). Also assert that a `Disconnected` mid-stream flips status back to `Unavailable` and the loop retries. This is the concrete test guarding the §4.1 design.
- **Unit — `watch()` bridge:** under `#[gpui::test]` / `TestAppContext`, build `AppState` with a subscriber, mutate `Mutable`, assert `on_update` fired with the new snapshot.
- **Integration (live, gated):** under a real Hyprland session, switch workspace → `CompositorState` updates → `watch()` delivers. Under running NetworkManager/UPower, assert initial state is fetched. These are NOT run in CI by default (require a session/bus) — mark with a `#[ignore]` or a feature flag.
- **`examples/status-printer`:** MUST be a **minimal GPUI app** (not a bare `main`), because `watch()` requires a `Context<C>`. It boots a GPUI `App`, initializes `Services` via the dedicated tokio thread (§6), subscribes through `watch()`, and logs updates to stdout. Proves the full reactive chain without pulling UI widgets into the main binary. Not compiled into the shipping binary.

## 10. Dependencies

`crates/services/Cargo.toml` adds (as workspace deps where available): `zbus` (D-Bus, version 5.x family — verify exact pin against the workspace lockfile), `futures-signals`, `tokio` with `rt-multi-thread` + `time` + `macros`, `hyprland` (Hyprland IPC), `niri-ipc` (scaffold only), `chrono`. `crates/app` gains `futures-signals` if not already present (it already depends on `gpui`).

> NOTE: version/protocol facts may have changed by 2026. Verify exact crate versions against the workspace `Cargo.lock` / network before implementation; do not hard-pin to memory.

## 11. Divergences from ARCHITECTURE.md — decisions to append to DECISIONS.log

1. **`Service` trait is typed and light (variant Б).** Only `subscribe` / `get` / `status` (+ associated `Data` / `Error`). Commands are concrete subscriber methods, NOT in the trait. This supersedes the untyped §7 sketch `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }`, which cannot be implemented as written.
2. **`AppState` + `watch()` live in `crates/app`** (Approach 1); `crates/services` stays GPUI-agnostic. Services never depend on `gpui`.
3. **Niri = scaffold-only** (ARCHITECTURE.md §13 confirmed): `CompositorBackend::Niri` exists in the enum and `detect_backend()`, but `niri.rs` methods are stubs returning `Unavailable` / empty state.
4. **Deps verified at implementation time** (2026): `zbus` 5.x, `hyprland`, `niri-ipc` — confirm pins against the lockfile/network; the §7/§10 architecture (dedicated tokio thread, `panic = "unwind"`, `futures_signals` bridge) is unchanged.
5. **v2 amendments (2026-07-10):** constructors are non-failing `fn new() -> Self` (no `Result`, no `unavailable()` fallback — the retry loop handles absence); `ServiceStatus` is `Initializing | Available | Unavailable | Degraded(String)` (no `Active`/`Error` variants); `subscribe()` returns `impl Signal<Item = Data> + Unpin + 'static`; two thread models — async `tokio::spawn` + `Handle::current()` for D-Bus services, plain `std::thread` + `catch_unwind` for the compositor; `init_all()` is called via `rt.block_on` so `Handle::current()` resolves; per-service `connect_timeout` replaces the global init timeout. These supersede the original spec's `Result<Self>` constructors, `unavailable()` fallbacks, and `MutableSignalCloned` return.

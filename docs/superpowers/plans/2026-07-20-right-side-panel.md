# Правая боковая панель — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lazy layer-shell overlay panel anchored to the right edge of the
monitor (hover-peek or pinned via hotkey/bar click) showing a full MPRIS
player, live CPU/RAM/GPU + network spectrum meters, and a power row
(log out/restart/shutdown, switch-user visible-but-disabled).

**Architecture:** New GPUI module `crates/app/src/side_panel_right/` (window
lifecycle mirrors `system_popup/`/`volume_popup/` — `Layer::Overlay`,
`KeyboardInteractivity::None`, `close_this` reentrancy guard). Two new
services in `crates/services/` (`system_resources`, `power`), one extension
to the existing `audio` service (per-app stream mute), one extraction of
existing bar-widget logic into a shared module (`net_stats`), one new
`Theme` field (`font_ui`).

**Tech Stack:** Rust, GPUI (`gpui-ce`, workspace git dep), `sysinfo` 0.39,
`nvml-wrapper` 0.12, existing `futures-signals`/`tokio` service pattern.

## Global Constraints

- Every new/modified crate keeps `[lints] workspace = true` — `unsafe_code
  = deny`, `unwrap_used`/`expect_used = warn`.
- Never `let _ = fallible_call()`. Propagate with `?`, or `.log_err()` if
  the result is deliberately ignored, or an explicit `match`/`if let Err`.
- Any value computed from elapsed time (spectrum-bar samples) must be
  immune to `render()` call frequency: time-gate + cached last value +
  `Instant`/`Duration` injected as parameters, exactly the pattern in
  `crates/services/src/net_stats.rs` (`update_speed`, `SAMPLE_INTERVAL`).
  Bar widget re-exports usage via `chronos_services::net_stats` (Task 1 done).
- Popup/panel window lifecycle follows `ARCHITECTURE.md §4.1`: explicit
  dismiss only, `close_this` for in-window close (never re-entrant
  `handle.update` for `remove_window()`). **Correction discovered during
  planning:** every existing popup in this codebase (`volume_popup`,
  `system_popup`, `updates_popup`) uses `KeyboardInteractivity::None` and
  implements **no** Esc-to-close path — Esc is not actually a working
  dismiss path anywhere in the tree today. The side panel follows the same
  real convention: dismiss = re-toggle / click-away (pinned) / mouse-leave
  debounce (peek). Esc is dropped from scope, not deferred — do not add it.
- Monitor selection always via `crate::monitor::pult_display(cx)`, never
  `window.display(cx)`.
- **No login/display manager exists on this system** (TTY-autologin →
  Hyprland directly; a custom login manager is a separate future project).
  "Switch user" ships as a visible, disabled button. Do not write a
  backend for it — there is nothing to hand a session to yet.
- **MPRIS data correction discovered during planning:** `MprisState`
  (`crates/services/src/mpris/types.rs`) exposes only `title`, `artist`,
  `playing`, `has_player`, `player_count`, `player_index`, `player_id` —
  **no album art URL, no position, no track length.** The brainstormed
  mockup's progress bar + timecode and real album art are not backed by
  any data source today. v1 ships a static gradient placeholder swatch
  (matches the mockup's visual block, `linear-gradient` via GPUI's own
  gradient fill helper if available, else a flat accent-colored square)
  and **drops the progress bar / timecode row entirely** — do not fake
  a progress value. The "open full player" chevron is present but wired
  to a no-op — do not build any expanded-player surface in this plan.
- Cool blue-cyan palette only for panel metrics — CPU `#5fd3e8`, RAM
  `#4fa3c9`, GPU `#33638a`, network down `#7cc4e8`, network up `#3d6d94`.
  No other hues anywhere in the panel's meters (this was explicitly
  rejected by the user — "не гей-парад").
- Fonts: `Inter` for UI text (new `Theme::font_ui` field, default
  `"Inter"`), `JetBrains Mono` for numbers/labels (existing
  `Theme::font_mono`, unchanged).
- Panel geometry: width `300px`, `Layer::Overlay`, anchor
  `RIGHT | TOP | BOTTOM`, `exclusive_zone: Some(px(0.))` (never reserves
  screen space, unlike the bar).

### Progress (evidence, not hope)

| Task | Status | Commit / evidence |
|---|---|---|
| 1 `net_stats` | **DONE** | `dbce8ac` — `crates/services/src/net_stats.rs` + bar rewire |
| 2 `Theme::font_ui` | **DONE** | `18c88f0` — `font_ui: "Inter"`, test `theme_default_font_ui_is_inter` |
| 3–5 | open | — |
| 6 audio stream mute | **code ready, uncommitted** | `audio/{types,pw_dump,mod}.rs` only; report `zed-report-6.md` |
| 7 `side_panel_right` skeleton | **DONE** | `da744a2` — `side_panel_right/{mod,view}.rs`, `main.rs` mod+init |
| 8–12 | open | mute UI = Task 9 |

Verify Task 1 tests (package name is **`chronos`**, not `chronos-app`; bar
lives in the **binary**, not `lib`):

```bash
cargo test -p chronos-services --lib net_stats          # 5 tests
cargo test -p chronos --bin chronos bar::widgets::network  # 14 UI tests
```

---

## Task 1: Extract network speed sampling into a shared module

> **DONE `dbce8ac` (2026-07-21).** Steps below left as a record; checkboxes
> marked complete. Do not re-run the extract.

**Files:**
- Create: `crates/services/src/net_stats.rs`
- Modify: `crates/services/src/lib.rs:1-30` (add `pub mod net_stats;`)
- Modify: `crates/app/src/bar/widgets/network.rs`
- Test: inline `#[cfg(test)]` in `crates/services/src/net_stats.rs`

**Interfaces:**
- Produces: `pub struct NetSample { pub rx: u64, pub tx: u64, pub time:
  Instant }`, `pub struct NetState { pub sample: Option<NetSample>,
  pub cached_dl: f64, pub cached_ul: f64 }` (with `impl Default`),
  `pub struct NetSpeed { pub dl: f64, pub ul: f64 }`,
  `pub fn read_interface_bytes() -> std::io::Result<(u64, u64)>`,
  `pub fn update_speed(state: &mut NetState, rx: u64, tx: u64, now:
  Instant, min_interval: Duration) -> NetSpeed`,
  `pub const SAMPLE_INTERVAL: Duration`.
- Consumes: nothing new — this task only moves existing private logic
  from `crates/app/src/bar/widgets/network.rs` (`read_interface_bytes`,
  `Sample`, `NetworkState`, `update_speed`, `SpeedSample`,
  `SAMPLE_INTERVAL`) into `chronos_services`, renaming `Sample` →
  `NetSample`, `NetworkState` → `NetState`, `SpeedSample` → `NetSpeed`
  to avoid clashing with the bar widget's own `NetworkState`-adjacent
  naming once both exist side by side.

- [x] **Step 1: Run the existing network widget tests to capture the baseline**

~~Run: `cargo test -p chronos-app --lib bar::widgets::network`~~ **stale
package name** — use:
`cargo test -p chronos --bin chronos bar::widgets::network`
Baseline at extract: **18** tests (incl. 4 `update_speed_*`).

- [x] **Step 2: Create `crates/services/src/net_stats.rs` with the moved pure logic**

```rust
//! Shared, render-frequency-immune network byte-rate sampling.
//!
//! Extracted from `crates/app/src/bar/widgets/network.rs` so the bar widget
//! and the right side panel's network spectrum meter read from one
//! implementation instead of two copies of procfs parsing drifting apart.
//!
//! ## Render immunity
//!
//! Callers (bar widget, side panel) may invoke `update_speed` many times
//! per frame. The time gate below only updates the procfs snapshot once
//! per `min_interval`; between updates the cached speed pair is returned
//! unchanged.

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Minimum time between procfs snapshot updates.
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// Snapshot of aggregate byte counters for speed computation.
pub struct NetSample {
    pub rx: u64,
    pub tx: u64,
    pub time: Instant,
}

/// Mutable state for one sampler instance (bar and panel each hold their own).
pub struct NetState {
    pub sample: Option<NetSample>,
    pub cached_dl: f64,
    pub cached_ul: f64,
}

impl Default for NetState {
    fn default() -> Self {
        Self {
            sample: None,
            cached_dl: 0.0,
            cached_ul: 0.0,
        }
    }
}

/// Result of a speed update (bytes/sec).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetSpeed {
    pub dl: f64,
    pub ul: f64,
}

/// Reads total rx/tx bytes from all non-loopback interfaces.
pub fn read_interface_bytes() -> io::Result<(u64, u64)> {
    let net_dir = Path::new("/sys/class/net");
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;

    for entry in std::fs::read_dir(net_dir)? {
        let entry = entry?;
        let ifname = entry.file_name();
        let ifname = ifname.to_string_lossy();

        if ifname == "lo" {
            continue;
        }

        if let Ok(rx_str) =
            std::fs::read_to_string(entry.path().join("statistics").join("rx_bytes"))
        {
            if let Ok(rx) = rx_str.trim().parse::<u64>() {
                total_rx += rx;
            }
        }
        if let Ok(tx_str) =
            std::fs::read_to_string(entry.path().join("statistics").join("tx_bytes"))
        {
            if let Ok(tx) = tx_str.trim().parse::<u64>() {
                total_tx += tx;
            }
        }
    }

    Ok((total_rx, total_tx))
}

/// Update network speed state from fresh byte counters.
///
/// `min_interval` controls the time gate — if less time has elapsed since
/// the last sample, cached speeds are returned without touching the
/// snapshot. Pass `SAMPLE_INTERVAL` in production; inject shorter
/// durations in tests.
pub fn update_speed(
    state: &mut NetState,
    rx: u64,
    tx: u64,
    now: Instant,
    min_interval: Duration,
) -> NetSpeed {
    match state.sample {
        Some(ref prev) => {
            let elapsed = now.duration_since(prev.time);
            if elapsed < min_interval {
                return NetSpeed {
                    dl: state.cached_dl,
                    ul: state.cached_ul,
                };
            }
            let elapsed_secs = elapsed.as_secs_f64();
            let dl = (rx.saturating_sub(prev.rx)) as f64 / elapsed_secs;
            let ul = (tx.saturating_sub(prev.tx)) as f64 / elapsed_secs;
            state.sample = Some(NetSample { rx, tx, time: now });
            state.cached_dl = dl;
            state.cached_ul = ul;
            NetSpeed { dl, ul }
        }
        None => {
            state.sample = Some(NetSample { rx, tx, time: now });
            state.cached_dl = 0.0;
            state.cached_ul = 0.0;
            NetSpeed { dl: 0.0, ul: 0.0 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_returns_zero_and_stores_snapshot() {
        let mut state = NetState::default();
        let now = Instant::now();
        let speed = update_speed(&mut state, 1000, 500, now, SAMPLE_INTERVAL);
        assert_eq!(speed, NetSpeed { dl: 0.0, ul: 0.0 });
        assert!(state.sample.is_some());
    }

    #[test]
    fn second_sample_after_interval_computes_real_delta() {
        let mut state = NetState::default();
        let t0 = Instant::now();
        update_speed(&mut state, 1000, 500, t0, Duration::from_millis(10));
        let t1 = t0 + Duration::from_secs(1);
        let speed = update_speed(&mut state, 2000, 1500, t1, Duration::from_millis(10));
        assert_eq!(speed, NetSpeed { dl: 1000.0, ul: 1000.0 });
    }

    #[test]
    fn sample_within_min_interval_returns_cached_value_not_fresh_delta() {
        // Regression guard for the exact bug fixed 2026-07-20 in
        // bar/widgets/network.rs: a render() call microseconds after the
        // last sample must not compute a near-zero delta.
        let mut state = NetState::default();
        let t0 = Instant::now();
        update_speed(&mut state, 1000, 500, t0, Duration::from_secs(1));
        let t1 = t0 + Duration::from_millis(5);
        let speed = update_speed(&mut state, 1001, 501, t1, Duration::from_secs(1));
        assert_eq!(speed, NetSpeed { dl: 0.0, ul: 0.0 });
    }

    // Ported verbatim in intent from the pre-extraction tests in
    // `crates/app/src/bar/widgets/network.rs` (renamed for the new field
    // names). Do NOT drop these when deleting the originals in Step 5 —
    // they cover the two cases the three tests above do not: repeated
    // same-frame calls, and u64 counter wraparound.

    #[test]
    fn repeated_calls_in_one_frame_never_collapse_the_cached_value() {
        let mut state = NetState::default();
        let t0 = Instant::now();

        let r1 = update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);
        assert_eq!(r1, NetSpeed { dl: 0.0, ul: 0.0 });

        // Simulate many render() calls inside the same frame.
        for _ in 0..10 {
            let r = update_speed(&mut state, 0, 0, t0, SAMPLE_INTERVAL);
            assert_eq!(r.dl, 0.0, "cached value collapsed between repeated calls");
            assert_eq!(r.ul, 0.0, "cached value collapsed between repeated calls");
        }

        // Advance past the gate with real counters.
        let t1 = t0 + SAMPLE_INTERVAL + Duration::from_millis(1);
        let r2 = update_speed(&mut state, 1_000_000, 500_000, t1, SAMPLE_INTERVAL);
        assert!(r2.dl > 900_000.0 && r2.dl < 1_100_000.0);
        assert!(r2.ul > 450_000.0 && r2.ul < 550_000.0);

        // Same instant → cache wins, even with wildly different counters.
        let r3 = update_speed(&mut state, 9_999_999, 8_888_888, t1, SAMPLE_INTERVAL);
        assert_eq!(r3.dl, r2.dl, "cached value changed despite different counters");
        assert_eq!(r3.ul, r2.ul, "cached value changed despite different counters");
    }

    #[test]
    fn counter_wraparound_yields_zero_then_recovers() {
        let mut state = NetState::default();
        let t0 = Instant::now();

        // First sample at near-wrap counters.
        update_speed(&mut state, u64::MAX, u64::MAX, t0, SAMPLE_INTERVAL);

        // Counters wrapped: `saturating_sub` must floor at 0, not underflow.
        let t1 = t0 + SAMPLE_INTERVAL;
        let wrapped = update_speed(&mut state, 100, 200, t1, SAMPLE_INTERVAL);
        assert_eq!(wrapped.dl, 0.0, "dl must be 0 on the wrap tick");
        assert_eq!(wrapped.ul, 0.0, "ul must be 0 on the wrap tick");

        // Next tick recovers normally from the new baseline.
        let t2 = t1 + SAMPLE_INTERVAL;
        let recovered = update_speed(&mut state, 100_000, 200_000, t2, SAMPLE_INTERVAL);
        assert!((recovered.dl - 99_900.0).abs() < 100.0);
        assert!((recovered.ul - 199_800.0).abs() < 100.0);
    }
}
```

- [x] **Step 3: Register the module in `crates/services/src/lib.rs`**

Add near the other `pub mod` declarations:

```rust
pub mod net_stats;
```

- [x] **Step 4: Run the new module's tests**

Run: `cargo test -p chronos-services --lib net_stats`
Expected: 5 tests pass (`first_sample_returns_zero_and_stores_snapshot`,
`second_sample_after_interval_computes_real_delta`,
`sample_within_min_interval_returns_cached_value_not_fresh_delta`,
`repeated_calls_in_one_frame_never_collapse_the_cached_value`,
`counter_wraparound_yields_zero_then_recovers`).

- [x] **Step 5: Rewire `crates/app/src/bar/widgets/network.rs` to use the shared module**

Remove the private `Sample`, `NetworkState`, `SpeedSample`,
`read_interface_bytes`, `update_speed`, `SAMPLE_INTERVAL` definitions from
this file. Replace the removed imports with:

```rust
use chronos_services::net_stats::{
    NetSpeed as SpeedSample, NetState as NetworkState, SAMPLE_INTERVAL, read_interface_bytes,
    update_speed,
};
```

Update every call site in this file that referenced `SpeedSample { dl_speed,
ul_speed }` field names to the new field names `NetSpeed { dl, ul }` — grep
for `.dl_speed` and `.ul_speed` in this file and rename to `.dl` / `.ul`.
Existing tests in this file that constructed `NetworkState`/`Sample`
directly must be deleted — but only after confirming each one has a
counterpart in `net_stats.rs` (Step 2). The four originals map as:

| `network.rs` original | `net_stats.rs` counterpart |
|---|---|
| `update_speed_first_call_returns_zero` | `first_sample_returns_zero_and_stores_snapshot` |
| `update_speed_computes_correct_speed` | `second_sample_after_interval_computes_real_delta` |
| `update_speed_immunity_to_frequency` | `repeated_calls_in_one_frame_never_collapse_the_cached_value` |
| `update_speed_handles_counter_wrap` | `counter_wraparound_yields_zero_then_recovers` |

Run `grep -n "fn update_speed_" crates/app/src/bar/widgets/network.rs`
before deleting and verify the list matches this table exactly. If it
contains a test not in the left column, **stop** — an extra test means
someone added coverage after this plan was written; port it to
`net_stats.rs` rather than deleting it.

Keep this file's own tests for `format_speed`, `indicator_color`,
`compute_view` — they don't touch the moved types.

- [x] **Step 6: Add `chronos-services` as a workspace path dependency check**

Run: `grep -n "chronos-services" crates/app/Cargo.toml`
Expected: a line already present (the bar widget already imports
`chronos_services::{ConnectivityState, Service}` per the file header) — no
`Cargo.toml` change needed.

- [x] **Step 7: Run both test suites and the full workspace build**

~~`cargo test -p chronos-services -p chronos-app --lib`~~ — package
`chronos-app` does not exist. Verified as:
`cargo test -p chronos-services --lib` (143) +
`cargo test -p chronos --bin chronos bar::widgets::network` (14 after move).
`cargo build --workspace` clean for zone files.

- [x] **Step 8: Commit**

```bash
git add crates/services/src/net_stats.rs crates/services/src/lib.rs crates/app/src/bar/widgets/network.rs
git commit -m "services : вынести сэмплинг сетевой скорости в net_stats — общий модуль для бара и панели"
```

Landed as **`dbce8ac`**.

---

## Task 2: Add `font_ui` to `Theme`

> **DONE 2026-07-21** — commit `18c88f0` (Архитектор за DeepSeek).
> Поле + дефолт + `theme_default_font_ui_is_inter`. Package name в
> Step 6 плана — `chronos` (не `chronos-app`).

**Files:**
- Modify: `crates/ui/src/theme/mod.rs`
- Test: inline `#[cfg(test)]` in the same file

**Interfaces:**
- Produces: `Theme::font_ui: &'static str` field, default `"Inter"`.
- Consumes: nothing new — sits next to the existing `font_mono` field.

- [x] **Step 1: Locate the existing `font_mono` field and its default**

Run: `grep -n "font_mono" crates/ui/src/theme/mod.rs`
Expected: one field declaration (`pub font_mono: &'static str,`) and one
default-construction site (`font_mono: "JetBrains Mono",`).

- [x] **Step 2: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `crates/ui/src/theme/mod.rs`:

```rust
#[test]
fn theme_default_font_ui_is_inter() {
    let theme = Theme::default();
    assert_eq!(theme.font_ui, "Inter");
}
```

- [x] **Step 3: Run test to verify it fails**

Run: `cargo test -p chronos-ui --lib theme_default_font_ui_is_inter`
Expected: FAIL with "no field `font_ui` on type `Theme`" (compile error).

- [x] **Step 4: Add the field next to `font_mono`**

In the `Theme` struct declaration, immediately after the `font_mono`
field:

```rust
    pub font_mono: &'static str,
    /// UI text (labels, titles, body copy) — everything that isn't a
    /// number or a mono-styled value. `font_mono` stays reserved for
    /// digits/code/mono-widgets per STYLE.md.
    pub font_ui: &'static str,
```

In every place `Theme { .. font_mono: "JetBrains Mono", .. }` is
constructed (the default light and dark scheme builders — grep confirmed
this is one shared default-construction site, add the field there):

```rust
            font_mono: "JetBrains Mono",
            font_ui: "Inter",
```

- [x] **Step 5: Run test to verify it passes**

Run: `cargo test -p chronos-ui --lib theme_default_font_ui_is_inter`
Expected: PASS.

- [x] **Step 6: Full crate build to confirm no other construction site broke**

Run: `cargo build -p chronos-ui -p chronos`
Expected: clean build — if there is a second, non-default `Theme { .. }`
struct literal anywhere else (e.g. a test fixture), the compiler error
names its exact file/line; add `font_ui: "Inter",` there too and rebuild.

- [x] **Step 7: Commit**

```bash
git add crates/ui/src/theme/mod.rs
git commit -m "ui : добавить Theme::font_ui (Inter) рядом с font_mono — задел под правую панель"
```

---

## Task 3: `system_resources` service — CPU/RAM via `sysinfo`

**Files:**
- Create: `crates/services/src/system_resources/mod.rs`
- Create: `crates/services/src/system_resources/types.rs`
- Modify: `crates/services/Cargo.toml` (add `sysinfo` dependency)
- Modify: `Cargo.toml` (workspace `[workspace.dependencies]`, add `sysinfo`)
- Modify: `crates/services/src/lib.rs` (register module + add to `Services`)
- Test: inline `#[cfg(test)]` in `system_resources/mod.rs`

**Interfaces:**
- Produces: `pub struct SystemResourcesState { pub cpu_percent: f32,
  pub ram_percent: f32, pub gpu_percent: Option<f32> }` (Task 4 fills
  `gpu_percent`; this task always sets it to `None`),
  `pub struct SystemResourcesSubscriber` implementing `Service<Data =
  SystemResourcesState, Error = anyhow::Error>`, `pub fn new() -> Self`.
- Consumes: `chronos_services::Service` trait (existing), the `Services`
  struct in `crates/services/src/lib.rs` (existing, add a field).

- [ ] **Step 1: Add `sysinfo` to the workspace dependency table**

In `Cargo.toml` (repo root), add to `[workspace.dependencies]` right after
the `serde_json` line:

```toml
sysinfo = "0.39.3"
```

- [ ] **Step 2: Add `sysinfo` to the services crate**

In `crates/services/Cargo.toml`, add to `[dependencies]`:

```toml
sysinfo.workspace = true
```

- [ ] **Step 3: Write `types.rs`**

```rust
//! System resources service data types (CPU/RAM/GPU).

/// Snapshot of current system resource utilization, 0.0–100.0 per field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemResourcesState {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    /// `None` when no supported GPU backend is available (non-Nvidia, or
    /// NVML failed to initialize) — the panel hides the GPU row in that case.
    pub gpu_percent: Option<f32>,
}
```

- [ ] **Step 4: Write the failing test for the pure sampling function**

In `crates/services/src/system_resources/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_cpu_ram_reads_real_host_values_in_range() {
        // Not a fixture test — sysinfo has no fixture path, this reads the
        // real host. Assert only the invariant that must always hold:
        // percentages are within [0, 100]. Live smoke (task 12) is where
        // "the number actually moves under load" gets verified.
        let mut sys = sysinfo::System::new_all();
        let sample = sample_cpu_ram(&mut sys);
        assert!(sample.cpu_percent >= 0.0 && sample.cpu_percent <= 100.0);
        assert!(sample.ram_percent >= 0.0 && sample.ram_percent <= 100.0);
    }
}
```

- [ ] **Step 5: Run test to verify it fails**

Run: `cargo test -p chronos-services --lib system_resources`
Expected: FAIL — `sample_cpu_ram` not defined.

- [ ] **Step 6: Implement `sample_cpu_ram` + the subscriber**

```rust
//! System resources service — CPU/RAM (this task) + GPU (task 4, nvml-wrapper).
//!
//! Same async-poll shape as `UPowerSubscriber`/`CavaSubscriber`: `new()`
//! captures `Handle::current()` (guard against use outside a tokio runtime)
//! and spawns a poll loop that refreshes a `Mutable<SystemResourcesState>`
//! on an interval.

use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use sysinfo::System;
use tokio::runtime::Handle;

use crate::Service;
use crate::ServiceStatus;
pub use types::SystemResourcesState;

pub mod types;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Pure sampling step — refreshes `sys` and returns the current CPU/RAM
/// percentages. Isolated from the async loop so it's testable without a
/// runtime. `sysinfo::System::refresh_cpu_all` needs two calls ~200ms apart
/// for a meaningful global CPU percentage on first read; the poll loop
/// below relies on being called repeatedly on `POLL_INTERVAL`, so the very
/// first sample after construction may read 0.0 — this is sysinfo's own
/// documented behavior, not a bug to work around here.
fn sample_cpu_ram(sys: &mut System) -> SystemResourcesState {
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let cpu_percent = sys.global_cpu_usage();
    let ram_percent = if sys.total_memory() == 0 {
        0.0
    } else {
        (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0
    };
    SystemResourcesState {
        cpu_percent,
        ram_percent,
        gpu_percent: None,
    }
}

#[derive(Clone)]
pub struct SystemResourcesSubscriber {
    data: Mutable<SystemResourcesState>,
    status: Mutable<ServiceStatus>,
}

impl SystemResourcesSubscriber {
    /// Non-failing, synchronous constructor.
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — same `Handle::current()`
    /// guard as every other subscriber in this crate.
    pub fn new() -> Self {
        let data = Mutable::new(SystemResourcesState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let _handle = Handle::current();
        tokio::spawn(run(data.clone(), status.clone()));
        Self { data, status }
    }
}

impl Service for SystemResourcesSubscriber {
    type Data = SystemResourcesState;
    type Error = anyhow::Error;
    fn subscribe(&self) -> impl Signal<Item = SystemResourcesState> + Unpin + 'static {
        self.data.signal_cloned()
    }
    fn get(&self) -> SystemResourcesState {
        self.data.get_cloned()
    }
    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

async fn run(data: Mutable<SystemResourcesState>, status: Mutable<ServiceStatus>) {
    let mut sys = System::new_all();
    status.set(ServiceStatus::Available);
    loop {
        let sample = sample_cpu_ram(&mut sys);
        data.set(sample);
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cargo test -p chronos-services --lib system_resources`
Expected: PASS.

- [ ] **Step 8: Register the module and add to `Services`**

In `crates/services/src/lib.rs`, add `pub mod system_resources;` next to
the other module declarations, add
`pub system_resources: system_resources::SystemResourcesSubscriber,` to
the `Services` struct, and
`system_resources: system_resources::SystemResourcesSubscriber::new(),`
to `init_all()`.

- [ ] **Step 9: Add the `AppState` accessor**

In `crates/app/src/state.rs`, add next to `pub fn brightness`:

```rust
    #[inline(always)]
    pub fn system_resources(cx: &App) -> &chronos_services::SystemResourcesSubscriber {
        &Self::global(cx).services.system_resources
    }
```

- [ ] **Step 10: Full workspace build**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml crates/services/Cargo.toml crates/services/src/system_resources crates/services/src/lib.rs crates/app/src/state.rs
git commit -m "services : system_resources — CPU/RAM через sysinfo, GPU-поле пока None"
```

---

## Task 4: GPU via `nvml-wrapper`

**Files:**
- Modify: `crates/services/Cargo.toml`
- Modify: `Cargo.toml` (workspace deps)
- Modify: `crates/services/src/system_resources/mod.rs`

**Interfaces:**
- Consumes: `SystemResourcesState` (Task 3, field `gpu_percent:
  Option<f32>` already exists, this task fills it).
- Produces: `fn sample_gpu(nvml: &nvml_wrapper::Nvml) -> Option<f32>` (new,
  private to the module — only `run()` calls it).

- [ ] **Step 1: Add `nvml-wrapper` to the workspace and services crate**

`Cargo.toml` (root), `[workspace.dependencies]`:

```toml
nvml-wrapper = "0.12.1"
```

`crates/services/Cargo.toml`, `[dependencies]`:

```toml
nvml-wrapper.workspace = true
```

- [ ] **Step 2: Write the failing test — graceful absence, not a panic**

```rust
    #[test]
    fn gpu_sample_none_when_nvml_unavailable_does_not_panic() {
        // On a machine without an Nvidia GPU / driver, Nvml::init() returns
        // Err — this must degrade to None, never panic or bubble an error
        // that kills the poll loop.
        let nvml = nvml_wrapper::Nvml::init();
        let sample = sample_gpu(nvml.as_ref().ok());
        // On THIS dev machine (RTX 3070, confirmed live) nvml initializes,
        // so sample is Some(_) here — the invariant under test is just
        // "never panics and stays within range" for either branch.
        if let Some(pct) = sample {
            assert!(pct >= 0.0 && pct <= 100.0);
        }
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p chronos-services --lib gpu_sample_none_when_nvml_unavailable_does_not_panic`
Expected: FAIL — `sample_gpu` not defined.

- [ ] **Step 4: Implement `sample_gpu` and wire it into `run()`**

Add to `crates/services/src/system_resources/mod.rs`:

```rust
/// Reads GPU utilization percent from device 0 via NVML. `None` on any
/// failure (no Nvidia GPU, driver mismatch, NVML not installed) — the
/// caller treats `None` as "hide the GPU row", never as an error to
/// propagate or a reason to stop polling CPU/RAM.
fn sample_gpu(nvml: Option<&nvml_wrapper::Nvml>) -> Option<f32> {
    let nvml = nvml?;
    let device = nvml.device_by_index(0).ok()?;
    let util = device.utilization_rates().ok()?;
    Some(util.gpu as f32)
}
```

Change the `run` function signature to initialize NVML once outside the
loop and pass it into `sample_gpu` each tick:

```rust
async fn run(data: Mutable<SystemResourcesState>, status: Mutable<ServiceStatus>) {
    let mut sys = System::new_all();
    let nvml = nvml_wrapper::Nvml::init().ok();
    if nvml.is_none() {
        tracing::info!("system_resources: NVML unavailable, GPU row will stay hidden");
    }
    status.set(ServiceStatus::Available);
    loop {
        let mut sample = sample_cpu_ram(&mut sys);
        sample.gpu_percent = sample_gpu(nvml.as_ref());
        data.set(sample);
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p chronos-services --lib gpu_sample_none_when_nvml_unavailable_does_not_panic`
Expected: PASS.

- [ ] **Step 6: Full test suite + build**

Run: `cargo test -p chronos-services --lib system_resources && cargo build --workspace`
Expected: all pass, clean build.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/services/Cargo.toml crates/services/src/system_resources/mod.rs
git commit -m "services : system_resources — GPU-процент через nvml-wrapper, деградация до None без Nvidia"
```

---

## Task 5: `power` service

**Files:**
- Create: `crates/services/src/power/mod.rs`
- Modify: `crates/services/src/lib.rs`

**Interfaces:**
- Produces: `pub struct PowerSubscriber` (plain struct, does **not**
  implement `Service` — there is no reactive state to subscribe to, only
  fire-and-forget commands; documented in the module doc comment so a
  future reader doesn't wonder why it's missing from the `Service`
  pattern), `pub fn new() -> Self`, `pub fn log_out(&self)`,
  `pub fn restart(&self)`, `pub fn shutdown(&self)`, and pure argument
  builders `pub fn log_out_command() -> (&'static str, Vec<&'static
  str>)`, `pub fn restart_command() -> (&'static str, Vec<&'static
  str>)`, `pub fn shutdown_command() -> (&'static str, Vec<&'static
  str>)`.
- Consumes: nothing new.

- [ ] **Step 1: Write the failing tests for the pure command builders**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_out_command_is_hyprctl_dispatch_exit() {
        assert_eq!(log_out_command(), ("hyprctl", vec!["dispatch", "exit"]));
    }

    #[test]
    fn restart_command_is_systemctl_reboot() {
        assert_eq!(restart_command(), ("systemctl", vec!["reboot"]));
    }

    #[test]
    fn shutdown_command_is_systemctl_poweroff() {
        assert_eq!(shutdown_command(), ("systemctl", vec!["poweroff"]));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chronos-services --lib power`
Expected: FAIL — module/functions not defined.

- [ ] **Step 3: Implement `crates/services/src/power/mod.rs`**

```rust
//! Power actions: log out (Hyprland session exit), restart, shutdown.
//!
//! **No `Service` impl.** Every other subscriber in this crate reacts to
//! external state (battery level, network connectivity, ...). Power
//! actions are pure one-shot commands with nothing to observe — modeling
//! this as `Service<Data = ()>` would be a trait implemented for the sake
//! of consistency, not because it carries meaning. Keep it a plain struct.
//!
//! **"Switch user" is intentionally absent.** There is no login/display
//! manager on this system to hand a session to (see
//! `docs/superpowers/specs/2026-07-20-right-side-panel-design.md` §3.4) —
//! the UI ships a disabled button, this service has nothing to back it.

use std::process::Command;

use tracing::warn;

/// `hyprctl dispatch exit` — ends the current Hyprland session. What
/// happens next (return to a TTY prompt, respawn) is decided by whatever
/// launched Hyprland, not by this service.
pub fn log_out_command() -> (&'static str, Vec<&'static str>) {
    ("hyprctl", vec!["dispatch", "exit"])
}

/// `systemctl reboot`.
pub fn restart_command() -> (&'static str, Vec<&'static str>) {
    ("systemctl", vec!["reboot"])
}

/// `systemctl poweroff`.
pub fn shutdown_command() -> (&'static str, Vec<&'static str>) {
    ("systemctl", vec!["poweroff"])
}

fn spawn_command((bin, args): (&'static str, Vec<&'static str>)) {
    match Command::new(bin).args(&args).spawn() {
        Ok(_) => {}
        Err(e) => warn!("power: failed to spawn `{bin} {args:?}`: {e}"),
    }
}

#[derive(Clone, Default)]
pub struct PowerSubscriber;

impl PowerSubscriber {
    pub fn new() -> Self {
        Self
    }

    /// Fire-and-forget. Caller (UI) is responsible for confirming with the
    /// user before calling this — this service does not gate on anything.
    pub fn log_out(&self) {
        spawn_command(log_out_command());
    }

    pub fn restart(&self) {
        spawn_command(restart_command());
    }

    pub fn shutdown(&self) {
        spawn_command(shutdown_command());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_out_command_is_hyprctl_dispatch_exit() {
        assert_eq!(log_out_command(), ("hyprctl", vec!["dispatch", "exit"]));
    }

    #[test]
    fn restart_command_is_systemctl_reboot() {
        assert_eq!(restart_command(), ("systemctl", vec!["reboot"]));
    }

    #[test]
    fn shutdown_command_is_systemctl_poweroff() {
        assert_eq!(shutdown_command(), ("systemctl", vec!["poweroff"]));
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chronos-services --lib power`
Expected: PASS (3 tests).

- [ ] **Step 5: Register in `Services`**

In `crates/services/src/lib.rs`: add `pub mod power;`, add
`pub power: power::PowerSubscriber,` to `Services`, add
`power: power::PowerSubscriber::new(),` to `init_all()`.

- [ ] **Step 6: Add the `AppState` accessor**

In `crates/app/src/state.rs`:

```rust
    #[inline(always)]
    pub fn power(cx: &App) -> &chronos_services::PowerSubscriber {
        &Self::global(cx).services.power
    }
```

- [ ] **Step 7: Full build**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 8: Commit**

```bash
git add crates/services/src/power crates/services/src/lib.rs crates/app/src/state.rs
git commit -m "services : power — log out/restart/shutdown, switch user намеренно не реализован (нет login manager)"
```

---

## Task 6: Per-app stream mute in the `audio` service

> **NOT ACCEPTED (2026-07-21).** WIP may sit in
> `crates/services/src/audio/{types,pw_dump,mod}.rs` (uncommitted). Do not
> treat this task as done until a named commit lands and Architect accepts.
> Checkboxes stay open. UI mute button is Task 9.

**Files:**
- Modify: `crates/services/src/audio/types.rs`
- Modify: `crates/services/src/audio/pw_dump.rs`
- Modify: `crates/services/src/audio/mod.rs`
- Modify: `crates/services/src/audio/wpctl.rs` (no signature change needed
  — `format_set_mute_toggle_args(id: &str)` already accepts any id string)

**Interfaces:**
- Produces: `AudioCommand::ToggleStreamMute(u32)` (new enum variant),
  `pub struct AudioStream { pub id: u32, pub application_name: String,
  pub node_name: String }` (in `types.rs`),
  `pub fn parse_pw_dump_streams(json: &str) -> anyhow::Result<Vec<AudioStream>>`
  (in `pw_dump.rs`), `pub fn find_stream_for_player(streams: &[AudioStream],
  player_hint: &str) -> Option<u32>` (in `pw_dump.rs` — the
  name-matching heuristic).
- Consumes: `MprisState::player_id` / `MprisState::title` (existing,
  `crates/services/src/mpris/types.rs`) as the `player_hint` string the
  panel passes in — this task does not touch MPRIS, it only accepts a
  `&str` hint from the caller.

- [ ] **Step 1: Confirm the live `pw-dump` schema for playback streams**

Run: `pw-dump | python3 -c "
import json,sys
for o in json.load(sys.stdin):
    p = (o.get('info') or {}).get('props') or {}
    mc = p.get('media.class','')
    if mc.startswith('Stream/Output/Audio'):
        print(o['id'], mc, p.get('application.name'), p.get('node.name'))
"`

Expected: with something actually playing (start any audio first —
`mpv`/browser tab/Spotify), at least one line printed. Note the exact
`media.class` string and whether `application.name` is populated for your
test app — this confirms the fixture below matches reality. If nothing
prints even with audio playing, re-run `pw-dump` without the filter and
inspect manually; do not proceed to Step 2 on a guess.

- [ ] **Step 2: Write the failing test with a fixture based on Step 1's real schema**

Add to `crates/services/src/audio/pw_dump.rs`:

```rust
#[cfg(test)]
mod stream_tests {
    use super::*;

    const FIXTURE: &str = r#"[
        {
            "id": 142,
            "info": {
                "props": {
                    "media.class": "Stream/Output/Audio",
                    "application.name": "Vivaldi",
                    "node.name": "Vivaldi",
                    "node.description": "Playback"
                }
            }
        },
        {
            "id": 143,
            "info": {
                "props": {
                    "media.class": "Stream/Output/Audio",
                    "application.name": "Spotify",
                    "node.name": "spotify",
                    "node.description": "Spotify"
                }
            }
        },
        {
            "id": 55,
            "info": {
                "props": {
                    "media.class": "Audio/Sink",
                    "node.name": "alsa_output.pci-0000_00_1f.3.analog-stereo"
                }
            }
        }
    ]"#;

    #[test]
    fn parses_only_output_audio_streams_not_sinks() {
        let streams = parse_pw_dump_streams(FIXTURE).unwrap();
        assert_eq!(streams.len(), 2);
        assert!(streams.iter().all(|s| s.id != 55));
    }

    #[test]
    fn finds_stream_by_case_insensitive_application_name_match() {
        let streams = parse_pw_dump_streams(FIXTURE).unwrap();
        let id = find_stream_for_player(&streams, "vivaldi");
        assert_eq!(id, Some(142));
    }

    #[test]
    fn returns_none_when_no_stream_matches_hint() {
        let streams = parse_pw_dump_streams(FIXTURE).unwrap();
        let id = find_stream_for_player(&streams, "firefox");
        assert_eq!(id, None);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p chronos-services --lib audio::pw_dump::stream_tests`
Expected: FAIL — `parse_pw_dump_streams`/`find_stream_for_player`/
`AudioStream` not defined.

- [ ] **Step 4: Add `AudioStream` to `types.rs`**

```rust
/// One PipeWire playback stream belonging to an application (e.g. a
/// browser tab, a media player) — distinct from `AudioDevice` (a
/// sink/source hardware endpoint).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioStream {
    pub id: u32,
    pub application_name: String,
    pub node_name: String,
}
```

- [ ] **Step 5: Implement `parse_pw_dump_streams` + `find_stream_for_player`**

Add to `crates/services/src/audio/pw_dump.rs` (same file as
`parse_pw_dump_devices`, following its exact parsing style):

```rust
use super::types::AudioStream;

/// Parse `pw-dump` JSON for application playback streams
/// (`media.class == "Stream/Output/Audio"`), distinct from sink/source
/// hardware devices parsed by `parse_pw_dump_devices`.
pub fn parse_pw_dump_streams(json: &str) -> anyhow::Result<Vec<AudioStream>> {
    let root: Value = serde_json::from_str(json)
        .map_err(|e| anyhow::anyhow!("pw-dump JSON parse: {e}"))?;
    let arr = root
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("pw-dump root is not an array"))?;

    let mut streams = Vec::new();
    for obj in arr {
        let Some(id) = obj.get("id").and_then(|v| v.as_u64()).map(|n| n as u32) else {
            continue;
        };
        let props = obj
            .get("info")
            .and_then(|i| i.get("props"))
            .and_then(|p| p.as_object());
        let Some(props) = props else {
            continue;
        };
        let media_class = props
            .get("media.class")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if media_class != "Stream/Output/Audio" {
            continue;
        }
        let application_name = props
            .get("application.name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let node_name = props
            .get("node.name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        streams.push(AudioStream {
            id,
            application_name,
            node_name,
        });
    }
    Ok(streams)
}

/// Match a stream to `player_hint` (an MPRIS player identity string, e.g.
/// `MprisState::player_id`) by case-insensitive substring against either
/// `application_name` or `node_name`. No 1:1 guarantee — a browser with
/// multiple tabs/streams, or an app whose PipeWire name doesn't resemble
/// its MPRIS identity, will not match. Callers must treat `None` as
/// "nothing to mute" and degrade silently, never panic.
pub fn find_stream_for_player(streams: &[AudioStream], player_hint: &str) -> Option<u32> {
    let hint = player_hint.to_lowercase();
    if hint.is_empty() {
        return None;
    }
    streams
        .iter()
        .find(|s| {
            s.application_name.to_lowercase().contains(&hint)
                || s.node_name.to_lowercase().contains(&hint)
        })
        .map(|s| s.id)
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p chronos-services --lib audio::pw_dump::stream_tests`
Expected: PASS (3 tests).

- [ ] **Step 7: Add `ToggleStreamMute` to `AudioCommand` + wire `command_to_wpctl_args`**

In `crates/services/src/audio/types.rs`, add a variant to `AudioCommand`:

```rust
pub enum AudioCommand {
    SetSinkVolume(f64),
    SetSourceVolume(f64),
    ToggleSinkMute,
    ToggleSourceMute,
    /// Mute/unmute one application's PipeWire playback stream by its
    /// `pw-dump` node id (see `pw_dump::find_stream_for_player`).
    ToggleStreamMute(u32),
    SetDefaultSink(u32),
    SetDefaultSource(u32),
}
```

In `crates/services/src/audio/mod.rs`, add a test first:

```rust
    #[test]
    fn command_to_wpctl_args_stream_mute_targets_the_given_id() {
        let args = command_to_wpctl_args(&AudioCommand::ToggleStreamMute(142));
        assert_eq!(args, vec!["set-mute", "142", "toggle"]);
    }
```

Run: `cargo test -p chronos-services --lib command_to_wpctl_args_stream_mute_targets_the_given_id`
Expected: FAIL — no match arm.

Add the match arm to `command_to_wpctl_args`:

```rust
        AudioCommand::ToggleStreamMute(id) => format_set_mute_toggle_args(&id.to_string()),
```

Run the same test again — Expected: PASS.

- [ ] **Step 8: Add a `dispatch_stream_mute` convenience method for the panel**

The panel needs to resolve the stream id (via `find_stream_for_player`)
and dispatch in one call, without re-shelling `pw-dump` parsing logic
into `crates/app`. Add to `AudioSubscriber` in
`crates/services/src/audio/mod.rs`:

```rust
    /// Resolve `player_hint` to a live PipeWire stream and toggle its mute.
    /// No-op (logged, not erred) if no matching stream is found — this is
    /// the expected outcome for many-tabs/mismatched-name cases documented
    /// on `find_stream_for_player`.
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
                    info!("toggle_stream_mute_for_player: no PipeWire stream matched '{player_hint}'");
                }
            }
        });
    }
```

Add `mod pw_dump;` visibility check — it's already `mod pw_dump;` (private)
per the existing file; since this method lives inside `mod.rs` itself,
`pw_dump::run_pw_dump`/`parse_pw_dump_streams`/`find_stream_for_player`
are reachable without changing visibility. Confirm `pw_dump::run_pw_dump`
is `pub(crate)` or `pub` — it's already `pub fn run_pw_dump()` per the
existing file, no change needed.

- [ ] **Step 9: Full test suite + build**

Run: `cargo test -p chronos-services --lib`
Expected: all tests pass, including every test added in Steps 2 and 7.

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 10: Commit**

```bash
git add crates/services/src/audio
git commit -m "services : audio — per-app stream mute (ToggleStreamMute + pw-dump stream parsing + name-match heuristика)"
```

---

## Task 7: `side_panel_right` window module — open/close/toggle skeleton

> **DONE `da744a2` (2026-07-21).** Steps below are a record; do not re-scaffold.
>
> **Registration errata (landed fact):** app crate is a **binary**. Shell
> modules are declared with `mod side_panel_right;` +
> `side_panel_right::init(cx);` in **`crates/app/src/main.rs`** only.
> `crates/app/src/lib.rs` exports only `monitor` / `notifications` /
> `state` for tests — **do not** add panel modules there. Plan text that
> still says `lib.rs` / `pub mod` / `chronos-app` is historical; follow
> the errata.

**Files (as landed):**
- Create: `crates/app/src/side_panel_right/mod.rs`
- Create: `crates/app/src/side_panel_right/view.rs`
- Modify: `crates/app/src/main.rs` — `mod side_panel_right;` + `init(cx)`
- ~~`crates/app/src/lib.rs`~~ — **not used** for this module (see errata)

**Interfaces:**
- Produces: `pub fn init(cx: &mut App)`, `pub fn toggle(_window: &mut
  Window, cx: &mut App)` (pins the panel open/closed — bar MPRIS-widget
  click target for Task 9), `pub fn open_pinned(cx: &mut App)`, `pub fn
  close(cx: &mut App)`, `pub(crate) fn close_this(window: &mut Window, cx:
  &mut App)`, `pub struct SidePanelRightView` (empty shell this task,
  filled by Tasks 9–11).
- Consumes: `crate::monitor::pult_display` (existing).
- **No public open path in product yet** — `toggle` is not wired to bar
  or IPC; live smoke used a temporary env hook that was **not** committed.

- [x] **Step 1: Confirm the module registration point**

~~`grep … crates/app/src/lib.rs`~~ **stale** — binary modules live in
`main.rs` (`mod bar;`, `mod system_popup;`, …). Landed: `main.rs`
`mod side_panel_right;` + `side_panel_right::init(cx);` next to other
popups.

- [x] **Step 2: Write `view.rs` — minimal renderable shell**

```rust
//! Right side panel view — window shell only in this task. MPRIS card
//! (task 9), system/network spectrum meters (task 10), and power row
//! (task 11) are added as children of `render()` in later tasks.

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};

use chronos_ui::Theme;

pub struct SidePanelRightView {}

impl SidePanelRightView {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for SidePanelRightView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .size_full()
            .bg(theme.bg.secondary)
            .border_l_1()
            .border_color(theme.border.default)
            .p(px(20.))
            .child("side panel: skeleton, tasks 9-11 fill this in")
    }
}
```

- [x] **Step 3: Write `mod.rs` — window lifecycle**

```rust
//! Right side panel — lazy layer-shell overlay, hover-peek (task 8) or
//! pinned (bar-widget click / hotkey, this task). Window lifecycle
//! mirrors `system_popup/`/`volume_popup/`: `Layer::Overlay`,
//! `KeyboardInteractivity::None`, `close_this` reentrancy guard
//! (`ARCHITECTURE.md §4.1` — never re-entrant `handle.update` for
//! `remove_window()` from inside that window's own callback).
//!
//! **No Esc-to-close** — matches the real convention already in this
//! codebase (`volume_popup`/`system_popup` have no Esc handler either,
//! `KeyboardInteractivity::None` doesn't deliver key events). Dismiss is
//! re-toggle / click-away (pinned) / mouse-leave debounce (peek, task 8).

pub mod view;

use gpui::{
    App, Bounds, DisplayId, Global, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use crate::side_panel_right::view::SidePanelRightView;

const PANEL_WIDTH: f32 = 300.;

#[derive(Default)]
pub struct SidePanelRightState {
    handle: Option<WindowHandle<SidePanelRightView>>,
}

impl Global for SidePanelRightState {}

fn window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(PANEL_WIDTH), px(0.)), // height filled by TOP|BOTTOM anchor
        })),
        app_id: Some("chronos-side-panel-right".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_right".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::RIGHT,
            exclusive_zone: Some(px(0.)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
        }),
        ..Default::default()
    }
}

/// Open pinned (idempotent — no-op if already open).
pub fn open_pinned(cx: &mut App) {
    if cx.global::<SidePanelRightState>().handle.is_some() {
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    match cx.open_window(window_options(display_id), |_, view_cx| {
        view_cx.new(|cx| SidePanelRightView::new(cx))
    }) {
        Ok(handle) => cx.global_mut::<SidePanelRightState>().handle = Some(handle),
        Err(err) => tracing::warn!("side_panel_right: failed to open: {err}"),
    }
}

/// Close from outside (bar toggle / hotkey).
///
/// Note the `match` instead of `let _ =`: `system_popup`/`volume_popup`
/// swallow this Err today, and a swallowed `handle.update` Err is exactly
/// what hid the ghost-window bug for a full session (HANDOFF.md
/// 2026-07-18). New code does not inherit that wart — an Err here means
/// the handle was taken but the window never closed, i.e. a ghost.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<SidePanelRightState>().handle.take() {
        match handle.update(cx, |_, window: &mut Window, _| window.remove_window()) {
            Ok(()) => {}
            Err(e) => tracing::warn!(
                "side_panel_right: close() could not reach the window ({e}) — possible ghost"
            ),
        }
    }
}

/// Close from inside a callback that already holds `&mut Window` for this
/// panel. Must not re-enter `handle.update` on the same id (ghost-window
/// guard, `ARCHITECTURE.md §4.1`).
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<SidePanelRightState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<SidePanelRightState>().handle.take();
    }
    window.remove_window();
}

/// Bar-widget click / hotkey target.
pub fn toggle(_window: &mut Window, cx: &mut App) {
    if cx.global::<SidePanelRightState>().handle.is_some() {
        close(cx);
    } else {
        open_pinned(cx);
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(SidePanelRightState::default());
}
```

- [x] **Step 4: Register the module**

~~`lib.rs` `pub mod`~~ **stale** — only:

```text
// crates/app/src/main.rs
mod side_panel_right;
// …
side_panel_right::init(cx);
```

- [x] **Step 5: Build**

Run: `cargo build --release -p chronos` (package name **`chronos`**, not
`chronos-app`). Expected: clean build.

- [x] **Step 6: Live smoke — window opens and closes on toggle**

Landed evidence (`hermes-report-7.md` + grim): release binary,
`namespace: side_panel_right`, `xywh: … 300 1410` on pult right edge,
residual 0 after close. ~~`cargo run --bin chronos-app`~~ stale name —
binary is `chronos`. Permanent bar/IPC bind = Task 9; smoke used a
temporary non-committed trigger.

- [x] **Step 7: Commit**

```bash
git add crates/app/src/side_panel_right crates/app/src/main.rs
# do NOT stage lib.rs for this task
git commit -m "app : side_panel_right — оконный скелет (layer-shell overlay, toggle/close_this)"
# landed: da744a2
```

---

## Task 8: Hover-peek hit-test strip

**Files:**
- Create: `crates/app/src/side_panel_right/hover_strip.rs`
- Modify: `crates/app/src/side_panel_right/mod.rs`

**Interfaces:**
- Produces: `pub fn init_hover_strip(cx: &mut App)` (opens a permanent
  4px-wide invisible layer-shell window anchored `TOP | BOTTOM | RIGHT`),
  extends `SidePanelRightState` with a `peeked: bool` field and
  `open_peek(cx)` / `close_peek_if_not_pinned(cx)` functions.
- Consumes: `open_pinned`/`close` primitives from Task 7 (reused — peek
  and pin share the same underlying window, distinguished only by the
  `peeked`/`pinned` state flags controlling dismiss behavior).

- [ ] **Step 1: Extend `SidePanelRightState` with peek/pin distinction**

In `crates/app/src/side_panel_right/mod.rs`, change the state struct:

```rust
#[derive(Default)]
pub struct SidePanelRightState {
    handle: Option<WindowHandle<SidePanelRightView>>,
    /// `true` when opened by hotkey/bar-click (Task 7's `toggle`/
    /// `open_pinned`) — stays open until re-toggled. `false` when opened
    /// by hover (this task) — closes on mouse-leave debounce unless a
    /// pin request arrives while peeked.
    pinned: bool,
}
```

Update `open_pinned` to set `pinned = true` after a successful open, and
`toggle`/`close` to also reset `pinned = false` on close. Add:

```rust
/// Open in peek mode (hover entered the strip). No-op if already open in
/// either mode.
pub fn open_peek(cx: &mut App) {
    if cx.global::<SidePanelRightState>().handle.is_some() {
        return;
    }
    let display_id = crate::monitor::pult_display(cx);
    match cx.open_window(window_options(display_id), |_, view_cx| {
        view_cx.new(|cx| SidePanelRightView::new(cx))
    }) {
        Ok(handle) => {
            let state = cx.global_mut::<SidePanelRightState>();
            state.handle = Some(handle);
            state.pinned = false;
        }
        Err(err) => tracing::warn!("side_panel_right: failed to open (peek): {err}"),
    }
}

/// Mouse left the strip and the panel. Closes only if not pinned.
pub fn close_peek_if_not_pinned(cx: &mut App) {
    if cx.global::<SidePanelRightState>().pinned {
        return;
    }
    close(cx);
}
```

- [ ] **Step 2: Write the failing test for the peek/pin state transition logic**

This is pure enough to unit test without a live window — extract the
decision as a free function first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_close_request_is_noop_while_pinned() {
        let mut state = SidePanelRightState::default();
        state.pinned = true;
        assert!(!should_close_on_peek_leave(&state));
    }

    #[test]
    fn peek_close_request_closes_when_not_pinned() {
        let mut state = SidePanelRightState::default();
        state.pinned = false;
        assert!(should_close_on_peek_leave(&state));
    }
}

fn should_close_on_peek_leave(state: &SidePanelRightState) -> bool {
    !state.pinned
}
```

Rewrite `close_peek_if_not_pinned` to use this function:

```rust
pub fn close_peek_if_not_pinned(cx: &mut App) {
    if !should_close_on_peek_leave(cx.global::<SidePanelRightState>()) {
        return;
    }
    close(cx);
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p chronos-app --lib side_panel_right`
Expected: PASS (2 tests).

- [ ] **Step 4: Implement the invisible hover strip window**

```rust
//! Invisible 4px hit-test strip on the right screen edge. A permanently
//! open, zero-content layer-shell window whose only job is to receive
//! `on_hover` — GPUI delivers mouse-enter/leave to any window under the
//! cursor regardless of visible content, so a fully transparent 4px-wide
//! strip is a legitimate hit-test surface, not a hack. This sidesteps
//! compositor-level pointer polling entirely (the alternative considered
//! in the spec's open question §8.1 and rejected here as unnecessary
//! complexity once GPUI's own hover event was confirmed sufficient).

use gpui::{
    App, Bounds, DisplayId, IntoElement, Render, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px,
};

const STRIP_WIDTH: f32 = 4.;

struct HoverStripView {}

impl Render for HoverStripView {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .on_hover(|hovered, _window, cx| {
                if *hovered {
                    super::open_peek(cx);
                } else {
                    // Debounce: a bare mouse-leave fires the instant the
                    // cursor crosses back onto the panel itself (which is
                    // a separate window) — closing here would immediately
                    // undo `open_peek` before the user's cursor lands on
                    // the panel. Use `cx.spawn` with a short delay and
                    // re-check hover state is out of scope for a 4px div's
                    // own `on_hover` (it can't see the panel window's
                    // state) — the debounce lives on the panel window's
                    // own hover handler instead (task 9 adds
                    // `on_hover` to the panel's root `div` calling
                    // `close_peek_if_not_pinned` after a delay). This
                    // strip only ever calls `open_peek` on enter; it never
                    // closes on its own leave.
                }
            })
    }
}

fn strip_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(STRIP_WIDTH), px(0.)),
        })),
        app_id: Some("chronos-side-panel-hover-strip".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "side_panel_hover_strip".to_string(),
            layer: gpui::layer_shell::Layer::Overlay,
            anchor: gpui::layer_shell::Anchor::TOP
                | gpui::layer_shell::Anchor::BOTTOM
                | gpui::layer_shell::Anchor::RIGHT,
            exclusive_zone: Some(px(0.)),
            margin: None,
            keyboard_interactivity: gpui::layer_shell::KeyboardInteractivity::None,
        }),
        ..Default::default()
    }
}

/// Open the permanent hover-detector strip. Called once from
/// `side_panel_right::init`, never toggled or closed for the life of the
/// process.
pub fn init_hover_strip(cx: &mut App) {
    let display_id = crate::monitor::pult_display(cx);
    if let Err(err) = cx.open_window(strip_window_options(display_id), |_, view_cx| {
        view_cx.new(|_| HoverStripView {})
    }) {
        tracing::warn!("side_panel_right: failed to open hover strip: {err}");
    }
}
```

- [ ] **Step 5: Call `init_hover_strip` from `init`**

In `crates/app/src/side_panel_right/mod.rs`, add `mod hover_strip;` and
update `init`:

```rust
pub fn init(cx: &mut App) {
    cx.set_global(SidePanelRightState::default());
    hover_strip::init_hover_strip(cx);
}
```

- [ ] **Step 6: Build**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 7: Live smoke — hover opens peek, moving away closes it**

Run the release binary with `RUST_LOG=info`, move the cursor to the right
edge of the screen without clicking anything, confirm via `grim`
screenshot that the panel appears; move the cursor away (not onto the
panel itself, into empty desktop) and confirm a follow-up screenshot
shows it gone. If the panel does NOT close on leave, the debounce wiring
in Task 9 (panel's own `on_hover` calling `close_peek_if_not_pinned`) is
the missing piece — do not consider Task 8 done until this round-trip
works after Task 9 lands; note this cross-task dependency in the Task 9
smoke step instead of re-testing here in isolation.

- [ ] **Step 8: Commit**

```bash
git add crates/app/src/side_panel_right
git commit -m "app : side_panel_right — hover-peek через invisible 4px layer-shell strip (не compositor polling)"
```

---

## Task 9: MPRIS card widget

**Files:**
- Create: `crates/app/src/side_panel_right/mpris_card.rs`
- Modify: `crates/app/src/side_panel_right/view.rs`

**Interfaces:**
- Consumes: `chronos_services::MprisState` (existing fields only:
  `title`, `artist`, `playing`, `player_id`), `MprisCommand::{PlayPause,
  Next, Previous}` (existing), `AppState::mpris(cx)` (existing),
  `AppState::audio(cx).toggle_stream_mute_for_player(player_id)` (Task 6).
- Produces: `pub fn render_mpris_card(state: &MprisState, theme: &Theme,
  cx: &mut Context<SidePanelRightView>) -> impl IntoElement`.

- [ ] **Step 1: Write the pure layout-decision test first**

The only genuinely pure logic here is "what do we show when there's no
player" vs "what do we show when there is one" — test that decision
before touching GPUI elements:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::MprisState;

    #[test]
    fn no_player_shows_placeholder_title() {
        let state = MprisState::default();
        assert_eq!(display_title(&state), "No player");
    }

    #[test]
    fn active_player_shows_real_title() {
        let state = MprisState {
            title: "Colour Temperature".into(),
            artist: "Ambient Systems".into(),
            playing: true,
            has_player: true,
            player_count: 1,
            player_index: 1,
            player_id: "vivaldi".into(),
        };
        assert_eq!(display_title(&state), "Colour Temperature");
    }
}

fn display_title(state: &MprisState) -> &str {
    if state.has_player {
        &state.title
    } else {
        "No player"
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-app --lib side_panel_right::mpris_card`
Expected: FAIL — module doesn't exist yet.

- [ ] **Step 3: Implement `mpris_card.rs`**

```rust
//! MPRIS card for the right side panel. Uses only the fields
//! `MprisState` actually exposes (`title`/`artist`/`playing`/`player_id`)
//! — no album art URL, no position/length exist in this service today
//! (confirmed during planning, see plan Global Constraints). This card
//! ships a static gradient placeholder swatch instead of real art, and
//! has no progress bar / timecode row.

use gpui::{Context, IntoElement, div, prelude::*, px, rgb};

use chronos_services::MprisState;
use chronos_ui::Theme;

use crate::side_panel_right::view::SidePanelRightView;
use crate::state::AppState;

fn display_title(state: &MprisState) -> &str {
    if state.has_player {
        &state.title
    } else {
        "No player"
    }
}

fn display_artist(state: &MprisState) -> &str {
    if state.has_player {
        &state.artist
    } else {
        ""
    }
}

pub fn render_mpris_card(
    state: &MprisState,
    theme: &Theme,
    cx: &mut Context<SidePanelRightView>,
) -> impl IntoElement {
    let player_id = state.player_id.clone();
    let player_id_for_mute = player_id.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(14.))
        .bg(theme.bg.tertiary)
        .border_1()
        .border_color(theme.border.default)
        .child(
            div()
                .flex()
                .gap(px(12.))
                .child(
                    // Placeholder swatch — no real album art source exists
                    // (see module doc comment). Do not wire an image fetch
                    // here without first extending `MprisState` with an
                    // art URL field in a follow-up spec.
                    div()
                        .w(px(64.))
                        .h(px(64.))
                        .flex_shrink_0()
                        .bg(rgb(0x5fd3e8)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(px(4.))
                        .flex_1()
                        .child(
                            div()
                                .font_family(theme.font_ui)
                                .text_size(px(13.))
                                .text_color(theme.text.primary)
                                .child(display_title(state).to_string()),
                        )
                        .child(
                            div()
                                .font_family(theme.font_ui)
                                .text_size(px(11.5))
                                .text_color(theme.text.secondary)
                                .child(display_artist(state).to_string()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .child(
                    div()
                        .id("mpris-prev")
                        .w(px(32.))
                        .h(px(32.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_color(theme.text.primary)
                        .child("<")
                        .on_click(move |_, _window, cx| {
                            AppState::mpris(cx).dispatch(chronos_services::MprisCommand::Previous);
                        }),
                )
                .child(
                    div()
                        .id("mpris-playpause")
                        .w(px(38.))
                        .h(px(38.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .bg(theme.bg.elevated)
                        .text_color(rgb(0x5fd3e8))
                        .child(if state.playing { "||" } else { ">" })
                        .on_click(move |_, _window, cx| {
                            AppState::mpris(cx).dispatch(chronos_services::MprisCommand::PlayPause);
                        }),
                )
                .child(
                    div()
                        .id("mpris-next")
                        .w(px(32.))
                        .h(px(32.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_color(theme.text.primary)
                        .child(">")
                        .on_click(move |_, _window, cx| {
                            AppState::mpris(cx).dispatch(chronos_services::MprisCommand::Next);
                        }),
                )
                .child(
                    div()
                        .id("mpris-mute")
                        .ml_auto()
                        .w(px(28.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_color(theme.text.muted)
                        .child("M")
                        .on_click(move |_, _window, cx| {
                            AppState::audio(cx)
                                .toggle_stream_mute_for_player(player_id_for_mute.clone());
                        }),
                ),
        )
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-app --lib side_panel_right::mpris_card`
Expected: PASS (2 tests).

- [ ] **Step 5: Wire into `view.rs`, subscribe to MPRIS updates, add the hover debounce**

Update `SidePanelRightView` to hold the latest `MprisState` and subscribe
via `state::watch` (same pattern as `SystemPopupBrightnessWatcher` in
`system_popup/mod.rs`):

```rust
use chronos_services::{MprisState, Service};

use crate::side_panel_right::mpris_card::render_mpris_card;
use crate::state::{self, AppState};

pub struct SidePanelRightView {
    mpris: MprisState,
}

impl SidePanelRightView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let signal = AppState::mpris(cx).subscribe();
        state::watch(cx, signal, |this: &mut Self, data: MprisState, cx| {
            this.mpris = data;
            cx.notify();
        });
        Self {
            mpris: AppState::mpris(cx).get(),
        }
    }
}

impl Render for SidePanelRightView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        div()
            .id("side-panel-right-root")
            .size_full()
            .bg(theme.bg.secondary)
            .border_l_1()
            .border_color(theme.border.default)
            .p(px(20.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .on_hover(|hovered, _window, cx| {
                if !*hovered {
                    crate::side_panel_right::close_peek_if_not_pinned(cx);
                }
            })
            .child(render_mpris_card(&self.mpris, &theme, cx))
    }
}
```

- [ ] **Step 6: Build**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 7: Live smoke — full peek round-trip + MPRIS controls**

With something playing (start `mpv`/a browser tab), open the panel via
hover, confirm title/artist match the playing track, click play/pause and
confirm the running player actually pauses/resumes, click mute and
confirm via `wpctl status` that the matched stream's mute flag flipped.
Move cursor off the panel (not onto the hover strip) and confirm it
closes — this is the cross-task check deferred from Task 8 Step 7.

Expected: all four behaviors (title match, play/pause, mute, close-on-
leave) work live. `RUST_LOG=info`, capture the log for the mute dispatch
specifically — confirm no "no PipeWire stream matched" warning when a
recognizable app (e.g. a browser with an obvious `application.name`) is
playing.

- [ ] **Step 8: Commit**

```bash
git add crates/app/src/side_panel_right
git commit -m "app : side_panel_right — MPRIS-карточка (transport + per-app mute), hover-debounce на закрытие"
```

---

## Task 10: System + Network spectrum widget

**Files:**
- Create: `crates/app/src/side_panel_right/spectrum_row.rs`
- Modify: `crates/app/src/side_panel_right/view.rs`

**Interfaces:**
- Consumes: `chronos_services::SystemResourcesState` (Task 3/4),
  `chronos_services::net_stats::{NetState, NetSpeed, update_speed,
  read_interface_bytes, SAMPLE_INTERVAL}` (Task 1),
  `AppState::system_resources(cx)` (Task 3).
- Produces: `pub struct SpectrumHistory { .. }` (fixed-size ring buffer,
  14 samples — matches the mockup's bar count), `pub fn
  push_sample(history: &mut SpectrumHistory, value: f32)`, `pub fn
  render_spectrum_row(label: &str, history: &SpectrumHistory, value_text:
  &str, color: gpui::Hsla, theme: &Theme) -> impl IntoElement`.

- [ ] **Step 1: Write the failing test for the ring buffer**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_holds_at_most_14_samples_oldest_dropped_first() {
        let mut history = SpectrumHistory::default();
        for i in 0..20 {
            push_sample(&mut history, i as f32);
        }
        assert_eq!(history.samples.len(), 14);
        // Oldest 6 (0..6) dropped, newest is 19.
        assert_eq!(history.samples.front().copied(), Some(6.0));
        assert_eq!(history.samples.back().copied(), Some(19.0));
    }

    #[test]
    fn empty_history_has_no_samples() {
        let history = SpectrumHistory::default();
        assert!(history.samples.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-app --lib side_panel_right::spectrum_row`
Expected: FAIL — module doesn't exist.

- [ ] **Step 3: Implement the ring buffer + row renderer**

```rust
//! Reusable spectrum-bar row: label, N thin vertical bars (history), and
//! a right-aligned formatted value. Used for CPU/RAM/GPU (task 3/4 data)
//! and network down/up (task 1 data) — one component, four call sites.

use std::collections::VecDeque;

use gpui::{Context, Hsla, IntoElement, div, prelude::*, px};

use chronos_ui::Theme;

use crate::side_panel_right::view::SidePanelRightView;

const HISTORY_LEN: usize = 14;
const BAR_HEIGHT_PX: f32 = 52.;

#[derive(Default)]
pub struct SpectrumHistory {
    pub samples: VecDeque<f32>,
}

/// Push a new sample, dropping the oldest once the buffer exceeds
/// `HISTORY_LEN`.
pub fn push_sample(history: &mut SpectrumHistory, value: f32) {
    history.samples.push_back(value);
    while history.samples.len() > HISTORY_LEN {
        history.samples.pop_front();
    }
}

pub fn render_spectrum_row(
    label: &str,
    history: &SpectrumHistory,
    value_text: &str,
    color: Hsla,
    theme: &Theme,
) -> impl IntoElement {
    let max = history.samples.iter().cloned().fold(1.0_f32, f32::max);
    let bars: Vec<_> = history
        .samples
        .iter()
        .map(|&v| {
            let height_pct = ((v / max).clamp(0.0, 1.0) * 100.0) as f32;
            div()
                .flex_1()
                .h(px(BAR_HEIGHT_PX * height_pct / 100.0))
                .bg(color)
        })
        .collect();

    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .py(px(14.))
        .child(
            div()
                .w(px(34.))
                .font_family(theme.font_mono)
                .text_size(px(10.))
                .text_color(theme.text.secondary)
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_end()
                .gap(px(2.))
                .h(px(BAR_HEIGHT_PX))
                .children(bars),
        )
        .child(
            div()
                .w(px(44.))
                .font_family(theme.font_mono)
                .text_size(px(13.))
                .text_color(theme.text.primary)
                .child(value_text.to_string()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_holds_at_most_14_samples_oldest_dropped_first() {
        let mut history = SpectrumHistory::default();
        for i in 0..20 {
            push_sample(&mut history, i as f32);
        }
        assert_eq!(history.samples.len(), 14);
        assert_eq!(history.samples.front().copied(), Some(6.0));
        assert_eq!(history.samples.back().copied(), Some(19.0));
    }

    #[test]
    fn empty_history_has_no_samples() {
        let history = SpectrumHistory::default();
        assert!(history.samples.is_empty());
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-app --lib side_panel_right::spectrum_row`
Expected: PASS (2 tests).

- [ ] **Step 5: Wire system resources + network history into `view.rs`**

```rust
use std::time::Instant;

use chronos_services::net_stats::{self, NetState};
use chronos_services::SystemResourcesState;

use crate::side_panel_right::spectrum_row::{push_sample, render_spectrum_row, SpectrumHistory};

pub struct SidePanelRightView {
    mpris: MprisState,
    system: SystemResourcesState,
    cpu_history: SpectrumHistory,
    ram_history: SpectrumHistory,
    gpu_history: SpectrumHistory,
    net_state: NetState,
    net_dl_history: SpectrumHistory,
    net_ul_history: SpectrumHistory,
}

impl SidePanelRightView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mpris_signal = AppState::mpris(cx).subscribe();
        state::watch(cx, mpris_signal, |this: &mut Self, data: MprisState, cx| {
            this.mpris = data;
            cx.notify();
        });

        let sys_signal = AppState::system_resources(cx).subscribe();
        state::watch(
            cx,
            sys_signal,
            |this: &mut Self, data: SystemResourcesState, cx| {
                push_sample(&mut this.cpu_history, data.cpu_percent);
                push_sample(&mut this.ram_history, data.ram_percent);
                if let Some(gpu) = data.gpu_percent {
                    push_sample(&mut this.gpu_history, gpu);
                }
                this.system = data;
                cx.notify();
            },
        );

        Self {
            mpris: AppState::mpris(cx).get(),
            system: AppState::system_resources(cx).get(),
            cpu_history: SpectrumHistory::default(),
            ram_history: SpectrumHistory::default(),
            gpu_history: SpectrumHistory::default(),
            net_state: NetState::default(),
            net_dl_history: SpectrumHistory::default(),
            net_ul_history: SpectrumHistory::default(),
        }
    }

    /// Sample network speed on every render (time-gated internally by
    /// `update_speed`'s `SAMPLE_INTERVAL` — safe to call every frame, see
    /// plan Global Constraints).
    fn sample_network(&mut self) {
        let Ok((rx, tx)) = net_stats::read_interface_bytes() else {
            return;
        };
        let speed = net_stats::update_speed(
            &mut self.net_state,
            rx,
            tx,
            Instant::now(),
            net_stats::SAMPLE_INTERVAL,
        );
        push_sample(&mut self.net_dl_history, speed.dl as f32);
        push_sample(&mut self.net_ul_history, speed.ul as f32);
    }
}
```

Update `Render::render` to call `self.sample_network();` at the top, then
append the spectrum rows after the MPRIS card:

```rust
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sample_network();
        let theme = cx.global::<Theme>().clone();
        div()
            .id("side-panel-right-root")
            .size_full()
            .bg(theme.bg.secondary)
            .border_l_1()
            .border_color(theme.border.default)
            .p(px(20.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .on_hover(|hovered, _window, cx| {
                if !*hovered {
                    crate::side_panel_right::close_peek_if_not_pinned(cx);
                }
            })
            .child(render_mpris_card(&self.mpris, &theme, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(render_spectrum_row(
                        "CPU",
                        &self.cpu_history,
                        &format!("{:.0}%", self.system.cpu_percent),
                        gpui::rgb(0x5fd3e8).into(),
                        &theme,
                    ))
                    .child(render_spectrum_row(
                        "RAM",
                        &self.ram_history,
                        &format!("{:.0}%", self.system.ram_percent),
                        gpui::rgb(0x4fa3c9).into(),
                        &theme,
                    ))
                    .children(self.system.gpu_percent.map(|gpu| {
                        render_spectrum_row(
                            "GPU",
                            &self.gpu_history,
                            &format!("{:.0}%", gpu),
                            gpui::rgb(0x33638a).into(),
                            &theme,
                        )
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(render_spectrum_row(
                        "dn",
                        &self.net_dl_history,
                        &format!("{:.1}M", self.net_state.cached_dl / 1_000_000.0),
                        gpui::rgb(0x7cc4e8).into(),
                        &theme,
                    ))
                    .child(render_spectrum_row(
                        "up",
                        &self.net_ul_history,
                        &format!("{:.1}M", self.net_state.cached_ul / 1_000_000.0),
                        gpui::rgb(0x3d6d94).into(),
                        &theme,
                    )),
            )
    }
```

- [ ] **Step 6: Build**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 7: Live smoke — bars actually move under load**

Start `glxgears` or a game to load the GPU, and `stress-ng --cpu 4` (or
similar) for CPU. Open the panel, capture two `grim` screenshots ~3s
apart, confirm the CPU and GPU bars visibly differ in height between the
two captures (not flat). Confirm the network rows react to
`curl --limit-rate 5M <large file>` the same way the bar's network widget
was verified earlier this project (measure the same window, compare
`↓` value to the `curl` rate).

Expected: CPU/RAM/GPU bars move with real load, network bars move with
real transfer, no panel crash under either.

- [ ] **Step 8: Commit**

```bash
git add crates/app/src/side_panel_right
git commit -m "app : side_panel_right — CPU/RAM/GPU + сеть spectrum-виджеты, живые данные"
```

---

## Task 11: Power row with arm/confirm

**Files:**
- Create: `crates/app/src/side_panel_right/power_row.rs`
- Modify: `crates/app/src/side_panel_right/view.rs`

**Interfaces:**
- Consumes: `AppState::power(cx)` (Task 5: `.log_out()`, `.restart()`,
  `.shutdown()`).
- Produces: `#[derive(Default)] pub enum ArmState { #[default] Idle,
  Armed(PowerAction) }`, `pub enum PowerAction { LogOut, Restart,
  Shutdown }`, pure transitions `pub fn on_click(ArmState, PowerAction)
  -> ArmState` / `pub fn is_confirming_click(&ArmState, PowerAction) ->
  bool` / `pub fn on_timeout(ArmState) -> ArmState`, `pub(crate) const
  ARM_TIMEOUT: Duration`, and `pub fn render_power_row(theme: &Theme,
  arm: ArmState, cx: &mut Context<SidePanelRightView>) -> impl
  IntoElement`. Also adds the view method `pub(crate) fn
  SidePanelRightView::on_power_click(&mut self, action: PowerAction, cx:
  &mut Context<Self>)` and the view field `power_arm: ArmState`.

**Design decision (no confirm-dialog primitive exists in this codebase —
building one is out of scope for this plan):** clicking Log out /
Restart / Shutdown arms that button — its label changes to "Confirm?"
for 3 seconds. A second click on the same armed button within that window
executes the action. Any other click, or the timeout elapsing, disarms
back to idle. Switch user is always rendered disabled and never
participates in this state machine.

- [ ] **Step 1: Write the failing tests for the arm/confirm state machine**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_idle_arms_that_action() {
        let mut arm = ArmState::Idle;
        arm = on_click(arm, PowerAction::Restart);
        assert_eq!(arm, ArmState::Armed(PowerAction::Restart));
    }

    #[test]
    fn clicking_the_same_armed_action_again_confirms() {
        let arm = ArmState::Armed(PowerAction::Restart);
        assert!(is_confirming_click(&arm, PowerAction::Restart));
    }

    #[test]
    fn clicking_a_different_action_while_armed_rearms_to_the_new_one() {
        let mut arm = ArmState::Armed(PowerAction::Restart);
        assert!(!is_confirming_click(&arm, PowerAction::Shutdown));
        arm = on_click(arm, PowerAction::Shutdown);
        assert_eq!(arm, ArmState::Armed(PowerAction::Shutdown));
    }

    #[test]
    fn timeout_disarms_to_idle() {
        let arm = ArmState::Armed(PowerAction::LogOut);
        assert_eq!(on_timeout(arm), ArmState::Idle);
    }
}
```

Add `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` to `PowerAction` and
`ArmState` so the assertions above compile.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chronos-app --lib side_panel_right::power_row`
Expected: FAIL — module doesn't exist.

- [ ] **Step 3: Implement the state machine + row renderer**

```rust
//! Power row: Switch user (always disabled — no login manager exists,
//! see plan Global Constraints), Log out, Restart, Shutdown. The three
//! active buttons use an arm/confirm pattern instead of a modal dialog
//! (no confirm-dialog primitive exists in this codebase yet — building
//! one is out of scope here): first click arms (label → "Confirm?" for
//! 3s), second click within the window executes, anything else disarms.

use std::time::Duration;

use gpui::{Context, IntoElement, div, prelude::*, px};

use chronos_ui::Theme;

use crate::side_panel_right::view::SidePanelRightView;
use crate::state::AppState;

const ARM_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    LogOut,
    Restart,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArmState {
    #[default]
    Idle,
    Armed(PowerAction),
}

pub fn on_click(_current: ArmState, clicked: PowerAction) -> ArmState {
    ArmState::Armed(clicked)
}

pub fn is_confirming_click(current: &ArmState, clicked: PowerAction) -> bool {
    *current == ArmState::Armed(clicked)
}

pub fn on_timeout(_current: ArmState) -> ArmState {
    ArmState::Idle
}

fn label_for(action: PowerAction, arm: &ArmState) -> &'static str {
    if *arm == ArmState::Armed(action) {
        "Confirm?"
    } else {
        match action {
            PowerAction::LogOut => "Log out",
            PowerAction::Restart => "Restart",
            PowerAction::Shutdown => "Power",
        }
    }
}

/// Renders the row from a plain `ArmState` value. Click wiring is added
/// in step 5 via `cx.listener` — kept out of this step so step 4's tests
/// cover only the pure state machine.
pub fn render_power_row(theme: &Theme, arm: ArmState) -> impl IntoElement {
    div()
        .flex()
        .gap(px(2.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.))
                .p(px(10.))
                .text_color(theme.text.disabled)
                .child("User")
                .child(
                    div()
                        .text_size(px(8.5))
                        .font_family(theme.font_mono)
                        .child("SWITCH USER"),
                ),
        )
        .child(power_button(
            PowerAction::LogOut,
            label_for(PowerAction::LogOut, &arm),
            theme,
        ))
        .child(power_button(
            PowerAction::Restart,
            label_for(PowerAction::Restart, &arm),
            theme,
        ))
        .child(power_button(
            PowerAction::Shutdown,
            label_for(PowerAction::Shutdown, &arm),
            theme,
        ))
}

fn power_button(action: PowerAction, label: &str, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(("power-btn", action as usize))
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.))
        .p(px(10.))
        .cursor_pointer()
        .text_color(theme.text.secondary)
        .text_size(px(8.5))
        .font_family(theme.font_mono)
        .child(label.to_string())
    // `.on_click(...)` is attached by the caller in step 5 via
    // `cx.listener` — the return type is `Stateful<Div>` (not
    // `impl IntoElement`) precisely so the caller can still chain
    // `.on_click` onto it; `on_click` lives on
    // `StatefulInteractiveElement`, which is why `.id(..)` above is
    // mandatory, not decorative.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_idle_arms_that_action() {
        let mut arm = ArmState::Idle;
        arm = on_click(arm, PowerAction::Restart);
        assert_eq!(arm, ArmState::Armed(PowerAction::Restart));
    }

    #[test]
    fn clicking_the_same_armed_action_again_confirms() {
        let arm = ArmState::Armed(PowerAction::Restart);
        assert!(is_confirming_click(&arm, PowerAction::Restart));
    }

    #[test]
    fn clicking_a_different_action_while_armed_rearms_to_the_new_one() {
        let mut arm = ArmState::Armed(PowerAction::Restart);
        assert!(!is_confirming_click(&arm, PowerAction::Shutdown));
        arm = on_click(arm, PowerAction::Shutdown);
        assert_eq!(arm, ArmState::Armed(PowerAction::Shutdown));
    }

    #[test]
    fn timeout_disarms_to_idle() {
        let arm = ArmState::Armed(PowerAction::LogOut);
        assert_eq!(on_timeout(arm), ArmState::Idle);
    }
}
```

Note: `action as usize` requires `PowerAction` to be a plain enum without
data, which it is — `LogOut = 0, Restart = 1, Shutdown = 2` by
declaration order, fine as an `.id()` disambiguator.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chronos-app --lib side_panel_right::power_row`
Expected: PASS (4 tests).

- [ ] **Step 5: Attach the click handlers via `cx.listener`**

`on_click`'s listener type is `Fn(&ClickEvent, &mut Window, &mut App)`
(`Source/gpui/src/elements/div.rs:1475`, alias at `:1584`) — it never
hands you `&mut Context<Self>` directly. The adapter for that is
**`Context::listener`** (`Source/gpui/src/app/context.rs:252`): it takes
`Fn(&mut T, &E, &mut Window, &mut Context<T>)` and returns exactly the
`Fn(&E, &mut Window, &mut App)` shape `on_click` wants, by downgrading
the entity and doing `view.update(cx, ..)` internally.

This is the idiomatic path and it is **already used in this codebase** —
`crates/app/src/volume_popup/view.rs:199` mutates view-local state
(`this.expanded`) from an `on_click` this exact way, and 15 of the fork's
examples do the same (`Source/gpui/examples/opacity.rs:92` is the
shortest read). Do **not** reach for a `Global` here: arm state is
view-local UI state, and a `Global` would be shared across every panel
instance if the panel ever opens on more than one display.

Add the field to the view (`view.rs`):

```rust
use crate::side_panel_right::power_row::{
    is_confirming_click, on_click as arm_on_click, on_timeout, render_power_row, ArmState,
    PowerAction, ARM_TIMEOUT,
};

pub struct SidePanelRightView {
    // .. existing fields from tasks 9-10 ..
    power_arm: ArmState,
}
```

Initialize it in `new()`'s returned `Self` with `power_arm: ArmState::default(),`.

Note the `on_click as arm_on_click` rename: `power_row::on_click` (the
pure state-machine function from Step 3) collides by name with GPUI's
`on_click` element method that's in scope via `prelude::*`. Renaming at
the import keeps both readable.

Make `ARM_TIMEOUT` `pub(crate)` in `power_row.rs` (it was private in
Step 3) so `view.rs` can reference it.

- [ ] **Step 6: Implement the click + timeout behavior as a view method**

In `view.rs`, on `impl SidePanelRightView`:

```rust
    fn on_power_click(
        &mut self,
        action: PowerAction,
        cx: &mut Context<Self>,
    ) {
        if is_confirming_click(&self.power_arm, action) {
            match action {
                PowerAction::LogOut => AppState::power(cx).log_out(),
                PowerAction::Restart => AppState::power(cx).restart(),
                PowerAction::Shutdown => AppState::power(cx).shutdown(),
            }
            self.power_arm = ArmState::Idle;
            cx.notify();
            return;
        }

        let armed = arm_on_click(self.power_arm, action);
        self.power_arm = armed;
        cx.notify();

        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(ARM_TIMEOUT).await;
            // NOT `let _ = view.update(..)` — an Err means the view is
            // gone while a power action sits armed, the exact class of
            // swallowed Err that hid the ghost-window bug (HANDOFF.md
            // 2026-07-18). Plan Global Constraints forbid it.
            match view.update(cx, |view, cx| {
                // Only disarm if nothing re-armed a different action in
                // the meantime — otherwise a stale timer would cancel a
                // fresh arm the user just made.
                if view.power_arm == armed {
                    view.power_arm = on_timeout(armed);
                    cx.notify();
                }
            }) {
                Ok(()) => {}
                Err(e) => tracing::warn!(
                    "side_panel_right: power arm timeout could not disarm ({e}) — \
                     a power button may still read 'Confirm?'"
                ),
            }
        })
        .detach();
    }
```

- [ ] **Step 7: Render the row with listeners attached**

`render_power_row` from Step 3 builds the buttons but attaches no
listeners — it has no `cx` to build them from. Give it one. Replace its
Step 3 signature and body in `power_row.rs` with:

```rust
use gpui::Context;

use crate::side_panel_right::view::SidePanelRightView;

pub fn render_power_row(
    theme: &Theme,
    arm: ArmState,
    cx: &mut Context<SidePanelRightView>,
) -> impl IntoElement {
    div()
        .flex()
        .gap(px(2.))
        .child(
            // Switch user — always disabled, never armed, no listener.
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(6.))
                .p(px(10.))
                .text_color(theme.text.disabled)
                .child("User")
                .child(
                    div()
                        .text_size(px(8.5))
                        .font_family(theme.font_mono)
                        .child("SWITCH USER"),
                ),
        )
        .child(
            power_button(PowerAction::LogOut, label_for(PowerAction::LogOut, &arm), theme)
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.on_power_click(PowerAction::LogOut, cx);
                })),
        )
        .child(
            power_button(PowerAction::Restart, label_for(PowerAction::Restart, &arm), theme)
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.on_power_click(PowerAction::Restart, cx);
                })),
        )
        .child(
            power_button(PowerAction::Shutdown, label_for(PowerAction::Shutdown, &arm), theme)
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.on_power_click(PowerAction::Shutdown, cx);
                })),
        )
}
```

`power_button` returns `Stateful<Div>` (Step 3) specifically so
`.on_click` can be chained here — `on_click` is a
`StatefulInteractiveElement` method, which is why the `.id(..)` inside
`power_button` is load-bearing, not cosmetic.

Make `on_power_click` (Step 6) `pub(crate)` so `power_row.rs` can call it
across the module boundary.

Call it from `view.rs`'s `render`, after the network section:

```rust
            .child(render_power_row(&theme, self.power_arm, cx))
```

- [ ] **Step 8: Build**

Run: `cargo build --workspace`
Expected: clean build, zero `todo!()` left in the tree — grep to confirm:
`grep -rn "todo!" crates/app/src/side_panel_right/` must print nothing.

- [ ] **Step 9: Live smoke — arm/confirm round trip, with explicit user permission before Restart**

**Stop and ask the user for explicit in-the-moment permission before this
step** — it triggers a real `systemctl reboot` on their machine (per plan
Global Constraints / spec §7, destructive actions need live confirmation,
not just a code review). Once permitted: click Restart once, confirm the
label changes to "Confirm?", wait 4 seconds, confirm it reverts to
"Restart" (timeout path verified) without rebooting. Then click Restart
twice in quick succession and confirm the machine actually reboots.

Expected: arm/timeout/confirm all behave exactly as tested in Step 4,
and the real reboot happens only on the second click within the window.

- [ ] **Step 10: Commit**

```bash
git add crates/app/src/side_panel_right
git commit -m "app : side_panel_right — power row (log out/restart/shutdown arm-confirm, switch user disabled)"
```

---

## Task 12: Final assembly — bar click wiring + full live smoke

**Files:**
- Modify: wherever the bar's MPRIS bar-widget click handler is defined
  (grep `MprisCommand::PlayPause` in `crates/app/src/bar/widgets/` to find
  the exact file — do not guess the path, confirm it first).

**Interfaces:**
- Consumes: `side_panel_right::toggle` (Task 7).
- Produces: nothing new — this task only adds one `on_click` call to an
  existing widget.

- [ ] **Step 1: Find the bar's MPRIS widget click handler**

Run: `grep -rn "MprisCommand::PlayPause" crates/app/src/bar/widgets/`
Expected: one file, one `on_click` site — read the surrounding ~20 lines
to see its exact click-target div (likely the whole widget container, or
a specific play/pause icon within it).

- [ ] **Step 2: Add a secondary click target that toggles the panel**

Without changing the existing play/pause click behavior, add a new small
click target (e.g., a chevron/expand icon already present in the widget,
or the widget's own container if it currently only reacts to a sub-element)
that calls `crate::side_panel_right::toggle`. Show the exact diff here
only after Step 1's grep confirms the real structure — if the widget
container itself is the play/pause click target already, add a
dedicated small icon element for the panel toggle instead of overloading
the existing click (overloading would break play/pause). Write the
concrete diff against the real file content once confirmed; do not
pre-guess it blind in this plan.

- [ ] **Step 3: Build**

Run: `cargo build --workspace`
Expected: clean build.

- [ ] **Step 4: Full end-to-end live smoke**

Run the release binary with `RUST_LOG=info`, tee to a log file. In order:
1. Click the new bar toggle target → panel opens pinned, stays open when
   cursor moves elsewhere on the bar.
2. Re-click → panel closes.
3. Hover the right screen edge (no click) → panel peeks; move cursor
   away → panel closes (peek path, Task 8/9 already verified — re-confirm
   here as part of the full assembly, not a fresh test).
4. With the panel pinned open: verify MPRIS transport controls, mute,
   CPU/RAM/GPU bars under load (`stress-ng`/`glxgears`), network bars
   under `curl --limit-rate`, and the power row's arm/confirm — all as
   individually verified in Tasks 9–11, now confirmed together in one
   running instance without interference (e.g., confirm hovering off the
   panel while it's pinned does NOT close it — pinned must survive
   mouse-leave, only peek closes on leave).
5. Capture `grim` screenshots of: closed state, peeked state, pinned
   state with a track playing, pinned state with the Restart button armed
   ("Confirm?" visible).

Expected: all five checks pass, no ghost windows (confirm via
`hyprctl clients | grep chronos-side-panel` showing exactly zero or one
window matching, never two), no panics in the log.

- [ ] **Step 5: Update `roadmap.md`**

Add a closed-wave entry for this feature under a new
`## Волна «правая боковая панель»` heading, listing every commit hash
from Tasks 1–12 (`git log --oneline` since the first commit of this plan
to get the exact hashes — do not guess them).

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/bar/widgets roadmap.md
git commit -m "app : side_panel_right — интеграция с баром (клик-триггер), roadmap закрыт"
```

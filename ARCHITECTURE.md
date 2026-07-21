# Chronos Shell — Architecture

> Status: approved design (brainstorming phase complete)
> Last updated: 2026-07-09
> Scope: desktop shell for Hyprland 0.55.4+ on CachyOS (RTX 3070, i5 12400F, 64GB DDR4)
> Stack: Rust + GPUI (gpui-ce) + mLua-LuauJIT

Canonical architecture doc. Full design rationale and source citations live in
the original spec: `docs/superpowers/specs/2026-07-08-chronos-architecture-design.md`
(brainstorming output, kept as historical record — do not edit it after the fact,
amend here instead). Rejected alternatives and why are tracked in `DECISIONS.log`,
not duplicated here.

Note: Luau is a typed dialect of Lua (developed by Roblox), not classic Lua.
The `crates/luau` runtime uses Luau via mlua — type checking at plugin load acts
as an extra shield at the boundary (fewer runtime VM crashes, cheaper error
budget). mlua accepts BOTH classic Lua and Luau with no restriction, so plugin
authors may use either; Luau is the recommended default for its type safety,
but it is not mandated.

## 1. Goal

A modular, GPU-accelerated Wayland desktop shell with hot-reloadable LuaU
plugins. Modules: `bar` / `dock` / `launcher` / `notifications` / `osd`,
extensible with new modules without recompiling the core. Target: 144 FPS, no
process restart on config/plugin reload.

## 2. GPUI source: gpui-ce

Verified in source (`gpui-ce-main/crates/gpui_linux/src/linux/wayland/client.rs:827-834`):
gpui-ce resolves `WindowParams.display_id` into a concrete `wl_output` and
passes it to `get_layer_surface`. This closes Zed issue #48501 (layer-shell
window ignores target monitor) — still open in zed/main. gpui-ce also adds
`input_region` + `exclusive_zone` (PR #82), required for dock/bar geometry.

See `DECISIONS.log` for why zed/main and crates.io were rejected.

### Pinned revisions (git, reproducible)

> Updated 2026-07-16 (reanimation after the copy-move to `chronos-ecosystem/`).
> The old `gpui-ce-main` checkout (rev `20340e14` + patch `6a7b386`) no longer
> exists on disk; its git history is lost. The dependency is now OUR OWN fork.

| Dependency | Source | Rev / Version |
|---|---|---|
| **gpui-ce chronos edition** (gpui + gpui_platform + 16 sibling crates) | local `../Source/` (own git repo since 2026-07-16) | commit `3ce3466` "workspace skeleton restored" |
| zed-internal leftovers (`util_macros`, `http_client`, `reqwest_client`) | `git+https://github.com/zed-industries/zed` | rev `876ec5a8a074ba83cce2129ed4d76b59c05a37e9` (fork candidate if rev vanishes) |
| `gpui-component` → planned `crates/ui` | fork of `longbridge/gpui-component` in `../Source/gpui-component` | PLANNED — excluded from Source workspace, own workspace; upstream NOT tracked |
| `gpui-shell` (reference only) | `andre-brandao/gpui-shell` `c3476bd` | `reference/gpui-shell/` is EMPTY after the move — re-clone if needed, code study only |

Structure of `../Source/` (flattened, no `crates/` nesting): 9 base crates
(gpui, gpui_platform, gpui_linux, gpui_macros, gpui_shared_string, gpui_util,
gpui_web, gpui_wgpu, gpui_tokio) + 9 forked zed-internal crates
(gpui_collections, gpui_scheduler, gpui_sum_tree, gpui_refineable,
gpui_derive_refineable, gpui_media, gpui_zed_util, gpui_ce_util,
gpui_elements — the last EXCLUDED from workspace, 7 API-drift errors, unused).
Workspace root `Source/Cargo.toml` created 2026-07-16. Notable: `gpui_util`
(old name) is the one with `TypeIdHashBuilder` — `gpui_ce_util` lacks it, so
the old crate stays wired. Upstream sync is on-demand only.

## 3. Workspace structure

```
crates/
  app/        # entry point, lifecycle, window-manager (layer-shell windows),
              # single-instance socket, hot-reload orchestration
  ui/         # PLANNED: fork of gpui-component, trimmed (no tree-sitter/webview/markdown
              # unless needed); BarWidget / LauncherView traits (dyn-safe)
              # NOT YET CREATED (see status below)
  services/   # D-Bus + IPC integrations, wrapped in a unified Service trait
  luau/       # mLua-LuauJIT runtime, per-plugin VM pool, runtime module registry,
              # sandbox allocator, LuaU<->Rust API bridge
  plugins/    # LuaU plugins + manifest.toml
```

`luau` and `plugins` are separate crates = physical sandbox boundary. A plugin
panic cannot take down `services` or `app`. The Luau VM is per-plugin, typed,
and isolated (see §5).

**Status: `crates/luau` core implemented** (2026-07-09): sandbox, DSL,
API, PluginManager, clock example. inotify hot-reload watcher — NOT YET
IMPLEMENTED (see §9). `crates/services` and `crates/ui` not yet created.

## 4. Layer-shell windowing

Window creation is declarative via GPUI:
`WindowOptions.kind = WindowKind::LayerShell(LayerShellOptions { namespace, layer, anchor, exclusive_zone, margin, keyboard_interactivity })`.
No manual `wl_surface` calls — GPUI encapsulates the protocol.

- **Bar**: opens immediately on every display (`cx.displays()` + `display_id` in
  `WindowOptions`), edge from config (top/bottom/left/right), `Layer::Top`,
  `exclusive_zone` = bar thickness.
- **Launcher / Control Center / Notifications / OSD**: lazy, opened on demand
  (toggle / IPC / event). `Layer::Overlay`, `keyboard_interactivity` per use.
- **Multi-monitor**: works because gpui-ce closes #48501.

### 4.1 Layer-shell popup conventions (established 2026-07-19, all popups follow this)

Three real live-smoke bugs (`updates_popup`, `notifications`, launcher — see
`DECISIONS.log` 2026-07-19) converged on two standing rules for every
layer-shell popup with dynamic content (`updates_popup`, `volume_popup`,
`notifications`, `tray_menu`, `side_panel_right` skeleton `da744a2`, and
anything added later):

1. **Sizing: hard `.max_h(px(N)).overflow_hidden()` clip on variable-length
   list content, never a pixel-counted height estimate.** Mandatory chrome
   (a footer button, etc.) is laid out *outside* the clipped box so it can
   never be pushed off-window regardless of how tall rows actually render
   in this GPUI fork. `.overflow_hidden()` (clip) works here; `.overflow_y_
   scroll()` (real scroll) **[CORRECTED 2026-07-20: DOES work — needs
   `.id()`; see DECISIONS.log]** was believed not to resolve in this fork — don't reach for
   it.
2. **Dismiss: explicit only, never on focus loss.** `follow_mouse=1` in
   Hyprland fires spurious keyboard-deactivation the instant the cursor
   leaves a window; this is indistinguishable from a real dismiss at the
   event level, so no debounce fixes it — the only correct fix is to not
   close on `observe_window_activation`'s inactive branch at all. That
   observer may only be used to re-focus input when activity is regained.
   Valid dismiss paths: Esc, click-a-result/click-away action, re-toggle
   hotkey, an explicit close button, or a timer (`tray_menu`'s
   `schedule_autoclose` — a timer is not focus-loss detection, this is
   fine).

New popups also get the reentrant-close guard from §"СИСТЕМНЫЙ БАГ" in
HANDOFF.md (`close_this` pattern — never call `handle.update()` for
`remove_window()` from inside that same window's own callback).

Single-instance via Unix socket (`XDG_RUNTIME_DIR/chronos.sock`), hand-rolled
(no GPUI dependency) — reuse pattern from gpui-shell `ipc/service.rs`.

## 5. LuaU boundary & sandbox

Every plugin is a folder: `manifest.toml` + `init.luau`.

`manifest.toml` declares `capabilities` (subset of `fs`, `spawn`, `network`, `ipc`)
and optional `unsafe = true`.

Host loading (implemented in `crates/luau/src/sandbox.rs`):
1. Create a dedicated `mlua::Lua` instance per plugin.
2. Strip `os` / `io` / `debug` globals.
3. Register `chronos.*` API table into Lua global scope — capability-gated:
   - Always present: `chronos.bar`, `chronos.time`, `chronos.log`, `chronos.on`
   - Gated by manifest: `chronos.fs`, `chronos.process`, `chronos.net`, `chronos.ipc`
4. Without manifest → minimal rights (only base `chronos.*` API).
5. `unsafe = true` → full trust (TOFU), all capabilities enabled, for first-party plugins only.

`PluginManager` (`crates/luau/src/manager.rs`) handles discovery across multiple
dirs (`~/.chronos/plugins`, `/usr/share/chronos/plugins`), loading, and tick
dispatch via GPUI executor. Invalid manifest → skip plugin, `tracing::error!`,
never crash.

This is Rust-way: explicit, checked at the boundary, zero-cost after load.
Compatible with JIT hot-reload: recreate the LuaU instance, state lives in Rust.

## 6. Runtime module registry

gpui-shell hardcodes widgets in `enum Widget` + `match` (`registry.rs`) and
launcher views in `all_views()` — adding a module requires editing core.
Chronos uses a runtime registry instead:

- `crates/luau` exposes `chronos.bar:register(kind, render_fn, config)` etc.
- Rust side holds the widget collection as a **runtime registry**. The bar
  scaffold (2026-07-09) uses an order-preserving `Vec<Box<dyn BarWidget>>`
  because widget ORDER within a section (left→right) is layout-significant;
  `HashMap` is unordered and would scramble it. The originally-planned
  `HashMap<String, Box<dyn BarWidget>>` (name-keyed replacement of individual
  widgets) is deferred until named widget replacement is actually needed.
  `LauncherView` similarly uses `Vec<Box<dyn LauncherView>>`. See
  `DECISIONS.log` (2026-07-09 — Bar registry: Vec, not HashMap).
  The Vec registry now supports `replace_by_name(name, widget)` and
  `unregister_by_name(name)` for hot-reload — added 2026-07-09 for the
  luau plugin layer. `BarWidget` trait has `fn name() -> &str` (default: `"unnamed"`).
- A LuaU widget = thin Rust adapter whose `render()` calls the LuaU callback,
  returning an intermediate element DSL (serialized, not `AnyElement` directly).

`BarWidget` / `LauncherView` traits made object-safe (`dyn`), mirroring
gpui-shell structure but dynamic.

## 7. Services layer

From gpui-shell `crates/services`: D-Bus (NetworkManager, BlueZ, UPower, MPRIS,
tray, notifications) + Hyprland/Niri IPC. Pattern: `struct XxxSubscriber`
holding `futures_signals::Mutable<T>`, UI subscribes via signal — not callbacks.

Chronos wraps all subscribers in a unified
`trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }`
(gpui-shell lacks this). Reactive bridge to UI via `watch()` (`state.rs:143-164`)
— no Mutex in view code.

Compositor abstraction: `enum CompositorBackend { Hyprland, Niri }` + free
functions per backend (gpui-shell style). Hyprland via `hyprland` crate; Niri
via `niri-ipc`.

**Not every module under `crates/services` is a `Service`.** Pure helpers that
share state between surfaces live as free functions. Example (2026-07-21,
`dbce8ac`): `crates/services/src/net_stats.rs` — time-gated procfs
byte-rate sampling (`SAMPLE_INTERVAL`, `NetState`, `update_speed`,
`read_interface_bytes`). Registered only as `pub mod net_stats` (not in
`Services` / `init_all`). Consumers: bar `network` widget today; right side
panel network spectrum next. Design §3.3 still points at
`bar/widgets/network.rs` as the data story — that path is UI only now;
shared sampling is `chronos_services::net_stats`.

`panic = "unwind"` — a `unwrap()` in a listener thread must not kill the shell.
Services use `Result`/`expect` rigorously.

**Audio (WirePlumber MVP, 2026-07-17):** default sink/source via `wpctl`
poll (`AudioState` / `EndpointState`); device list via `pw-dump`
(`AudioDevice`). **Per-app playback streams / `ToggleStreamMute` (panel
Task 6)** — decided (DECISIONS.log 2026-07-21: stay on `wpctl`+`pw-dump`,
no native pipewire) and **implemented in the working tree**
(`AudioStream`, `parse_pw_dump_streams`, `find_stream_for_player`,
`AudioCommand::ToggleStreamMute`) but **not on master until accepted
and committed** — do not treat stream mute as shipped API. UI button =
panel Task 9 (also open).

## 8. Performance (144 FPS)

- LuaU is NEVER in the render path. Widgets render in Rust; LuaU only on events
  (workspace change, focus, config tick) and config load.
- Synchronous LuaU call budget: **< 4 ms** (144 Hz frame = 6.94 ms).
- State in Rust (`AppState` global + `futures_signals`); LuaU state lost on
  plugin reload (acceptable — reload is rare).

## 9. Hot-reload

- **Config**: inotify watch → `Config::reload` + `cx.refresh_windows()`
  (gpui-shell `config/mod.rs:133-185`). Bar does in-place update, not window
  teardown (avoid flicker at 144 fps).
- **LuaU plugins**: inotify watcher implemented (2026-07-09). Watches plugin
  dirs for CLOSE_WRITE | MOVED_TO | CREATE | DELETE events. Debounced at 300ms.
  New subdirectories get watches() + immediate poll (race condition fix).
  Reload drops old VM, recreates, re-registers widgets via
  BarWidgetRegistry::replace_by_name(). If new VM fails → keep old widgets.
  `replace_by_name` infrastructure exists; inotify watcher calls it.

## 10. Runtime strategy (tokio + GPUI executors)

Two runtimes, no conflict:

1. **GPUI main thread** — `App::background_executor()` / `cx.spawn()` for UI
   futures (widget timers, animations, hot-reload). Single-threaded, no tokio.
2. **Services layer** — dedicated `tokio::runtime::Runtime` (multi-thread) in
   a separate OS thread. All D-Bus (zbus), Hyprland IPC, upower, network,
   bluetooth run here. They communicate with UI via `futures_signals::Mutable`
   + `watch()` bridge (gpui-shell `state.rs:143-164`), NOT callbacks.

This matches gpui-shell pattern: `#[tokio::main]` in main, services spawn
their own `current_thread` tokio runtimes in `thread::spawn`. GPUI executor
stays UI-only.

`panic = "unwind"` — a panic in a service thread must not kill the shell.

## 11. What we reuse from gpui-shell (reference only, not a dependency)

- `window_options()` layer-shell matrix (`bar.rs:168-218`) — copy 1:1.
- Multi-monitor bar loop (`bar.rs:236-252`).
- Single-instance Unix socket (`ipc/service.rs`).
- `watch()` + `futures_signals` reactive bridge (`state.rs:143-164`).
- `BarWidget` / `LauncherView` trait shapes (make dyn-safe).
- TOML config + `FileWatcher` hot-reload (`config/mod.rs`).
- D-Bus service modules (network/upower/bluetooth/tray/notification) — near-ready.

## 12. What we do NOT reuse / fix

- Static `enum Widget` / `all_views()` → runtime registry (§6).
- `panic = "abort"` → `unwind` (§7, §10).
- `gpui = zed/main` (no pin) → `gpui-ce` pinned rev (§2).
- No Luau layer → **partially done** (2026-07-09): `crates/luau` core (sandbox, DSL, API, PluginManager) + `crates/plugins/clock` implemented (§3, §5, §6). inotify hot-reload watcher still missing (§9).
- Niri backend incomplete in gpui-shell (special workspaces bail) —
  acceptable, Hyprland is primary target.
- Audio tied to PulseAudio without graceful degradation — revisit if
  PipeWire-only.

## 13. Out of scope (YAGNI)

- Niri-first support (Hyprland primary).
- Plugin marketplace / signing.
- Remote/network plugin loading (local files only).
- Custom shaders (`runtime_shaders`) — not needed for MVP.

## 14. Top Bar redesign wave — DONE (2026-07-19…20)

All wave pieces landed (rationale per piece in `DECISIONS.log`
2026-07-19/20; live status of anything newer — `HANDOFF.md` top block):
cava visualizer (`c519e2e`), popup border/hover/badge polish (`8d74583`),
dock absorbed into the bar's left cluster + Start button (`07df942`,
standalone `dock/` window is gone; `dock/config.rs` persistence re-hosted
as-is), workspace dots (`8457bbc`), notification history + bell
(`f4ddd72`), system popup (`f7de445`), chrome consolidation onto the pult
display (`0a99a67`, see `monitor.rs`), token foundation (`3e04264`),
mockup-parity layout + separators (`c7ccc02`), SVG icon assets
(`f370618`), project switcher (`6061736`).

Durable architecture introduced by the wave:

- **`crates/app/src/monitor.rs`** — `pult_display(cx)` is the single
  source of display choice for ALL chrome windows (bar, launcher, every
  popup). Config `~/.config/chronos/monitor.toml` keyed by display
  `uuid()`; largest-display fallback auto-designates on first run.
- **`crates/app/src/assets.rs`** — GPUI `AssetSource` over
  `include_bytes!` (`crates/app/assets/icons/*.svg`), wired via
  `application().with_assets(...)`. Icons are single-color line-art;
  the SVG renderer uses them as an alpha mask tinted by `text_color`.
  New icon = file in `assets/icons/` + one line in the `icons!` macro.
- **`crates/app/src/project_switcher/`** — persistent project registry
  (`~/.config/chronos/projects.toml`, dock.toml pattern), bar pill with
  the active project's git branch (direct `.git/HEAD` parse on the bar's
  1s ticker — no subprocess, no inotify; worktrees + detached handled),
  picker popup (§4.1 lifecycle), "Add project" through the real
  `org.freedesktop.portal.FileChooser` via `ashpd` (feature `async-io`,
  NOT `tokio` — unification conflict with the gpui fork; portal runs on
  `async_io::block_on` in a dedicated thread).
- **Bar widget order = registration order** in
  `bar/widgets/mod.rs::register_builtin` — separators are positional
  widgets, the comment there is normative.

## Module scope

Own module set, shell fully functional without third-party tools:
`bar` / `dock` / `launcher` / `notifications` / `osd`, with architecture
allowing new modules to be added as plugins without rebuilding the core.

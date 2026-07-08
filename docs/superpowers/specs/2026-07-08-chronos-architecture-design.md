# Chronos Shell — Architecture Design

> Status: approved design (brainstorming phase complete)
> Date: 2026-07-08
> Scope: desktop shell for Hyprland 0.55.4+ on CachyOS (RTX 3070, i5 12400F, 64GB DDR4)
Stack: Rust + GPUI (gpui-ce) + mLua-luauJIT

Note: Luau is a typed dialect of Lua (developed by Roblox), not classic Lua.
The `crates/luau` runtime uses Luau via mlua — type checking at plugin load
acts as an extra shield at the boundary (fewer runtime VM crashes, cheaper
error budget). This is preferred over classic Lua for a sandboxed shell.

## 1. Goal

A modular, GPU-accelerated Wayland desktop shell with hot-reloadable LuaU
plugins. Modules: `bar` / `dock` / `launcher` / `notifications` / `osd`, extensible
with new modules without recompiling the core. Target: 144 FPS, no process
restart on config/plugin reload.

## 2. Why gpui-ce (not zed/main, not crates.io)

Verified in source (`gpui-ce-main/crates/gpui_linux/src/linux/wayland/client.rs:827-834`):
gpui-ce resolves `WindowParams.display_id` into a concrete `wl_output` and passes
it to `get_layer_surface`. This closes Zed issue #48501 (layer-shell window
ignores target monitor) — which is **still open in zed/main**. gpui-ce also adds
`input_region` + `exclusive_zone` (PR #82), required for dock/bar geometry.

crates.io is rejected: `gpui-ce` there is ~6 months stale (v0.3.3, no wayland
fixes); `gpui-component` there (v0.5.1) requires `gpui ^0.2.2` and is 10 versions
behind our local `0.58.0`. Local checkouts are the only consistent pair.

### Pinned revisions (git, reproducible)

| Dependency | Source | Rev / Version |
|---|---|---|
| `gpui-ce` (gpui + gpui_platform) | local `SOURCE/gpui/gpui-ce-main` → `gpui-ce/gpui-ce` | `20340e14874a3b55122e5cb2aa0d023874e08b2d` (2026-07-06) |
| `gpui-component` → our `crates/ui` | fork of `longbridge/gpui-component` `49d1bef84cb374c42d82b2e8d7e8b0d685d9ed48` | folded into workspace, upstream NOT tracked |
| `gpui-shell` (reference only) | `andre-brandao/gpui-shell` `c3476bd` | NOT a dependency, code study only |

Dependency mechanism: path-dep during early dev, migrate to `git = "...", rev = "..."`
once `gpui-ce-main` is a git repo. Upstream sync is on-demand only (when something
breaks), not a maintained relationship.

## 3. Workspace structure

```
crates/
  app/        # entry point, lifecycle, window-manager (layer-shell windows),
              # single-instance socket, hot-reload orchestration
  ui/         # fork of gpui-component, trimmed (no tree-sitter/webview/markdown
              # unless needed); BarWidget / LauncherView traits (dyn-safe)
  services/   # D-Bus + IPC integrations, wrapped in a unified Service trait
  luau/       # mLua-luauJIT runtime, per-plugin VM pool, runtime module registry,
              # sandbox allocator, LuaU<->Rust API bridge
  plugins/    # LuaU plugins + manifest.toml
```

`luau` and `plugins` are separate crates = physical sandbox boundary. A plugin
panic cannot take down `services` or `app`. The Luau VM is per-plugin, typed,
and isolated (see §5).

## 4. Layer-shell windowing (from gpui-shell reference)

Window creation is declarative via GPUI:
`WindowOptions.kind = WindowKind::LayerShell(LayerShellOptions { namespace, layer, anchor, exclusive_zone, margin, keyboard_interactivity })`.
No manual `wl_surface` calls — GPUI encapsulates the protocol.

- **Bar**: opens immediately on every display (`cx.displays()` + `display_id` in
  `WindowOptions`), edge from config (top/bottom/left/right), `Layer::Top`,
  `exclusive_zone` = bar thickness.
- **Launcher / Control Center / Notifications / OSD**: lazy, opened on demand
  (toggle / IPC / event). `Layer::Overlay`, `keyboard_interactivity` per use.
- **Multi-monitor**: works because gpui-ce closes #48501.

Single-instance via Unix socket (`XDG_RUNTIME_DIR/chronos.sock`), hand-rolled
(no GPUI dependency) — reuse pattern from gpui-shell `ipc/service.rs`.

## 5. LuaU boundary & sandbox (ADR-005 inverted)

Every plugin is a folder: `manifest.toml` + `init.luau`.

`manifest.toml` declares `capabilities` (subset of `fs`, `spawn`, `network`, `ipc`)
and optional `unsafe = true`.

Host loading:
1. Create a dedicated `mlua::Lua` instance per plugin.
2. Strip `os` / `io` / raw socket globals.
3. Register only the `chronos.*` API surface + capabilities declared in manifest.
4. Without manifest → minimal rights (only `chronos.*` declarative API, no `fs`/`spawn`).
5. `unsafe = true` → full trust (TOFU), for first-party plugins only.

This is Rust-way: explicit, checked at the boundary, zero-cost after load.
Compatible with JIT hot-reload: recreate the LuaU instance, state lives in Rust.

## 6. Runtime module registry (replaces gpui-shell static enums)

gpui-shell hardcodes widgets in `enum Widget` + `match` (`registry.rs`) and launcher
views in `all_views()` — adding a module requires editing core. Chronos uses a
runtime registry instead:

- `crates/luau` exposes `chronos.bar:register(kind, render_fn, config)` etc.
- Rust side holds `HashMap<String, Box<dyn BarWidget>>` (and `Vec<Box<dyn LauncherView>>`).
- A LuaU widget = thin Rust adapter whose `render()` calls the LuaU callback,
  returning an intermediate element DSL (serialized, not `AnyElement` directly).

`BarWidget` / `LauncherView` traits made object-safe (`dyn`), mirroring gpui-shell
structure but dynamic.

## 7. Services layer

From gpui-shell `crates/services`: D-Bus (NetworkManager, BlueZ, UPower, MPRIS,
tray, notifications) + Hyprland/Niri IPC. Pattern: `struct XxxSubscriber` holding
`futures_signals::Mutable<T>`, UI subscribes via signal — not callbacks.

Chronos wraps all subscribers in a unified `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }` (gpui-shell lacks this). Reactive bridge to
UI via `watch()` (`state.rs:143-164`) — no Mutex in view code.

Compositor abstraction: `enum CompositorBackend { Hyprland, Niri }` + free functions
per backend (gpui-shell style). Hyprland via `hyprland` crate; Niri via `niri-ipc`.

`panic = "abort"` from gpui-shell is REJECTED — a `unwrap()` in a listener thread
must not kill the shell. Use `panic = "unwind"` or rigorous `expect` audit.

## 8. Performance (144 FPS)

- LuaU is NEVER in the render path. Widgets render in Rust; LuaU only on events
  (workspace change, focus, config tick) and config load.
- Synchronous LuaU call budget: **< 4 ms** (144 Hz frame = 6.94 ms). Old draft's
  "16.7 ms / 60 fps" is rejected.
- State in Rust (`AppState` global + `futures_signals`); LuaU state lost on plugin
  reload (acceptable — reload is rare).

## 9. Hot-reload

- **Config**: inotify watch → `Config::reload` + `cx.refresh_windows()` (gpui-shell
  `config/mod.rs:133-185`). Bar does in-place update, not window teardown (avoid
  flicker at 144 fps).
- **LuaU plugins**: recreate VM instance, re-run `init.luau`, re-bind hooks. State
  in Rust survives.

## 10. What we reuse from gpui-shell (reference)

- `window_options()` layer-shell matrix (`bar.rs:168-218`) — copy 1:1.
- Multi-monitor bar loop (`bar.rs:236-252`).
- Single-instance Unix socket (`ipc/service.rs`).
- `watch()` + `futures_signals` reactive bridge (`state.rs:143-164`).
- `BarWidget` / `LauncherView` trait shapes (make dyn-safe).
- TOML config + `FileWatcher` hot-reload (`config/mod.rs`).
- D-Bus service modules (network/upower/bluetooth/tray/notification) — near-ready.

## 11. What we do NOT reuse / fix

- Static `enum Widget` / `all_views()` → runtime registry (§6).
- `panic = "abort"` → `unwind` (§7).
- `gpui = zed/main` (no pin) → `gpui-ce` pinned rev (§2).
- No LuaU layer → add `crates/luau` + `crates/plugins` (§3, §5).
- Niri backend incomplete in gpui-shell (special workspaces bail) — acceptable,
  Hyprland is primary target.
- Audio tied to PulseAudio without graceful degradation — revisit if PipeWire-only.

## 12. Out of scope (YAGNI)

- Niri-first support (Hyprland primary).
- Plugin marketplace / signing.
- Remote/network plugin loading (local files only).
- Custom shaders (`runtime_shaders`) — not needed for MVP.

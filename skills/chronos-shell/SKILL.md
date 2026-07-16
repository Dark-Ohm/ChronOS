---
name: chronos-shell
description: Working on THIS repo — a Rust/GPUI (gpui-ce) desktop shell for Hyprland/Niri with a sandboxed mlua/LuauJIT plugin system. Use when touching crates/app, crates/services, crates/luau, crates/plugins, the bar/dock/launcher modules, the Service trait, CompositorSubscriber, or the Lua plugin hot-reload path.
---

# Chronos Shell

Canonical design doc: `ARCHITECTURE.md` (accepted decisions) and `DECISIONS.log`
(rejected alternatives + why) at repo root. This skill is the "how the code is
actually laid out and wired together" companion — read those two for *why*,
this for *where*. For session orientation and skill routing, see `start-here`.

Stack: Rust (edition 2024) + GPUI via a local path dependency on `gpui-ce`
(`Cargo.toml` — `/home/neo/Projects/SOURCE/gpui/gpui-ce-main`, not crates.io
`gpui`) + `mlua` (Luau dialect) for plugins. Workspace members:
`crates/app` (bin `chronos`), `crates/luau` (`chronos-luau`), `crates/services`
(`chronos-services`). No UI component library — every element is built from
raw `gpui::div()`/`.flex()` (`crates/app/src/bar/mod.rs`); `gpui_component`
is **not** a dependency of this repo (that crate belongs to the sibling
`chronos-fm` project — don't reach for `h_flex()`/`v_flex()` here).

## Three real architectural patterns (verified against code, not aspirational)

### 1. Layer-shell windowing, not a normal window
The bar opens via GPUI's Wayland layer-shell support (a gpui-ce feature —
`gpui::layer_shell::*`), not a plain `WindowOptions` window:
`WindowKind::LayerShell(LayerShellOptions { namespace, layer: Layer::Top,
anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::TOP, exclusive_zone, .. })`
(`crates/app/src/bar/mod.rs:85-93`). One window is opened per display
(`bar::init`, same file, `~L110-133`), with a 100 ms startup delay to let
Wayland enumerate displays first.

### 2. `Service` trait — reactive, no async in the trait itself
`crates/services/src/lib.rs`: `trait Service { type Data; type Error: Send +
Sync + 'static; fn subscribe(&self) -> impl Signal<...>; fn get(&self) ->
Data; fn status(&self) -> ServiceStatus; }`. Backed by `futures_signals::Mutable`.
Commands are NOT part of the trait — they're concrete methods per subscriber
(e.g. `CompositorSubscriber::dispatch`). `CompositorSubscriber`
(`crates/services/src/compositor/mod.rs`) detects Hyprland vs Niri at runtime
and deliberately runs its listener on a plain `std::thread` (NOT tokio, NOT
the GPUI executor) — a sync connect/retry loop that `.join()`s the listener
thread so a panic or clean exit both fall through to re-fetch-and-restart,
never freezing at `Unavailable`. This is a documented, tested contract
(see `listener_panic_restarts_instead_of_freezing` in the same file) — don't
"fix" it into an async task without re-reading spec §5.2 in `ARCHITECTURE.md`.

### 3. Runtime split — three different executors, on purpose
- `tokio` (via `#[tokio::main]` in `main.rs`): the single-instance IPC Unix
  socket (`crates/app/src/ipc/service.rs` — accept loop, ping protocol).
- Plain `std::thread`: compositor listener (see above) — explicitly chosen
  to avoid a hidden tokio-runtime dependency in `chronos-services`.
- GPUI's own executor (`cx.spawn` / `cx.background_executor()`): plugin tick
  loop (`PluginManager::start_tick_loop`) and the inotify hot-reload watcher
  bridge (`crates/luau/src/watcher.rs`) — UI-adjacent work stays on GPUI's
  executor per the "GPUI main thread owns UI futures" split. The inotify
  *read* itself still needs a dedicated OS thread (inotify fds are
  non-blocking by crate design), which then forwards batches through a
  channel into a `cx.spawn` task — read the comment block at the top of
  `watcher.rs` before changing this, it explains exactly why both threads
  exist.

## Plugin system (`crates/luau`)

- Discovery: each subdir of a plugin dir needs both `manifest.toml` and
  `init.luau` (`manager.rs::scan_dir`) — missing either skips the dir with a
  warning, doesn't error.
- Manifest (`capabilities.rs`): `[plugin] name/version/author/description`,
  optional `[plugin.capabilities] fs/spawn/network/ipc` (all default false),
  optional `unsafe = true`.
- Sandbox (`sandbox.rs`): fresh `mlua::Lua` per plugin, strips `os`/`io`/`debug`
  globals, then registers a capability-gated `chronos.*` API table — `bar`,
  `time`, `log`, `on` are always present; `fs`/`process`/`net`/`ipc` are only
  added if the manifest declares that capability.
- Widget bridge: a plugin calls `chronos.bar:register({name, section, render})`
  in Lua; `PluginManager::get_registered_widgets` / `reregister_widgets` reads
  that back via `mlua::Table` reflection and wraps it in a
  `dsl::LuaWidgetAdapter`, registered into `BarWidgetRegistry` by
  **`replace_by_name`** — reload always replaces, never duplicates.
- **Identity gotcha (has a regression test, don't reintroduce it)**: plugins
  are identified by directory path, not by the manifest's `name` field — a
  plugin's directory (`test-race-plugin`) and its manifest name (`race`) are
  allowed to differ, and reload/unregister must still work
  (`manager.rs::reload_registers_widget_when_dir_name_differs_from_manifest_name`,
  `plugin_bridge.rs::register_plugin_widgets_handles_name_mismatch`). Anything
  that tries to re-derive "which plugin owns this widget" from the widget
  name alone is wrong.
- Hot-reload: `crates/luau/src/watcher.rs`, inotify on `CLOSE_WRITE |
  MOVED_TO | CREATE | DELETE`, 300 ms debounce, routes into
  `cx.update_global::<PluginManager, _>(|mgr, cx| mgr.reload(dir, cx))`. There's
  a dedicated regression test for the nested-lease case
  (`reload_through_update_global_updates_registry`) because `update_global`
  leases `PluginManager` out of the globals map — calling
  `global_mut::<BarWidgetRegistry>()` inside that closure is a *different*
  `TypeId` key and must not double-borrow-panic.

## Gotchas when editing this repo

- Edition is 2024 (`crates/app/Cargo.toml`, `crates/services/Cargo.toml`).
  Inline linters that don't pass `--edition 2024` will spuriously flag async
  code as invalid Rust 2015 — trust `cargo build`/`cargo test`, not the
  inline lint, when that happens.
- `gpui` and `gpui_platform` are **path** dependencies pointing outside this
  repo (`/home/neo/Projects/SOURCE/gpui/gpui-ce-main`). API questions about
  GPUI itself (not this project's usage of it) belong to the generic `gpui`
  skill, which covers upstream/gpui-ce APIs, not this repo's code.
- Don't confuse this repo with the similarly-named sibling projects on this
  machine: `Chronos-IDE` (a Hermes Agent GPUI IDE, separate Cargo workspace,
  member `chronos-agent`) and `chronos-fm` (a gpui-component-based file
  manager). Neither shares code, dependencies, or architecture with this
  repo despite the name overlap — content about ACP protocol, MCP tools,
  `chronos-agent`, `tokio-tungstenite`/`html2text`, or `gpui-component`
  belongs to one of those, not here.

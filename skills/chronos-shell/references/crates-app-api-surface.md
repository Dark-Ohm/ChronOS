# `crates/app` — Exhaustive API Surface Reference

**⚠️ STALE (2026-07-11 migration):** This reference was written for gpui-ce.
After the Kael migration, import paths (`gpui::App` → `kael::App`),
window kinds (`LayerShell` → `Overlay`/`Window`), and the entry point
(`gpui_platform::application()` → `Application::new()`) have changed.
Line numbers and many code patterns are wrong. Verify against actual
source before relying on specific claims here.

Auto-generated from source audit. Re-reading this is faster than re-reading
all 9 source files.

## Dependencies (Cargo.toml)

| Crate | Source | Version |
|---|---|---|
| gpui | workspace | path (gpui-ce-main) |
| gpui_platform | workspace | path (gpui-ce-main) |
| anyhow | workspace | 1.0.100 |
| tokio | workspace | 1.44.1 |
| tracing | workspace | 0.1.41 |
| tracing-subscriber | workspace | 0.3.19 |
| futures-signals | workspace | 0.3.34 |
| futures-util | workspace | 0.3 |
| chronos-luau | local | `../luau` |
| chronos-services | local | `../services` |
| dirs | crates.io | 6 |

Edition 2024. Library name: `chronos_app`. Binary name: `chronos`.

## Module tree

```
src/lib.rs           → pub mod state;                    (only public entry)
src/main.rs          → mod bar; mod ipc; mod plugin_bridge; pub mod state;
src/state.rs
src/plugin_bridge.rs
src/ipc/mod.rs       → mod messages; mod service; pub use service::IpcSubscriber;
src/ipc/messages.rs
src/ipc/service.rs
src/bar/mod.rs       → re-exports from chronos_luau::bar
examples/status-printer.rs
```

**Library public API** is minimal: only `state::AppState` + `state::watch`.
Everything else is internal to the binary.

## `src/state.rs` (129 lines)

### Public symbols

| Symbol | Kind | Lines | Signature |
|---|---|---|---|
| `AppState` | struct (Clone, Global) | 11–13 | `services: Services` (private field) |
| `AppState::init` | pub fn | 19–21 | `(services: Services, cx: &mut App)` |
| `AppState::global` | pub fn | 24–26 | `(cx: &App) -> &Self` |
| `AppState::compositor` | pub fn | 29–31 | `(cx: &App) -> &CompositorSubscriber` |
| `AppState::network` | pub fn | 34–36 | `(cx: &App) -> &NetworkSubscriber` |
| `AppState::upower` | pub fn | 39–41 | `(cx: &App) -> &UPowerSubscriber` |
| `watch` | pub fn | 48–69 | `<C, S, T, F>(cx: &mut Context<C>, signal: S, on_update: F)` |

`watch` constraints: `C: 'static, S: Signal<Item=T> + Unpin + 'static,
T: Clone + 'static, F: Fn(&mut C, T, &mut Context<C>) + 'static`.

### Tests (4)

- `app_state_module_compiles` (L80) — signature existence check
- `app_state_accessor_types` (L93) — return-type inference check
- `service_status_variants` (L113) — ServiceStatus enum accessibility
- `subscriber_types_accessible` (L123) — type_name check

## `src/ipc/` (216 lines total)

### `ipc/messages.rs` (30 lines)

| Symbol | Kind | Lines | Value |
|---|---|---|---|
| `PING_PAYLOAD` | pub const | 1 | `"ping"` |
| `encode_ping` | pub fn | 3–5 | `() -> String` |
| `is_ping` | pub fn | 7–9 | `(payload: &str) -> bool` (trims whitespace) |

### `ipc/service.rs` (188 lines)

| Symbol | Kind | Lines | Signature |
|---|---|---|---|
| `IpcReceiver` | pub type | 10 | `mpsc::UnboundedReceiver<()>` |
| `AcquireResult` | pub enum | 12–16 | `Primary(IpcSubscriber)`, `Secondary`, `Error(String)` |
| `IpcSubscriber` | pub struct | 19–22 | `listener: Option<TokioUnixListener>, socket_path: PathBuf` |
| `IpcSubscriber::init` | pub fn | 27–36 | `() -> Option<IpcSubscriber>` |
| `IpcSubscriber::start_listener` | pub fn | 40–50 | `(&mut self) -> IpcReceiver` |
| `Drop for IpcSubscriber` | impl | 53–59 | cleans up socket file |
| `acquire_at` | pub fn | 66–103 | `(path: &Path, payload: &str) -> AcquireResult` |
| `socket_path_in` | pub fn | 112–117 | `(runtime_dir: Option<&str>) -> PathBuf` |
| `socket_path` | pub fn | 119–121 | `() -> PathBuf` |

Private: `get_user_id` (L105), `accept_loop` (L123).

Socket path logic: `$XDG_RUNTIME_DIR/chronos.sock` or fallback
`/tmp/chronos-{UID}.sock`.

### `ipc/mod.rs` (28 lines)

Re-exports `IpcSubscriber` from service. Adds one method:
- `IpcSubscriber::start(&mut self, cx: &mut App)` (L11–27) — starts accept loop,
  logs pings, keeps self alive for socket lifetime.

### IPC Tests (6)

- `encodes_and_recognizes_ping` (L16)
- `rejects_non_ping_payload` (L22)
- `trims_surrounding_whitespace` (L27)
- `prefers_xdg_runtime_dir_when_set` (L159)
- `falls_back_to_tmp_when_unset` (L165)
- `second_acquire_on_same_path_becomes_secondary` (L172, tokio::test)

## `src/plugin_bridge.rs` (82 lines)

| Symbol | Kind | Lines | Signature |
|---|---|---|---|
| `register_plugin_widgets` | pub fn | 7–32 | `(plugin_manager: &PluginManager, cx: &mut gpui::App)` |

Iterates `plugin_manager.get_registered_widgets()`, extracts `render` fn from
Lua spec, maps section string → `BarSection`, sets render fn as global
`__chronos_render_{name}`, wraps in `LuaWidgetAdapter`, registers via
`BarWidgetRegistry::replace_by_name`.

### Tests (1)

- `register_plugin_widgets_handles_name_mismatch` (L45, #[gpui::test]) —
  regression: plugin dir name ≠ manifest widget name must not panic

## `src/bar/mod.rs` (133 lines)

### Re-exports (from chronos_luau::bar)

`BarSection`, `BarWidget`, `BarWidgetRegistry`, `BAR_COLOR`, `BAR_HEIGHT`

### Private symbols

| Symbol | Kind | Lines |
|---|---|---|
| `Bar` | struct | 12 |
| `Render for Bar` | impl | 14–39 |
| `section_div` | fn | 42–66 |
| `window_options` | fn | 69–96 |
| `open_on_display` | fn | 98–106 |

### Public

| Symbol | Kind | Lines | Signature |
|---|---|---|---|
| `bar::init` | pub fn | 110–133 | `(cx: &mut App)` — one window per display, 100ms startup delay |

Bar config: namespace `"bar"`, layer `Layer::Top`, anchors LEFT|RIGHT|TOP,
`WindowBackgroundAppearance::Transparent`, app_id `"chronos-bar"`.

## `src/main.rs` (57 lines)

Binary entrypoint. Startup sequence:
1. tracing_subscriber init (env filter)
2. `IpcSubscriber::init()` — exit if another instance
3. Tokio multi-thread runtime build
4. `chronos_services::init_all()` via block_on
5. `gpui_platform::application()` → `app.run()`
   - `AppState::init(services, cx)`
   - `subscriber.start(cx)`
   - `bar::init(cx)`
   - Plugin discovery + load from `~/.config/chronos/plugins` + `/usr/share/chronos/plugins`
   - `plugin_bridge::register_plugin_widgets`
   - `cx.set_global(plugin_manager)`
   - `PluginManager::start_tick_loop` + `start_watcher`

## `examples/status-printer.rs` (61 lines)

Minimal GPUI app subscribing to all three services (compositor, network, upower)
and logging updates. Proves the full reactive chain (spec §9).
Uses `AppState`, `Service::subscribe()`, `SignalExt::to_stream()`.

## TODO/FIXME/HACK

None found in any source file.

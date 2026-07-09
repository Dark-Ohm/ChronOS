# Chronos Luau Plugin Layer — Design Spec (MVP)

> Status: approved design (brainstorming complete)
> Date: 2026-07-09
> Scope: `crates/luau` runtime + `chronos.bar` bridge + 1 example plugin. Minimal `chronos.*` API surface sufficient for a working LuaU clock widget in the bar.
> Relation: implements `ARCHITECTURE.md` §5 (LuaU boundary & sandbox) + §6 (runtime module registry bridge) for the bar scaffold.

## 1. Goal

Give Chronos the ability to load LuaU plugins that render widgets in the bar (and eventually other modules) **without recompiling the core**. The bar scaffold (merged 2026-07-09) provides `BarWidgetRegistry::register(Box<dyn BarWidget>)`; this layer wraps a LuaU render callback in a Rust adapter and calls that seam.

The deliverable: a `crates/luau` crate (Rust, depends on `mlua` with the Luau feature) that discovers plugins, sandboxes them, exposes a minimal `chronos.*` API, and hot-reloads on file change via inotify. One shipped example plugin (`plugins/clock`) proves the path end-to-end.

## 2. File structure

```
crates/
  luau/
    Cargo.toml              # members of workspace; deps: mlua, toml, inotify, anyhow
    src/
      lib.rs                # pub: PluginManager, Element, TextStyle, Alignment
      manager.rs            # PluginManager: discovery, load, hot-reload (inotify)
      sandbox.rs            # per-plugin VM: create mlua::Lua, strip globals, register API
      dsl.rs                # Element enum + Lua→Element deserialize + Element→AnyElement
      capabilities.rs       # Manifest parsing, capability enforcement, unsafe=TOFU
      api/
        mod.rs              # register_chronos_api(plugins_ctx, mlua_instance)
        bar.rs              # chronos.bar.register(widget_spec)
        time.rs             # chronos.time.now()
        log.rs              # chronos.log(msg)
        events.rs           # chronos.on(event, callback), event dispatch
  plugins/                  # NOT a Cargo crate — directory with example LuaU plugins
    clock/
      manifest.toml
      init.luau
```

`crates/plugins/` is **not** a Cargo workspace member; it is a directory of example plugins shipped in the repo for documentation and testing. Real user plugins live at runtime in `~/.chronos/plugins/` and `/usr/share/chronos/plugins/`.

## 3. Manifest format

```toml
[plugin]
name = "clock"          # unique ID, used as key in PluginHandle
version = "0.1.0"
author = "Chronos Team"
description = "Simple clock widget for the bar"

[plugin.capabilities]
fs = false
spawn = false
network = false
ipc = false

unsafe = false
```

- `[plugin]` is mandatory: `name` (unique), `version`, `author`, `description` required.
- `[plugin.capabilities]` is optional (defaults to all `false` when omitted): subset of `fs`, `spawn`, `network`, `ipc`.
- `unsafe = true` → all capabilities enabled, no capability checks; strip is simplified (os/io removed only). TOFU model: trust the author on first use, for first-party plugins only.
- Invalid manifest → `tracing::error!` + skip plugin, never crash the shell.

Parsed via the `toml` crate.

## 4. Plugin lifecycle

### 4.1 Startup

1. `PluginManager::new(dirs, cx)` receives an ordered list of plugin directories (user-level first, then system-level).
2. For each directory: scan for subdirectories; for each subdirectory, attempt to read `manifest.toml`.
3. Valid manifest → create `mlua::Lua` instance via `sandbox::create_plugin_vm(manifest, plugins_ctx)`.
4. Register `chronos.*` API into the Lua global scope (capability-gated: only modules matching declared capabilities are registered).
5. Run `init.luau` inside the VM; the script calls `chronos.bar:register(widget_spec)`.
6. `PluginManager` stores the `PluginHandle { name, path, capabilities, lua, manifest_hash }`.
7. Registration is immediate: `BarWidgetRegistry::register(...)` is called synchronously during `cx.update()`.

### 4.2 Hot-reload

8. `PluginManager` starts an `inotify` watcher on each plugin directory (`IN_CLOSE_WRITE | IN_MOVED_TO`).
9. File change detected → identify affected plugin by path → drop old `mlua::Lua` instance (state lost; acceptable per ARCHITECTURE §9).
10. Re-read manifest → re-create VM → re-run `init.luau` → re-register widgets.
11. `BarWidgetRegistry` is updated; the next render frame picks up the new widgets (no flicker).

### 4.3 Shutdown

12. Drop all `mlua::Lua` instances (graceful; LuaU GC handles cleanup).
13. inotify watcher dropped automatically with `PluginManager`.

## 5. Sandbox

Per-plugin `mlua::Lua` instance, created in `sandbox.rs`:

1. **Strip dangerous globals**: `os`, `io`, `debug`, `rawget`/`rawset` (Luau has `rawget`/`rawset`; `setfenv`/`getfenv` are absent in Luau 5.2+). Keep: `math`, `string`, `table`, `coroutine`, `pcall`, `error`, `tostring`, `tonumber`, `type`, `pairs`, `ipairs`, `select`, `unpack` (or `table.unpack` in Luau), `require`.
2. **Register `chronos.*` table**: `globals().set("chronos", chronos_table)` where `chronos_table` contains:
   - `chronos.bar` (always present)
   - `chronos.time` (always present)
   - `chronos.log` (always present)
   - `chronos.on` (always present)
   - `chronos.fs` (only if `capabilities.fs = true`)
   - `chronos.process` (only if `capabilities.spawn = true`)
   - `chronos.net` (only if `capabilities.network = true`)
   - `chronos.ipc` (only if `capabilities.ipc = true`)
3. **`unsafe = true`**: skip strip (keep os/io for convenience); all capabilities enabled regardless of manifest.

Zero-cost after load: sandbox is enforced at VM creation time, not per-call.

## 6. Element DSL

### 6.1 Rust representation (`dsl.rs`)

```rust
pub enum Element {
    Text { content: String, style: TextStyle },
    Row { children: Vec<Element>, gap: f32, alignment: Alignment },
    Column { children: Vec<Element>, gap: f32, alignment: Alignment },
}

pub struct TextStyle {
    pub color: Option<Color>,   // parsed from "#rrggbb" hex string
    pub size: Option<Pixels>,   // font size in logical pixels
}

pub enum Alignment { Start, Center, End }
```

### 6.2 LuaU → Rust deserialization

`mlua::Table → Element` recursive converter:

- `table.type == "text"` → `Element::Text`
- `table.type == "row"` → `Element::Row` (recurse `children` table)
- `table.type == "column"` → `Element::Column`
- Unknown type → `tracing::warn!` + return empty `Element::Text`

Style parsing: `table.style.color` (string `"#rrggbb"`) → `Color`; `table.style.size` (number) → `Pixels`. Omitted fields → `None` (use defaults).

### 6.3 Rust → GPUI (`Element → AnyElement`)

```rust
impl Element {
    pub fn into_any_element(self) -> AnyElement {
        match self {
            Element::Text { content, style } => div()
                .text(style.size.unwrap_or(px(14.)))
                .text_color(style.color.unwrap_or(rgb(0xffffff)))
                .child(content)
                .into_any_element(),
            Element::Row { children, gap, alignment } => {
                let row = div().flex().items_center().gap(px(gap));
                // apply alignment justify_start/center/end
                row.children(children.into_iter().map(|c| c.into_any_element()))
                    .into_any_element()
            }
            Element::Column { children, gap, alignment } => {
                let col = div().flex().flex_col().gap(px(gap));
                col.children(children.into_iter().map(|c| c.into_any_element()))
                    .into_any_element()
            }
        }
    }
}
```

Minimum DSL for MVP: `text`, `row`, `column` + `style` (color/size). Extensions (icons, images, click handlers, gradients) deferred to future sessions.

## 7. `chronos.*` API surface

| Module | Requires capability | Description |
|---|---|---|
| `chronos.bar.register(spec)` | none (base) | Register a widget. `spec = { name: string, section: "left"/"center"/"right", render: function() → Element, on_tick?: function(callback) }` |
| `chronos.time.now()` | none | Returns Unix epoch timestamp as float |
| `chronos.log(msg)` | none | Writes to `tracing::info!` |
| `chronos.on(event, callback)` | none | Subscribe to events: `tick` (1s interval), `focus`, `workspace_change` |
| `chronos.fs.read_file(path)` | `fs` | Read file contents, returns string |
| `chronos.fs.write_file(path, data)` | `fs` | Write string to file |
| `chronos.fs.list_dir(path)` | `fs` | List directory, returns table of names |
| `chronos.process.spawn(cmd, args?)` | `spawn` | Spawn child process, returns handle |
| `chronos.net.fetch(url, method?)` | `network` | HTTP fetch, returns `{ status: number, body: string }` |
| `chronos.ipc.send(topic, data)` | `ipc` | Send IPC message |

### `tick` event dispatch

`PluginManager` holds a `tokio::spawn` timer (1-second interval). On each tick:
```rust
cx.update(|cx: &mut App| {
    for handle in &plugin_handles {
        if let Some(tick_cb) = &handle.tick_callback {
            // call LuaU tick callback via mlua
            let _ = handle.lua.globals().get::<mlua::Function>("__chronos_tick_callback")
                .and_then(|f| f.call(()));
        }
    }
});
```

`on_tick` registration in `chronos.bar.register`: if `spec.on_tick` is provided, `api/bar.rs` stores a Lua function reference and calls it on tick events.

## 8. Integration with `crates/app`

In `main.rs`, after `bar::init(cx)`:

```rust
let plugin_dirs = vec![
    dirs::config_dir().unwrap().join("chronos/plugins"),
    PathBuf::from("/usr/share/chronos/plugins"),
];
let plugin_manager = PluginManager::new(plugin_dirs, cx);
// PluginManager::start: loads plugins + starts inotify watcher (in background executor)
```

`PluginManager` lives as a `cx`-owned struct (not a GPUI global — it's unique per app lifetime). `BarWidgetRegistry::register(...)` is called via `cx.update_global::<BarWidgetRegistry, _>(|r| r.register(...))` from the plugin load path.

## 9. Error handling

- Invalid manifest → skip plugin, `tracing::error!`, continue.
- `init.luau` panics → `mlua::pcall` catches it, skip plugin, `tracing::error!`.
- `render()` callback error → return empty `Element::Text("")`, log warning.
- Hot-reload: old VM dropped, new one created; if new one fails → keep old widgets (don't unregister until new ones succeed).

## 10. Testing

- **Unit tests** (`crates/luau/src/` `#[cfg(test)]`):
  - Manifest parsing (valid, invalid, missing fields).
  - Sandbox strip: create VM, verify `os` global is `nil`.
  - Element DSL: Lua table → `Element` round-trip.
  - Capability gates: plugin without `fs` capability cannot access `chronos.fs`.
- **Integration test**: create `PluginManager` with a test plugins directory containing a mock `clock` plugin; verify `chronos.bar:register` was called and widget appears in `BarWidgetRegistry`.
- `cargo test -p chronos-luau` must pass.

## 11. Out of scope (YAGNI)

- `chronos.tray.*`, `chronos.workspace.*`, `chronos.compositor.*` — API modules for other shell modules (launcher/osd/notifications). Deferred until those modules exist.
- Plugin signing, marketplace, versioned distribution.
- Remote/network plugin loading.
- Custom shaders, GPU-accelerated plugin rendering.
- Plugin-to-plugin communication (IPC between plugins).

## 12. Acceptance criteria

1. `cargo build -p chronos-luau` compiles with 0 warnings.
2. `cargo test -p chronos-luau` passes (unit + integration tests).
3. `plugins/clock/` example plugin loads at startup, renders a clock in the bar's left section.
4. Editing `init.luau` while Chronos is running triggers hot-reload; clock updates without restart.
5. A plugin with `fs = false` in manifest cannot call `chronos.fs.read_file()` (sandbox enforced).
6. `ARCHITECTURE.md` §5/§6 remain consistent with this spec.

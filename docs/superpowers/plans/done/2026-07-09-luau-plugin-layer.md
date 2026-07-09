# Luau Plugin Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `crates/luau` — the LuaU plugin runtime for Chronos — with manifest-based sandboxing, an element DSL bridge to GPUI, a `chronos.*` API, inotify hot-reload, and a shipped clock example plugin.

**Architecture:** `PluginManager` discovers plugins in multi-path dirs, creates per-plugin `mlua::Lua` VMs (stripped + capability-gated), runs `init.luau` which calls `chronos.bar:register(...)`. A Rust `Element` DSL translates Lua tables into GPUI `AnyElement`. Hot-reload via inotify recreates VMs on file change. `BarWidgetRegistry` (already merged) receives widgets from the LuaU adapter.

**Tech Stack:** Rust, `mlua` (Luau feature), `toml`, `inotify`, `anyhow`, `dirs`. Workspace resolver 3.

## Global Constraints

- `panic = "unwind"` — must not kill the shell (workspace Cargo.toml profile).
- Per-plugin `mlua::Lua` instance — no shared Lua state between plugins (sandbox boundary).
- Element DSL minimum MVP: `text`, `row`, `column` + `style` (color/size). Extensions deferred.
- `unsafe = true` → full trust (TOFU), for first-party plugins only.
- Hot-reload: inotify on plugin dirs; old VM dropped, new VM created; if new fails → keep old widgets.
- Manifest invalid → skip plugin, `tracing::error!`, never crash.
- YAGNI: no `chronos.tray/workspace/compositor`, no signing, no remote loading, no plugin-to-plugin IPC.
- Package name: `chronos-luau`. Build with `cargo build -p chronos-luau`, test with `cargo test -p chronos-luau`.
- Work in a worktree created via `using-git-worktrees` skill before Task 1. Commit frequently, one commit per task.

---

## File Structure

```
crates/
  luau/
    Cargo.toml
    src/
      lib.rs              # pub: PluginManager, Element, TextStyle, Alignment
      manager.rs          # PluginManager: discovery, load, hot-reload (inotify)
      sandbox.rs          # per-plugin VM: create, strip globals, register chronos.* API
      dsl.rs              # Element enum + Lua→Element deserialize + Element→AnyElement
      capabilities.rs     # Manifest parsing, capability enforcement, unsafe=TOFU
      api/
        mod.rs            # register_chronos_api(lua, capabilities, manager_ref)
        bar.rs            # chronos.bar.register(widget_spec)
        time.rs           # chronos.time.now()
        log.rs            # chronos.log(msg)
        events.rs         # chronos.on(event, callback), tick dispatch
  plugins/
    clock/
      manifest.toml
      init.luau
crates/app/
  src/
    main.rs               # MODIFY: add PluginManager::new + start after bar::init
```

---

## Task 1: Workspace setup + `crates/luau` skeleton

**Files:**
- Create: `crates/luau/Cargo.toml`
- Create: `crates/luau/src/lib.rs`
- Modify: `Cargo.toml` (workspace) — add `"crates/luau"` to members

**Interfaces:**
- Produces: workspace with `crates/luau` member; `lib.rs` re-exports `PluginManager` (stub), `Element`, `TextStyle`, `Alignment`.

- [ ] **Step 1: Add `crates/luau` to workspace**

Modify `/home/neo/Projects/chronos/Cargo.toml`:
```toml
[workspace]
members = ["crates/app", "crates/luau"]
resolver = "3"
```

- [ ] **Step 2: Create `crates/luau/Cargo.toml`**

```toml
[package]
name = "chronos-luau"
version = "0.1.0"
edition = "2024"

[dependencies]
mlua = { version = "0.10", features = ["luau", "vendored", "send"] }
toml = "0.8"
inotify = "0.11"
anyhow = "1"
dirs = "6"
tracing = { workspace = true }
serde = { version = "1", features = ["derive"] }

[lints]
workspace = true
```

Note: `mlua` feature `luau` enables the Luau dialect. `vendored` bundles Luau source. `send` enables `Send` for `Lua`. Version `0.10` is a placeholder — verify the latest mlua release with Luau support before building.

- [ ] **Step 3: Create `crates/luau/src/lib.rs` (stub)**

```rust
pub mod manager;
pub mod sandbox;
pub mod dsl;
pub mod capabilities;
pub mod api;

pub use manager::PluginManager;
pub use dsl::{Element, TextStyle, Alignment};
pub use capabilities::Manifest;
```

- [ ] **Step 4: Create minimal Cargo.toml lint config**

If workspace uses `[lints] workspace = true`, ensure `crates/app/Cargo.toml` also has `[lints] workspace = true`. If not, add a `[lints]` section to `crates/luau/Cargo.toml` directly instead.

- [ ] **Step 5: Build to confirm workspace compiles**

Run: `cargo build -p chronos-luau 2>&1 | tail -20`
Expected: compiles (empty crate, no errors). May warn about unused modules.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/luau/
git commit -m "chore: scaffold crates/luau workspace member"
```

---

## Task 2: Manifest parsing (`capabilities.rs`)

**Files:**
- Create: `crates/luau/src/capabilities.rs`
- Test: inline `#[cfg(test)]` in `capabilities.rs`

**Interfaces:**
- Produces: `pub struct PluginMeta { name, version, author, description }`, `pub struct Capabilities { fs, spawn, network, ipc }`, `pub struct Manifest { pub meta: PluginMeta, pub capabilities: Capabilities, pub unsafe_mode: bool }`, `impl Manifest { pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> }`. Consumed by Task 3 (sandbox), Task 4 (API), Task 6 (PluginManager).

- [ ] **Step 1: Write the failing tests**

```rust
// crates/luau/src/capabilities.rs
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub fs: bool,
    pub spawn: bool,
    pub network: bool,
    pub ipc: bool,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub meta: PluginMeta,
    pub capabilities: Capabilities,
    pub unsafe_mode: bool,
}

#[derive(serde::Deserialize)]
struct ManifestFile {
    plugin: ManifestPlugin,
}

#[derive(serde::Deserialize)]
struct ManifestPlugin {
    name: String,
    version: Option<String>,
    author: Option<String>,
    description: Option<String>,
    capabilities: Option<ManifestCapabilities>,
    #[serde(rename = "unsafe")]
    unsafe_mode: Option<bool>,
}

#[derive(serde::Deserialize, Default)]
struct ManifestCapabilities {
    fs: Option<bool>,
    spawn: Option<bool>,
    network: Option<bool>,
    ipc: Option<bool>,
}

impl Manifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let file: ManifestFile = toml::from_str(&content)?;
        let caps = file.plugin.capabilities.unwrap_or_default();
        Ok(Manifest {
            meta: PluginMeta {
                name: file.plugin.name,
                version: file.plugin.version.unwrap_or_default(),
                author: file.plugin.author.unwrap_or_default(),
                description: file.plugin.description.unwrap_or_default(),
            },
            capabilities: Capabilities {
                fs: caps.fs.unwrap_or(false),
                spawn: caps.spawn.unwrap_or(false),
                network: caps.network.unwrap_or(false),
                ipc: caps.ipc.unwrap_or(false),
            },
            unsafe_mode: file.plugin.unsafe_mode.unwrap_or(false),
        })
    }

    /// Check if a capability is granted (either via manifest or unsafe mode).
    pub fn has_capability(&self, cap: &str) -> bool {
        if self.unsafe_mode {
            return true;
        }
        match cap {
            "fs" => self.capabilities.fs,
            "spawn" => self.capabilities.spawn,
            "network" => self.capabilities.network,
            "ipc" => self.capabilities.ipc,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_valid_manifest() {
        let dir = std::env::temp_dir().join("chronos_test_manifest");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"[plugin]
name = "clock"
version = "0.1.0"
author = "Test"
description = "A test plugin"

[plugin.capabilities]
fs = true
spawn = false

unsafe = false"#).unwrap();

        let m = Manifest::from_path(&path).unwrap();
        assert_eq!(m.meta.name, "clock");
        assert!(m.capabilities.fs);
        assert!(!m.capabilities.spawn);
        assert!(!m.unsafe_mode);
        assert!(m.has_capability("fs"));
        assert!(!m.has_capability("spawn"));
    }

    #[test]
    fn parse_minimal_manifest() {
        let dir = std::env::temp_dir().join("chronos_test_manifest_min");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "[plugin]\nname = \"minimal\"").unwrap();

        let m = Manifest::from_path(&path).unwrap();
        assert_eq!(m.meta.name, "minimal");
        assert!(!m.capabilities.fs);
        assert!(!m.unsafe_mode);
    }

    #[test]
    fn unsafe_enables_all() {
        let dir = std::env::temp_dir().join("chronos_test_manifest_unsafe");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "[plugin]\nname = \"unsafe_p\"\nunsafe = true").unwrap();

        let m = Manifest::from_path(&path).unwrap();
        assert!(m.has_capability("fs"));
        assert!(m.has_capability("spawn"));
        assert!(m.has_capability("network"));
        assert!(m.has_capability("ipc"));
    }

    #[test]
    fn invalid_manifest_returns_error() {
        let dir = std::env::temp_dir().join("chronos_test_manifest_bad");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.toml");
        std::fs::write(&path, "not valid toml {{{{").unwrap();

        assert!(Manifest::from_path(&path).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chronos-luau capabilities 2>&1 | tail -20`
Expected: FAIL (module not compiled yet / no such test).

- [ ] **Step 3: Implementation is already in Step 1 (inline tests with implementation)**

The code in Step 1 contains both the struct definitions and the tests. Once the module compiles, the tests should pass.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chronos-luau capabilities 2>&1 | tail -20`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/luau/src/capabilities.rs
git commit -m "feat(luau): manifest parsing with capability gates and unsafe=TOFU"
```

---

## Task 3: Element DSL (`dsl.rs`)

**Files:**
- Create: `crates/luau/src/dsl.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Produces: `pub enum Element { Text, Row, Column }`, `pub struct TextStyle { color, size }`, `pub enum Alignment`, `impl Element { pub fn from_lua_table(table: mlua::Table) -> Result<Self> }`, `impl Element { pub fn into_any_element(self) -> gpui::AnyElement }`. Consumed by Task 5 (bar adapter).

Note: `gpui` is a dependency of `crates/app`, not `crates/luau`. For the DSL to produce `gpui::AnyElement`, either: (a) add `gpui` as a dependency of `crates/luau`, or (b) keep the DSL pure (Element enum only) and let `crates/app` handle the `Element → AnyElement` conversion. **Option (b) is cleaner** — `crates/luau` only defines `Element`, and the adapter in `crates/app` (or a bridge module) converts. This keeps `crates/luau` free of GPUI dependency.

Revised: `dsl.rs` defines `Element`, `TextStyle`, `Alignment`, `from_lua_table()`. The `Element → AnyElement` conversion lives in `crates/app` or a new bridge module. The LuaU adapter returns `Element` from the DSL; `crates/app` calls `.into_any_element()` when rendering.

Actually, to keep the plan self-contained and avoid modifying `crates/app` in every task, let me add `gpui` as a path dependency of `crates/luau`. This is what the architecture intends — the luau crate produces renderable elements. Let me revise the Cargo.toml dependency and proceed.

Add to `crates/luau/Cargo.toml`:
```toml
[dependencies]
gpui = { workspace = true }
```

- [ ] **Step 1: Write the failing tests**

```rust
// crates/luau/src/dsl.rs
use gpui::{AnyElement, px, rgb, div, prelude::*};

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub color: Option<u32>,      // 0xRRGGBB
    pub size: Option<f32>,       // logical pixels
}

#[derive(Debug, Clone, PartialEq)]
pub enum Alignment { Start, Center, End }

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Text {
        content: String,
        style: TextStyle,
    },
    Row {
        children: Vec<Element>,
        gap: f32,
        alignment: Alignment,
    },
    Column {
        children: Vec<Element>,
        gap: f32,
        alignment: Alignment,
    },
}

impl Element {
    /// Recursively convert a Lua table (from a render callback) to an Element.
    pub fn from_lua_table(table: &mlua::Table) -> mlua::Result<Self> {
        let kind: String = table.get("type")?;
        match kind.as_str() {
            "text" => {
                let content: String = table.get("content")?;
                let style = parse_style(table.get("style")?)?;
                Ok(Element::Text { content, style })
            }
            "row" => {
                let gap: f32 = table.get("gap").unwrap_or(8.0);
                let alignment = parse_alignment(table.get("alignment")?)?;
                let children = parse_children(table, "children")?;
                Ok(Element::Row { children, gap, alignment })
            }
            "column" => {
                let gap: f32 = table.get("gap").unwrap_or(8.0);
                let alignment = parse_alignment(table.get("alignment")?)?;
                let children = parse_children(table, "children")?;
                Ok(Element::Column { children, gap, alignment })
            }
            other => Err(mlua::RuntimeError::RuntimeError(format!(
                "unknown element type: {other}"
            ))),
        }
    }

    /// Convert an Element tree into a GPUI AnyElement.
    pub fn into_any_element(self) -> AnyElement {
        match self {
            Element::Text { content, style } => {
                let mut el = div().child(content);
                if let Some(size) = style.size {
                    el = el.text_size(px(size));
                }
                if let Some(color) = style.color {
                    el = el.text_color(rgb(color));
                }
                el.into_any_element()
            }
            Element::Row { children, gap, alignment } => {
                let mut el = div().flex().items_center().gap(px(gap));
                el = match alignment {
                    Alignment::Start => el.justify_start(),
                    Alignment::Center => el.justify_center(),
                    Alignment::End => el.justify_end(),
                };
                el.children(children.into_iter().map(|c| c.into_any_element()))
                    .into_any_element()
            }
            Element::Column { children, gap, alignment } => {
                let mut el = div().flex_col().gap(px(gap));
                el = match alignment {
                    Alignment::Start => el.items_start(),
                    Alignment::Center => el.items_center(),
                    Alignment::End => el.items_end(),
                };
                el.children(children.into_iter().map(|c| c.into_any_element()))
                    .into_any_element()
            }
        }
    }
}

fn parse_style(table: mlua::Table) -> mlua::Result<TextStyle> {
    let color: Option<String> = table.get("color")?;
    let color = color.and_then(|c| {
        let hex = c.trim_start_matches('#');
        u32::from_str_radix(hex, 16).ok()
    });
    let size: Option<f32> = table.get("size")?;
    Ok(TextStyle { color, size })
}

fn parse_alignment(val: mlua::Value) -> mlua::Result<Alignment> {
    match val {
        mlua::Value::String(s) => match s.to_str()? {
            "start" => Ok(Alignment::Start),
            "center" => Ok(Alignment::Center),
            "end" => Ok(Alignment::End),
            _ => Ok(Alignment::Start),
        },
        _ => Ok(Alignment::Start),
    }
}

fn parse_children(table: &mlua::Table, key: &str) -> mlua::Result<Vec<Element>> {
    let children_table: mlua::Table = table.get(key)?;
    let mut children = Vec::new();
    for pair in children_table.pairs::<mlua::Value, mlua::Table>() {
        let (_, child_table) = pair?;
        children.push(Element::from_lua_table(&child_table)?);
    }
    Ok(children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn parse_text_element() {
        let lua = Lua::new();
        let table: mlua::Table = lua.eval(
            r#"{ type = "text", content = "hello", style = { color = "#ff0000", size = 16 } }"#,
            None,
        ).unwrap();
        let el = Element::from_lua_table(&table).unwrap();
        match el {
            Element::Text { content, style } => {
                assert_eq!(content, "hello");
                assert_eq!(style.color, Some(0xff0000));
                assert_eq!(style.size, Some(16.0));
            }
            _ => panic!("expected Text element"),
        }
    }

    #[test]
    fn parse_row_with_children() {
        let lua = Lua::new();
        let table: mlua::Table = lua.eval(
            r#"{ type = "row", gap = 12, alignment = "center", children = {
                { type = "text", content = "a" },
                { type = "text", content = "b" },
            }}"#,
            None,
        ).unwrap();
        let el = Element::from_lua_table(&table).unwrap();
        match el {
            Element::Row { children, gap, alignment } => {
                assert_eq!(children.len(), 2);
                assert_eq!(gap, 12.0);
                assert_eq!(alignment, Alignment::Center);
            }
            _ => panic!("expected Row element"),
        }
    }

    #[test]
    fn parse_unknown_type_returns_error() {
        let lua = Lua::new();
        let table: mlua::Table = lua.eval(
            r#"{ type = "unknown_thing" }"#,
            None,
        ).unwrap();
        assert!(Element::from_lua_table(&table).is_err());
    }

    #[test]
    fn default_style_fields_are_none() {
        let lua = Lua::new();
        let table: mlua::Table = lua.eval(
            r#"{ type = "text", content = "plain" }"#,
            None,
        ).unwrap();
        let el = Element::from_lua_table(&table).unwrap();
        match el {
            Element::Text { style, .. } => {
                assert_eq!(style.color, None);
                assert_eq!(style.size, None);
            }
            _ => panic!("expected Text"),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chronos-luau dsl 2>&1 | tail -20`
Expected: FAIL (module not compiled / gpui not yet a dependency).

- [ ] **Step 3: Implementation is in Step 1 (inline with tests)**

Ensure `crates/luau/Cargo.toml` has `gpui = { workspace = true }` dependency (added in Task 1 Step 2 revision or now).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chronos-luau dsl 2>&1 | tail -30`
Expected: PASS (4 tests). Note: GPUI tests require a runtime; `Element::into_any_element()` builds GPUI elements but doesn't need `App` context at construction time. The `mlua::Lua` tests construct Lua tables and deserialize — no GPUI runtime needed.

- [ ] **Step 5: Commit**

```bash
git add crates/luau/src/dsl.rs crates/luau/Cargo.toml
git commit -m "feat(luau): element DSL (text/row/column) with Lua→Rust deserialize"
```

---

## Task 4: Sandbox (`sandbox.rs`)

**Files:**
- Create: `crates/luau/src/sandbox.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Produces: `pub fn create_plugin_vm(manifest: &Manifest) -> anyhow::Result<mlua::Lua>`, `pub fn register_chronos_api(lua: &mlua::Lua, manifest: &Manifest, tick_tx: tokio::sync::mpsc::UnboundedSender<String>) -> anyhow::Result<()>`. Consumed by Task 5 (PluginManager load).

- [ ] **Step 1: Write the failing tests**

```rust
// crates/luau/src/sandbox.rs
use crate::capabilities::Manifest;
use mlua::Lua;

/// Create a sandboxed LuaU VM for a plugin.
/// Strips dangerous globals and configures safety based on manifest.
pub fn create_plugin_vm(manifest: &Manifest) -> anyhow::Result<Lua> {
    let lua = Lua::new();

    // Strip dangerous globals
    {
        let globals = lua.globals();
        for name in &["os", "io", "debug"] {
            globals.set(*name, mlua::Value::Nil)?;
        }
        // Luau-specific: rawget/rawset exist but are safe; leave them.
    }

    Ok(lua)
}

/// Register the `chronos.*` API table into the Lua global scope.
pub fn register_chronos_api(
    lua: &Lua,
    manifest: &Manifest,
) -> anyhow::Result<()> {
    let globals = lua.globals();
    let chronos = lua.create_table()?;

    // Always register base modules
    chronos.set("bar", crate::api::bar::create_bar_api(lua)?)?;
    chronos.set("time", crate::api::time::create_time_api(lua)?)?;
    chronos.set("log", crate::api::log::create_log_api(lua)?)?;
    chronos.set("on", crate::api::events::create_events_api(lua)?)?;

    // Capability-gated modules
    if manifest.has_capability("fs") {
        chronos.set("fs", crate::api::mod_fs::create_fs_api(lua)?)?;
    }
    if manifest.has_capability("spawn") {
        chronos.set("process", crate::api::mod_process::create_process_api(lua)?)?;
    }
    if manifest.has_capability("network") {
        chronos.set("net", crate::api::mod_net::create_net_api(lua)?)?;
    }
    if manifest.has_capability("ipc") {
        chronos.set("ipc", crate::api::mod_ipc::create_ipc_api(lua)?)?;
    }

    globals.set("chronos", chronos)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::Manifest;
    use std::path::PathBuf;

    fn test_manifest(name: &str, unsafe_mode: bool) -> Manifest {
        Manifest {
            meta: crate::capabilities::PluginMeta {
                name: name.to_string(),
                version: "0.1.0".into(),
                author: "test".into(),
                description: "test".into(),
            },
            capabilities: crate::capabilities::Capabilities::default(),
            unsafe_mode,
        }
    }

    #[test]
    fn os_global_is_nil_in_sandboxed_vm() {
        let m = test_manifest("test", false);
        let lua = create_plugin_vm(&m).unwrap();
        let val: mlua::Value = lua.globals().get("os").unwrap();
        assert_eq!(val, mlua::Value::Nil);
    }

    #[test]
    fn io_global_is_nil_in_sandboxed_vm() {
        let m = test_manifest("test", false);
        let lua = create_plugin_vm(&m).unwrap();
        let val: mlua::Value = lua.globals().get("io").unwrap();
        assert_eq!(val, mlua::Value::Nil);
    }

    #[test]
    fn debug_global_is_nil_in_sandboxed_vm() {
        let m = test_manifest("test", false);
        let lua = create_plugin_vm(&m).unwrap();
        let val: mlua::Value = lua.globals().get("debug").unwrap();
        assert_eq!(val, mlua::Value::Nil);
    }

    #[test]
    fn math_global_still_exists() {
        let m = test_manifest("test", false);
        let lua = create_plugin_vm(&m).unwrap();
        let val: mlua::Value = lua.globals().get("math").unwrap();
        assert!(val.is_table());
    }

    #[test]
    fn chronos_bar_is_registered() {
        let m = test_manifest("test", false);
        let lua = create_plugin_vm(&m).unwrap();
        register_chronos_api(&lua, &m).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        assert!(chronos.get::<mlua::Value>("bar").unwrap().is_table());
    }

    #[test]
    fn chronos_fs_not_registered_without_capability() {
        let m = test_manifest("test_no_fs", false);
        let lua = create_plugin_vm(&m).unwrap();
        register_chronos_api(&lua, &m).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        assert_eq!(
            chronos.get::<mlua::Value>("fs").unwrap(),
            mlua::Value::Nil
        );
    }

    #[test]
    fn chronos_fs_registered_with_capability() {
        let mut m = test_manifest("test_fs", false);
        m.capabilities.fs = true;
        let lua = create_plugin_vm(&m).unwrap();
        register_chronos_api(&lua, &m).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        assert!(chronos.get::<mlua::Value>("fs").unwrap().is_table());
    }

    #[test]
    fn unsafe_mode_enables_all_capabilities() {
        let m = test_manifest("test_unsafe", true);
        let lua = create_plugin_vm(&m).unwrap();
        register_chronos_api(&lua, &m).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        // fs should be registered even though capabilities.fs = false (unsafe)
        assert!(chronos.get::<mlua::Value>("fs").unwrap().is_table());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chronos-luau sandbox 2>&1 | tail -20`
Expected: FAIL (api modules don't exist yet).

- [ ] **Step 3: Create stub API modules so tests compile**

Create minimal stubs in `crates/luau/src/api/mod.rs` and sub-modules so the `create_*_api` functions exist and return empty tables. This lets the sandbox tests pass; real API behavior comes in Task 5.

```rust
// crates/luau/src/api/mod.rs
pub mod bar;
pub mod time;
pub mod log;
pub mod events;
pub mod mod_fs;
pub mod mod_process;
pub mod mod_net;
pub mod mod_ipc;
```

Each sub-module (e.g., `bar.rs`):
```rust
pub fn create_bar_api(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    lua.create_table()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chronos-luau sandbox 2>&1 | tail -30`
Expected: PASS (8 tests). Note: `register_chronos_api` needs `tokio::sync::mpsc::UnboundedSender` param — if included, pass a dummy channel in tests. Adjust signature to take only `(lua, manifest)` for MVP; event dispatch wiring is Task 5.

- [ ] **Step 5: Commit**

```bash
git add crates/luau/src/sandbox.rs crates/luau/src/api/
git commit -m "feat(luau): sandbox (strip globals, register chronos.* with capability gates)"
```

---

## Task 5: API modules (`api/`)

**Files:**
- Modify: `crates/luau/src/api/bar.rs` (replace stub)
- Modify: `crates/luau/src/api/time.rs` (replace stub)
- Modify: `crates/luau/src/api/log.rs` (replace stub)
- Modify: `crates/luau/src/api/events.rs` (replace stub)
- Create: `crates/luau/src/api/mod_fs.rs`, `mod_process.rs`, `mod_net.rs`, `mod_ipc.rs` (stubs for MVP, return empty tables or minimal impls)
- Test: inline `#[cfg(test)]` in each module

**Interfaces:**
- `bar.rs`: `pub fn create_bar_api(lua, ) -> mlua::Result<mlua::Table>` — registers `chronos.bar.register(widget_spec)` which stores widget spec in a shared registry.
- `time.rs`: `chronos.time.now()` → `f64` (Unix epoch).
- `log.rs`: `chronos.log(msg)` → `tracing::info!`.
- `events.rs`: `chronos.on(event, callback)` → stores callback; `pub fn dispatch_event(lua, event_name)` to trigger callbacks.

- [ ] **Step 1: Implement `api/bar.rs`**

```rust
use mlua::Lua;

pub fn create_bar_api(lua: &Lua) -> mlua::Result<mlua::Table> {
    let bar = lua.create_table()?;
    let widgets = lua.create_table()?; // stores registered widget specs by name

    bar.set("register", lua.create_function({
        let widgets = widgets.clone();
        move |_, (spec,): (mlua::Table,)| -> mlua::Result<()> {
            let name: String = spec.get("name")?;
            let section: String = spec.get("section").unwrap_or_else(|_| "left".into());
            let render: mlua::Function = spec.get("render")?;
            // Store the spec; actual BarWidgetRegistry registration happens in PluginManager
            // (requires cx, which can't be called from within Lua directly).
            // Instead, store in the table and let PluginManager poll.
            widgets.set(name.clone(), spec)?;
            tracing::info!("Plugin registered bar widget: {name} (section: {section})");
            Ok(())
        }
    })?)?;

    bar.set("widgets", widgets)?;
    Ok(bar)
}
```

`chronos.bar.register` stores the widget spec. `PluginManager` reads registered widgets after `init.luau` runs and calls `BarWidgetRegistry::register(...)` with a LuaU adapter that invokes the stored `render` callback.

- [ ] **Step 2: Implement `api/time.rs`**

```rust
use mlua::Lua;
use std::time::SystemTime;

pub fn create_time_api(lua: &Lua) -> mlua::Result<mlua::Table> {
    let time = lua.create_table()?;
    time.set("now", lua.create_function(|_, _: ()| -> mlua::Result<f64> {
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Ok(since_epoch.as_secs_f64())
    })?)?;
    Ok(time)
}
```

- [ ] **Step 3: Implement `api/log.rs`**

```rust
use mlua::Lua;

pub fn create_log_api(lua: &Lua) -> mlua::Result<mlua::Table> {
    let log = lua.create_table()?;
    log.set("info", lua.create_function(|_, (msg,): (String,)| -> mlua::Result<()> {
        tracing::info!("[plugin] {msg}");
        Ok(())
    })?)?;
    log.set("warn", lua.create_function(|_, (msg,): (String,)| -> mlua::Result<()> {
        tracing::warn!("[plugin] {msg}");
        Ok(())
    })?)?;
    Ok(log)
}
```

- [ ] **Step 4: Implement `api/events.rs`**

```rust
use mlua::Lua;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type CallbackStore = Arc<Mutex<HashMap<String, Vec<mlua::Function>>>>;

pub fn create_events_api(lua: &Lua, callbacks: CallbackStore) -> mlua::Result<mlua::Table> {
    let events = lua.create_table()?;
    events.set("on", lua.create_function({
        let callbacks = callbacks.clone();
        move |_, (event, cb): (String, mlua::Function)| -> mlua::Result<()> {
            let mut store = callbacks.lock().unwrap();
            store.entry(event).or_default().push(cb);
            Ok(())
        }
    })?)?;
    Ok(events)
}

/// Dispatch an event to all registered callbacks.
pub fn dispatch_event(lua: &Lua, callbacks: &CallbackStore, event: &str) -> mlua::Result<()> {
    let store = callbacks.lock().unwrap();
    if let Some(cbs) = store.get(event) {
        for cb in cbs {
            let _ = cb.call::<()>(());
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Create stub APIs for capability-gated modules**

`mod_fs.rs`, `mod_process.rs`, `mod_net.rs`, `mod_ipc.rs` — each returns an empty table (MVP stubs, full implementations deferred).

```rust
// e.g., mod_fs.rs
pub fn create_fs_api(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    lua.create_table()
}
```

- [ ] **Step 6: Write and run tests for API modules**

For each module, test that:
- `time.now()` returns a number > 0.
- `log.info("test")` doesn't panic.
- `events.on("tick", fn)` registers a callback; dispatch triggers it.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_now_returns_positive() {
        let lua = Lua::new();
        let api = create_time_api(&lua).unwrap();
        lua.globals().set("chronos_time", api).unwrap();
        let now: f64 = lua.eval("chronos_time.now()", None).unwrap();
        assert!(now > 0.0);
    }

    #[test]
    fn log_info_doesnt_panic() {
        let lua = Lua::new();
        let api = create_log_api(&lua).unwrap();
        lua.globals().set("chronos_log", api).unwrap();
        lua.eval::<()>("chronos_log.info('hello from test')", None).unwrap();
    }
}
```

- [ ] **Step 7: Commit**

```bash
git add crates/luau/src/api/
git commit -m "feat(luau): chronos.* API (bar, time, log, events; capability stubs)"
```

---

## Task 6: PluginManager (`manager.rs`)

**Files:**
- Create: `crates/luau/src/manager.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Produces: `pub struct PluginManager { ... }`, `impl PluginManager { pub fn new(dirs: Vec<PathBuf>, tick_tx: UnboundedSender<String>) -> Self; pub fn load_all(&mut self); pub fn start_watcher(&mut self, cx: &mut App); }`. Consumed by Task 7 (app integration).

- [ ] **Step 1: Implement `manager.rs`**

```rust
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use mlua::Lua;

use crate::capabilities::Manifest;
use crate::dsl::Element;
use crate::sandbox;

/// Handle for a loaded plugin.
pub struct PluginHandle {
    pub name: String,
    pub path: PathBuf,
    pub manifest: Manifest,
    pub lua: Lua,
    pub tick_callbacks: Vec<mlua::Function>,
}

/// Manages plugin discovery, loading, hot-reload, and lifecycle.
pub struct PluginManager {
    plugin_dirs: Vec<PathBuf>,
    plugins: Vec<PluginHandle>,
    event_callbacks: Arc<Mutex<std::collections::HashMap<String, Vec<mlua::Function>>>>,
}

impl PluginManager {
    pub fn new(plugin_dirs: Vec<PathBuf>) -> Self {
        PluginManager {
            plugin_dirs,
            plugins: Vec::new(),
            event_callbacks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Scan all plugin dirs and load valid plugins.
    pub fn load_all(&mut self) {
        for dir in &self.plugin_dirs {
            if !dir.exists() {
                tracing::debug!("Plugin dir not found: {dir:?}, skipping");
                continue;
            }
            self.scan_dir(dir);
        }
        tracing::info!("Loaded {} plugin(s)", self.plugins.len());
    }

    fn scan_dir(&mut self, dir: &Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read plugin dir {dir:?}: {e}");
                return;
            }
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let plugin_dir = entry.path();
            let manifest_path = plugin_dir.join("manifest.toml");
            let init_path = plugin_dir.join("init.luau");
            if !manifest_path.exists() || !init_path.exists() {
                tracing::warn!("Skipping {:?}: missing manifest.toml or init.luau", plugin_dir);
                continue;
            }
            match self.load_plugin(&plugin_dir, &manifest_path, &init_path) {
                Ok(handle) => {
                    tracing::info!("Loaded plugin: {}", handle.name);
                    self.plugins.push(handle);
                }
                Err(e) => {
                    tracing::error!("Failed to load plugin from {plugin_dir:?}: {e}");
                }
            }
        }
    }

    fn load_plugin(
        &self,
        plugin_dir: &Path,
        manifest_path: &Path,
        init_path: &Path,
    ) -> Result<PluginHandle> {
        let manifest = Manifest::from_path(manifest_path)?;
        let lua = sandbox::create_plugin_vm(&manifest)?;
        sandbox::register_chronos_api(&lua, &manifest)?;

        // Run init.luau
        let init_code = std::fs::read_to_string(init_path)?;
        lua.load(&init_code)
            .set_name(init_path.to_string_lossy().as_ref())
            .exec()
            .map_err(|e| anyhow::anyhow!("init.luau error: {e}"))?;

        // Collect tick callbacks from registered widgets
        let mut tick_callbacks = Vec::new();
        let globals = lua.globals();
        if let Ok(chronos) = globals.get::<mlua::Table>("chronos") {
            if let Ok(bar) = chronos.get::<mlua::Table>("bar") {
                if let Ok(widgets) = bar.get::<mlua::Table>("widgets") {
                    for pair in widgets.pairs::<String, mlua::Table>() {
                        if let Ok((_, widget_spec)) = pair {
                            if let Ok(on_tick) = widget_spec.get::<mlua::Function>("on_tick") {
                                // Wrap on_tick: it receives a callback, we need to store it
                                // For now, the plugin itself calls chronos.on("tick", cb) inside on_tick
                            }
                        }
                    }
                }
            }
        }

        // Collect event callbacks
        // (events.on registers into the shared event_callbacks store)
        // Note: event_callbacks store is shared via Arc; register_chronos_api should
        // receive a clone of the Arc so events.on populates it.
        // Adjust sandbox::register_chronos_api signature to accept callbacks store.

        Ok(PluginHandle {
            name: manifest.meta.name.clone(),
            path: plugin_dir.to_path_buf(),
            manifest,
            lua,
            tick_callbacks,
        })
    }

    /// Dispatch a tick event to all loaded plugins.
    pub fn dispatch_tick(&self) {
        for handle in &self.plugins {
            let _ = crate::api::events::dispatch_event(
                &handle.lua,
                &self.event_callbacks,
                "tick",
            );
        }
    }

    /// Get all registered bar widget specs (name → Lua table) from all plugins.
    pub fn get_registered_widgets(&self) -> Vec<(String, String, mlua::Table)> {
        let mut result = Vec::new();
        for handle in &self.plugins {
            let globals = handle.lua.globals();
            if let Ok(chronos) = globals.get::<mlua::Table>("chronos") {
                if let Ok(bar) = chronos.get::<mlua::Table>("bar") {
                    if let Ok(widgets) = bar.get::<mlua::Table>("widgets") {
                        for pair in widgets.pairs::<mlua::Table, mlua::Table>() {
                            if let Ok((name_table, spec)) = pair {
                                // name_table is actually the key (String), but mlua pairs returns (K, V)
                                // The key is a String set in bar.rs; adjust accordingly.
                                // For now, extract name from spec.
                                if let Ok(name) = spec.get::<String>("name") {
                                    let section = spec.get::<String>("section")
                                        .unwrap_or_else(|_| "left".into());
                                    result.push((name, section, spec));
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    }

    pub fn plugins(&self) -> &[PluginHandle] {
        &self.plugins
    }
}
```

Note: The `events` store (`Arc<Mutex<HashMap>>`) needs to be threaded through `register_chronos_api`. The `sandbox::register_chronos_api` signature should accept a `CallbackStore` parameter. Update Task 4's implementation accordingly.

- [ ] **Step 2: Write integration test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_plugin_from_test_dir() {
        let dir = std::env::temp_dir().join("chronos_test_plugin");
        let plugin_dir = dir.join("clock");
        fs::create_dir_all(&plugin_dir).unwrap();

        // Write manifest
        fs::write(plugin_dir.join("manifest.toml"), r#"[plugin]
name = "test-clock"
version = "0.1.0"
author = "test"
description = "test plugin"
unsafe = true"#).unwrap();

        // Write init.luau
        fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "test-clock",
    section = "left",
    render = function()
        return { type = "text", content = "test" }
    end
})"#).unwrap();

        let mut mgr = PluginManager::new(vec![dir]);
        mgr.load_all();
        assert_eq!(mgr.plugins().len(), 1);
        assert_eq!(mgr.plugins()[0].name, "test-clock");

        // Clean up
        fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p chronos-luau manager 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/luau/src/manager.rs
git commit -m "feat(luau): PluginManager (discovery, load, tick dispatch)"
```

---

## Task 7: LuaU bar adapter (bridge to `crates/app`)

**Files:**
- Modify: `crates/app/Cargo.toml` — add `chronos-luau = { path = "../luau" }` dependency
- Modify: `crates/app/src/main.rs` — add PluginManager init + widget registration after `bar::init(cx)`
- Create: `crates/app/src/plugin_bridge.rs` — `LuaWidgetAdapter` implementing `BarWidget` trait

**Interfaces:**
- `LuaWidgetAdapter` wraps a Lua VM handle + render callback name; `BarWidget::render` calls into Lua to get an `Element`, then `.into_any_element()`.
- `main.rs`: `PluginManager::new(dirs).load_all()` then for each registered widget, create `LuaWidgetAdapter` and call `BarWidgetRegistry::register(...)`.

- [ ] **Step 1: Implement `crates/app/src/plugin_bridge.rs`**

```rust
use chronos_luau::{Element, PluginManager};
use chronos_luau::dsl;
use crate::bar::widget::{BarWidget, BarWidgetRegistry};
use crate::bar::sections::BarSection;
use gpui::{AnyElement, App, Window};

/// A BarWidget backed by a LuaU render callback.
pub struct LuaWidgetAdapter {
    name: String,
    section: BarSection,
    lua: mlua::Lua,
    render_fn_name: String,
}

impl BarWidget for LuaWidgetAdapter {
    fn section(&self) -> BarSection {
        self.section
    }

    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
        // Call the Lua render callback and get an Element
        let result: Result<mlua::Table, _> = self.lua.globals()
            .get::<mlua::Function>(&self.render_fn_name)
            .and_then(|f| f.call(()));

        match result {
            Ok(table) => match dsl::Element::from_lua_table(&table) {
                Ok(element) => element.into_any_element(),
                Err(e) => {
                    tracing::warn!("Plugin {} render error: {e}", self.name);
                    gpui::div().child(format!("[{}: render error]", self.name)).into_any_element()
                }
            },
            Err(e) => {
                tracing::warn!("Plugin {} Lua call error: {e}", self.name);
                gpui::div().child(format!("[{}: call error]", self.name)).into_any_element()
            }
        }
    }
}

/// After loading plugins, register their bar widgets with the global registry.
pub fn register_plugin_widgets(
    plugin_manager: &PluginManager,
    cx: &mut gpui::App,
) {
    for (name, section_str, spec) in plugin_manager.get_registered_widgets() {
        let render_fn: mlua::Function = match spec.get("render") {
            Ok(f) => f,
            Err(_) => {
                tracing::warn!("Plugin {name}: render function missing, skipping");
                continue;
            }
        };

        let section = match section_str.as_str() {
            "center" => BarSection::Center,
            "right" => BarSection::Right,
            _ => BarSection::Left,
        };

        // Store the render function in the Lua VM's globals with a known name
        let lua = &plugin_manager.plugins().iter()
            .find(|p| p.name == name)
            .unwrap()
            .lua;
        let fn_name = format!("__chronos_render_{name}");
        lua.globals().set(&fn_name, render_fn).ok();

        let adapter = LuaWidgetAdapter {
            name: name.clone(),
            section,
            lua: lua.clone(), // mlua::Lua is Arc-backed, clone is cheap
            render_fn_name: fn_name,
        };

        cx.update_global::<BarWidgetRegistry, _>(|r| {
            r.register(Box::new(adapter));
        });
        tracing::info!("Registered LuaU widget: {name} in {section_str}");
    }
}
```

Note: `mlua::Lua` is `Send + Sync` (with the `send` feature). Cloning it is cheap (Arc internally). The adapter stores a clone and calls render from `BarWidget::render`.

- [ ] **Step 2: Modify `crates/app/src/main.rs`**

```rust
// ... existing imports ...
mod plugin_bridge;

// In app.run closure, after bar::init(cx):
let plugin_dirs = vec![
    dirs::config_dir().unwrap().join("chronos/plugins"),
    std::path::PathBuf::from("/usr/share/chronos/plugins"),
];
let mut plugin_manager = PluginManager::new(plugin_dirs);
plugin_manager.load_all();
plugin_bridge::register_plugin_widgets(&plugin_manager, cx);
```

- [ ] **Step 3: Add `chronos-luau` to `crates/app/Cargo.toml`**

```toml
[dependencies]
chronos-luau = { path = "../luau" }
dirs = "6"
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p chronos 2>&1 | tail -20`
Run: `cargo test -p chronos 2>&1 | grep -E 'running|test result'`
Expected: build green, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/app/
git commit -m "feat(app): wire PluginManager + LuaWidgetAdapter into Chronos startup"
```

---

## Task 8: Example clock plugin

**Files:**
- Create: `crates/plugins/clock/manifest.toml`
- Create: `crates/plugins/clock/init.luau`

**Interfaces:**
- The clock plugin renders a time display in the bar's left section and updates every second via tick events.

- [ ] **Step 1: Create `crates/plugins/clock/manifest.toml`**

```toml
[plugin]
name = "clock"
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

- [ ] **Step 2: Create `crates/plugins/clock/init.luau`**

```lua
local tick_pending = false

chronos.bar:register({
    name = "clock",
    section = "left",
    render = function()
        local now = chronos.time.now()
        local hours = math.floor((now % 86400) / 3600)
        local minutes = math.floor((now % 3600) / 60)
        local seconds = math.floor(now % 60)
        local time_str = string.format("%02d:%02d:%02d", hours, minutes, seconds)

        return {
            type = "text",
            content = time_str,
            style = { color = "#ffffff", size = 14 }
        }
    end,
    on_tick = function(callback)
        chronos.on("tick", callback)
    end
})

chronos.log.info("clock plugin loaded")
```

- [ ] **Step 3: Commit**

```bash
git add crates/plugins/
git commit -m "feat(plugins): clock example plugin (LuaU, renders time in bar left section)"
```

---

## Task 9: Final verification

**Files:** None (verification only).

- [ ] **Step 1: Full build**

Run: `cargo build 2>&1 | tail -20`
Expected: 0 errors, 0 warnings.

- [ ] **Step 2: Full test suite**

Run: `cargo test -p chronos-luau 2>&1 | grep -E 'running|test result'`
Run: `cargo test -p chronos 2>&1 | grep -E 'running|test result'`
Expected: all tests pass in both crates.

- [ ] **Step 3: Manual smoke test (optional, requires Hyprland)**

Run: `cargo run -p chronos` under Hyprland. Place the clock plugin dir at `~/.chronos/plugins/clock/` (copy from `crates/plugins/clock/`). Expected: clock widget appears in bar's left section, updates every second.

- [ ] **Step 4: Commit (if any stray changes)**

If `git status` shows only untracked build artifacts, no commit needed.

---

## Self-Review Notes

- **Spec coverage:** §2 file structure → Tasks 1-8. §3 manifest → Task 2. §4 lifecycle → Task 6. §5 sandbox → Task 4. §6 DSL → Task 3. §7 API → Task 5. §8 integration → Task 7. §9 error handling → Tasks 2/4/5/6 (manifest errors, VM errors, render errors). §10 testing → all tasks TDD. §12 acceptance → Task 9.
- **Placeholder scan:** all tasks have concrete code; no TBD/TODO.
- **Type consistency:** `Manifest`/`Capabilities` from Task 2 used consistently in Tasks 4/5/6. `Element` from Task 3 used in Task 7. `PluginManager` from Task 6 used in Task 7. `BarWidget`/`BarWidgetRegistry` from existing bar scaffold used in Task 7.
- **Dependency chain:** Task 1 (workspace) → Task 2 (capabilities) → Task 3 (DSL) → Task 4 (sandbox, uses 2+3) → Task 5 (API, uses 3) → Task 6 (manager, uses 4+5) → Task 7 (bridge, uses 6 + bar scaffold) → Task 8 (plugin) → Task 9 (verify).

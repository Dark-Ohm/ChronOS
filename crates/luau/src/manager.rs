use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use mlua::Lua;

use crate::capabilities::Manifest;
use crate::sandbox;

impl gpui::Global for PluginManager {}

/// Handle for a loaded plugin.
pub struct PluginHandle {
    pub name: String,
    pub path: PathBuf,
    pub manifest: Manifest,
    pub lua: Lua,
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
        let dirs: Vec<PathBuf> = self.plugin_dirs.clone();
        for dir in &dirs {
            if !dir.exists() {
                tracing::debug!("Plugin dir not found: {dir:?}, skipping");
                continue;
            }
            self.scan_dir(dir);
        }
        tracing::info!("Loaded {} plugin(s)", self.plugins.len());
    }

    fn scan_dir(&mut self, dir: &Path) {
        eprintln!("scan_dir: scanning {:?}", dir);
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("scan_dir: failed to read {:?}: {}", dir, e);
                tracing::warn!("Failed to read plugin dir {dir:?}: {e}");
                return;
            }
        };
        for entry in entries.flatten() {
            eprintln!("scan_dir: entry {:?}", entry.path());
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                eprintln!("scan_dir: skipping non-dir {:?}", entry.path());
                continue;
            }
            let plugin_dir = entry.path();
            let manifest_path = plugin_dir.join("manifest.toml");
            let init_path = plugin_dir.join("init.luau");
            eprintln!("scan_dir: manifest={}, init={}", manifest_path.exists(), init_path.exists());
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
                    eprintln!("load_plugin error: {e}");
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
        sandbox::register_chronos_api(&lua, &manifest, self.event_callbacks.clone())?;

        // Run init.luau
        let init_code = std::fs::read_to_string(init_path)?;
        lua.load(&init_code)
            .set_name(init_path.to_string_lossy().as_ref())
            .exec()
            .map_err(|e| anyhow::anyhow!("init.luau error: {e}"))?;

        Ok(PluginHandle {
            name: manifest.meta.name.clone(),
            path: plugin_dir.to_path_buf(),
            manifest,
            lua,
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

    /// Reload a single plugin. If new VM fails, keeps old widgets.
    /// Updates BarWidgetRegistry via cx — call through cx.update_global.
    /// Identifies the plugin by `plugin_dir` (path), not by re-deriving a
    /// name from the directory basename — the manifest `[plugin] name` is
    /// free-form Lua-authored data with no requirement to match the
    /// directory, and matching by path is what stays correct in both the
    /// "just discovered, not loaded yet" and "dir deleted" cases.
    pub fn reload(&mut self, plugin_dir: &std::path::Path, cx: &mut gpui::App) {
        let manifest_path = plugin_dir.join("manifest.toml");
        let init_path = plugin_dir.join("init.luau");

        // Case 1: dir deleted or files missing → unregister from registry
        if !plugin_dir.exists() || !manifest_path.exists() || !init_path.exists() {
            self.unregister_plugin(plugin_dir, cx);
            return;
        }

        // Case 2: try to create new VM (don't touch old one yet)
        match self.load_plugin(plugin_dir, &manifest_path, &init_path) {
            Ok(new_handle) => {
                let name = new_handle.name.clone();
                // Drop old VM for this path, if any
                if let Some(old_idx) = self.plugins.iter().position(|p| p.path == plugin_dir) {
                    let old = self.plugins.remove(old_idx);
                    drop(old);
                }
                self.plugins.push(new_handle);
                // Re-register widgets in BarWidgetRegistry
                self.reregister_widgets(plugin_dir, cx);
                tracing::info!("Hot-reloaded plugin: {name} ({plugin_dir:?})");
            }
            Err(e) => {
                tracing::error!("Hot-reload failed for {plugin_dir:?}: {e}, keeping old version");
            }
        }
    }

    /// Remove a plugin (identified by its directory) and unregister its
    /// widgets from BarWidgetRegistry.
    pub fn unregister_plugin(&mut self, plugin_dir: &std::path::Path, cx: &mut gpui::App) {
        if let Some(idx) = self.plugins.iter().position(|p| p.path == plugin_dir) {
            let handle = self.plugins.remove(idx);
            // Get widget names from Lua state
            let widget_names = self.get_widgets_for_plugin(&handle);
            drop(handle);
            // Unregister each widget from BarWidgetRegistry
            let registry = cx.global_mut::<crate::bar::BarWidgetRegistry>();
            for wname in &widget_names {
                registry.unregister_by_name(wname);
                tracing::info!("Unregistered widget: {wname}");
            }
        }
    }

    /// Get widget names registered by a specific plugin.
    fn get_widgets_for_plugin(&self, handle: &PluginHandle) -> Vec<String> {
        let mut names = Vec::new();
        let globals = handle.lua.globals();
        if let Ok(chronos) = globals.get::<mlua::Table>("chronos") {
            if let Ok(bar) = chronos.get::<mlua::Table>("bar") {
                if let Ok(widgets) = bar.get::<mlua::Table>("widgets") {
                    for pair in widgets.pairs::<String, mlua::Table>() {
                        if let Ok((name, _)) = pair {
                            names.push(name);
                        }
                    }
                }
            }
        }
        names
    }

    /// Re-register all widgets for a specific plugin (identified by its
    /// directory) via replace_by_name.
    pub fn reregister_widgets(&mut self, plugin_dir: &std::path::Path, cx: &mut gpui::App) {
        let handle = match self.plugins.iter().find(|p| p.path == plugin_dir) {
            Some(h) => h,
            None => {
                tracing::warn!("reregister_widgets: plugin at {plugin_dir:?} not found");
                return;
            }
        };

        let globals = handle.lua.globals();
        let chronos: mlua::Table = match globals.get("chronos") {
            Ok(c) => c,
            Err(_) => return,
        };
        let bar: mlua::Table = match chronos.get("bar") {
            Ok(b) => b,
            Err(_) => return,
        };
        let widgets: mlua::Table = match bar.get("widgets") {
            Ok(w) => w,
            Err(_) => return,
        };

        let registry = cx.global_mut::<crate::bar::BarWidgetRegistry>();

        for pair in widgets.pairs::<String, mlua::Table>() {
            if let Ok((wname, spec)) = pair {
                let section_str: String = spec.get("section").unwrap_or_else(|_| "left".into());
                let render_fn: mlua::Function = match spec.get("render") {
                    Ok(f) => f,
                    Err(_) => {
                        tracing::warn!("Widget {wname}: render function missing, skipping");
                        continue;
                    }
                };

                let section = match section_str.as_str() {
                    "center" => crate::bar::BarSection::Center,
                    "right" => crate::bar::BarSection::Right,
                    _ => crate::bar::BarSection::Left,
                };

                // Store render function in Lua globals with known name
                let fn_name = format!("__chronos_render_{wname}");
                handle.lua.globals().set(&*fn_name, render_fn).ok();

                // Create adapter and register via replace_by_name
                let adapter = crate::dsl::LuaWidgetAdapter::new(
                    wname.clone(),
                    section,
                    handle.lua.clone(),
                    fn_name,
                );
                registry.replace_by_name(&wname, Box::new(adapter));
            }
        }
    }

    /// Start a periodic tick loop using GPUI executor (not tokio).
    /// Matches the runtime-split decision: GPUI main thread owns UI futures.
    /// Requires `PluginManager` to be set as a GPUI global via `cx.set_global()`.
    pub fn start_tick_loop(cx: &mut gpui::App) {
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let _ = cx.update(|cx| {
                    cx.global::<PluginManager>().dispatch_tick();
                });
            }
        })
        .detach();
    }

    /// Start the inotify file watcher for hot-reload.
    /// Requires `PluginManager` to be set as a GPUI global via `cx.set_global()`.
    pub fn start_watcher(cx: &mut gpui::App) {
        let dirs = cx.global::<PluginManager>().plugin_dirs.clone();
        crate::watcher::start_watcher_loop(cx, dirs);
    }

    /// Get all registered bar widgets, each paired with the `Lua` handle of
    /// the plugin that registered it. Returns the handle directly — instead
    /// of just the widget's registered name — because the widget name
    /// (`chronos.bar:register({name=...})`) and the plugin's manifest name
    /// are different namespaces; a caller that tries to re-derive "which
    /// plugin owns this widget" from the widget name alone will get it wrong
    /// whenever they differ (see plugin_bridge.rs's regression test).
    pub fn get_registered_widgets(&self) -> Vec<(mlua::Lua, String, String, mlua::Table)> {
        let mut result = Vec::new();
        for handle in &self.plugins {
            let globals = handle.lua.globals();
            if let Ok(chronos) = globals.get::<mlua::Table>("chronos") {
                if let Ok(bar) = chronos.get::<mlua::Table>("bar") {
                    if let Ok(widgets) = bar.get::<mlua::Table>("widgets") {
                        for pair in widgets.pairs::<String, mlua::Table>() {
                            if let Ok((name, spec)) = pair {
                                let section = spec.get::<String>("section")
                                    .unwrap_or_else(|_| "left".into());
                                result.push((handle.lua.clone(), name, section, spec));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar::BarWidgetRegistry;
    use gpui::BorrowAppContext;
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

        let mut mgr = PluginManager::new(vec![dir.clone()]);
        mgr.load_all();
        assert_eq!(mgr.plugins().len(), 1);
        assert_eq!(mgr.plugins()[0].name, "test-clock");

        // Clean up
        fs::remove_dir_all(&dir).ok();
    }

    /// reload() must actually update BarWidgetRegistry via cx — not just
    /// internal PluginManager state. This is the regression guard for the
    /// "reload promises registry update but never calls replace_by_name" bug.
    #[gpui::test]
    fn reload_updates_registry(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join("chronos_test_reload_reg");
        let plugin_dir = dir.join("myplugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        fs::write(plugin_dir.join("manifest.toml"), r#"[plugin]
name = "myplugin"
version = "0.1.0"
unsafe = true"#).unwrap();
        fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "test-widget",
    section = "left",
    render = function()
        return { type = "text", content = "v1" }
    end
})"#).unwrap();

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());

            let mut mgr = PluginManager::new(vec![dir.clone()]);
            mgr.load_all();
            assert_eq!(mgr.plugins.len(), 1);

            // Register initial widget
            mgr.reregister_widgets(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                1
            );

            // Modify init.luau to v2
            fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "test-widget",
    section = "left",
    render = function()
        return { type = "text", content = "v2" }
    end
})"#).unwrap();

            // Reload — must update the registry, not just mgr.plugins
            mgr.reload(&plugin_dir, cx);

            // Verify registry still has exactly 1 widget (replaced, not duplicated)
            let registry = cx.global::<BarWidgetRegistry>();
            assert_eq!(
                registry.widgets_for(crate::bar::BarSection::Left).count(),
                1,
                "reload must replace_by_name, not push a duplicate"
            );
        });

        fs::remove_dir_all(&dir).ok();
    }

    /// unregister_plugin() must remove widgets from BarWidgetRegistry.
    #[gpui::test]
    fn unregister_removes_from_registry(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join("chronos_test_unregister_reg");
        let plugin_dir = dir.join("myplugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("manifest.toml"), r#"[plugin]
name = "myplugin"
version = "0.1.0"
unsafe = true"#).unwrap();
        fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "test-widget",
    section = "left",
    render = function()
        return { type = "text", content = "v1" }
    end
})"#).unwrap();

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());

            let mut mgr = PluginManager::new(vec![dir.clone()]);
            mgr.load_all();
            assert_eq!(mgr.plugins.len(), 1);

            mgr.reregister_widgets(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                1
            );

            // Unregister
            mgr.unregister_plugin(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                0,
                "unregister must call unregister_by_name on the registry"
            );
        });

        fs::remove_dir_all(&dir).ok();
    }

    /// reload() on a missing plugin dir must unregister widgets, not panic.
    #[gpui::test]
    fn reload_missing_dir_unregisters(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join("chronos_test_reload_missing");
        let plugin_dir = dir.join("myplugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("manifest.toml"), r#"[plugin]
name = "myplugin"
version = "0.1.0"
unsafe = true"#).unwrap();
        fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "test-widget",
    section = "left",
    render = function()
        return { type = "text", content = "v1" }
    end
})"#).unwrap();

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());

            let mut mgr = PluginManager::new(vec![dir.clone()]);
            mgr.load_all();
            mgr.reregister_widgets(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                1
            );

            // Delete the plugin dir
            fs::remove_dir_all(&plugin_dir).ok();

            // Reload should detect missing dir and unregister
            mgr.reload(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                0
            );
        });

        fs::remove_dir_all(&dir).ok();
    }

    /// reload() MUST work when invoked through the exact watcher path:
    /// `cx.update_global::<PluginManager, _>(|mgr, cx| mgr.reload(dir, cx))`.
    /// This is the runtime guard for the lease boundary — `update_global` leases
    /// `PluginManager` out of the globals map (lease_global does
    /// `globals_by_type.remove(TypeId::of::<PluginManager>())`), so calling
    /// `global_mut::<BarWidgetRegistry>()` (a *different* TypeId key, still in
    /// the map) inside the closure must not panic with a double borrow.
    ///
    /// Without this test the "no double-borrow" property is only source-read
    /// (lease_global/end_global_lease in gpui-ce), not execution-verified. The
    /// other reload tests call `mgr.reload` directly on a plain `cx` and never
    /// exercise the nested lease.
    #[gpui::test]
    fn reload_through_update_global_updates_registry(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join("chronos_test_reload_ug");
        let plugin_dir = dir.join("myplugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("manifest.toml"), r#"[plugin]
name = "myplugin"
version = "0.1.0"
unsafe = true"#).unwrap();
        fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "test-widget",
    section = "left",
    render = function()
        return { type = "text", content = "v1" }
    end
})"#).unwrap();

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());
            cx.set_global(PluginManager::new(vec![dir.clone()]));

            // Exact watcher call site:
            cx.update_global::<PluginManager, _>(|mgr, cx| {
                mgr.reload(&plugin_dir, cx);
            });

            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                1,
                "reload through update_global must reach BarWidgetRegistry under the lease"
            );
        });

        fs::remove_dir_all(&dir).ok();
    }

    /// reload() must register the widget even when the plugin's directory
    /// name differs from its manifest `[plugin] name` — this is not an edge
    /// case, the watcher always identifies plugins by directory (see
    /// watcher.rs), and nothing requires the two to match. Regression test
    /// for the live repro: dir `test-race-plugin`, manifest `name = "race"`.
    #[gpui::test]
    fn reload_registers_widget_when_dir_name_differs_from_manifest_name(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join("chronos_test_name_mismatch");
        let plugin_dir = dir.join("test-race-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("manifest.toml"), r#"[plugin]
name = "race"
unsafe = true"#).unwrap();
        fs::write(plugin_dir.join("init.luau"), r#"
chronos.bar:register({
    name = "race-widget",
    section = "left",
    render = function()
        return { type = "text", content = "race" }
    end
})"#).unwrap();

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());

            let mut mgr = PluginManager::new(vec![dir.clone()]);
            // reload() on a not-yet-loaded dir is exactly the watcher's CREATE path
            mgr.reload(&plugin_dir, cx);

            assert_eq!(mgr.plugins.len(), 1);
            assert_eq!(mgr.plugins[0].name, "race"); // manifest name, for display
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                1,
                "widget must register even though dir name (test-race-plugin) \
                 != manifest name (race)"
            );

            // Editing it again (existing-plugin reload path) must replace, not duplicate
            mgr.reload(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                1,
                "second reload must replace_by_name, not duplicate or drop the widget"
            );

            // Deleting it must unregister — same mismatched-name path
            fs::remove_dir_all(&plugin_dir).ok();
            mgr.reload(&plugin_dir, cx);
            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(crate::bar::BarSection::Left)
                    .count(),
                0,
                "reload on deleted mismatched-name dir must unregister"
            );
        });

        fs::remove_dir_all(&dir).ok();
    }
}

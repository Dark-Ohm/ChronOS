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

    /// Get all registered bar widget specs from all plugins.
    pub fn get_registered_widgets(&self) -> Vec<(String, String, mlua::Table)> {
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
                                result.push((name, section, spec));
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
        eprintln!("Loaded {} plugins, dirs: {:?}", mgr.plugins().len(), mgr.plugin_dirs);
        eprintln!("Dir exists: {}, clock exists: {}", dir.exists(), plugin_dir.exists());
        eprintln!("manifest exists: {}, init exists: {}",
            plugin_dir.join("manifest.toml").exists(),
            plugin_dir.join("init.luau").exists());
        assert_eq!(mgr.plugins().len(), 1);
        assert_eq!(mgr.plugins()[0].name, "test-clock");

        // Clean up
        fs::remove_dir_all(&dir).ok();
    }
}

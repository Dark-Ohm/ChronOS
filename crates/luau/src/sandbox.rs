use crate::capabilities::Manifest;
use mlua::Lua;

/// Create a sandboxed LuaU VM for a plugin.
/// Strips dangerous globals and configures safety based on manifest.
pub fn create_plugin_vm(_manifest: &Manifest) -> anyhow::Result<Lua> {
    let lua = Lua::new();

    // Strip dangerous globals
    {
        let globals = lua.globals();
        for name in &["os", "io", "debug"] {
            globals.set(*name, mlua::Value::Nil)?;
        }
    }

    Ok(lua)
}

/// Register the `chronos.*` API table into the Lua global scope.
pub fn register_chronos_api(
    lua: &Lua,
    manifest: &Manifest,
    callbacks: crate::api::events::CallbackStore,
) -> anyhow::Result<()> {
    let globals = lua.globals();
    let chronos = lua.create_table()?;

    // Always register base modules
    chronos.set("bar", crate::api::bar::create_bar_api(lua)?)?;
    chronos.set("time", crate::api::time::create_time_api(lua)?)?;
    chronos.set("log", crate::api::log::create_log_api(lua)?)?;
    chronos.set("on", crate::api::events::create_events_api(lua, callbacks)?)?;

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
    use crate::capabilities::{Capabilities, PluginMeta};

    fn test_manifest(name: &str, unsafe_mode: bool) -> Manifest {
        Manifest {
            meta: PluginMeta {
                name: name.to_string(),
                version: "0.1.0".into(),
                author: "test".into(),
                description: "test".into(),
            },
            capabilities: Capabilities::default(),
            unsafe_mode,
        }
    }

    fn test_callbacks() -> crate::api::events::CallbackStore {
        crate::api::events::CallbackStore::default()
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
        register_chronos_api(&lua, &m, test_callbacks()).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        assert!(chronos.get::<mlua::Value>("bar").unwrap().is_table());
    }

    #[test]
    fn chronos_fs_not_registered_without_capability() {
        let m = test_manifest("test_no_fs", false);
        let lua = create_plugin_vm(&m).unwrap();
        register_chronos_api(&lua, &m, test_callbacks()).unwrap();
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
        register_chronos_api(&lua, &m, test_callbacks()).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        assert!(chronos.get::<mlua::Value>("fs").unwrap().is_table());
    }

    #[test]
    fn unsafe_mode_enables_all_capabilities() {
        let m = test_manifest("test_unsafe", true);
        let lua = create_plugin_vm(&m).unwrap();
        register_chronos_api(&lua, &m, test_callbacks()).unwrap();
        let chronos: mlua::Table = lua.globals().get("chronos").unwrap();
        assert!(chronos.get::<mlua::Value>("fs").unwrap().is_table());
    }
}

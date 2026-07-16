use chronos_luau::PluginManager;
use chronos_luau::bar::{BarSection, BarWidgetRegistry};
use chronos_luau::dsl::LuaWidgetAdapter;

/// After loading plugins, register their bar widgets with the global registry.
/// Uses replace_by_name for hot-reload safety.
pub fn register_plugin_widgets(plugin_manager: &PluginManager, cx: &mut gpui::App) {
    for (lua, name, section_str, spec) in plugin_manager.get_registered_widgets() {
        let render_fn: chronos_luau::mlua::Function = match spec.get("render") {
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

        let fn_name = format!("__chronos_render_{name}");
        lua.globals().set(&*fn_name, render_fn).ok();

        let adapter = LuaWidgetAdapter::new(name.clone(), section, lua, fn_name);

        cx.global_mut::<BarWidgetRegistry>()
            .replace_by_name(&name, Box::new(adapter));
        tracing::info!("Registered LuaU widget: {name} in {section_str}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_luau::PluginManager;
    use std::fs;

    /// register_plugin_widgets must not assume the widget's registered name
    /// (`chronos.bar:register({name=...})`) equals the plugin's manifest
    /// `[plugin] name` — they are different namespaces. Regression test for
    /// the `unwrap() on None` panic found via live smoke testing.
    #[gpui::test]
    fn register_plugin_widgets_handles_name_mismatch(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join("chronos_test_bridge_mismatch");
        let plugin_dir = dir.join("test-race-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("manifest.toml"),
            r#"[plugin]
name = "race"
unsafe = true"#,
        )
        .unwrap();
        fs::write(
            plugin_dir.join("init.luau"),
            r#"
chronos.bar:register({
    name = "race-widget",
    section = "left",
    render = function()
        return { type = "text", content = "race" }
    end
})"#,
        )
        .unwrap();

        cx.update(|cx| {
            cx.set_global(BarWidgetRegistry::default());

            let mut mgr = PluginManager::new(vec![dir.clone()]);
            mgr.load_all();
            assert_eq!(mgr.plugins().len(), 1);

            register_plugin_widgets(&mgr, cx);

            assert_eq!(
                cx.global::<BarWidgetRegistry>()
                    .widgets_for(BarSection::Left)
                    .count(),
                1,
                "widget must register even though its name (race-widget) \
                 differs from the plugin's manifest name (race)"
            );
        });

        fs::remove_dir_all(&dir).ok();
    }
}

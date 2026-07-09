use chronos_luau::dsl;
use chronos_luau::dsl::LuaWidgetAdapter;
use chronos_luau::PluginManager;
use chronos_luau::bar::{BarSection, BarWidgetRegistry};
use gpui::{AnyElement, App, Window, ParentElement, IntoElement};

/// After loading plugins, register their bar widgets with the global registry.
/// Uses replace_by_name for hot-reload safety.
pub fn register_plugin_widgets(plugin_manager: &PluginManager, cx: &mut gpui::App) {
    for (name, section_str, spec) in plugin_manager.get_registered_widgets() {
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

        let lua = &plugin_manager
            .plugins()
            .iter()
            .find(|p| p.name == name)
            .unwrap()
            .lua;
        let fn_name = format!("__chronos_render_{name}");
        lua.globals().set(&*fn_name, render_fn).ok();

        let adapter = LuaWidgetAdapter::new(
            name.clone(),
            section,
            lua.clone(),
            fn_name,
        );

        cx.global_mut::<BarWidgetRegistry>()
            .replace_by_name(&name, Box::new(adapter));
        tracing::info!("Registered LuaU widget: {name} in {section_str}");
    }
}

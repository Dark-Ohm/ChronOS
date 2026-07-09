use chronos_luau::dsl;
use chronos_luau::PluginManager;
use crate::bar::sections::BarSection;
use crate::bar::widget::{BarWidget, BarWidgetRegistry};
use gpui::{AnyElement, App, Window, ParentElement, IntoElement};

/// A BarWidget backed by a LuaU render callback.
pub struct LuaWidgetAdapter {
    name: String,
    section: BarSection,
    lua: chronos_luau::mlua::Lua,
    render_fn_name: String,
}

impl BarWidget for LuaWidgetAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn section(&self) -> BarSection {
        self.section
    }

    fn render(&self, _window: &mut Window, _cx: &App) -> AnyElement {
        let result: Result<chronos_luau::mlua::Table, _> = self
            .lua
            .globals()
            .get::<chronos_luau::mlua::Function>(&*self.render_fn_name)
            .and_then(|f| f.call(()));

        match result {
            Ok(table) => match dsl::Element::from_lua_table(&table) {
                Ok(element) => element.into_any_element(),
                Err(e) => {
                    tracing::warn!("Plugin {} render error: {e}", self.name);
                    gpui::div()
                        .child(format!("[{}: render error]", self.name))
                        .into_any_element()
                }
            },
            Err(e) => {
                tracing::warn!("Plugin {} Lua call error: {e}", self.name);
                gpui::div()
                    .child(format!("[{}: call error]", self.name))
                    .into_any_element()
            }
        }
    }
}

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

        let adapter = LuaWidgetAdapter {
            name: name.clone(),
            section,
            lua: lua.clone(),
            render_fn_name: fn_name,
        };

        cx.global_mut::<BarWidgetRegistry>()
            .replace_by_name(&name, Box::new(adapter));
        tracing::info!("Registered LuaU widget: {name} in {section_str}");
    }
}

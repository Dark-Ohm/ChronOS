use mlua::Lua;

pub fn create_bar_api(lua: &Lua) -> mlua::Result<mlua::Table> {
    let bar = lua.create_table()?;
    let widgets = lua.create_table()?;

    bar.set("register", lua.create_function({
        let widgets = widgets.clone();
        move |_, (spec,): (mlua::Table,)| -> mlua::Result<()> {
            let name: String = spec.get("name")?;
            let section: String = spec.get("section").unwrap_or_else(|_| "left".into());
            let _render: mlua::Function = spec.get("render")?;
            widgets.set(name.clone(), spec)?;
            tracing::info!("Plugin registered bar widget: {name} (section: {section})");
            Ok(())
        }
    })?)?;

    bar.set("widgets", widgets)?;
    Ok(bar)
}

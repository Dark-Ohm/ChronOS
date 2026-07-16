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

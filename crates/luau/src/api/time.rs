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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type CallbackStore = Arc<Mutex<HashMap<String, Vec<mlua::Function>>>>;

pub fn create_events_api(lua: &mlua::Lua, _callbacks: CallbackStore) -> mlua::Result<mlua::Table> {
    lua.create_table()
}

pub fn dispatch_event(
    _lua: &mlua::Lua,
    _callbacks: &CallbackStore,
    _event: &str,
) -> mlua::Result<()> {
    Ok(())
}

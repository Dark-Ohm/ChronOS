use mlua::Lua;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type CallbackStore = Arc<Mutex<HashMap<String, Vec<mlua::Function>>>>;

pub fn create_events_api(lua: &Lua, callbacks: CallbackStore) -> mlua::Result<mlua::Table> {
    let events = lua.create_table()?;
    events.set("on", lua.create_function({
        let callbacks = callbacks.clone();
        move |_, (event, cb): (String, mlua::Function)| -> mlua::Result<()> {
            let mut store = callbacks.lock().unwrap();
            store.entry(event).or_default().push(cb);
            Ok(())
        }
    })?)?;
    Ok(events)
}

/// Dispatch an event to all registered callbacks.
pub fn dispatch_event(_lua: &Lua, callbacks: &CallbackStore, event: &str) -> mlua::Result<()> {
    let store = callbacks.lock().unwrap();
    if let Some(cbs) = store.get(event) {
        for cb in cbs {
            let _ = cb.call::<()>(());
        }
    }
    Ok(())
}

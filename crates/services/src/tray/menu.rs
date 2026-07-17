//! com.canonical.dbusmenu client (Fdo Menu spec).
//!
//! A `StatusNotifierItem` exposes a `Menu` object-path property; the object
//! behind that path implements `com.canonical.dbusmenu`. We call `GetLayout`
//! to fetch the tree and `Event` to activate a node.
//!
//! Live-verified against `udiskie --appindicator` via `busctl introspect`
//! (`com.canonical.dbusmenu.GetLayout iias` -> `u(ia{sv}av)`,
//! `com.canonical.dbusmenu.Event isvu`).

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tracing::info;
use zbus::Connection;
use zbus::zvariant::{OwnedValue, Value};

use crate::tray::types::{MenuNode, MenuToggleType, strip_mnemonic};

/// Typed D-Bus struct matching the `GetLayout` reply signature `(ia{sv}av)`.
/// Fields are positional to match the D-Bus struct layout.
#[derive(Deserialize, zbus::zvariant::Type, Debug)]
struct MenuLayoutRaw(i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// Fetch the full menu tree for the given destination + menu path. `dest` is
/// the item's unique-name or well-known bus name (same convention as the
/// `StatusNotifierItem` proxy).
pub async fn fetch_tree(
    conn: &Connection,
    dest: &str,
    path: &str,
) -> anyhow::Result<Vec<MenuNode>> {
    let names: Vec<&str> = vec![
        "label", "enabled", "visible", "type",
        "toggle-type", "toggle-state", "children-display",
    ];

    let msg = conn
        .call_method(
            Some(dest),
            path,
            Some("com.canonical.dbusmenu"),
            "GetLayout",
            &(&(0i32), &(-1i32), &names),
        )
        .await?;

    let body = msg.body();
    let (_, raw): (u32, MenuLayoutRaw) = body
        .deserialize()
        .map_err(|e| anyhow::anyhow!("deserialize GetLayout reply: {e}"))?;

    let props: Vec<(String, Value<'static>)> = raw.1.into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect();
    let children: Vec<Value<'static>> = raw.2.into_iter()
        .map(|v| v.into())
        .collect();

    let node = build_node(raw.0, props, children);
    Ok(vec![node])
}

fn flatten_children(values: Vec<Value<'static>>) -> Vec<MenuNode> {
    values
        .iter()
        .filter_map(|child_val| {
            match child_val.clone() {
                Value::Structure(s) => {
                    let fields = s.into_fields();
                    if fields.len() < 3 {
                        return None;
                    }
                    let id = match &fields[0] {
                        Value::I32(i) => *i,
                        _ => return None,
                    };
                    let props = match &fields[1] {
                        Value::Dict(d) => {
                            d.iter()
                                .filter_map(|(k, v)| {
                                    if let Value::Str(s) = k {
                                        Some((s.to_string(), v.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        }
                        _ => return None,
                    };
                    let children_values = match &fields[2] {
                        Value::Array(arr) => {
                            arr.iter()
                                .filter_map(|v| match v {
                                    Value::Value(boxed) => Some(boxed.as_ref().clone()),
                                    other => Some(other.clone()),
                                })
                                .collect()
                        }
                        _ => return None,
                    };
                    Some(build_node(id, props, children_values))
                }
                _ => None,
            }
        })
        .collect()
}

fn build_node(id: i32, properties: Vec<(String, Value<'static>)>, child_values: Vec<Value<'static>>) -> MenuNode {
    let props: HashMap<&str, &Value<'static>> = properties
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    let label = props
        .get("label")
        .and_then(|v| {
            if let Value::Str(s) = v {
                Some(strip_mnemonic(s))
            } else {
                None
            }
        })
        .unwrap_or_default();

    let enabled = props
        .get("enabled")
        .and_then(|v| {
            if let Value::Bool(b) = v {
                Some(*b)
            } else {
                None
            }
        })
        .unwrap_or(true);

    let visible = props
        .get("visible")
        .and_then(|v| {
            if let Value::Bool(b) = v {
                Some(*b)
            } else {
                None
            }
        })
        .unwrap_or(true);

    let separator = props
        .get("type")
        .and_then(|v| {
            if let Value::Str(s) = v {
                Some(s.as_str() == "separator")
            } else {
                None
            }
        })
        .unwrap_or(false);

    let toggle = if separator {
        None
    } else {
        match props
            .get("toggle-type")
            .and_then(|v| {
                if let Value::Str(s) = v {
                    Some(s.as_str())
                } else {
                    None
                }
            }) {
            None => None,
            Some(t) => {
                let kind = match t {
                    "radio" => MenuToggleType::Radio,
                    _ => MenuToggleType::Checkmark,
                };
                let checked = props
                    .get("toggle-state")
                    .and_then(|v| {
                        if let Value::Bool(b) = v {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false);
                Some((kind, checked))
            }
        }
    };

    let children = flatten_children(child_values);

    MenuNode {
        id,
        label,
        enabled,
        visible,
        separator,
        toggle,
        children,
    }
}

/// Send `Event(id, "clicked", data=empty, timestamp=now)` to a menu.
pub async fn send_clicked(
    conn: &Connection,
    dest: &str,
    path: &str,
    id: i32,
) -> zbus::Result<()> {
    let now: u32 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(0);

    let empty = Value::Str("".to_string().into());
    conn.call_method(
        Some(dest),
        path,
        Some("com.canonical.dbusmenu"),
        "Event",
        &(&id, &"clicked", &empty, &now),
    )
    .await?;
    info!("tray: dbusmenu Event(clicked) sent to {dest}{path} id={id}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_mnemonic_basic() {
        assert_eq!(strip_mnemonic("A_B"), "AB");
        assert_eq!(strip_mnemonic("A__B"), "A_B");
        assert_eq!(strip_mnemonic("_A_B"), "AB");
        assert_eq!(strip_mnemonic("__A"), "_A");
    }

    #[test]
    fn strip_mnemonic_plain() {
        assert_eq!(strip_mnemonic("Plain text"), "Plain text");
        assert_eq!(strip_mnemonic(""), "");
    }

    #[test]
    fn strip_mnemonic_edge() {
        assert_eq!(strip_mnemonic(""), "");
        assert_eq!(strip_mnemonic("_"), "");
        assert_eq!(strip_mnemonic("__"), "_");
        assert_eq!(strip_mnemonic("___"), "_");
    }
}
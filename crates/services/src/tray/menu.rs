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
        // T263: icons + shortcuts — the canon menu model carries both
        // (`.ci-ic` / `.ci-short`), and DBusMenu supports `icon-name` and
        // `shortcut` as standard properties.
        "icon-name", "shortcut",
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
    Ok(top_level_nodes(node))
}

/// `GetLayout(0, …)` returns a synthetic root node whose children are the
/// actual top-level menu items. Do not render that empty root as a submenu.
fn top_level_nodes(root: MenuNode) -> Vec<MenuNode> {
    root.children
}

fn flatten_children(values: Vec<Value<'static>>) -> Vec<MenuNode> {
    values
        .into_iter()
        .filter_map(|child_val| match child_val {
            Value::Structure(s) => {
                let mut fields = s.into_fields();
                if fields.len() < 3 {
                    return None;
                }
                let id = match fields.remove(0) {
                    Value::I32(i) => i,
                    _ => return None,
                };
                let props = match fields.remove(0) {
                    Value::Dict(d) => d
                        .iter()
                        .filter_map(|(k, v)| {
                            if let Value::Str(s) = k {
                                Some((s.to_string(), v.clone()))
                            } else {
                                None
                            }
                        })
                        .collect(),
                    _ => return None,
                };
                let children_values: Vec<Value<'static>> = match fields.remove(0) {
                    Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| match v {
                            Value::Value(boxed) => Some(boxed.as_ref().clone()),
                            other => Some(other.clone()),
                        })
                        .collect(),
                    _ => return None,
                };
                Some(build_node(id, props, children_values))
            }
            _ => None,
        })
        .collect()
}

/// Unwrap nested `Value::Value(boxed)` variant wrappers.
/// D-Bus `av` arrays wrap children in variants, and an `a{sv}` property may
/// add another variant around the array value itself.
fn unwrap_variant(mut value: Value<'static>) -> Value<'static> {
    while let Value::Value(boxed) = value {
        value = *boxed;
    }
    value
}

fn build_node(id: i32, properties: Vec<(String, Value<'static>)>, child_values: Vec<Value<'static>>) -> MenuNode {
    let props: HashMap<&str, &Value<'static>> = properties
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    let label = props
        .get("label")
        .and_then(|v| {
            if let Value::Str(s) = unwrap_variant((*v).clone()) {
                Some(strip_mnemonic(&*s))
            } else {
                None
            }
        })
        .unwrap_or_default();

    let enabled = props
        .get("enabled")
        .and_then(|v| {
            if let Value::Bool(b) = unwrap_variant((*v).clone()) {
                Some(b)
            } else {
                None
            }
        })
        .unwrap_or(true);

    let visible = props
        .get("visible")
        .and_then(|v| {
            if let Value::Bool(b) = unwrap_variant((*v).clone()) {
                Some(b)
            } else {
                None
            }
        })
        .unwrap_or(true);

    let separator = props
        .get("type")
        .and_then(|v| {
            if let Value::Str(s) = unwrap_variant((*v).clone()) {
                Some(s.as_str() == "separator")
            } else {
                None
            }
        })
        .unwrap_or(false);

    // `icon-name` — freedesktop icon-theme name for the row's leading glyph.
    // Rendered in the 16px `.ci-ic` gutter; resolved at render time by the
    // shared `icon_resolution` (not stored as a path in the service).
    let icon_name = if separator {
        None
    } else {
        props
            .get("icon-name")
            .and_then(|v| {
                if let Value::Str(s) = unwrap_variant((*v).clone()) {
                    Some(s.to_string())
                } else {
                    None
                }
            })
            .filter(|s| !s.is_empty())
    };

    // `shortcut` — DBusMenu format `av` of `as`: e.g. `[["Control","X"]]`
    // or `[["F2"]]`. Stored raw (array of key arrays); the view converts to
    // display glyphs (`⌃X`) at render time, never in the service.
    let shortcut = if separator {
        None
    } else {
        props.get("shortcut").and_then(|v| {
            let v = unwrap_variant((*v).clone());
            match v {
                Value::Array(combos) => {
                    let combos: Vec<Vec<String>> = combos
                        .iter()
                        .filter_map(|combo| {
                            let combo = unwrap_variant(combo.clone());
                            match combo {
                                Value::Array(keys) => {
                                    let keys: Vec<String> = keys
                                        .iter()
                                        .filter_map(|k| {
                                            if let Value::Str(s) = unwrap_variant(k.clone()) {
                                                Some(s.to_string())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    (!keys.is_empty()).then_some(keys)
                                }
                                _ => None,
                            }
                        })
                        .collect();
                    (!combos.is_empty()).then_some(combos)
                }
                _ => None,
            }
        })
    };

    let toggle = if separator {
        None
    } else {
        let toggle_type = props
            .get("toggle-type")
            .and_then(|v| {
                if let Value::Str(s) = unwrap_variant((*v).clone()) {
                    Some(s.to_string())
                } else {
                    None
                }
            });

        match toggle_type {
            None => None,
            Some(t) => {
                let kind = match t.as_str() {
                    "radio" => MenuToggleType::Radio,
                    _ => MenuToggleType::Checkmark,
                };
                let checked = props
                    .get("toggle-state")
                    .and_then(|v| {
                        match unwrap_variant((*v).clone()) {
                            Value::I32(state) => Some(state > 0),
                            // Be permissive with non-conforming exporters.
                            Value::Bool(state) => Some(state),
                            _ => None,
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
        icon_name,
        shortcut,
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
    use zbus::zvariant::{Array, Str, StructureBuilder};

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

    /// Test that variant-wrapped dict values (`Value::Value(Box::new(...))`)
    /// are correctly unwrapped by `unwrap_variant` in `build_node`.
    /// This simulates real D-Bus `a{sv}` where dict values are variants.
    #[test]
    fn parse_variant_wrapped_dict_values() {
        // Build a MenuNode directly with variant-wrapped property values.
        // This is what real D-Bus data looks like: dict values are variants (v).
        let props = vec![
            ("label".into(), Value::Value(Box::new(Value::Str("Browse /dev/sdb1".into())))),
            ("enabled".into(), Value::Value(Box::new(Value::Bool(true)))),
            ("visible".into(), Value::Value(Box::new(Value::Bool(true)))),
        ];
        let node = build_node(1, props, vec![]);

        assert_eq!(node.id, 1);
        assert_eq!(node.label, "Browse /dev/sdb1");
        assert!(node.enabled);
        assert!(node.visible);
    }

    /// T263: `icon-name` and `shortcut` properties are parsed into the node.
    #[test]
    fn parse_icon_name_and_shortcut() {
        let props = vec![
            ("label".into(), Value::Value(Box::new(Value::Str("Copy".into())))),
            ("icon-name".into(), Value::Value(Box::new(Value::Str("edit-copy".into())))),
            ("shortcut".into(), Value::Value(Box::new(Value::Array(Array::from(vec![
                Value::Value(Box::new(Value::Array(Array::from(vec![
                    Value::Str(Str::from("Control")),
                    Value::Str(Str::from("C")),
                ])))),
            ]))))),
        ];
        let node = build_node(7, props, vec![]);
        assert_eq!(node.icon_name.as_deref(), Some("edit-copy"));
        assert_eq!(
            node.shortcut,
            Some(vec![vec!["Control".to_string(), "C".to_string()]])
        );
    }

    #[test]
    fn root_layout_exposes_top_level_children() {
        let child = build_node(
            1,
            vec![("label".into(), Value::Str("Open".into()))],
            vec![],
        );
        let root = MenuNode {
            id: 0,
            label: String::new(),
            enabled: true,
            visible: true,
            separator: false,
            toggle: None,
            icon_name: None,
            shortcut: None,
            children: vec![child],
        };

        let nodes = top_level_nodes(root);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label, "Open");
    }

    #[test]
    fn integer_toggle_state_one_is_checked() {
        let node = build_node(
            2,
            vec![
                ("toggle-type".into(), Value::Str("radio".into())),
                ("toggle-state".into(), Value::I32(1)),
            ],
            vec![],
        );
        assert_eq!(node.toggle, Some((MenuToggleType::Radio, true)));
    }

    /// T263: shortcut with no modifiers (plain `F2`) still parses.
    #[test]
    fn parse_shortcut_plain_key() {
        let props = vec![
            ("label".into(), Value::Value(Box::new(Value::Str("Rename".into())))),
            ("shortcut".into(), Value::Value(Box::new(Value::Array(Array::from(vec![
                Value::Value(Box::new(Value::Array(Array::from(vec![Value::Str(
                    Str::from("F2"),
                )])))),
            ]))))),
        ];
        let node = build_node(8, props, vec![]);
        assert_eq!(node.shortcut, Some(vec![vec!["F2".to_string()]]));
        assert!(node.icon_name.is_none());
    }

    /// T263: missing/empty `icon-name` stays `None`; separator keeps both `None`.
    #[test]
    fn parse_icon_name_missing_and_separator() {
        let no_icon = build_node(9, vec![("label".into(), Value::Value(Box::new(Value::Str("x".into()))))], vec![]);
        assert!(no_icon.icon_name.is_none());
        assert!(no_icon.shortcut.is_none());

        let sep = build_node(
            10,
            vec![("type".into(), Value::Value(Box::new(Value::Str("separator".into()))))],
            vec![],
        );
        assert!(sep.separator);
        assert!(sep.icon_name.is_none());
        assert!(sep.shortcut.is_none());
    }

    #[test]
    fn parse_recursive_variant_wrapped() {
        let mut gc_props: HashMap<String, OwnedValue> = HashMap::new();
        gc_props.insert("label".into(), Str::from("Browse /dev/sdb1").into());
        gc_props.insert("enabled".into(), true.into());
        gc_props.insert("visible".into(), true.into());

        let gc_val = Value::Structure(
            StructureBuilder::new()
                .append_field(Value::I32(3))
                .append_field(Value::Dict(gc_props.into()))
                .append_field(Value::Array(Array::from(vec![] as Vec<Value<'static>>)))
                .build()
                .unwrap(),
        );

        let mut child_props: HashMap<String, OwnedValue> = HashMap::new();
        child_props.insert("label".into(), Str::from("Managed devices").into());
        child_props.insert("enabled".into(), true.into());
        child_props.insert("visible".into(), true.into());

        let child_val = Value::Structure(
            StructureBuilder::new()
                .append_field(Value::I32(2))
                .append_field(Value::Dict(child_props.into()))
                .append_field(Value::Array(Array::from(vec![gc_val])))
                .build()
                .unwrap(),
        );

        let children = flatten_children(vec![child_val]);
        assert_eq!(children.len(), 1);

        let child = &children[0];
        assert_eq!(child.id, 2);
        assert_eq!(child.label, "Managed devices");
        assert_eq!(child.children.len(), 1);

        let gc = &child.children[0];
        assert_eq!(gc.id, 3);
        assert_eq!(gc.label, "Browse /dev/sdb1");
        assert!(gc.enabled);
        assert!(gc.visible);
    }
}

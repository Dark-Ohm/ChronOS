use gpui::{AnyElement, px, rgb, div, prelude::*};

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub color: Option<u32>,      // 0xRRGGBB
    pub size: Option<f32>,       // logical pixels
}

#[derive(Debug, Clone, PartialEq)]
pub enum Alignment { Start, Center, End }

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Text {
        content: String,
        style: TextStyle,
    },
    Row {
        children: Vec<Element>,
        gap: f32,
        alignment: Alignment,
    },
    Column {
        children: Vec<Element>,
        gap: f32,
        alignment: Alignment,
    },
}

impl Element {
    /// Recursively convert a Lua table (from a render callback) to an Element.
    pub fn from_lua_table(table: &mlua::Table) -> mlua::Result<Self> {
        let kind: String = table.get("type")?;
        match kind.as_str() {
            "text" => {
                let content: String = table.get("content")?;
                let style_val: mlua::Value = table.get("style").unwrap_or(mlua::Value::Nil);
                let style = parse_style(style_val)?;
                Ok(Element::Text { content, style })
            }
            "row" => {
                let gap: f32 = table.get("gap").unwrap_or(8.0);
                let alignment_val: mlua::Value = table.get("alignment").unwrap_or(mlua::Value::Nil);
                let alignment = parse_alignment(alignment_val)?;
                let children = parse_children(table, "children")?;
                Ok(Element::Row { children, gap, alignment })
            }
            "column" => {
                let gap: f32 = table.get("gap").unwrap_or(8.0);
                let alignment_val: mlua::Value = table.get("alignment").unwrap_or(mlua::Value::Nil);
                let alignment = parse_alignment(alignment_val)?;
                let children = parse_children(table, "children")?;
                Ok(Element::Column { children, gap, alignment })
            }
            other => Err(mlua::Error::RuntimeError(format!(
                "unknown element type: {other}"
            ))),
        }
    }

    /// Convert an Element tree into a GPUI AnyElement.
    pub fn into_any_element(self) -> AnyElement {
        match self {
            Element::Text { content, style } => {
                let mut el = div().child(content);
                if let Some(size) = style.size {
                    el = el.text_size(px(size));
                }
                if let Some(color) = style.color {
                    el = el.text_color(rgb(color));
                }
                el.into_any_element()
            }
            Element::Row { children, gap, alignment } => {
                let mut el = div().flex().items_center().gap(px(gap));
                el = match alignment {
                    Alignment::Start => el.justify_start(),
                    Alignment::Center => el.justify_center(),
                    Alignment::End => el.justify_end(),
                };
                el.children(children.into_iter().map(|c| c.into_any_element()))
                    .into_any_element()
            }
            Element::Column { children, gap, alignment } => {
                let mut el = div().flex_col().gap(px(gap));
                el = match alignment {
                    Alignment::Start => el.items_start(),
                    Alignment::Center => el.items_center(),
                    Alignment::End => el.items_end(),
                };
                el.children(children.into_iter().map(|c| c.into_any_element()))
                    .into_any_element()
            }
        }
    }
}

fn parse_style(val: mlua::Value) -> mlua::Result<TextStyle> {
    match val {
        mlua::Value::Table(table) => {
            let color: Option<String> = table.get("color")?;
            let color = color.and_then(|c| {
                let hex = c.trim_start_matches('#');
                u32::from_str_radix(hex, 16).ok()
            });
            let size: Option<f32> = table.get("size")?;
            Ok(TextStyle { color, size })
        }
        _ => Ok(TextStyle { color: None, size: None }),
    }
}

fn parse_alignment(val: mlua::Value) -> mlua::Result<Alignment> {
    match val {
        mlua::Value::String(s) => {
            let s = s.to_str()?;
            match &*s {
                "start" => Ok(Alignment::Start),
                "center" => Ok(Alignment::Center),
                "end" => Ok(Alignment::End),
                _ => Ok(Alignment::Start),
            }
        }
        _ => Ok(Alignment::Start),
    }
}

fn parse_children(table: &mlua::Table, key: &str) -> mlua::Result<Vec<Element>> {
    let children_table: mlua::Table = table.get(key)?;
    let mut children = Vec::new();
    for pair in children_table.pairs::<mlua::Value, mlua::Table>() {
        let (_, child_table) = pair?;
        children.push(Element::from_lua_table(&child_table)?);
    }
    Ok(children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn parse_text_element() {
        let lua = Lua::new();
        let table: mlua::Table = lua.load(
            r##"{ type = "text", content = "hello", style = { color = '#ff0000', size = 16 } }"##
        ).eval().unwrap();
        let el = Element::from_lua_table(&table).unwrap();
        match el {
            Element::Text { content, style } => {
                assert_eq!(content, "hello");
                assert_eq!(style.color, Some(0xff0000));
                assert_eq!(style.size, Some(16.0));
            }
            _ => panic!("expected Text element"),
        }
    }

    #[test]
    fn parse_row_with_children() {
        let lua = Lua::new();
        let table: mlua::Table = lua.load(
            r#"{ type = "row", gap = 12, alignment = "center", children = {
                { type = "text", content = "a" },
                { type = "text", content = "b" },
            }}"#
        ).eval().unwrap();
        let el = Element::from_lua_table(&table).unwrap();
        match el {
            Element::Row { children, gap, alignment } => {
                assert_eq!(children.len(), 2);
                assert_eq!(gap, 12.0);
                assert_eq!(alignment, Alignment::Center);
            }
            _ => panic!("expected Row element"),
        }
    }

    #[test]
    fn parse_unknown_type_returns_error() {
        let lua = Lua::new();
        let table: mlua::Table = lua.load(
            r#"{ type = "unknown_thing" }"#
        ).eval().unwrap();
        assert!(Element::from_lua_table(&table).is_err());
    }

    #[test]
    fn default_style_fields_are_none() {
        let lua = Lua::new();
        let table: mlua::Table = lua.load(
            r#"{ type = "text", content = "plain" }"#
        ).eval().unwrap();
        let el = Element::from_lua_table(&table).unwrap();
        match el {
            Element::Text { style, .. } => {
                assert_eq!(style.color, None);
                assert_eq!(style.size, None);
            }
            _ => panic!("expected Text"),
        }
    }
}

/// A BarWidget backed by a LuaU render callback.
/// Used by both `plugin_bridge.rs` (crates/app) and `reregister_widgets` (crates/luau)
/// so the watcher can rebuild widgets without depending on crates/app.
pub struct LuaWidgetAdapter {
    name: String,
    section: crate::bar::BarSection,
    lua: mlua::Lua,
    render_fn_name: String,
}

impl LuaWidgetAdapter {
    pub fn new(
        name: String,
        section: crate::bar::BarSection,
        lua: mlua::Lua,
        render_fn_name: String,
    ) -> Self {
        Self { name, section, lua, render_fn_name }
    }
}

impl crate::bar::BarWidget for LuaWidgetAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn section(&self) -> crate::bar::BarSection {
        self.section
    }

    fn render(&self, _window: &mut gpui::Window, _cx: &gpui::App) -> gpui::AnyElement {
        let result: Result<mlua::Table, _> = self
            .lua
            .globals()
            .get::<mlua::Function>(&*self.render_fn_name)
            .and_then(|f| f.call(()));

        match result {
            Ok(table) => match Element::from_lua_table(&table) {
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

//! Hyprland binds tab — read-only list of keybinds from the modular Lua config.
//!
//! Source: every `*.lua` under `~/.config/hypr/modules/` (Hyprland 0.55+ Lua).
//! Read-only: no writes, no hyprctl (§13). Each module contributes a **group**
//! named by an optional metadata comment — `-- # group = "Apps"` — falling back
//! to `"Custom"` (never the raw filename; PRODUCT.md §1: binds are an onboarding
//! surface, not a verbatim config dump). Fallback: when `modules/` is missing,
//! try `~/.config/hypr/hyprland.lua` (monolith, noted in UI); if still empty →
//! honest empty state, never a panic.
//!
//! Parser is a targeted line scan for `hl.bind(...)` — not a full Lua AST.
//! The `mainMod` variable is tracked so `mainMod .. " + L"` renders as `SUPER + L`.

use std::path::{Path, PathBuf};

use chronos_ui::Theme;
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
};

use crate::side_panel_right::preview_target::PreviewTarget;

/// One parsed bind row. `source`/`path` identify the file it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindRow {
    /// Resolved modifier+key, e.g. `SUPER + L` or `XF86AudioLowerVolume`.
    pub keys: String,
    /// Action expression (shortened / possibly truncated on multi-line binds).
    pub action: String,
    /// 1-based line in the source file.
    pub line: usize,
    /// Human-readable group label from the module's metadata comment
    /// (`-- # group = "..."`), or `"Custom"` when the module has none.
    /// Never the raw module filename.
    pub source: String,
    /// Absolute path of the source file (for click-to-open).
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadState {
    Ready,
    /// Honest error message (§13) — never a panic.
    Error(String),
}

pub struct HyprBindsTab {
    binds: Vec<BindRow>,
    load: LoadState,
    /// Set when the monolith `hyprland.lua` was used instead of `modules/`.
    using_monolith: bool,
    scroll: ScrollHandle,
}

impl HyprBindsTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            binds: Vec::new(),
            load: LoadState::Ready,
            using_monolith: false,
            scroll: ScrollHandle::new(),
        };
        this.reload(cx);
        this
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let (binds, load, using_monolith) = load_all();
        self.binds = binds;
        self.load = load;
        self.using_monolith = using_monolith;
        cx.notify();
    }

    /// Open the bind's source file in the Preview target (same channel Files
    /// uses — path-only, the user opens Preview themselves; no auto-switch).
    fn open_source(&self, path: PathBuf, cx: &mut Context<Self>) {
        let generation = {
            let t = cx.global::<PreviewTarget>();
            t.generation.wrapping_add(1)
        };
        cx.set_global(PreviewTarget {
            path: Some(path.clone()),
            generation,
            intent: crate::side_panel_right::preview_target::PreviewIntent::View,
        });
        tracing::debug!("hypr_binds: open {} in Preview", path.display());
    }
}

impl Render for HyprBindsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let count = self.binds.len();

        let header = div()
            .px(px(12.))
            .py(px(10.))
            .border_b_1()
            .border_color(theme.border.subtle)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text.primary)
                    .child(format!("Hyprland binds · {count}")),
            )
            .child(
                div()
                    .id("hypr-binds-reload")
                    .px(px(8.))
                    .py(px(4.))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.interactive.hover))
                    .on_click(cx.listener(|this, _ev, _w, cx| this.reload(cx)))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text.muted)
                            .child("Reload"),
                    ),
            );

        let mut list = div()
            .id("hypr-binds-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .px(px(6.))
            .py(px(4.));

        match &self.load {
            LoadState::Error(msg) => {
                list = list.child(
                    div()
                        .px(px(10.))
                        .py(px(16.))
                        .text_size(px(12.))
                        .text_color(theme.status.error)
                        .child(msg.clone()),
                );
            }
            LoadState::Ready => {
                if self.using_monolith {
                    list = list.child(
                        div()
                            .px(px(10.))
                            .py(px(6.))
                            .rounded_md()
                            .bg(theme.bg.elevated)
                            .text_size(px(11.))
                            .text_color(theme.text.muted)
                            .child("Loaded hyprland.lua (modules/ missing)"),
                    );
                }
                if self.binds.is_empty() {
                    list = list.child(
                        div()
                            .px(px(10.))
                            .py(px(16.))
                            .text_size(px(12.))
                            .text_color(theme.text.muted)
                            .child("No binds found — see ~/.config/hypr/modules/ (PRODUCT.md)."),
                    );
                } else {
                    let mut seen: Vec<&str> = Vec::new();
                    for row in &self.binds {
                        if !seen.contains(&row.source.as_str()) {
                            seen.push(&row.source.as_str());
                            list = list.child(section_header(&row.source, &theme));
                        }
                        list = list.child(bind_row(row, &theme, cx));
                    }
                }
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(list)
    }
}

// ---------------------------------------------------------------------------
// Loading + pure parser — the parser is unit-testable without cx/AppState.
// ---------------------------------------------------------------------------

fn load_all() -> (Vec<BindRow>, LoadState, bool) {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
    let modules = config_dir.join("hypr/modules");
    let mut binds = Vec::new();
    let mut using_monolith = false;

    if modules.is_dir() {
        // Read every *.lua module — groups come from each module's metadata
        // comment (`-- # group = "..."`), falling back to "Custom". Sorting by
        // name keeps module order stable across reloads. Non-bind helpers
        // (monitors, autostart, windowrules…) naturally yield zero rows.
        let mut files: Vec<PathBuf> = std::fs::read_dir(&modules)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| p.extension().map(|x| x == "lua").unwrap_or(false))
            .collect();
        files.sort();

        for p in files {
            match std::fs::read_to_string(&p) {
                Ok(src) => {
                    let group = parse_group(&src);
                    let parsed = parse_binds(&src, &group, &p);
                    if !parsed.is_empty() {
                        binds.extend(parsed);
                    }
                }
                Err(e) => tracing::warn!("hypr_binds: cannot read {}: {e}", p.display()),
            }
        }
    } else {
        // modules/ is gone — fall back to a monolith hyprland.lua (warn in UI).
        let mono = config_dir.join("hypr/hyprland.lua");
        if mono.is_file() {
            if let Ok(src) = std::fs::read_to_string(&mono) {
                binds = parse_binds(&src, "Custom", &mono);
                using_monolith = !binds.is_empty();
            }
        }
    }

    let load = if binds.is_empty() {
        LoadState::Error(
            "No Hyprland binds found — check ~/.config/hypr/modules/ (see PRODUCT.md).".into(),
        )
    } else {
        LoadState::Ready
    };
    (binds, load, using_monolith)
}

/// Resolve a module's human-readable group from its metadata comment.
///
/// Looks for a `--` comment line containing `# group = "Label"` (the `#` is
/// illustrative — any `--` comment with `group = "..."` matches). Returns the
/// quoted label, or `"Custom"` when the module has no metadata.
fn parse_group(source: &str) -> String {
    for line in source.lines() {
        let t = line.trim_start();
        if !t.starts_with("--") {
            continue; // only metadata in comments, never in code
        }
        if let Some(eq) = t.find("group =") {
            if let Some(g) = quoted(&t[eq + "group =".len()..]) {
                return g;
            }
        }
    }
    "Custom".to_string()
}

/// Parse every `hl.bind(...)` in a Lua source, tracking the `mainMod` value.
fn parse_binds(source: &str, label: &str, path: &Path) -> Vec<BindRow> {
    let mut main_mod: Option<String> = None;
    let mut out = Vec::new();
    for (ix, line) in source.lines().enumerate() {
        let lineno = ix + 1;
        if line.trim_start().starts_with("--") {
            continue;
        }
        if let Some(m) = parse_main_mod(line) {
            main_mod = Some(m);
        }
        if let Some(pos) = line.find("hl.bind(") {
            if let Some((keys, action)) = parse_bind_line(&line[pos..], main_mod.as_deref()) {
                out.push(BindRow {
                    keys,
                    action,
                    line: lineno,
                    source: label.to_string(),
                    path: path.to_path_buf(),
                });
            }
        }
    }
    out
}

/// Extract the `mainMod` value from `mainMod = "SUPER"` or
/// `local mainMod = mainMod or "SUPER"`. None if the line doesn't define it.
fn parse_main_mod(line: &str) -> Option<String> {
    let t = line.trim();
    if !(t.starts_with("mainMod") || t.starts_with("local mainMod")) {
        return None;
    }
    let after = t.find('=')?;
    let rhs = t[after + 1..].trim();
    // `mainMod or "SUPER"` → `"SUPER"`; plain `"SUPER"` stays.
    let rhs = rhs
        .strip_prefix("mainMod")
        .map(str::trim)
        .filter(|s| s.starts_with("or"))
        .and_then(|s| s.strip_prefix("or").map(str::trim))
        .unwrap_or(rhs);
    quoted(rhs)
}

/// Read a double-quoted Lua string literal at the start of `s` (no escapes).
fn quoted(s: &str) -> Option<String> {
    let s = s.trim();
    let inner = s.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

/// Resolve the key argument to a display string: `"literal"`, bare `mainMod`,
/// or `mainMod .. "suffix"` (which becomes `value + suffix`).
fn resolve_keys(key_text: &str, main_mod: Option<&str>) -> Option<String> {
    let t = key_text.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(lit) = quoted(t) {
        return Some(lit);
    }
    let ident_len = t
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '_')
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    if ident_len == 0 {
        return None;
    }
    let ident = &t[..ident_len];
    let base = if ident == "mainMod" {
        main_mod.unwrap_or("SUPER").to_string()
    } else {
        ident.to_string()
    };
    let rest = t[ident_len..].trim();
    if let Some(suffix) = rest.strip_prefix("..") {
        if let Some(s) = quoted(suffix.trim()) {
            return Some(format!("{base}{s}"));
        }
    }
    Some(base)
}

/// Parse one `hl.bind(` slice. Returns `(keys, action)` or None if unparseable.
/// The action runs to the rightmost `)` on the line (the bind's close paren)
/// and any trailing `--` comment is stripped. Multi-line binds truncate so the
/// action only shows the first line — acceptable for the shortened column.
fn parse_bind_line(text: &str, main_mod: Option<&str>) -> Option<(String, String)> {
    let body = text.strip_prefix("hl.bind(")?.trim_start();
    let comma = body.find(',')?;
    let key_text = body[..comma].trim();
    let action_rest = &body[comma + 1..];

    let keys = resolve_keys(key_text, main_mod)?;

    let end = action_rest.rfind(')').unwrap_or(action_rest.len());
    let mut action = action_rest[..end].trim().to_string();
    if let Some(ci) = action.find("--") {
        action = action[..ci].trim().to_string();
    }
    if action.is_empty() {
        return None;
    }
    Some((keys, action))
}

fn section_header(label: &str, theme: &Theme) -> AnyElement {
    div()
        .px(px(8.))
        .pt(px(10.))
        .pb(px(4.))
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text.muted)
        .child(label.to_string())
        .into_any_element()
}

/// One binds row: keys · action · :line. Click opens the source in Preview.
fn bind_row(row: &BindRow, theme: &Theme, cx: &mut Context<HyprBindsTab>) -> AnyElement {
    let path = row.path.clone();
    let line = row.line;
    div()
        .id(SharedString::from(format!("hypr-bind-{}", row.line)))
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(8.))
        .py(px(5.))
        .rounded_md()
        .cursor_pointer()
        .hover(|s| s.bg(theme.interactive.hover))
        .on_click(cx.listener(move |this, _ev, _w, cx| {
            this.open_source(path.clone(), cx);
        }))
        .child(
            div()
                .min_w(px(0.))
                .text_size(px(11.5))
                .font_family(theme.font_mono)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text.primary)
                .whitespace_nowrap()
                .child(row.keys.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .text_size(px(11.))
                .text_color(theme.text.muted)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(row.action.clone()),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme.text.muted)
                .font_family(theme.font_mono)
                .child(format!(":{line}")),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KITCHEN_SNIPPET: &str = r#"
local M = os.getenv("HOME") .. "/.config/hypr/modules"
mainMod = "SUPER"
hi = "ignored line"
hl.bind(mainMod .. " + E",     app(thunar))                          -- thunar
hl.bind(mainMod .. " + Q",     hl.dsp.window.close())                -- close window
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"))
-- hl.bind("SUPER + COMMENTED", ignore_me()) -- commented-out bind is skipped
"#;

    const CHRONOS_SNIPPET: &str = r#"
local mainMod = mainMod or "SUPER"
hl.bind(mainMod .. " + L",     hl.dsp.exec_cmd("chronos-ipc toggle-launcher"))  -- ChronOS launcher
hl.bind(mainMod .. " + SHIFT + T", hl.dsp.exec_cmd("chronos-ipc toggle-theme"))  -- ChronOS theme
"#;

    fn p() -> PathBuf {
        PathBuf::from("/tmp/chronos-test.lua")
    }

    #[test]
    fn parses_literal_key_and_line_number() {
        let rows = parse_binds(KITCHEN_SNIPPET, "Kitchen", &p());
        let media = rows.iter().find(|r| r.keys == "XF86AudioLowerVolume").expect("media bind");
        assert_eq!(media.action, "hl.dsp.exec_cmd(\"wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-\")");
        assert_eq!(media.line, 7, "literal bind is on line 7 of the snippet");
        assert_eq!(media.source, "Kitchen");
    }

    #[test]
    fn resolves_main_mod_concat_to_super() {
        let rows = parse_binds(KITCHEN_SNIPPET, "Kitchen", &p());
        assert!(rows.iter().any(|r| r.keys == "SUPER + E"), "E bind resolves via mainMod");
        assert!(rows.iter().any(|r| r.keys == "SUPER + Q"), "Q bind resolves via mainMod");
    }

    #[test]
    fn uses_super_default_when_no_assignment_seen() {
        let src = "hl.bind(mainMod .. \" + A\", app(menu, \"--show drun\"))\n";
        let rows = parse_binds(src, "X", &p());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].keys, "SUPER + A");
    }

    #[test]
    fn parses_local_mainmod_or_assignment() {
        let rows = parse_binds(CHRONOS_SNIPPET, "ChronOS", &p());
        assert!(rows.iter().any(|r| r.keys == "SUPER + L"));
        assert!(rows.iter().any(|r| r.keys == "SUPER + SHIFT + T"));
    }

    #[test]
    fn strips_comment_and_closing_paren_from_action() {
        let rows = parse_binds(CHRONOS_SNIPPET, "ChronOS", &p());
        let l = rows.iter().find(|r| r.keys == "SUPER + L").expect("L bind");
        assert_eq!(l.action, "hl.dsp.exec_cmd(\"chronos-ipc toggle-launcher\")");
        assert!(!l.action.contains("--"), "comment must be stripped: {}", l.action);
    }

    #[test]
    fn skips_unknown_and_commented_lines() {
        let rows = parse_binds(KITCHEN_SNIPPET, "Kitchen", &p());
        assert!(!rows.iter().any(|r| r.keys.contains("COMMENTED")), "commented bind skipped");
        // All binds resolve to SUPER or a literal — no "MOD" placeholder leaks.
        for r in &rows {
            assert!(!r.keys.contains("MOD"));
        }
    }

    #[test]
    fn empty_source_yields_empty() {
        assert!(parse_binds("", "X", &p()).is_empty());
        assert!(parse_binds("-- nothing here\nlocal x = 1\n", "X", &p()).is_empty());
    }

    #[test]
    fn group_metadata_comment_is_used_as_source_label() {
        let src = "-- # group = \"Apps & Media\"\nhl.bind(mainMod .. \" + E\", app(thunar))\n";
        let rows = parse_binds(src, &parse_group(src), &p());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, "Apps & Media");
    }

    #[test]
    fn missing_metadata_falls_back_to_custom() {
        // Real-shaped module: no `group =` comment anywhere → "Custom", and the
        // raw filename/stem must NOT leak into the UI label.
        let src = KITCHEN_SNIPPET; // starts with `local M = ...`, no metadata
        assert_eq!(parse_group(src), "Custom");
        assert!(!parse_group(src).contains("kitchen"));

        let rows = parse_binds(src, &parse_group(src), &p());
        assert!(!rows.is_empty());
        for r in &rows {
            assert_eq!(r.source, "Custom", "no metadata → Custom, not a filename");
        }
    }

    #[test]
    fn group_ignored_in_code_but_read_in_comment() {
        // A `group =` outside a comment must be ignored.
        let src = "group = \"NotThis\"\n-- group = \"This\"\nhl.bind(mainMod .. \" + A\", app(x))\n";
        assert_eq!(parse_group(src), "This");
    }

    #[test]
    fn group_missing_when_unquoted() {
        let src = "-- group = unquoted\nhl.bind(mainMod .. \" + A\", app(x))\n";
        assert_eq!(parse_group(src), "Custom");
    }
}

# Hyprland binds tab (right panel) — how it reads and groups

Where the code lives and how it wires, so you don't re-derive it every session.

## Location

`crates/app/src/side_panel_right/tab/hypr_binds.rs` — the `HyprlandBinds`
right-panel tab (`TabContent::HyprlandBinds`). Read-only: no writes, no
`hyprctl`. It opens the source file in Preview via the shared `PreviewTarget`
global (path-only, like the Files tab).

## Data source (Hyprland 0.55+ modular Lua)

- Reads **every** `*.lua` under `~/.config/hypr/modules/` (found via
  `dirs::config_dir()`), sorted by name for stable order across Reload.
  There is **no** hardcoded allowlist — new modules appear as new groups
  automatically. Non-bind helpers (monitors/autostart/windowrules…) yield zero
  rows because the parser only matches `hl.bind(...)` lines.
- Fallback: when `modules/` is missing, parse the monolith
  `~/.config/hypr/hyprland.lua` (UI shows a note that modules/ is absent).
  Monolith group label is `"Custom"` too — never the raw filename.

## Grouping — metadata, not filenames (T236, 2026-08-04)

Sections are named from an **optional metadata comment** in the module:
`-- # group = "Apps & Media"`. `parse_group(src)` scans `--` comment lines for
`group = "Label"` and returns the quoted label; **fallback is `"Custom"`**.
The raw module filename/stem **never** leaks into the UI header.

Canon (PRODUCT.md §1 — binds are an onboarding surface, not a config dump for
yourself): **never rename or rewrite the user's `.lua` modules** — the
file→category mapping is UI-display only. The author adds `-- # group = "..."`
to their own config at their discretion; with no metadata, all bind modules
collapse into one honest `"Custom"` group (that's the intended fallback).

Parser is a targeted line scan for `hl.bind(...)` — not a full Lua AST. It
tracks the `mainMod` variable so `mainMod .. " + L"` renders as `SUPER + L`.

## gpui-ce fork gotcha (found while fixing T236)

In this gpui-ce fork, **`.on_click` only exists on the stateful element** — you
must chain `.id("...")` **before** `.on_click(...)`. Calling `.on_click` on a
plain `div()` (no `.id()`) is a compile error (`no method named on_click found
for struct gpui::Div`). This is the exact breakage in a pre-existing
`preview.rs` WIP (missing `.id()`); interactive rows in `hypr_binds.rs` use
`div().id(...).on_click(...)`.

## Tests

`cargo test -p chronos --lib hypr_binds` — 11 tests in-module (group metadata,
`Custom` fallback, no filename leak, `mainMod` resolution, commented-out binds
skipped). The parser helpers are pure (`parse_binds`, `parse_group`,
`parse_main_mod`, `quoted`, `resolve_keys`, `parse_bind_line`) — unit-testable
without `App`/`cx`.

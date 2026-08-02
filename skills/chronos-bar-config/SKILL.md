---
name: chronos-bar-config
description: >
  Use when asked to change ChronOS's top bar — height, edge, corner radius,
  floating/margins, or which widgets sit in the left/center/right sections.
  Covers reading the current config, writing a change, and the invariants
  that keep the file valid (clamped ranges, unknown widget names dropped,
  floating forces the exclusive zone off). The bar hot-reloads on save — no
  restart needed.
---

# ChronOS bar config

The bar's live config is one file: `~/.config/chronos/bar.toml`. There is no
other API — no RPC, no CLI. Read it with your normal file tools, understand
it against the schema below, write it back with your normal file tools. The
shell watches this path (inotify) and re-applies within ~300ms of a save.

**Ground truth for this schema**: `crates/app/src/bar/appearance.rs` and
`crates/app/src/bar/layout_config.rs` (types `BarAppearance` /
`BarLayoutConfig`) — if this doc and the code ever disagree, the code wins;
the reference implementation of everything below also lives in
`crates/app/src/bar/agent_api.rs` (`merge_patch`/`snapshot`/`list_widgets`)
if you need to see the exact merge/sanitize logic spelled out in Rust.

## Reading the current config ("get_bar_config" / "list_bar_widgets")

```
cat ~/.config/chronos/bar.toml
```

If the file doesn't exist yet, the shell is running on hardcoded defaults —
treat that as:

```toml
version = 2
left = ["dock", "separator", "workspaces"]
center = ["mpris", "cava"]
right = ["project", "workspace_mode", "separator", "volume", "network",
          "tray", "updates", "system", "notification_bell", "separator",
          "battery", "clock"]
# [appearance] omitted = all fields at their defaults (see table below)
```

**Available widget names** (`left`/`center`/`right` may only contain these):

```
dock, separator, workspaces, mpris, cava, project, workspace_mode, volume,
network, tray, updates, system, notification_bell, battery, clock
```

Plus any currently-loaded Luau plugin widget — if you're not sure a plugin
name is live, ask the user or check `~/.config/chronos/plugins/` rather than
guessing; a name outside this list is silently dropped on next load (see
Sanitize below), it will not error, but it also will not appear.

## Full schema

```toml
version = 2   # ALWAYS write this when you touch appearance — see note below

[appearance]
edge = "top"        # "top" | "bottom" | "left" | "right" (left/right parse but aren't applied to layer-shell placement yet)
height = 30          # px, clamped to [20, 80]
width = "full"       # "full" | "hug" | "fraction:0.7"  (fraction clamped to [0.2, 1.0])
align = "center"     # "start" | "center" | "end" — only matters when width != "full"
margin = { x = 0, y = 0 }   # px, negative values zeroed
floating = false     # true = bar doesn't reserve compositor space
exclusive = true     # reserve a layer-shell exclusive zone; FORCED false if floating = true, no matter what you write
radius = 0           # px corner radius, clamped to [0, 24]
elevation = "none"   # "none" | "soft" | "strong" — shadow depth

left = [...]
center = [...]
right = [...]
```

**Why `version = 2` matters**: files without `version` (or `version = 1`)
have their `[appearance]` section **ignored entirely** on load, even if it's
present and well-formed — this is a backward-compat gate for pre-T199
configs. If you write appearance changes without `version = 2`, they will
silently not apply. Widget list changes (`left`/`center`/`right`) don't need
`version = 2` — those are honored regardless of version.

## Writing a change ("set_bar_config")

1. Read the current file (or use the defaults above if it doesn't exist).
2. Apply your change **as a merge, not a replacement** — keep every field
   you're not changing exactly as it was. Concretely:
   - Changing one appearance field (e.g. `height`) → keep every other
     `[appearance]` field and both other sections (`left`/`center`/`right`)
     untouched.
   - Changing a widget section (e.g. `center`) → write the **full new
     array** for that section; the other two sections stay untouched. There
     is no "add one widget" file syntax — compute the full array yourself
     (read current `center`, add/remove the name, write the whole list
     back).
3. Write the whole file back with `toml::to_string_pretty`-equivalent
   formatting (plain TOML, one `[section]` per top-level table — see the
   Full schema block above for the exact shape).
4. Do not invent new top-level keys or new `[appearance]` fields — anything
   not in the schema above is ignored on load (harmless, but it means your
   change had no effect, so don't rely on it).

## Sanitize — what happens if you write something out of range

The shell always clamps/corrects on load, so a slightly-wrong write is
**never** a crash or a corrupted file — but the correction may silently
differ from what you asked for, so check the result if precision matters:

| You write | Shell does |
|---|---|
| `height` outside `[20, 80]` | clamps to nearest bound |
| `radius` outside `[0, 24]` | clamps to nearest bound |
| `width = "fraction:N"` with N outside `[0.2, 1.0]` | clamps N |
| negative `margin.x` / `margin.y` | zeroed |
| `floating = true` and `exclusive = true` together | `exclusive` forced to `false` |
| unknown widget name in `left`/`center`/`right` | dropped from that list, rest kept |
| unrecognized `edge`/`align`/`elevation` string | falls back to that field's default |
| wrong TOML **type** (e.g. `width = 5` instead of a string) | whole file fails to parse → shell falls back to **all** defaults, your widget lists included — this is the one mistake that loses more than the one bad field, so get the TOML types right (strings quoted, numbers bare) |

## Example: move `clock` from right to left

```toml
# before
right = ["project", "workspace_mode", "separator", "volume", "network",
          "tray", "updates", "system", "notification_bell", "separator",
          "battery", "clock"]
left = ["dock", "separator", "workspaces"]

# after — write BOTH full arrays, remove from one, add to the other
right = ["project", "workspace_mode", "separator", "volume", "network",
          "tray", "updates", "system", "notification_bell", "separator",
          "battery"]
left = ["dock", "separator", "workspaces", "clock"]
```

## Example: floating pill bar

```toml
version = 2

[appearance]
floating = true
height = 36
radius = 18
elevation = "soft"
margin = { x = 12, y = 8 }
width = "fraction:0.6"
align = "center"
# exclusive not written — floating forces it off regardless
```

## What this skill does NOT cover

- Theme/color changes — separate config (`theme_config`), not this file.
- Per-widget internal settings (e.g. clock 12h/24h) — not schema-covered by
  this task (T201); ask the user or check if a later slice added it.
- Hyprland keybinds — unrelated file, unrelated skill.
- A GPUI-native settings page for this exists as a separate roadmap item
  (T202) — until it ships, this file is the only interface, for humans and
  agents alike.

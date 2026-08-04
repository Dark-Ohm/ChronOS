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

**Always `cat` the file (or call `get_bar_config`, if you have Rust/tool
access to `crates/app/src/bar/agent_api.rs`) immediately before writing.**
This is a read-modify-write, not a blind overwrite — skipping the read means
you're guessing at fields you didn't ask to change, and a guess that's wrong
silently reverts something the user set five minutes ago.

**Never tell the user to restart ChronOS, log out, or `pkill chronos` to see
a bar change.** That is never the correct answer for anything in this
schema — the bar hot-reloads on save (T134 inotify watch), typically within
300ms. If a change doesn't seem to apply, the fix is: re-read the file to
confirm your write landed and check for a TOML type error (see the sanitize
table below, last row) — never "have you tried restarting".

## Natural-language phrase → schema key

| User says | Field(s) |
|---|---|
| "move the bar to the bottom" | `appearance.edge = "bottom"` |
| "make it float" / "floating bar" | `appearance.floating = true` (this also forces `exclusive = false` — don't write `exclusive = true` alongside it, it will be silently overridden) |
| "80% width", "narrower", "not full width" | `appearance.width = "fraction:0.8"` |
| "full width" | `appearance.width = "full"` |
| "center it", "align center" | `appearance.align = "center"` (only visible when `width != "full"`) |
| "left-aligned", "right-aligned" | `appearance.align = "start"` / `"end"` |
| "rounder corners", "round the corners", "radius 12" | `appearance.radius = 12` |
| "sharp corners", "no rounding" | `appearance.radius = 0` |
| "taller bar", "make it bigger", "height 40" | `appearance.height = 40` (clamped to [20, 80]) |
| "add a shadow", "give it depth" | `appearance.elevation = "soft"` (or `"strong"` for "more shadow" / "stronger depth") |
| "flat", "no shadow" | `appearance.elevation = "none"` |
| "add margin", "give it some breathing room" / "gap from the edge" | `appearance.margin = { x = ..., y = ... }` |
| "hide cava" / "remove the visualizer" | remove `"cava"` from whichever section currently has it (read first — it's `center` by default) |
| "clock on the right" | ensure `"clock"` is in `right`, not wherever it currently is — if it's already elsewhere, remove it from there and add it to `right` |
| "add \<widget\> to the \<section\>" | append the name to that section's array (full-array write, not "add" sugar in the file itself) |

If a phrase doesn't map cleanly to one row here, say so to the user rather
than guessing a field name that doesn't exist in the schema — an unknown
top-level key is silently ignored (§Writing a change, point 4), so a wrong
guess looks like nothing happened, which is worse than admitting you're not
sure.

## Full worked example — the epic demo phrase

> "бар снизу, 80% ширины по центру, скругление 12, тень, без cava, clock справа"
> ("bar on the bottom, 80% width centered, radius 12, shadow, no cava, clock on the right")

Read current `right` first (assume it's still the shipped default — always
verify against the real file, this is illustrative):

```
right (before) = ["project", "workspace_mode", "separator", "volume",
                    "network", "tray", "updates", "system",
                    "notification_bell", "separator", "battery", "clock"]
center (before) = ["mpris", "cava"]
```

`clock` is already in `right` — "clock on the right" is a no-op for that
field, nothing to move. `cava` is in `center` — remove it. Full patch, one
multi-field turn:

```toml
version = 2

[appearance]
edge = "bottom"
width = "fraction:0.8"
align = "center"
radius = 12
elevation = "soft"
# height/margin/floating/exclusive not mentioned → left as they were, don't touch

left = [...]        # unchanged — copy the current value verbatim
center = ["mpris"]  # cava removed, mpris kept
right = [...]        # unchanged — clock already here, copy current value verbatim
```

Write the whole file (all fields, not a diff) with this shape. This is one
`set_bar_config`-equivalent turn even though it touches five appearance
fields and one widget section — do it as a single file write, not five
separate edits (five separate writes each retrigger the hot-reload/apply
cycle for no benefit and risk an intermediate state being visible).

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

## After you write: the user sees which file changed

If ChronOS's own `set_bar_config`/`set_bar_config_applied` path is what
actually performed the write (rather than you editing the file directly),
the shell points its Editor tab at `bar.toml` automatically after a
successful apply (T203) — the user sees the exact file and the exact diff
without asking. If you're editing the file directly with your own file
tools instead of going through that path, mention the file path
(`~/.config/chronos/bar.toml`) in your reply so the user has the same
visibility either way.

There is no undo command. If the shell ever writes a `.bak` alongside
`bar.toml` before an agent write, mention it; as of this schema version it
does not — treat every write as final, and lean harder on "read before you
write" for that reason.

## Right-section visual grouping (T234) — spacing, not dividers

The right bar section is laid out with **two levels of spacing**, not a flat
gap and not 1px `separator` dividers:

- **`4px`** between widgets *inside* one semantic cluster.
- **`14px`** between clusters.

Clusters are decided by widget *role*, not by `separator` entries. The
grouping lives in `crates/app/src/bar/mod.rs`
(`right_section_div` + `group_right_names` + `right_widget_group`); the role
map is:

| Role (group id) | Widgets |
|---|---|
| `project` (2) | `project` |
| `mode` (3) | `workspace_mode` |
| `keyboard_layout` (4) | `keyboard_layout` |
| `clock` (5) | `clock` |
| `status` (1) | everything else (`volume`, `network`, `battery`, `tray`, `updates`, `system`, `notification_bell`, …) |
| `separator` (0) | forced group break, **dropped from render** |

Consequences when you edit `right = [...]`:

- **`separator` in the right section no longer draws a 1px line.** It only
  forces a cluster break; if you want a visible divider there, you won't get
  one — the 14px gap is the delimiter. (Left/center sections still render the
  1px `separator` divider as before.)
- A cluster break also happens automatically when the *role* of consecutive
  widgets changes (e.g. `project` → `workspace_mode` is already two clusters),
  so you generally don't need `separator` in `right` at all.
- Reordering widgets is safe: `move_widget` uses the config order index, which
  is independent of which cluster a widget renders in.
- The network ↓/↑ KB/s counters render in `theme.text.muted` (recolored from
  `secondary` in T234) so they don't compete with the clock.

Left section keeps its 12px gap; center keeps 8px. Only the right section is
grouped this way.

## What this skill does NOT cover

- Theme/color changes — separate config (`theme_config`), not this file.
- Per-widget internal settings (e.g. clock 12h/24h) — not schema-covered by
  this task (T201); ask the user or check if a later slice added it.
- Hyprland keybinds — unrelated file, unrelated skill.
- A GPUI-native settings page for this exists as a separate roadmap item
  (T202) — until it ships, this file is the only interface, for humans and
  agents alike.

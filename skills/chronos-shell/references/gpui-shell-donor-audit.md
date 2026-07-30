# gpui-shell donor — rewrite-by-pattern audit (2026-07-16)

Donor: `/home/neo/projects/chronos-ecosystem/ChronOS/reference/gpui-shell-main`
(upstream `zed-industries/zed` gpui, **unpinned `branch = "main"`**).
Target: ChronOS gpui-ce fork (`../Source/gpui`).

**Legal:** NO LICENSE file anywhere in the donor tree → "all rights reserved"
→ only legal path is REWRITE-BY-PATTERN (our code, their architecture).
README.md DOES exist (don't claim "no README" — that sub-claim is false).

**gpui dep drift (critical):** donor `Cargo.toml:6` pins nothing
(`git = ".../zed", branch = "main"`). Anything touching upstream-`main` gpui
API (`cx.theme()`, `canvas()`, `on_drag`, `EntityId`, div-builder,
`ActiveTheme`) will NOT compile against gpui-ce without rewrite. Treat all
donor UI as rewrite-by-pattern, never vendored.

## Reusable techniques (gpui-ce-safe)

### 1. Layer-shell focus hygiene (partial fix for our Critical launcher focus bug)
Donor `crates/app/src/launcher/mod.rs:336-338` re-asserts focus every frame:
```rust
// first line of Render::render:
if !self.focus_handle.is_focused(window) {
    self.focus_handle.focus(window, cx);
}
```
plus `.track_focus(&self.focus_handle)` on the root div (mod.rs:376) and
`.key_context("Launcher")`. OUR `crates/app/src/launcher/view.rs` does
NEITHER — it only stores a `FocusHandle` and handles raw `on_key_down`.
Steal #1+#2 as an **S-cost hygiene** patch.
CAVEAT: mitigation only, not the real fix. Root cause is compositor-level
(layer-shell `OnDemand` + `activate_window()`→`xdg_activation_v1` rejected for
layer-shell). Real fix = XDG-toplevel migration (XL, undecided in docs/DECISIONS.log).
NEVER copy donor's `KeyboardInteractivity::Exclusive` (mod.rs:642) — our MEMORY
proves that crashes Hyprland/Niri.

### 2. In-app keybinds (donor `crates/app/src/keybinds.rs`, 84 LOC)
Pure GPUI, NO global daemon/evdev/compositor-IPC:
```rust
actions!(keybinds, [Cancel, Confirm, CursorUp, CursorDown, /* … */]);
cx.bind_keys([ KeyBinding::new("escape", Cancel, Some("Launcher")), /* … */ ]);
```
consumed via `.key_context("Launcher")` + `.on_action(cx.listener(|this, _: &Cancel, window, cx| …))`.
Steal as our input model (cleaner than raw `on_key_down` string matching).
gpui-ce has `actions!` / `KeyBinding` / `bind_keys`.

### 3. TOML config + inotify hot-reload (donor `crates/app/src/config/mod.rs`)
`Config` is a `#[serde(default)]` GPUI Global; `start_hot_reload()` spawns a
`FileWatcher::watch(path)` per file (config.toml + theme.toml); the inotify
thread (`crates/services/src/watcher.rs`, 200 ms debounce) sends `()` over an
`mpsc` channel; a `cx.spawn` loop does `reload()` + `cx.refresh_windows()`.
Better than ours: per-file `watch_config` / `watch_theme` booleans.
FLAG: ChronOS app-level config/theme hot-reload does NOT exist yet (only luau
+ launcher-entry inotify from merged `feat-inotify-hot-reload`);
docs/ARCHITECTURE.md:81/218 confirm "NOT YET" for app config.

## Item-by-item port cost (donor LOC → our analog → cost)

| Item | Donor LOC | Our analog | Cost | Note |
|---|---|---|---|---|
| control_center/ | 2785 | ABSENT | XL (gated on ~10 missing service backends: audio/bluetooth/brightness/privacy/tray/mpris/wallpaper/notification/applications) | architecture reusable (subscribe+dispatch), code throwaway |
| config/ + hot-reload | ~707 + 90 + 356 | PARTIAL (inotify yes; app config/theme Global no) | M–L | steal Theme Global + base16/Stylix |
| keybinds.rs | 84 | ABSENT | S | bind_keys pattern |
| ui/ crate | 2299 | ABSENT | M–L | only `InputBuffer` (349) + theme color math portable; slider/switch/list/label tied to div-builder; Button primitive ABSENT (empty file) |
| launcher focus | 665 | EXISTS, focus gap | S (hygiene) / XL (toplevel) | see technique #1 |

## AGENTS.md / claim flags
- Donor `components/button.rs` is a 0-byte file (no Button primitive to assess).
- Donor `traits/` is negligible (1 + 20 LOC).
- Donor `panel.rs` (claimed in their AGENTS.md) EXISTS — confirmed.
- Donor AGENTS.md implies "no README" — FALSE; README.md present (3846 B).
- Donor `launcher/mod.rs:642` uses `KeyboardInteractivity::Exclusive` — our
  MEMORY proves this crashes Hyprland/Niri. Do not copy.

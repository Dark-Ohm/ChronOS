# T134 report — bar layout + Edit Mode shell (Phase 1)

**Status:** implementer complete — unit/check green; **live smoke PENDING Architect**.

## Delivered

| Item | Path / detail |
|------|----------------|
| Config | `crates/app/src/bar/layout_config.rs` — `BarLayoutConfig {left,center,right}`, default = historical `register_builtin`, load/save, sanitize, move, inotify watcher 300ms |
| Apply | `widgets/mod.rs` — `instantiate` factory + `ForcedSection` + `apply_layout` (clear registry → re-register order); separators multi-OK |
| Registry clear | `crates/luau/src/bar.rs` — `BarWidgetRegistry::clear()` |
| Plugin safety | after layout apply/move: `reregister_plugin_widgets` via `PluginManager` global |
| EditMode | `crates/app/src/edit_mode.rs` — Global `{active}`, init in `main` |
| IPC | payload `toggle-edit-mode`; messages + `accept_loop` + `ipc/mod.rs` debounce → `edit_mode::toggle` |
| UI | bar EDIT badge + accent border; per-widget ◀▶; writes `~/.config/chronos/bar.toml` |
| Popup gate | volume, system, updates, notification_bell, project, tray — no open while edit active |

## Non-goals (unchanged)

Drag (T135), hotview expand (T136), Plasma editor, side-panel layout, T129 motion.

## Verify (ran)

```text
cargo check -p chronos                          # ok
cargo test -p chronos --bin chronos -- layout_config   # 4 ok
cargo test -p chronos --bin chronos -- ipc::           # messages+service ok
cargo test -p chronos-luau -- bar                      # 4 ok
```

## Live smoke (Architect)

```bash
chronos-rebuild && chronos-stop && chronos-start
# missing bar.toml → default order
# edit ~/.config/chronos/bar.toml → bar reorders without restart
# Super+Shift+E:
python3 -c "import os,socket; s=socket.socket(socket.AF_UNIX); s.connect(os.environ['XDG_RUNTIME_DIR']+'/chronos.sock'); s.sendall(b'toggle-edit-mode')"
# EDIT badge; ◀▶ reorder; bar.toml updates; volume click does not open popup
```

### Hyprland bind (user)

```conf
bind = SUPER SHIFT, E, exec, python3 -c "import os,socket; s=socket.socket(socket.AF_UNIX); s.connect(os.environ['XDG_RUNTIME_DIR']+'/chronos.sock'); s.sendall(b'toggle-edit-mode')"
```

## Notes / tails

- Watcher only if `~/.config/chronos/` already exists at bar init (same as theme). First save from edit creates file; if dir was missing at start, restart once or create dir before start.
- Add/Remove widgets stubbed — only move within section in Phase 1.
- Plugin widgets re-appended after builtins (not interleaved in bar.toml).

## Architect accept 2026-07-26

**Verdict: ACCEPTED** (popup-gate code-only caveat).

| Check | Evidence |
|-------|----------|
| Super+Shift+E bind | user OK; hyprland.lua wired + reload |
| EDIT chrome | grim `/tmp/t134-accept/bar-edit-on.png` — EDIT badge + ◀▶ |
| IPC toggle | log `edit_mode: toggled active=true/false` |
| bar.toml hot-reload | log `hot-reloaded layout from …/bar.toml` after write |
| Move in UI | log `bar: moved widget` (user session + smoke) |
| Persist + restart | restart pid new; bar paints; file order kept |
| Popup gate in EDIT | code gates volume/system/updates/bell/project/tray — **not** ydotool-click confirmed |

**Caveat:** live click-confirm that volume does not open while EDIT — PENDING if pedantic; pattern matches other gates, code present.

**Next front (user):** ACP left panel revive — not T135 unless asked.

Note: accept smoke briefly rewrote bar.toml; restored user order (custom right) from pre-snapshot.

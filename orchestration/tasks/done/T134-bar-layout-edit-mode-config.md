# T134 — Bar layout config + Edit Mode shell (Phase 1)

**Статус: IMPLEMENTER DONE — live smoke PENDING.**  
**Канон:**  
`docs/superpowers/specs/2026-07-26-edit-mode-and-hot-reload-design.md`  
`docs/superpowers/specs/2026-07-24-bar-widget-layout-config-design.md`  
**Не:** Plasma editor, drag-drop (T135), hotview expansion (T136), full
`crates/app` dylib, T129 motion, side-panel layout config.

| | |
|---|---|
| **Skills** | `chronos-shell`, `hot-lib-reloader` (read-only — do not expand hotview) |
| **Паттерн** | `theme_config.rs` watcher + `dock/config.rs` persistence |
| **Отчёт** | `orchestration/tasks/report/T134-bar-layout-edit-mode-config-report.md` |
| **Коммит** | `bar : layout.toml hot-reload + edit mode shell (T134)` |

## Цель

1. **`~/.config/chronos/bar.toml`** — порядок виджетов Left/Center/Right.  
2. **Hot-reload** без рестарта (inotify, как theme).  
3. **Edit Mode** global + IPC/keybind + visual badge; minimal reorder UI
   (move left/right or ⋯ menu) that **writes the same file**.

Default config = byte-identical order to today’s `register_builtin`.

## Задачи

### Task 1 — Config types + default

- `crates/app/src/bar/layout_config.rs` (or `bar/config.rs`).
- `BarLayoutConfig { left, center, right: Vec<String> }` serde.
- `Default` = current hardcoded order (copy from `widgets/mod.rs` exactly).
- Path: `dirs::config_dir()/chronos/bar.toml`.
- Load: missing → default; parse error → warn + default; **no silent write**
  on load.
- Unit tests: default non-empty; unknown name filter helper; empty section OK.

### Task 2 — Apply to registry

- Named factory table: `"network" → register_fn` for all builtins.
- `apply_layout(cx, &BarLayoutConfig)` rebuilds order (unregister-all
  builtins or clear sections + re-register in order — pick one clear model,
  document in report).
- Separators: if today multiple separators share one type, use unique keys
  (`separator` multi-ok if register always pushes; or `separator` allowed
  multiple times). Prefer **name may appear multiple times** = multiple
  separator widgets.
- Unknown names: warn skip.
- Call apply at bar init **after** factories known.

### Task 3 — Hot-reload watcher

- Mirror `theme_config::spawn_watcher` (parent dir watch, basename
  `bar.toml`, debounce 300ms, GPUI `cx.update` apply).
- Init from `bar::init` / main.
- Log: `bar: hot-reloaded layout from …`.

### Task 4 — EditModeState

- `crates/app/src/edit_mode.rs`: `EditModeState { active: bool }`, Global,
  `toggle`, `is_active`.
- Init in main.
- IPC: `toggle-edit-mode` payload (messages + service accept loop + hypr
  note in report).
- Keybind suggestion Super+Shift+E (document; wire if hypr conf in tree).

### Task 5 — Bar chrome in edit mode

- When `EditModeState.active`: bar shows clear **EDIT** indicator (text or
  accent strip).
- Widget primary open (volume/system/…) **disabled** while edit active
  (or only left-click opens edit menu).
- Minimal affordance: each widget (or bar context) can **Move left / Move
  right** within section → mutate config → **write bar.toml** → apply.
  Implement at least move within section; Add/Remove can be stub if time.

### Task 6 — Verify + report

```bash
cargo test -p chronos --lib bar  # or layout_config tests
cargo build --release -p chronos
# live:
# - no bar.toml → same order as before
# - edit bar.toml → bar updates live
# - toggle edit mode → badge; move widget → file + UI
# - release binary does not need --features hot-reload
```

Report: files, default table, IPC name, known limitations (no drag yet).

## Зона файлов

**Писать:**
- `crates/app/src/bar/layout_config.rs` (new)
- `crates/app/src/bar/widgets/mod.rs` (factory + apply)
- `crates/app/src/bar/mod.rs` / view (edit chrome)
- `crates/app/src/edit_mode.rs` (new)
- `crates/app/src/main.rs` (init)
- `crates/app/src/ipc/messages.rs` + `service.rs`
- optional `docs/dev-cli.md` one paragraph

**Не трогать:**
- `crates/hotview` (T136)
- side_panel_* layout
- theme schemes, motion.rs
- services backends

**Читать:**
- `theme_config.rs`, `dock/config.rs`
- `bar/widgets/mod.rs::register_builtin`
- both specs under `docs/superpowers/specs/`

## Accept

1. Default layout ≡ pre-change order.  
2. Hand-edit `bar.toml` live-updates bar.  
3. Edit mode toggle + visual cue.  
4. At least one UI path mutates order + persists file.  
5. Release build green without hot-reload feature.  
6. Report honest about no drag.

## Reject

- Drag-and-drop Phase 2.  
- Full app dylib reload.  
- Breaking volume click in normal mode.  
- Silent rewrite of bar.toml on every start.  
- Fabricated “works” without live or clear test evidence.

## Commit style

```
bar : layout.toml load/apply/hot-reload (T134)
edit_mode : global toggle + IPC (T134)
bar : edit-mode badge + move affordance (T134)
```

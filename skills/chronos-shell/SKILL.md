---
name: chronos-shell
description: >
  Working on THIS repo — a Rust/Kael desktop shell for Hyprland/Niri with a
  sandboxed mlua/LuauJIT plugin system. Use when touching crates/app,
  crates/services, crates/luau, crates/ui, crates/plugins, bar/dock/launcher/
  notifications/osd/tray_menu, the Service trait, any *Subscriber, or the Lua
  plugin hot-reload path.
---

# Chronos Shell

Canonical design: `ARCHITECTURE.md` (accepted) + `DECISIONS.log` (rejected +
why) at repo root. **Operational field state:** `HANDOFF.md` — read first in
every multi-agent / minion session. This skill is *where the code lives and how
it wires*; those docs win on *why*. Session routing: `start-here`.

**Stack:** Rust edition **2024** + GPUI via path dep on the local fork
**`../Source`** (gpui-ce ChronOS edition — **not** crates.io `gpui`) + `mlua`
(Luau) for plugins + `zbus` 5 for D-Bus. Workspace members: `crates/app`
(bin `chronos`, lib `chronos_app`), `crates/luau`, `crates/services`,
`crates/ui`. No `gpui_component` — raw `gpui::div()` only (that crate is
`chronos-fm`).

**GPUI path (ground truth, `Cargo.toml`):**
```toml
gpui = { path = "../Source/gpui" }
gpui_platform = { path = "../Source/gpui_platform" }
```
Old paths like `/home/neo/Projects/SOURCE/gpui/gpui-ce-main` are **stale**.
Worktrees must be a **sibling of ChronOS** (so `../Source` resolves) — never
`/tmp` alone.

## Module map (2026-07-17)

### `crates/services` — subscribers (`Service` trait)

| Module | Bus / backend | Notes |
|---|---|---|
| `compositor` | Hyprland / Niri | listener on **`std::thread`**, not tokio |
| `network` | NetworkManager (system) | zbus + retry |
| `upower` | UPower (system) | battery + `has_battery` |
| `notification` | fdo Notifications (session) | server |
| `tray` | StatusNotifierWatcher (session) | + `tray/menu.rs` DBusMenu client |
| `audio` | `wpctl` MVP poll 250ms | `dispatch` + immediate re-read |
| `applications` | `.desktop` scan + inotify | launcher data, **mpsc** debounce (not crossbeam), `strip_field_codes` in parser |
| `wallpaper` | awww MVP + multi-backend enum | 5 engines on host |
| `mpris` | session `org.mpris.MediaPlayer2.*` | ListNames + NameOwnerChanged |

`Services` / `init_all()` in `lib.rs` — **shared file**, add only your lines.
Commands are concrete methods (`dispatch`), **not** on the trait.

### `crates/app` — shell UI

| Path | Role |
|---|---|
| `bar/` | layer-shell TOP strip; widgets via registry |
| `bar/widgets/` | clock, workspaces, battery, network, tray, **volume**, **mpris** |
| `osd/` | volume OSD overlay (soft-hide, no Exclusive keyboard) |
| `notifications/` | fdo popup stack (rubber-band height — see `gpui-layer-shell`) |
| `launcher/` | app launcher — **uses `AppState::applications(cx)` via `state::watch`** (no more local cache); `launch.rs` re-uses `strip_field_codes` from services |
| `dock/` | pinned launch panel (not a live taskbar) — icon resolver + PINNED_IDS hardcoded |
| `tray_menu/` | DBusMenu popup UI (paired with tray right-click) |
| `ipc/` | single-instance Unix socket + wallpaper-next/set payloads |
| `wallpaper_ctl.rs` | IPC wallpaper-next / wallpaper-set — scan `~/Pictures/Wallpapers`, round-robin |
| `state.rs` | `AppState` global + `watch()` signal bridge |
| `plugin_bridge.rs` | Lua → `BarWidget` |

### Bar widgets + watches

`Bar::new` subscribes (via `watch`) so service updates repaint the bar. The
list includes **compositor, network, upower, notification, audio**. Adding a
new reactive widget usually needs a matching
`watch(cx, AppState::<svc>(cx).subscribe(), …)` line — if `bar/mod.rs` is
outside your zone, **ask**; do not freestyle. Clock still has a 1s ticker.

| Widget | Section | Interaction |
|---|---|---|
| workspaces | Left | click → focus |
| clock | Center | — |
| mpris | Center | click → PlayPause |
| battery / network / tray / volume | Right | volume: click mute, scroll ±5% |

## Three real architectural patterns

### 1. Layer-shell windowing
Surfaces use `WindowKind::LayerShell(LayerShellOptions { … })`, not plain
windows. Bar: `Layer::Top`, TOP|LEFT|RIGHT, exclusive zone. Dock: `Layer::Top`,
BOTTOM, exclusive zone (independent of bar). OSD / notifications / tray_menu:
`Layer::Overlay`. **`KeyboardInteractivity::Exclusive`
is FORBIDDEN forever** — freezes Hyprland input stack. Use `None` (or
`OnDemand` only if you have a proven need). Soft-hide pattern (OSD): keep the
window, empty content / empty input region — do **not** `remove_window` if
re-open races produce `window not found`.

One bar window per display (`bar::init`), short startup delay for display
enumeration. Height-tracking popups: skill **`gpui-layer-shell`**.

### 2. `Service` trait — reactive, no commands on the trait
```text
trait Service {
  type Data; type Error: Send + Sync + 'static;
  fn subscribe(&self) -> impl Signal<…>;
  fn get(&self) -> Data;
  fn status(&self) -> ServiceStatus;
}
```
Backed by `futures_signals::Mutable`. Async constructors call
`Handle::current()` and **panic outside a tokio runtime** — `init_all()` runs
inside `rt.block_on`. Template: UPower / audio / mpris / wallpaper.

**D-Bus variant trap (`a{sv}`):** dict values often arrive as nested
`Value::Value`. Recipe: `unwrap_variant` in `tray/menu.rs` (also used by
MPRIS metadata). Fixtures must mirror live `busctl`/`gdbus` shape — invented
fixtures have failed twice.

### 3. Runtime split — three executors on purpose
- **tokio** (`#[tokio::main]`): IPC, D-Bus loops, audio poll, `dispatch` spawns.
- **`std::thread`:** compositor listener only (must not freeze at Unavailable).
- **GPUI executor** (`cx.spawn` / `background_executor()`): bar clock, OSD hide
  timer, plugin tick, UI-adjacent work. **Never** drive UI from tokio.

## Patterns for new work

### New service
1. `crates/services/src/<name>/{mod.rs,types.rs}` — copy UPower (zbus) or
   audio (poll + dispatch) or mpris (dynamic discovery).
2. Own lines only in `lib.rs`: `pub mod`, re-export, `Services` field,
   `init_all()`, optional runtime-guard test.
3. `AppState::<name>(cx)` accessor in `state.rs`.
4. If bar needs live repaint: ask for / add `watch` in `bar/mod.rs`.

### New bar widget
1. `bar/widgets/<name>.rs` — `BarWidget`, pure `describe` + unit tests
   (see `network.rs` / `volume.rs` / `mpris.rs`).
2. Two lines at **end** of `widgets/mod.rs`: `mod` + `register` — do not
   reorder others' lines.
3. Click: `on_click` + `AppState::…(cx).dispatch(...)` (tray / volume pattern).
4. Scroll: `on_scroll_wheel` + `ScrollDelta` (volume pattern).
5. Icon-theme lookup (tray pattern):
   - Check `icon_name` for absolute path first.
   - Build theme chain: `[gtk-icon-theme, ...Inherits, hicolor]` from
     `settings.ini` and `default/index.theme` (read at most once via `OnceLock`).
   - Walk bases × themes × `{scalable, 16x16, ...}` × `{devices, apps, ...}` × exts.
   - Cache resolved paths in `thread_local! RefCell<HashMap<String, Option<PathBuf>>>`.
   - Fallback chain: icon_name → icon_pixmap → letter badge.

### Launcher (migrated to applications service)

Launcher no longer has its own desktop entry cache. `view.rs` uses
`AppState::applications(cx)` + `state::watch()` for live updates. The old
`cache.rs` and `entry.rs` are deleted. `launch.rs` imports
`strip_field_codes` from `chronos_services`.

### New launcher widget

If the widget is focusable (text input) or reacts to mouse clicks, the activation
observer must be gated to avoid race conditions:

```rust
// In view struct:
pub interacted: bool,  // set by click handler

// In activation observer (mod.rs):
if window.is_window_active() {
    was_active = true;
} else if was_active {
    if view.interacted {
        view.interacted = false; // reset gate
        return;                  // click handler already closed
    }
    close_this(window, cx);
}

// In click handler on result rows:
vh.update(cx, |view, _| view.interacted = true);
launch(&entry.exec);
close_this(window, cx);
```

This prevents a click inside the launcher from triggering `active=false` (Wayland
spurious deactivation) before the handler runs.

### Soft-hide / popup lifecycle
Prefer empty render + kept surface over remove/recreate when Hyprland races
appear. See `osd/mod.rs` after f4edb88.

## Plugin system (`crates/luau`)

- Discovery: subdir needs both `manifest.toml` and `init.luau`.
- Sandbox: fresh Lua, strip `os`/`io`/`debug`, capability-gated `chronos.*`.
- Identity by **directory path**, not manifest `name` (regression tests exist).
- Hot-reload: inotify + 300 ms debounce → `PluginManager::reload` via
  `cx.update_global` — nested lease rules in `watcher.rs` comments.

## Field rules (blood, 2026-07-17) — also in HANDOFF

- **`git stash` of foreign WIP — FORBIDDEN.** Isolation =
  `git worktree add <sibling-of-ChronOS> <commit>` only.
- **No `git checkout` / `mv` of others' files** to "clean" the tree.
- **No `cargo clean` on the shared tree** (wipes everyone's `target/`).
- **`pkill -x chronos` only** — never `pkill -f` (kills the parent shell).
- **Single-instance shell:** second `chronos` pings and exits — restarts
  without pkill are fake.
- **UX smoke = release only** (`cargo build --release -p chronos`).
- Named `git add` + `git diff --staged` before commit; no AI trailers.
- **`reference/gpui-shell` unlicensed** — rewrite-by-pattern, 0 copied lines.
  **`reference/waytrogen-main` Unlicense** — copy OK (NOTICE in `../Source`).
- **Claims must match tree.** If you say "I did X", `grep`/`read` the file
  to confirm X is actually in the working copy, not in a stash or a branch
  that didn't get committed. False claims cost a full re-work cycle.
- **Watch handlers need `cx.notify()`.** Data update without notify = stale UI.
  Pattern: `state::watch(cx, signal, |this, state, cx| { this.update(state); cx.notify(); })`.

## Verification (before claiming done)

```bash
cargo test --workspace --lib --bins   # count drifts; all green
cargo build --release -p chronos
pkill -x chronos; RUST_LOG=info ./target/release/chronos &
# then live action + grim; screenshot is evidence, not a claim
```

Package name is **`chronos`** (`-p chronos`), not `chronos-app`.

## Gotchas

- Edition 2024: inline linters without edition flag lie; trust `cargo`.
- `gen` is a reserved keyword in edition 2024 — rename locals (OSD hide token).
- Float volumes → **no `Eq`** on types with `f64` (UPower trap, third hit).
- Shared files (`lib.rs`, `widgets/mod.rs`, `main.rs`): only your lines;
  parallel minions share them.
- Do not confuse with siblings: `Chronos-IDE` (Hermes/ACP), `chronos-fm`
  (`gpui-component`). Name overlap is not code overlap.

## Related skills

| Need | Skill |
|---|---|
| Session bootstrap / routing | `start-here` |
| Popup height / layer-shell resize | `gpui-layer-shell` |
| Generic GPUI API | `gpui` |
| Isolation for parallel work | `using-git-worktrees` (+ ChronOS sibling path rule above) |
| "Done" claims | `verification-before-completion` |

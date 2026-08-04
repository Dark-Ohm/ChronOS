---
name: chronos-shell
description: >
  Working on THIS repo — a Rust desktop shell for Hyprland/Niri built on a
  local gpui-ce fork (Source/gpui) with a sandboxed mlua/LuauJIT plugin
  system. Use when touching crates/app, crates/services, crates/luau,
  crates/ui, crates/plugins, bar/dock/launcher/ notifications/osd/tray_menu,
  the Service trait, any *Subscriber, or the Lua plugin hot-reload path.
  NOTE (2026-07-16): the repo migrated OFF Kael back to gpui-ce; any skill
  text mentioning Kael is historical and superseded by Cargo.toml.
---

# Chronos Shell

Canonical design: `docs/ARCHITECTURE.md` (accepted) + `docs/DECISIONS.log` (rejected +
why) at repo root. **Operational field state:** `docs/HANDOFF.md` — read first in
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
| `network` | NetworkManager (system) | zbus + retry (connectivity, **not** byte rates) |
| `net_stats` | `/sys/class/net` procfs | **not a `Service`** — pure sampling (`update_speed`, `SAMPLE_INTERVAL`); bar + future right-panel spectrum |
| `upower` | UPower (system) | battery + `has_battery` |
| `notification` | fdo Notifications (session) | server |
| `tray` | StatusNotifierWatcher (session) | + `tray/menu.rs` DBusMenu client |
| `audio` | `wpctl` poll 250ms + `pw-dump` on poll | **Set volume:** optimistic + `watch` latest-wins, **no** pw-dump on command path (T123). Per-app mute: `ToggleStreamMute` / streams if present |
| `brightness` | `ddcutil` on demand | **Set:** optimistic + **debounce ~150ms** then one `write_all(latest)`; no getvcp after set; Refresh generation-gated (T125 errata). MVP one slider → all displays |
| `applications` | `.desktop` scan + inotify | launcher data, **mpsc** debounce (not crossbeam), `strip_field_codes` in parser |
| `wallpaper` | awww MVP + multi-backend enum | 5 engines on host |
| `mpris` | session `org.mpris.MediaPlayer2.*` | ListNames + NameOwnerChanged |

`Services` / `init_all()` in `lib.rs` — **shared file**, add only your lines.

**Slow backends (drag/volume/brightness):** see
`references/slow-service-dispatch.md` and skill **`chronos-gpui-popup`**
(slider markers + track bounds + optimism). Dev loop: `docs/dev-cli.md`.


Commands are concrete methods (`dispatch`), **not** on the trait. Pure modules
like `net_stats` get **only** `pub mod` — never a field on `Services`.

### `crates/app` — shell UI

| Path | Role |
|---|---|
| `bar/` | layer-shell TOP strip; widgets via registry |
| `bar/widgets/` | clock, workspaces, battery, network, tray, **volume**, **mpris** |
| `osd/` | volume OSD overlay (soft-hide, no Exclusive keyboard) |
| `notifications/` | fdo popup stack — toast cards (`render_toast_card`: icon 28×28, app name, ✕ with on_click close, summary, body, actions, 2px progress bar colored by urgency; `first_seen` age tracking; 100ms tick loop capping at `MAX_TOAST_AGE`; `overlay_x` scroll offset for swipe-dismiss). T124 redesign: separated content from window manager — `mod.rs` owns `notif_list: Vec<LiveNotification>` + `remove_by_id` + hide-after-close; dedicated `view.rs` diverged from proxy view in `notification.rs`; `NotificationRenderer` trait aligns state-method signatures between fdo and libnotify types. |
| `launcher/` | app launcher — **uses `AppState::applications(cx)` via `state::watch`** (no more local cache); `launch.rs` re-uses `strip_field_codes` from services |
| `dock/` | pinned launch panel (not a live taskbar) — icon resolver + PINNED_IDS hardcoded. **As of 2026-07-17 NOT accepted** — `on_click` calls `window.remove_window()`, destroying the (persistent, bar-like) surface after the first click; see gotchas |
| `tray_menu/` | DBusMenu popup UI (paired with tray right-click) |
| `system_popup/` | anchored popup (brightness slider drag/buttons, power 3-segment, gaming toggle) — `AnchoredPopup` from gpui fork, `POPUP_WIDTH=360`, `BASE_HEIGHT=274`, offset(0,4), grab, LayerShell fallback; toggled via `Rc<Cell<Bounds>>` + canvas from bar/system widget |
| `ipc/` | single-instance Unix socket + wallpaper-next/set payloads |
| `wallpaper_ctl.rs` | IPC wallpaper-next / wallpaper-set — scan `~/Pictures/Wallpapers`, round-robin |
| `state.rs` | `AppState` global + `watch()` signal bridge |
| `plugin_bridge.rs` | Lua → `BarWidget` |
| `side_panel_right/` | right overlay skeleton (`da744a2`); meters/power still open tasks |

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
| battery / network / tray / volume / system | Right | volume: click mute, scroll ±5%; system: click → system_popup (brightness, power, gaming) |

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
0. **Nothing in `render()` may depend on HOW OFTEN it is called.** It runs
   many times per frame (measure/layout/paint) *and* on every watched
   service signal —
   cava alone pushes 30 fps. Anything that samples over time (rates,
   counters, deltas) must carry its own **time gate + cached value**, or
   it silently collapses: the network widget computed its delta between
   consecutive `render()` calls and showed `↓ 0` during a real 15 MB/s
   download (2026-07-20). Pattern that works —
   `chronos_services::net_stats::update_speed` (was private in
   `bar/widgets/network.rs` until `dbce8ac`): take `now: Instant` and
   `min_interval: Duration` **as parameters**, bail out returning the cache
   when `elapsed < min_interval`, divide by real `elapsed.as_secs_f64()`.
   Injecting time is what makes the "immune to call frequency" property
   unit-testable — the pre-fix version had green tests and was still
   broken live. Tests live in `net_stats` (5); bar keeps format/view tests.

   **Mutating state in `render()` is NOT itself the sin** — say it precisely,
   or the rule gets cargo-culted. `dock.rs` (`ICON_CACHE`), `tray.rs`
   (`PIXMAP_CACHE`, `ICON_CACHE`), `project_switcher` (`CONFIG_CACHE`) all
   mutate from render and are fine: a memoization cache is a pure function
   of its key, so call frequency cannot change the answer. The defect is
   **frequency-dependence** — a value derived from the interval between
   calls. Rate sampling for the network widget is **only** in `net_stats`
   now; bar `network.rs` only formats and paints.
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

### New anchored popup (copy volume/system, not invent)

**Canonical skill:** **`chronos-gpui-popup`** (skeleton, blur, anim boot,
slider blood facts). Lifecycle reentrancy: `references/popup-lifecycle-patterns.md`.
Slow audio/DDC: `references/slow-service-dispatch.md`.

0. **Own window** if the parent is the bar (bar re-renders constantly; a
   child div dies every frame). Kind is **`WindowKind::AnchoredPopup`**, not
   `Normal` and not a nested popover.
1. **Bar trigger:** `Rc<Cell<Bounds>>` + zero-opacity `canvas` in widget
   `render`; open via **`on_mouse_down(Left)`** (not `on_click` — grab
   handshake). Pass bounds + parent `window_handle` into `toggle`.
2. **Sliders (T123/T125):**
   - **One empty marker type per slider** in the window
     (`SinkVolumeSliderDrag` ≠ `SourceVolumeSliderDrag` ≠ `BrightnessSliderDrag`).
     Shared `DragMoveEvent<T>` → every listener of `T` moves together.
   - Frac from **measured track** bounds when the track is not full-width
     (brightness between ±); full-width volume may use PAD math.
   - Optimistic: `dispatched = Some(v)` **before** dispatch; clear in
     `render` only when `service.value == dispatched`.
   - ±: absolute `Set(next)` from `dispatched.unwrap_or(value)`, not `Step`
     after optimism (double-step).
   - Service side: latest-wins + debounce for DDC — UI throttle is not enough.
3. **`close_this`:** clear global handle **before** `remove_window` (ghost
   window otherwise). External close uses `close(cx)` via handle.update.
4. **Fallback:** catch `PopupNotSupportedError` → LayerShell TOP|RIGHT Overlay.
5. **Watchers** in `init` / view `new`, not `render`; on change: resize if
   needed + `view_cx.notify()`.
6. **Verify live** (release + grim): unit tests do not catch grab/anchor/clip.

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
- **Shared-file line contamination — FOUR incidents (OMP, Hermes, Autohand,
  Mimo).** A minion's `git add <own files>` sweeps up ANOTHER agent's
  uncommitted lines in a shared file (`main.rs`, `widgets/mod.rs`, `lib.rs`)
  because those lines were sitting unstaged in the same working tree. Worst
  case (Mimo dock, `d646406`): committed `mod tray_menu;` for a module that
  was never itself committed — broke `cargo build` on a clean checkout.
  `git diff --staged` alone doesn't catch this if you don't recognize the
  extra lines as not-yours — **check `git status` for OTHER modified files
  before you `git add`, and read every line of your own diff against what
  you actually wrote this session.**

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
- **`remove_window()` on a PERSISTENT surface (bar/dock) is a different bug
  flavor than the OSD popup race** (§ Layer-shell windowing above) — it's
  not a re-open race, it's calling `remove_window` from an ordinary click
  handler on a surface that's supposed to outlive the click (dock bug,
  2026-07-17). Reserve `remove_window` for actual transient popups
  (tray_menu, notifications) that are MEANT to close; a bar/dock window
  should never call it from inside its own content's event handlers.
- **RPIT capture (Rust 2024, `impl IntoElement` + `cx.listener`):** functions
  returning `impl IntoElement` that call `cx.listener` inside create borrow
  conflicts with subsequent RPIT calls in the same render. **Fix: create all
  `cx.listener()` closures BEFORE calling any RPIT function**, or use the
  `+ use<>` syntax on the return type signature. See `panel.rs:38-43` and the
  `build_sessions_sidebar` signature for the canonical pattern. Any new
  render function with listeners — same pattern.
- **gpui-component BLOCKER:** `gpui-component` depends on Zed's gpui
  (`gpui = { git = "https://github.com/zed-industries/zed" }`); ChronOS uses
  `gpui-ce` via `../Source/gpui` path dep. API incompatibility manifests as
  missing types (`AssetSource`, `Result`, `SharedString`). Resolution: either
  port TextInput into the gpui-ce fork, or update gpui-ce to Zed API parity
  (large undertaking). Homemade textarea fallback works for v1.
- **`relative()` NOT available for `line_height`** — our fork has
  `relative(f32)` for `DefiniteLength::Fraction` in layout geometry, but
  `line_height` does not accept it. **Use `px(f32)`** for line-height values.
- **`rgba(hex_string, alpha)` signature does NOT exist** in this fork.
  Use `rgba(0xRRGGBBAA)` with baked-in alpha bytes, or call `.opacity(f32)`
  on an `Rgba` value. See `crates/ui/src/theme/mod.rs::parse_hex` for
  string-hex → color conversion.
- **A sidebar-beside-content split needs `flex_row`, not `flex_col`.**
  `side_panel_left/panel.rs` had `sidebar`/`header`/`chat`/`composer` as
  direct siblings of one `.flex_col()` container — sidebar's `.h_full()`
  then competes for *vertical* space against the other three instead of
  sitting in its own column, so it comes up short of the panel bottom and
  everything else gets squeezed into a sliver (only surfaced once someone
  ran the EXPANDED sidebar state live — the collapsed 48px state hid it).
  Fix: `.flex_row()` with sidebar as one child and a `.flex_col()` wrapper
  (header+chat+composer) as the other, `min_w(0)` on that wrapper same as
  `main-content`'s fbcadd6 fix. Any panel with a fixed-width side rail next
  to a variable-height stack — check the parent's flex direction first.
- **`exclusive_zone` on a corner anchor (`LEFT|TOP`, not a stretched edge
  like the bar's `LEFT|RIGHT|TOP`) is silently a no-op without
  `exclusive_edge`.** wlr-layer-shell treats a two-edge anchor as
  ambiguous for which direction to reserve; `hyprctl monitors` shows
  `reserved` unchanged, no protocol error, easy to miss. Fix: also call
  (or set in `LayerShellOptions`) `exclusive_edge: Some(Anchor::LEFT)` —
  a single bit that `anchor.contains()`. `Window::set_exclusive_zone` /
  `set_exclusive_edge` (`gpui/src/window.rs:2005/2014`) are live-callable,
  not create-time-only, if you want it to track a resizing surface.
  (side_panel_left tried this for tiled-window reflow 2026-07-23, verified
  working via `hyprctl monitors`/`clients`, then reverted — see below.)
- **Hover-peek auto-open/close is a *choice*, not the only pattern for a
  layer-shell panel.** `side_panel_right`/`side_panel_left` both shipped
  with hover-strip peek (open on edge-hover, close on debounced leave,
  `hold_peek`/`schedule_release_peek`/`close_peek_if_not_pinned` — see
  `gpui-layer-shell` Part A's canonical files). For a panel someone keeps
  open *while working* (an agent chat, not a quick-glance popup), that
  auto-hide fights the user constantly. `side_panel_left` switched to
  keybind-only (`side_panel_left::toggle(cx)` via IPC, hover_strip's
  `init_hover_strip` call commented out, not deleted — the debounce
  machinery is correct code, just unused while hover is off). Same
  reasoning applies to `exclusive_zone`: fine for a bar that opens rarely,
  bad UX for a panel resized/toggled constantly — tried, reverted the
  same session (`docs/DECISIONS.log` 2026-07-23).
- **IPC toggle command pattern** (`crates/app/src/ipc/`): a new externally
  triggerable action mirrors `toggle-launcher` exactly — payload const +
  `encode_*`/`is_*` pair in `messages.rs`, a new `mpsc::unbounded_channel`
  threaded through `IpcSubscriber::start_listener`'s return tuple AND
  `accept_loop`'s params in `service.rs`, matched in the payload
  if/else-chain, and a `tokio::select!` arm with its own debounce timer in
  `ipc/mod.rs::start` calling into the target module. The external trigger
  itself is NOT a Rust caller — `encode_toggle_launcher()` exists only to
  keep the string constant tested; the real caller is a raw socket write
  from a Hyprland `hl.bind` (`~/.config/hypr/hyprland.lua`, `SUPER+L`
  pattern: `python3 -c "...s.sendall(b'toggle-launcher')..."`). The
  target function called from the IPC handler must take `&mut App` only
  (no `Window` — there isn't one in that context), same shape as
  `launcher::toggle(cx)`.
- **ACP `session/new` carries capabilities; `initialize` does not.**
  Hermes' ACP agent only returns `models`/`modes` (available list + current
  id) in the `session/new` response, not in the connection `initialize`
  handshake. If you fetch modes/models only after the first prompt
  (`send_prompt`'s response), a fresh unmessaged thread shows no model/mode
  UI at all. Fetch a session eagerly at connect time
  (`HermesClient::create_session()`) instead of waiting. Also: don't
  fully hide a picker/indicator just because its data is empty at a given
  moment — a disabled placeholder pill communicates "this exists, not
  loaded yet" where a hidden element reads as "this feature doesn't
  exist." Verified via `RUST_LOG=debug` reading the raw
  `agent_client_protocol::jsonrpc` response, not assumed from a design
  convention borrowed from another product.

## Theming (2026-07-20 — two schemes now exist)

Source of truth: `crates/ui/src/theme/`. Scheme selection lives in
`crates/app/src/theme_config.rs`: **`CHRONOS_THEME` env → `~/.config/
chronos/theme.toml` → dark default**, with inotify hot-reload (watch the
*parent dir* — inotify on a not-yet-existing file fails; debounce 300 ms;
apply with `cx.set_global(theme)` + `cx.refresh_windows()`).

- **`cx.set_global`, never `Theme::set`, for the first apply.**
  `Theme::set` is `*cx.global_mut::<Theme>()` and panics while the global
  does not exist yet (`no state of type Theme exists`) — it cold-start
  crashed the shell once.
- **Content ON a saturated fill → `chronos_ui::on_fill(fill)`**, never
  `theme.text.*`. Text tokens flip with the scheme; fills (`accent.*`,
  `status.*`) do not, so a "primary text" foreground breaks in exactly one
  of the two schemes. Live cases: badge digit on `status.error`, toggle
  knob on its track.
- **`status.*` differ per scheme by design:** dark = Catppuccin Mocha,
  light = **Latte**. Pastel Mocha as *text* on a light surface is
  unreadable (`↑ 19` vanished on the light bar). Do not "unify" them —
  `light_scheme_status_is_latte_not_mocha` guards this.
- Verify any theme work in **both** schemes; a light-only fix that shifts
  a dark pixel is a regression.

## Popup height budgets (2026-07-20)

The hard clip (`max_h` + `overflow_hidden` on the list, chrome outside it)
protects against a long **list** — it does *not* protect against a
**fattened footer/header**. Window height comes from
`estimate_popup_height(count)`, which knows only the row count, while the
footer gets a fixed `FOOTER_BUDGET_H`. Adding a status line to the footer
of `updates_popup` left ~2 px of slack in a 64 px budget — on the very
popup whose documented history is "pixel estimates render taller than
guessed" and whose button once left the screen entirely. **Touching a
popup's header/footer content ⇒ re-check `*_BUDGET_H` and whether
`estimate_popup_height` accounts for it.**

## Compositor events (2026-07-20)

Subscribing to *"the active thing changed"* is not subscribing to *"the
list changed"*. `add_workspace_changed_handler` only re-flagged `active`
over a list snapshotted once at startup, so workspaces created later never
appeared as dots, emptied ones never left — and switching to a
post-startup workspace lit *no* dot at all (its id wasn't in the list).
Fix: `refresh_workspaces()` re-reads `Workspaces::get()` on **all three**
events (changed / added / deleted).

**Handler names in the `hyprland` crate are generated by the `events!`
macro from enum variants** (`WorkspaceAdded` → `add_workspace_added_handler`),
so grepping the crate for `pub fn add_` finds only one method and lies.
Read the variant list in `event_listener/shared.rs` instead.

## Tray hygiene (2026-07-20)

Chromium/Vivaldi registers a fresh `StatusNotifierItem` per event and
never unregisters while alive — 13 anonymous items (`icon=None`,
`title=""`) accumulated and ate the right cluster as identical fallback
glyphs. Our `remove_item` only fires when the whole bus name vanishes.
Defence lives in the **widget**, not the service (the service keeps the
bus truth): filter unidentifiable items → dedupe by bus owner → cap at
`MAX_TRAY_ITEMS` with a `+N` badge.

## Smoke methodology traps (2026-07-20)

- **Transient load is not a smoke.** Screenshotting a 142 MB download 5 s
  in proves nothing — it had already finished, and the widget honestly
  read ~0. Use **sustained** load (`curl --limit-rate 5M`) and measure the
  counters over **the same window as the screenshot**, then compare
  magnitudes. (The Architect got this wrong first and rejected a widget on
  non-evidence; the code diagnosis happened to be right anyway.)
- Some conditions cannot be recreated on demand (the tray palisade
  disappears when the offending app restarts). Say "covered by unit test
  only, live unconfirmed" — do not upgrade a half-proof to a full one.

## IPC: программное управление панелями (2026-08-04, T226/T230)

Три новых IPC-команды для автоматизации live-smoke без мыши:

| Команда | Принимающая функция | Что делает |
|---------|-------------------|------------|
| `expand-left` | `side_panel_left::expand_with_composer(cx)` | Открывает левую панель, докает чат, фокусирует композер |
| `select-tab:<id>` | `side_panel_right::select_tab(tab, cx)` | Переключает правую панель на вкладку, деферит фокус 50ms |
| `preview-target:<path>` | `side_panel_right::preview_target(path, cx)` | Устанавливает `PreviewTarget` global, `PreviewIntent::Edit` |

**Паттерн добавления новой IPC-команды** (ровно по `toggle-launcher`):
1. `messages.rs`: константа префикса/пейлоада + `classify_*`/`is_*`/`encode_*` + тесты
2. `service.rs`: `mpsc::unbounded_channel` + новый тип ресивера + поле в `IpcSubscriber::start_listener` + `accept_loop`
3. `ipc/mod.rs`: `tokio::select!` arm с дебаунс-таймером + вызов целевой функции (только `&mut App`, без `Window`)

**Дебаунс:** 100ms для `select-tab` (быстрые переключения), 200ms для `expand-left` (дорогой compose-фокус).

**Фокус клавиатуры через IPC:** layer-shell окна GPUI не получают фокус от синтетических кликов (`ydotool`/`wtype`). Для каждого таба, который должен принимать клавиатурный ввод, нужен `FocusHandle`:
- `TerminalTab` → `Focusable::focus_handle()` (терминал сам реализует `Focusable`)
- `PreviewTab` → `editor_focus_handle()` → `InputState::focus_handle()` (editor создаётся лениво в Edit mode)
- `select_tab()` деферит фокус на 50ms через `cx.spawn` + `background_executor().timer()` — этого достаточно для первого рендера и материализации editor'а
- `preview_target()` использует `PreviewIntent::Edit` чтобы файл открылся сразу в Edit mode (view-only не создаёт `InputState`)

## Related skills

| Need | Skill |
|---|---|
| Session bootstrap / routing | `start-here` |
| Bar-anchored popups, sliders, blur, anim boot | **`chronos-gpui-popup`** |
| Popup height / layer-shell resize | `gpui-layer-shell` |
| Generic GPUI API | `gpui` |
| Isolation for parallel work | `using-git-worktrees` (+ ChronOS sibling path rule above) |
| "Done" claims | `verification-before-completion` |

## References (recovered 2026-07-21 from hermes profile — deep-dive files)

`references/` — symbol-level and pattern-level audits; open instead of re-reading source:

| File | What |
|---|---|
| `crates-app-api-surface.md` | Exhaustive pub-symbol audit of `crates/app` (line ranges, tests, startup sequence) |
| `bar-widget-contract.md` | Live `mod X; X::register(cx)` widget registration contract + isolation verify |
| `compositor-lua-dispatch.md` | Compositor→Lua event dispatch path |
| `popup-lifecycle-patterns.md`, `tray-menu-popup-patterns.md`, `tray-widget-patterns.md`, `notifications-module-patterns.md` | Surface lifecycle patterns per module |
| **`slow-service-dispatch.md`** | Audio/brightness drag: optimistic + latest-wins + DDC debounce (T123/T125) |
| `gpui-fork-api-surface.md`, `gpui-shell-donor-audit.md`, `donor-crate-port-cost-audit.md`, `kael-patches.md` | Fork/donor audits (Kael content historical) |
| `live-smoke-wayland.md` | Live smoke procedure (hyprctl/grim evidence) |
| `wallpaper-awww-service.md`, `zbus-server-5.17.md`, `hindsight-llama-infra.md`, `doc-audit-discrepancies.md` | Service-level notes |

# Chronos — Decisions Log

Append-only. Each entry: what was considered, what was rejected and why, what
was decided. Full rationale for the currently-approved architecture lives in
`ARCHITECTURE.md`; this file is the history, including things ARCHITECTURE.md
no longer needs to spell out because they're settled.

---

## 2026-07-08 — GPUI source: gpui-ce

- Considered: `zed/main` (upstream Zed GPUI), `crates.io` (`gpui` + `gpui-component`),
  local `gpui-ce` fork.
- Rejected `zed/main`: Zed issue #48501 (layer-shell window ignores target
  monitor) is still open there — breaks multi-monitor bar placement.
- Rejected `crates.io`: `gpui-ce` published there is ~6 months stale (v0.3.3,
  no Wayland fixes); `gpui-component` there (v0.5.1) requires `gpui ^0.2.2`,
  10 versions behind the local checkout (`0.58.0`). The two would not form a
  consistent pair.
- Decided: local `gpui-ce` checkout, pinned rev `20340e14874a3b55122e5cb2aa0d023874e08b2d`
  (2026-07-06). Verified fix in source at
  `gpui-ce-main/crates/gpui_linux/src/linux/wayland/client.rs:827-834`. Also
  gained `input_region` + `exclusive_zone` (PR #82), required for dock/bar.
  Path-dep now, migrate to `git = "...", rev = "..."` once gpui-ce-main is a
  git repo. Upstream sync on-demand only, not a maintained relationship.

## 2026-07-08 — gpui-component: fork, not dependency

- Considered: depend on `gpui-component` as-is (crates.io or git), fork and
  trim into `crates/ui`.
- Rejected as-is: pulls tree-sitter + all grammars, WebView (wry), Markdown —
  none needed, all add build weight and attack surface for a shell that
  doesn't need a text editor or embedded browser.
- Decided: fork `longbridge/gpui-component` at `49d1bef84cb374c42d82b2e8d7e8b0d685d9ed48`
  into `crates/ui`, strip tree-sitter/wry/Markdown, keep Button/Input/List/Slider/Switch,
  rewrite internal `gpui` dep to point at gpui-ce. Upstream not tracked, sync
  only on demand.

## 2026-07-08 — Module registry: runtime, not static enum

- Considered: gpui-shell's pattern (`enum Widget` + `match` in `registry.rs`,
  `all_views()` for launcher), a runtime `HashMap`/`Vec` registry.
- Rejected static enum: adding a module means editing core and recompiling —
  directly conflicts with the plugin-without-core-rebuild requirement.
- Decided: `HashMap<String, Box<dyn BarWidget>>` + `Vec<Box<dyn LauncherView>>`,
  populated at runtime via `chronos.bar:register(...)` from LuaU. Traits made
  object-safe (`dyn`).

## 2026-07-08 — Panic strategy: unwind, not abort

- Considered: gpui-shell's `panic = "abort"`, switch to `panic = "unwind"`.
- Rejected abort: an `unwrap()` in a D-Bus listener thread or a plugin VM
  callback would kill the entire shell — unacceptable for a long-running
  desktop process with untrusted plugin code in the loop.
- Decided: `panic = "unwind"`. Services and plugin host still expected to use
  `Result`/`expect` rigorously — unwind is a backstop, not a substitute for
  error handling.

## 2026-07-08 — Frame/LuaU budget: 144 FPS, not 60 FPS

- Considered: an earlier draft's "16.7 ms / 60 fps" call budget for LuaU
  event callbacks.
- Rejected: target hardware (RTX 3070) and stated goal are 144 FPS; 60 fps
  budget was leftover from an earlier, less specific draft and doesn't match
  the actual target refresh rate.
- Decided: synchronous LuaU call budget < 4 ms, against a 144 Hz frame budget
  of 6.94 ms. LuaU never runs in the render path regardless — only on events
  (workspace/focus/tick) and config load.

## 2026-07-08 — LuaU vs classic Lua: recommended, not mandated

- Considered: mandate Luau only, allow classic Lua + Luau both.
- Context: initial phrasing (`AGENTS.md`, early spec draft) was ambiguous
  about whether "LuaU" meant the plugin layer strictly required Roblox's
  typed Luau dialect, or just used it as the primary/expected language.
  Commits `a390280` and `c4a0ea4` clarified this after review.
- Decided: `mlua` accepts both classic Lua and Luau with no code-level
  restriction — nothing stops a plugin author using either. Luau is the
  recommended default (type checking at load time is an extra shield at the
  plugin boundary, catches more errors before the VM runs), but it is not
  enforced or mandated.

## 2026-07-08 — Runtime split: tokio (services) + GPUI executor (UI)

- Considered: single tokio runtime driving everything including UI, GPUI
  executor only, two separate runtimes bridged via signals.
- Rejected single-runtime-for-everything: GPUI's executor model expects to
  own the UI thread; mixing tokio driving UI code risks executor conflicts
  and defeats GPUI's own scheduling.
- Decided: GPUI main thread owns UI futures via `cx.spawn()` (single-threaded,
  no tokio). A dedicated multi-thread `tokio::runtime::Runtime` on its own OS
  thread runs all D-Bus (zbus), Hyprland/Niri IPC, upower, network, bluetooth.
  Bridge is `futures_signals::Mutable` + `watch()` (gpui-shell `state.rs:143-164`),
  not callbacks. Matches gpui-shell's existing pattern.

## 2026-07-08 — Scope cuts (YAGNI for MVP)

- Considered vs. rejected for MVP:
  - Niri-first support — rejected, Hyprland is the stated primary target;
    Niri backend stays incomplete like in gpui-shell, acceptable for now.
  - Plugin marketplace / signing — rejected, no distribution story needed yet.
  - Remote/network plugin loading — rejected, local files only; no reason to
    open a network attack surface for plugin loading at this stage.
  - Custom shaders (`runtime_shaders`) — rejected, not needed to hit the
    144 FPS / visual-polish goals for MVP.
- Decided: all four explicitly out of scope, revisit only if a concrete need
  appears post-MVP.

## 2026-07-08 — gpui-ce Linux bug: quit() before any window hangs forever

- Found while building the Workspace + Bar MVP smoke test (Task 1): calling
  `cx.quit()` from `on_finish_launching` with zero windows opened hung the
  process forever on Linux (Wayland and X11; both share `LinuxPlatform`).
- Root cause (verified in gpui-ce source): `LinuxPlatform::run()` called
  `on_finish_launching()` synchronously *before* starting the calloop event
  loop. `calloop::EventLoop::run()` unconditionally resets its internal stop
  flag to `false` on entry, so the early `cx.quit()` -> `signal.stop()` call
  was wiped before the loop ever saw it. `quit()` also never called
  `signal.wakeup()`, so even a later `stop()` couldn't interrupt an
  indefinite `poll(None)`.
- Considered: work around it in application code (never call `cx.quit()`
  with zero windows, let the process run forever and rely on `kill`); patch
  `gpui-ce` directly.
- Rejected the app-code workaround: real fix was well-scoped once traced
  (each backend already stores a calloop `LoopHandle` — no new plumbing
  needed at the field level), and leaving it broken would resurface the
  moment any future graceful-shutdown path calls `cx.quit()` while the loop
  is idle.
- Decided: patched `gpui-ce` directly at
  `/home/neo/Projects/SOURCE/gpui/gpui-ce-main` (commit `6a7b386`, on top of
  baseline commit `352c9f2` — this checkout had no git history before, so a
  git repo was initialized there first as a rollback safety net). Fix: new
  `LinuxClient::insert_idle()` (implemented in all three backends —
  wayland, x11, headless — via their existing stored `LoopHandle`) defers
  `on_finish_launching` to run from inside the loop's first idle-dispatch
  cycle instead of before the loop starts, plus an explicit
  `signal.wakeup()` in both `run()` (to unblock the first `poll()`) and
  `quit()` (so a later `stop()` while the loop is genuinely idle isn't
  stranded either). Verified: `app.run(|cx| cx.quit())` with no window
  opened now exits with code 0 instead of hanging.
- Scope note: macOS/Windows platform backends are untouched (not a target
  for this project) and were not affected by this bug — they defer or queue
  `on_finish_launching` differently and don't share calloop's reset-on-entry
  semantics.

## 2026-07-08 — IPC `/tmp` fallback: per-user id, not PID

- Found during the final whole-branch review of the Workspace + Bar MVP
  plan: `ipc/service.rs`'s `socket_path_in(None)` (used only when
  `$XDG_RUNTIME_DIR` is unset) embedded `std::process::id()` in the socket
  path. Every process has a different PID, so the fallback path differs on
  every invocation — a second instance could never find the first's socket,
  silently defeating single-instance detection in that branch.
- Root cause: transcription error while adapting `gpui-shell`'s
  `ipc/service.rs` for this plan. The reference implementation used a
  per-*user* identifier (`UID` / `SUDO_UID` / `USER` env vars) for exactly
  this reason — stable across separate invocations by the same user, unlike
  a PID. The plan text substituted `std::process::id()` by mistake.
- Considered: fix now vs. leave as tracked backlog (the bug is dormant —
  Hyprland under a systemd user session always sets `XDG_RUNTIME_DIR`, so
  the buggy branch is not reachable in normal operation).
- Decided: fix now. Correctness bugs in code paths meant to provide
  single-instance guarantees shouldn't ship dormant, even if today's target
  platform happens not to trigger them — a future headless/container/minimal
  environment could unset `XDG_RUNTIME_DIR` and hit this silently. Replaced
  with the same `UID`/`SUDO_UID`/`USER` fallback chain as the `gpui-shell`
  reference. Plan doc corrected to match.

## 2026-07-09 — Bar registry: Vec, not HashMap

- Context: ARCHITECTURE.md §6 (written 2026-07-08) described the Rust side as
  `HashMap<String, Box<dyn BarWidget>>`. The bar scaffold implemented on
  2026-07-09 ships `Vec<Box<dyn BarWidget>>` instead — a deliberate,
  plan-recorded deviation (see the bar scaffold spec/plan: Global Constraints
  "Reconciliation note").
- Considered: keep `HashMap` (name-keyed, supports replacing a widget by key),
  use `Vec` (order-preserving, append-only).
- Rejected `HashMap` for the scaffold: widget ORDER within a bar section
  (left→right) is layout-significant and `HashMap` iteration order is
  unspecified — it would scramble widget layout. Name-keyed replacement isn't
  needed at scaffold stage (no widgets exist to replace yet).
- Decided: `Vec<Box<dyn BarWidget>>` as the registry backing store, registered
  globally and read by `Bar::render` to lay widgets into Left/Center/Right
  sections in registration order. If named-widget replacement becomes a
  requirement later, revisit (a `HashMap<String, …>` keyed by a `name()`
  method, or a position index, layered on top of the `Vec`).
- Reconciled into ARCHITECTURE.md §6 so the canonical doc matches the code
  (per AGENTS.md: canonical doc wins; deviations must be recorded there, not
  only in plan/spec).

## 2026-07-09 — Luau plugin layer: three architectural fixes

During the luau-plugin-layer implementation, cross-doc review found three
issues in the plan/spec that contradicted existing code or DECISIONS.log:

1. **BarWidgetRegistry hot-reload support.** The Vec registry had only
   `register()` (push). Hot-reload requires replacing old widgets by name.
   - Considered: add `unregister(name)` + `replace(name, widget)` to the Vec,
     or switch to `HashMap` for native key-based operations.
   - Rejected `HashMap`: widget ORDER within sections is layout-significant
     (already decided 2026-07-09). Switching back would scramble layout.
   - Decided: add `fn name(&self) -> &str` to `BarWidget` trait (default:
     `"unnamed"`), add `replace_by_name(name, widget)` and
     `unregister_by_name(name)` to `BarWidgetRegistry`. These do linear scan
     on the Vec — acceptable because widget count per section is small (<20)
     and registration happens at startup/reload, not per-frame.

2. **`chronos.log` API: table with methods, not flat function.** Spec §7
   defined `chronos.log(msg)` (flat function call), but the implementation
   and example plugin both used `chronos.log.info(msg)` / `chronos.log.warn(msg)`.
   - Decided: table-with-methods is more idiomatic Lua and extensible. Spec §7
     corrected to match. `api/log.rs` returns a table with `.info` and `.warn`
     methods.

3. **Tick timer via GPUI executor, not tokio.** The plan specified
   `tokio::spawn` for the 1-second tick timer. DECISIONS.log (2026-07-08)
   explicitly decided: GPUI main thread owns UI futures via `cx.spawn()`;
   tokio is for services only.
   - Decided: tick timer uses `cx.spawn()` with
     `cx.background_executor().timer(Duration::from_secs(1))` in a loop,
     matching the existing pattern in `bar::init()`. `PluginManager::start_tick_loop(&self, cx: &mut App)` is the public API.

## 2026-07-09 — Luau plugin layer: implementation

Implemented `crates/luau` — the LuaU plugin runtime for Chronos:
- `capabilities.rs`: TOML manifest parsing, capability gates, unsafe=TOFU
- `dsl.rs`: Element DSL (text/row/column) with Lua→Rust deserialize and
  `into_any_element()` GPUI conversion
- `sandbox.rs`: per-plugin VM creation, global stripping, chronos.* API registration
- `api/`: bar (widget registration), time (epoch), log (table with methods),
  events (callback store + dispatch); capability stubs (fs/process/net/ipc)
- `manager.rs`: PluginManager — discovery, load, tick dispatch via GPUI executor
- `plugin_bridge.rs` (in `crates/app`): LuaWidgetAdapter implementing BarWidget,
  wires PluginManager into Chronos startup
- `crates/plugins/clock/`: example plugin rendering time in bar left section

Package name: `chronos-luau`. 17 unit tests passing.

## 2026-07-09 — PluginManager ownership: Global, not raw pointer

- Found during post-merge review of the luau plugin layer: `start_tick_loop(&self,
  cx)` took `self as *const Self` (raw pointer) and passed it to a detached async
  task via `cx.spawn(...).detach()`. The `PluginManager` was a stack-local variable
  inside `app.run`'s closure — dropped when the closure returned, while the detached
  task continued running. Result: use-after-free on every tick (1 Hz deref of a
  dangling pointer).
- SAFETY comment ("manager lives for the app lifetime") was factually wrong —
  `plugin_manager` lived on the closure's stack, not as a static or leaked
  allocation.
- Considered: `Box::leak(self)` to get `&'static Self`. Rejected: gives only shared
  access permanently — future inotify watcher needs `&mut self` for reload (drop old
  VM → recreate → `replace_by_name`). Would require reverting the signature again
  within one task. Also introduces an unnecessary leak when GPUI's `Global` already
  provides `'static`-equivalent lifetime through the `App` itself.
- Considered: `Rc<RefCell<PluginManager>>` shared to the async task. Rejected:
  creates a second ownership pattern alongside the existing `cx.global()` pattern
  used by `BarWidgetRegistry`. One sharing mechanism for the whole project, not two.
- Decided: `impl gpui::Global for PluginManager {}` + `cx.set_global(plugin_manager)`
  in `main.rs`. `start_tick_loop` becomes an associated function
  `start_tick_loop(cx: &mut App)` that reads state via
  `cx.global::<PluginManager>().dispatch_tick()`. Future watcher reload will use
  `cx.update_global::<PluginManager, _>(|mgr| mgr.reload(...))` — same pattern,
  no new ownership mechanics.
- `Global` requires `'static`; `mlua::Lua` with `send` feature is `Send + Sync +
  'static` — no conflict. Verified by build: 0 errors, 29 tests pass.
- Re-verified 2026-07-09 in the feat-inotify-hot-reload worktree: `grep -rn "as
  \*const\|as \*mut" crates/` returns no matches; `impl gpui::Global for
  PluginManager {}` present (`crates/luau/src/manager.rs:10`); `start_tick_loop`
  reads via `cx.global::<PluginManager>()`. The fix landed before this branch
  forked, confirmed by source, not assumed from this log entry.

## 2026-07-09 — BarWidget types moved to crates/luau

- Context: Hot-reload watcher in `crates/luau` needs `BarWidgetRegistry` access
  via `cx.update_global` to call `replace_by_name`/`unregister_by_name`. Moving
  the types to `crates/luau` avoids circular dependency (`app` → `luau` → `app`).
- `crates/luau` already depends on `gpui` (workspace), so `AnyElement`, `Window`,
  `App` are available. `LuaWidgetAdapter` also moved to `crates/luau/src/dsl.rs`
  since it implements `BarWidget` (now local) and uses `dsl::Element`.
- `crates/app/src/bar/mod.rs` re-exports types from `chronos_luau::bar` for
  backward compatibility — no import changes needed in `bar/mod.rs` itself.

## 2026-07-09 — Inotify hot-reload: design decisions

Three design iterations during spec refinement, each rejected alternative documented
(per explicit request: record rejected alternatives as a standalone entry before the
agent runs, not after). Rejected alternatives that were considered and why they were
dropped:

1. **Debounce: 300ms, not 150ms.**
   - Considered: 150ms ("shell plugins are simple, fast response").
   - Rejected: write-to-temp+rename in editors emits multiple filesystem events per
     save regardless of editor. 150ms is too tight to coalesce all chunks of a single
     `:w`; risk of reading a partial file or reloading mid-write. Defaulting to the
     proven 300ms.
   - Decided: 300ms debounce, coalesce all events within the window.

2. **Events: CLOSE_WRITE | MOVED_TO | CREATE | DELETE, not MODIFY.**
   - Considered: MODIFY (simplest, catches all changes).
   - Rejected: MODIFY fires on every `write()` syscall — a single `:w` emits multiple
     MODIFY events (one per write chunk), increasing debounce noise and partial-read
     risk. CLOSE_WRITE fires exactly once when the handle closes after writing — the
     correct "file is ready" signal. MOVED_TO catches atomic temp+rename replace.
   - Decided: CLOSE_WRITE | MOVED_TO | CREATE | DELETE.

3. **Recursive watch: add() + immediate poll, not add() alone.**
   - Considered: just `watches().add()` on new subdirectories.
   - Rejected: race window between the CREATE event for a directory and the
     `watches().add()` call. Fast sequences (`mkdir plugin && cp manifest.toml
     init.luau plugin/`) can finish both copies before the watch registers — their
     events fire against an un-watched dir, so the plugin never loads.
   - Decided: `watches().add()` followed immediately by synchronous `read_dir()` +
     `load_one()` if files are already present. Closes the race even if events missed.

Note: the architectural question of whether `reload` can reach `BarWidgetRegistry`
was also settled here — `BarWidget`/`BarWidgetRegistry` were moved into `crates/luau`
(commit 8276616) so `reload(name, cx)` calls
`cx.global_mut::<BarWidgetRegistry>().replace_by_name/unregister_by_name` directly,
with no deferred "watcher is responsible" hand-off. GPUI's `update_global` leases the
PluginManager global then releases it before the closure returns, so a
`global_mut::<BarWidgetRegistry>()` call inside it does not conflict.

## 2026-07-09 — Inotify watcher: dedicated OS thread, not GPUI foreground executor

- Found via live smoke test (not caught by any unit test): the watcher task,
  spawned with `cx.spawn(async move |cx| { ... inotify.read_events(&mut buf) ... })`,
  died within ~130ms of every single startup and never watched anything again
  afterward. Unit tests never caught this because all of them call `mgr.reload()`
  or `cx.update_global(...)` directly, bypassing the watcher's actual read loop
  entirely.
- Root cause: `Inotify::init()` always sets `IN_NONBLOCK` on the underlying fd —
  this is not optional, per the `inotify` crate's own design (confirmed against
  docs.rs, not assumed). `read_events()` is the non-blocking variant; it returns
  `io::ErrorKind::WouldBlock` when no events are ready, which is near-guaranteed
  on the very first loop iteration since the watch was just registered. The code
  treated every `Err` from `read_events`, including `WouldBlock`, as fatal
  (`break`, comment said "Non-recoverable") — so the watcher died almost
  immediately, silently, every time.
- Considered: swap to `read_events_blocking()` in place, still inside the
  existing `cx.spawn`. Rejected: `App::spawn`/`cx.spawn` runs its future on
  GPUI's *foreground* executor — the main UI thread (confirmed by reading the
  vendored gpui-ce source, `app.rs:1739`, doc comment: "Spawns the future...
  on the main thread"). A genuinely blocking call there would freeze the
  entire UI between file events — a new bug of the same species as the
  raw-pointer and tokio-driving-UI bugs already recorded above, not a fix.
- Decided: the blocking inotify read loop (including all event
  interpretation) now runs on its own `std::thread::spawn` OS thread, which
  owns the `Inotify` instance directly and forwards affected-directory
  batches through a `tokio::sync::mpsc` channel. A separate `cx.spawn` task on
  the GPUI foreground executor `.await`s that channel (a non-blocking yield,
  not a blocking call) and applies reloads via `cx.update_global` — the same
  channel-bridge shape already used by `crates/app/src/ipc/service.rs` +
  `ipc/mod.rs` for exactly this "blocking OS work → GPUI foreground" need.
  `crates/luau` gained `tokio` as a dependency (workspace already pins it;
  `sync`/`time`/`macros` features) to reuse that established pattern instead
  of introducing a second channel primitive.
- Verified by execution, not just by reading: live run showed the
  `WouldBlock` error exactly once at startup before the fix, and zero after;
  a `clock` plugin edit produced a `Hot-reloaded plugin: clock` log line
  after the fix, none before (the watcher was already dead by the time of
  the edit in the unfixed version).

## 2026-07-09 — Inotify watcher: trailing debounce, not send-on-first-read

- Found via the same live smoke test, immediately after the fix above: a
  single `sed -i` edit to `clock/init.luau` produced **two**
  `Hot-reloaded plugin: clock` log lines ~300ms apart, not one.
- Root cause: the debounce loop sent on the *first* `read_events_blocking()`
  batch, then slept 300ms, then read again — it only coalesced events that
  landed in the *same* kernel read, not events split across two reads that
  straddle the sleep boundary. `sed -i`'s temp-write-then-rename save
  produces events in two waves close enough together that they sometimes
  land in separate reads, so the "one `:w` = one reload" claim in this log's
  earlier rejected-alternatives entry (300ms debounce, entry above) did not
  fully hold as implemented, even though the debounce *interval* chosen was
  right.
- Decided: trailing debounce instead of send-immediately. The OS thread
  forwards every read's raw affected-dir batch, undebounced, through the
  channel. The GPUI-foreground consumer task holds a `pending: HashSet<PathBuf>`
  and a `deadline: Option<Instant>` that *resets* on every new batch
  (`tokio::select!` between `rx.recv()` and `tokio::time::sleep_until(deadline)`)
  and only flushes `pending` into `reload()` calls 300ms after the *last*
  batch, not the first. This is what "debounce" is supposed to mean and what
  the original design intended — the earlier implementation approximated it
  by accident (send-then-sleep looks similar but coalesces only within one
  read, not across the window).
- Verified by execution: after the fix, the same `sed -i` edit produces
  exactly one `Hot-reloaded plugin: clock` line, confirmed twice independently
  (`grep -c "Hot-reloaded plugin: clock"` = 1 both times).

## 2026-07-09 — Inotify watcher: match events by WatchDescriptor, not basename-join

- Found via the same live smoke test's race-condition check (Task 4 Step 4):
  copying a second plugin (`test-race-plugin/`, containing files named
  `manifest.toml`/`init.luau` — the same basenames every plugin has, by fixed
  convention) into the plugins directory while `clock` was already loaded and
  untouched spuriously triggered `Hot-reloaded plugin: clock` too.
- Root cause: the event-to-directory resolution loop joined the event's
  basename (`event.name`, e.g. `"init.luau"`) against **every** currently
  watched directory and checked filesystem existence to decide which one(s)
  matched — it never consulted `event.wd`, the watch descriptor inotify
  provides that identifies exactly which watched directory the event fired
  on. `inotify.watches().add(dir, WATCH_MASK)?` discarded its return value;
  `watched_dirs` was a bare `HashSet<PathBuf>`. Since every plugin has a file
  literally named `init.luau`, any event on `test-race-plugin/init.luau` also
  matched `clock/init.luau` on disk and queued `clock` for a spurious reload —
  dropping and recreating its Lua VM (losing its runtime state) for no reason.
  This is not an edge case: it fires on every multi-plugin install without
  exception, any time any one plugin's files change.
- Decided: `watched` is now `HashMap<WatchDescriptor, PathBuf>`, keyed by the
  descriptor `inotify.watches().add()` actually returns. Event resolution
  looks up `watched.get(&event.wd)` to get the one true directory the event
  fired on, then joins the basename against only that directory — no
  filesystem-existence-based guessing across every watched path.
- Verified by execution: after the fix, copying `test-race-plugin/` alongside
  an already-loaded, untouched `clock` no longer produces any
  `Hot-reloaded plugin: clock` line (confirmed twice independently via
  `grep -c`, staying at whatever count preceded the copy).

## 2026-07-09 — Plugin identity: match by directory path, not by name string

- Found via the same live race-condition test, after the two watcher.rs
  fixes above stopped masking it: `WARN reregister_widgets: plugin
  test-race-plugin not found`, and separately (via a synthetic test, not the
  live run, to avoid the panic) an `unwrap() on None` panic in
  `crates/app/src/plugin_bridge.rs:27`. Both are the same underlying defect,
  in two different files, not two coincidentally similar bugs:
  - `crates/luau/src/manager.rs`'s `reload()`/`unregister_plugin()`/
    `reregister_widgets()` derived their lookup key from the plugin
    *directory's basename* (`plugin_dir.file_name()`), while `PluginHandle.name`
    is always the *manifest*'s `[plugin] name` field. Nothing in the manifest
    format or the design spec ever required these to match (a directory name
    is a deployment detail; the manifest name is Lua-authored plugin
    identity) — they only happened to coincide in every plugin fixture tested
    so far (`clock` dir + `name = "clock"`).
  — they only happened to coincide in every plugin fixture tested so far (`clock` dir + `name = "clock"`).
    - `crates/app/src/plugin_bridge.rs`'s `register_plugin_widgets()` made the
      same category of mistake one level down: it re-derived "which plugin owns
      this widget" by matching the *widget's* registered name
      (`chronos.bar:register({name=...})`) against `PluginHandle.name` — but a
      widget's name and its owning plugin's manifest name are different,
      unrelated namespaces. They only happened to coincide for the one shipped
      example plugin (`clock`'s widget is also named `"clock"`), which is
      exactly what let this ship unnoticed until a plugin with a differently-
      named widget was tested live.
  - Rejected: patching `plugin_bridge.rs`'s lookup to match by directory path
    too (mirroring the `manager.rs` fix). `get_registered_widgets()` already
    iterates each owning `PluginHandle` internally to build its result — it can
    hand the caller the owning `Lua` handle directly with no lookup at all,
    which is strictly more correct than any name- or path-based re-search
    performed after the fact.
  - Decided (one fix for the whole defect class, not two patches): `PluginHandle.path`
    is the single lookup key across `reload()`, `unregister_plugin()`, and
    `reregister_widgets()` — the only field guaranteed unique (the filesystem
    cannot have two directories at the same path) and independent of anyone's
    discipline filling in `manifest.toml`. `PluginHandle.name` (manifest-derived)
    remains for logs/display only, never for lookups. `get_registered_widgets()`
    now returns `(Lua, widget_name, section, spec)` tuples instead of
    `(widget_name, section, spec)`, so `plugin_bridge.rs` needs no lookup at all.
    Pre-flight swept for a third occurrence of the same pattern before fixing
    (`grep -rn "\.name ==" crates/luau crates/app`) — confirmed exactly these
    four call sites, no other location.
  - Also found and fixed in the same pass, unrelated to plugin identity but
    discovered while touching this code: `crates/luau/Cargo.toml` had
    `gpui`'s `test-support` feature under `[dependencies]` instead of
    `[dev-dependencies]` (added in this same branch's Task 1 commit,
    undocumented, no `DECISIONS.log` entry, no design-doc discussion — an
    unexamined pattern, not a conscious choice). Since `crates/luau` is a plain
    library linked into `crates/app`'s release `chronos` binary, this compiled
    GPUI's test scaffolding into every release build. Confirmed every
    `TestAppContext`/`gpui::test` usage in the crate sits inside
    `#[cfg(test)]` before moving it; moved both crates' `test-support` to
    `[dev-dependencies]` rather than propagate the pattern when adding it to
    `crates/app` for its own new test.
  - Verified by execution: live run with a deliberately mismatched plugin
    (dir `test-race-plugin`, manifest `name = "race"`, widget name
    `race-widget`) produced `Hot-reloaded plugin: race (".../test-race-plugin")`
    with no `WARN ... not found` — the exact scenario that previously always
    produced that warning. New regression tests
    (`reload_registers_widget_when_dir_name_differs_from_manifest_name` in
    `manager.rs`, `register_plugin_widgets_handles_name_mismatch` in
    `plugin_bridge.rs`) exercise this mismatch directly; full suites pass
    25 (chronos-luau) + 7 (chronos) = 32/32.

  ## 2026-07-10 — Services layer: crate scaffolding + Service trait (Task 1)

  - Considered: `niri-ipc = "=25.11.0"` per plan — does not exist on crates.io (available: 26.x only).
  - Decided: use `niri-ipc = "26"` (latest 26.x). Scaffold only; Niri backend is stubbed (Task 2).
  - Considered: `anyhow::Error` as `Service::Error` type — does not satisfy `std::error::Error` bound directly (impl via Deref only).
  - Decided: **remove `std::error::Error` bound from `Service::Error`**, keep only `Send + Sync + 'static`.
    - Rationale: trait doesn't invoke Error methods; errors are logged via `tracing::{warn,error}` in impls.
    - Contract test uses `type Error = anyhow::Error` and passes.
    - Future services (Compositor/Network/UPower) can use `anyhow::Error` or domain-specific errors.
  - `hyprland = "0.4.0-beta.3"` — only available version (prerelease). API surface may shift; Task 2 will verify against this version.

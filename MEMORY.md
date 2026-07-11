# Project memory
_Durable project-level knowledge. Persists across all sessions in this project. Edit only content under italic instructions._

## Project context
_What is this project? What's its goal? High-level identity._

Chronos — a desktop shell for Hyprland 0.55.4+ (C++ compositor; config layer moved to Lua at 0.55.0, core still C++). Goal: a fully functional shell without third-party tools, with own modules `bar` / `dock` / `launcher` / `notifications` / `osd`, architected so new modules can be added as plugins without recompiling the core. Priorities: visuals, animations, modularity, light customization, plugins, 144 FPS, hot-reload without restart.

## Rules
_Hard constraints from user that every session must respect._

- Respond in Russian by default, regardless of the message language, unless the user explicitly asks otherwise.
- Never add a `Co-Authored-By: Claude` (or similar AI attribution) trailer to git commits in this repo — user requested no AI co-authorship on GitHub.
- Before any non-trivial implementation, verify against `ARCHITECTURE.md` / `DECISIONS.log` (canonical docs) rather than relying on recalled prior-chat context.
- For version/protocol/library-ecosystem facts that may have changed (it's 2026), web-verify before asserting.
- AGENTS.md self-awareness clause: dialog memory does not replace project documentation — check the repo docs first.
- **SESSION_REPORT.md format is strict.** Every agent (subagent or workflow) that completes a task block MUST produce a SESSION_REPORT.md with this exact structure. Deviations are bugs:
  ```
  # Session: <task name> — <date>
  ## Сделано (факт, не намерение)
  - файл: что изменилось, одной строкой
  ## Расхождения со спекой/планом
  - что план требовал → что реально сделано → почему (если решение, а не забыли — иначе явно писать "TODO, не решение")
  ## Не реализовано из acceptance criteria
  - список пунктов из design doc, которые НЕ закрыты в этом проходе, с указанием где именно (файл/функция отсутствует, не "почти готово")
  ## Проверено фактом, не на словах
  - команда → вывод (grep/test run/build), не "должно работать"
  ## Новые риски / известные баги
  - если что-то шаткое зафиксировано как compromise — здесь, с severity
  ## Статус ARCHITECTURE.md / DECISIONS.log
  - обновлены? какие секции? если нет — почему
  ```
  Ключевые анти-враньё механизмы: «Проверено фактом» запрещает заменять `cargo test passes` на «должно работать»; «Не реализовано» — прямая защита от повторения бага со статусом «done» при отсутствии компонента; «Расхождения» — самая ценная секция, т.к. 3 из 4 находок в прошлом диалоге были именно расхождениями, которые агент знал, но не поднял явно.

## Architecture decisions
_Major design choices with rationale. The "why" matters more than the "what" for future sessions._

- **Bar = scaffold + dyn-safe widget contract (widgets deferred).** The top bar ships as a plain strip now; structure delivered: left/center/right sections, `trait BarWidget` (`section() -> BarSection` + `render(&self, &mut Window, &App) -> AnyElement`), and a runtime `BarWidgetRegistry` (GPUI global, `Vec<Box<dyn BarWidget>>`, `register()` + `widgets_for(section)`). Deliberately diverges from the `reference/gpui-shell` pattern (`trait BarWidget: Sized` chrome + static `enum Widget`/`match` dispatch): we make the trait itself dyn-safe and use it as the dispatch mechanism so adding a widget needs NO core recompile. Spec: `docs/superpowers/specs/2026-07-09-bar-scaffold-widget-contract-design.md`. (User-confirmed 2026-07-09.)
- `BarWidget::render` is `&self` (not `&mut self`) so the bar can call it through a shared `&dyn BarWidget` borrowed immutably from the global registry during `Bar::render`; it also takes `&App` (not `&mut App`) for the same borrow reason — `Bar::render` already holds an immutable `cx.global()` borrow of the registry, so two immutable borrows coexist. Reactivity later via interior mutability / global `AppState`, not `&mut self`/`&mut App`.
- **BarWidgetRegistry must grow named-widget replacement for LuaU hot-reload.** The 2026-07-09 decision to use Vec (not HashMap) was correct for scaffold, but LuaU hot-reload now requires replacing widgets by name. Approach: add `replace_by_name(name: &str, widget: Box<dyn BarWidget>)` to the Vec-backed registry (iterate, find by name, swap). This preserves insertion order while enabling replacement. Must be done in `crates/app` before Task 7 (bridge). Also add `unregister_by_name(name)` for full hot-reload cycle.
- **`chronos.log` API: table with methods, not flat function.** Spec §7 originally said `chronos.log(msg)`, but implementation and plugin examples use `chronos.log.info(msg)` / `chronos.log.warn(msg)`. Table-with-methods is more idiomatic Lua and extensible. Spec §7 corrected in plan.
- **Tick timer: GPUI executor, not tokio.** Plan originally said `tokio::spawn` timer, but DECISIONS.log 2026-07-08 explicitly rejects tokio driving UI. Correct pattern: `cx.spawn()` + `cx.background_executor().timer(Duration::from_secs(1))` in a loop. `PluginManager::start_tick_loop(&self, cx: &mut App)` added to plan.

## Discovered durable knowledge
_Cross-task facts that survive across sessions. Promoted from session checkpoints' §7 when proven durable._

- Stack: Rust + GPUI (gpui-ce) + mlua (LuauJIT). Workstation: CachyOS, RTX 3070, i5 12400F, 64GB.
- The real Chronos bar code lives at `crates/app/src/bar.rs` (layer-shell window, single `div` background, `BAR_HEIGHT=32.0`, `BAR_COLOR=0x1e1e2e`). `reference/gpui-shell/` is ONLY the code-study copy — do not edit it as if it were Chronos code. (Corrected 2026-07-09: an earlier "OPEN GAP" claiming `crates/` was missing was a misread; `git ls-files`/`find` both surface `crates/app/`.)
- Branch `feat/bar-scaffold-widget-contract` and its worktree `.worktrees/feat-bar-scaffold-widget-contract` are gone (merged, worktree removed) — do not go looking for them; if a future session can't find a branch/worktree mentioned here, check for a merge commit on `master` before assuming it's still in progress or lost.
- Branch `feat-inotify-hot-reload` merged to `master` (fast-forward, `d7ab5a7`) 2026-07-09; worktree `.worktrees/feat-inotify-hot-reload` removed same session. Delivered: inotify watcher (dedicated OS thread, trailing debounce, WatchDescriptor-based event matching) + plugin identity fixed to key by directory path instead of name string (manager.rs + plugin_bridge.rs). Full writeup: `SESSION_REPORT.md`, `fixplan.md` (repo root, copied out of the worktree before removal so nothing from that session's investigation was lost).
- App crate package name is `chronos` (NOT `chronos-app`): build/test with `cargo build -p chronos` / `cargo test -p chronos`.
- Rust forbids a module being resolvable to BOTH `bar.rs` and `bar/mod.rs`; converting `crates/app/src/bar.rs` into the `bar/` directory requires `git rm crates/app/src/bar.rs` first.
- gpui-ce: `div().into_any_element()` needs `gpui::IntoElement` in scope (trait method). Verify exact API when coding (2026 dataset may be stale).
- **Luau plugin layer: `crates/luau` as standalone crate with gpui dependency.** The Element DSL lives in `crates/luau` and does Element → AnyElement conversion directly (gpui added as workspace dependency). This keeps the plan self-contained — avoids touching `crates/app` in every task. `BarWidgetAdapter` in `crates/app/src/plugin_bridge.rs` wraps a Lua VM + render callback and implements `BarWidget`. `mlua::Lua` is Arc-backed; cloning is cheap. `register_chronos_api` for MVP takes `(lua, manifest)` — event callback threading handled in manager layer. Spec: `docs/superpowers/specs/2026-07-09-luau-plugin-layer-design.md`. Plan: `docs/superpowers/plans/2026-07-09-luau-plugin-layer.md` (9 tasks). User-confirmed 2026-07-09.
- **BarWidgetRegistry is Vec-based with named-widget ops** (`widget.rs:13-55`). `replace_by_name(name, widget)` + `unregister_by_name(name)` added 2026-07-09 for LuaU hot-reload support. Vec chosen for order-preservation; HashMap rejected because iteration order would scramble widget layout. DECISIONS.log 2026-07-09.
- **Tick timer must use GPUI executor, not tokio.** DECISIONS.log 2026-07-08 "Runtime split" explicitly rejects tokio driving UI code. `bar::init()` (mod.rs:120) demonstrates the correct pattern: `cx.spawn()` + `cx.background_executor().timer()`. The luau tick dispatcher must follow this, not `tokio::spawn`. Design spec §7's "tokio::spawn timer" contradicts the accepted architecture.
- **gpui-ce local patch: commit 6a7b386** on top of baseline 352c9f2. Fixes quit()-before-window-hang (calloop reset-on-entry). ARCHITECTURE.md §2 pinned revisions table does NOT mention this patch — documentation gap.
- **mlua API patterns (verified with mlua 0.10 + Luau).** `lua.load(source).eval()` (not `lua.eval()`). Raw string `r#"..."#` conflicts with Lua `#hex` colors — use `r##"..."##` or single-quoted Lua strings. `BorrowedStr` from `to_str()` needs `&*s` deref for `&str` matching. `table.get("opt")?` errors on nil — use `unwrap_or(mlua::Value::Nil)` + explicit nil handling. `mlua::Error::RuntimeError(msg)` for Lua runtime errors (not `mlua::RuntimeError::RuntimeError`).
- **Hyprland 0.55+ (Lua config): `hyprctl dispatch` syntax changed.** Legacy `hyprctl dispatch workspace N` returns 0 but errors with `')' expected near 'N'`. New syntax: `hyprctl dispatch 'hl.dsp.focus({ workspace = "N" })'` (or numeric without quotes). This is an environment change, not a code bug — affects any external tooling/scripts using `hyprctl dispatch`.
- **Launcher: `nucleo 0.5` fuzzy search engine.** `crates/app/src/launcher/search.rs` wraps `nucleo` (MPL-2.0, Helix editor engine). API changes in 0.5: `CaseMatching`/`Normalization` moved to `nucleo::pattern`, `Normalization::None` → `Normalization::Never`, `Item.data` is `&T` (deref needed), `Snapshot::matched_items(range)` panics if range end > matched count (clamp `max` before call).
- **Launcher layer-shell centering:** spec described `window_bounds.origin` centering. Layer-shell protocol centers via `anchor = TOP|BOTTOM|LEFT|RIGHT` + symmetric `margin` (stretch to full output, inset by margin). `Anchor::empty()` + `window_bounds.origin = center` is compositor-dependent and fails on Hyprland. Implemented: `anchor = TOP|BOTTOM|LEFT|RIGHT`, `margin = (margin_y, margin_x, margin_y, margin_x)`.
- **Launcher keyboard interactivity — Exclusive vs OnDemand:** Spec requested `KeyboardInteractivity::Exclusive` (rofi-like). Tested: `Exclusive` on `Layer::Overlay` + `anchor=empty` on Hyprland/Niri → compositor freezes entire input stack (session crash/reboot required). Root cause: exclusive layer-shell surface never acks keyboard focus, compositor waits indefinitely. Fallback: `KeyboardInteractivity::OnDemand` + explicit `window.activate_window()` + `window.focus(&focus_handle)`. Window opens but **does not receive keyboard focus automatically** on Hyprland/Niri — requires click or Alt+Tab to acquire focus. Once focused, key events work. Root cause: layer-shell `OnDemand` requires compositor to explicitly grant focus (usually on click/focus policy). `activate_window()` sends `xdg_activation_v1` token, but layer-shell surfaces don't participate in xdg_activation. **Deferred fix (Critical):** investigate `zwlr_layer_surface_v1.set_keyboard_interactivity` timing / proper focus ack in GPUI platform, or fallback to XDG popup/toplevel for launcher.
- **Launcher IPC toggle:** Spec proposed separate `ShowLauncher`/`HideLauncher`/`ToggleLauncher`. Simplified to single `ToggleLauncher` payload (stateless toggle) — reduces message surface, matches user workflow (single keybind).
- **Launcher IPC dual-channel accept loop:** `accept_loop` merges `ping` + `toggle_launcher` channels via `tokio::select!` — avoids head-of-line blocking if one channel stalls.
- **Launcher `DesktopEntry` fields `icon`/`terminal`/`no_display` parsed but unused:** Parsed per XDG spec, `no_display` filters at parse time, `icon`/`terminal` stored for future (icon rendering, terminal launch). YAGNI deferred.
- **Launcher `setsid` launch:** `launch()` uses `setsid sh -c "exec"` + triple `Stdio::null()` — detached process survives parent death. Validated by `setsid --version` test.
- **Launcher `setsid` availability test:** Added compile-time check that `setsid` exists on host. Fails fast if missing (non-POSIX environments).
- **IPC reactor fix:** `IpcSubscriber::init()` called before tokio reactor → panic "no reactor running". Fixed: `acquire_at` stores std `UnixListener`, `start_listener` converts to tokio listener (inside runtime). `init_all()` runs inside `rt.block_on` so reactor lives for full GUI lifecycle.
- **App run inside tokio runtime:** `app.run()` moved inside `rt.block_on` so tokio reactor lives for full GUI lifecycle, enabling `tokio::spawn` in IPC listener.
- **`services` crate: `Service::Error` bound relaxed.** Removed `std::error::Error` bound (anyhow::Error doesn't satisfy it directly). Kept `Send + Sync + 'static`. Contract test passes with `type Error = anyhow::Error`.
- **zbus 5.17 per-property streams:** Plan assumed `receive_properties_changed()`; actual generates per-property `receive_*_changed().await` returning `PropertyStream` directly (not `Result`). Applied to Network (`connectivity`) and UPower (`percentage` + `state` merged via `tokio::select!`).
- **Float `Eq` trap:** `Monitor.scale: f32`, `UPowerData.battery_percent: f64` — `Eq` not derivable. Dropped `Eq` from those structs (`Service::Data` only requires `Clone`). Pattern: any service data struct with float must NOT derive `Eq`. Third hit (Monitor.scale, CompositorState, UPowerData).
- **`i128` monitor IDs:** `hyprland` crate uses `MonitorId = i128`. Stored directly (no `i64`/`i32` truncation). `Service::Data: Clone` has no size bound.
- **`hyprland` crate `Address` opaque newtype:** No public accessor, but derives `Display`. Used `address.to_string()` to extract inner hex string safely.

## На горизонте (known gaps / follow-up specs, not yet built)

_Cross-session durable: these are explicitly deferred, each tied to a named future consumer. Do not start them without the linked spec._

- **Tray-сервис отсутствует в текущем services-layer плане.** Design doc (`2026-07-10-services-layer-design.md` §1 scope) выносит tray/notifications/bluetooth/audio/mpris/sysinfo в отдельные будущие спеки. `Alloy`-порт (Tauri+React → Rust+GPUI tray-виджет с меню, AUR helper) требует `StatusNotifierItem`/`StatusNotifierWatcher` D-Bus — нужна отдельная спека tray-сервиса **до** старта работы над Alloy-плагином. Не блокирует текущий Task 2-8 services-layer.
- **Desktop-widget plugin API отсутствует.** Текущий `chronos.bar:register(spec)` (crates/luau design §7) привязан только к `BarWidgetRegistry` (left/center/right внутри одного bar-окна). `plasminal` (standalone desktop-widget с абсолютным позиционированием) требует отдельный layer-shell surface (`Layer::Background`, не `Layer::Top`) и новый API-namespace (например `chronos.desktop:register(...)`). Точки опоры уже добавлены: `Monitor.x/y/scale/id` (geometry, 2026-07-10) — но сам API и surface ещё не спроектированы. Отдельная спека, отдельная сессия.
- **ARCHITECTURE.md §4 устарел частично.** Добавление `Monitor` geometry (2026-07-10) делает §4 не единственным источником truth для позиционирования плагин-окон — desktop-widget плагины позиционируются абсолютно через `Monitor`, не через layer-shell `exclusive_zone`. §4 нужно пересмотреть/дополнить отдельной правкой (зафиксировано в DECISIONS.log 2026-07-10, НЕ сделано).
- **Launcher keyboard focus (Critical):** `KeyboardInteractivity::OnDemand` + explicit focus не работает автоматически на Hyprland/Niri. Нужен fix в gpui_platform layer-shell keyboard focus handling или fallback на XDG popup/toplevel.
- **Launcher deferred features:** icon rendering (нужен SVG/PNG loader), terminal launch (`Terminal=true`), recent/frecency sorting, categories/filtering, multi-monitor (focused monitor via `CompositorSubscriber`).
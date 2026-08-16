# Chronos — Decisions Log

Append-only. Each entry: what was considered, what was rejected and why, what
was decided. Full rationale for the currently-approved architecture lives in
`docs/ARCHITECTURE.md`; this file is the history, including things docs/ARCHITECTURE.md
no longer needs to spell out because they're settled.

If its empty\not full - its a fresh log. - see previous log in /home/neo/Projects/chronos/log-dump

---
  ## 2026-07-10 — Services layer: crate scaffolding + Service trait (Task 1)

  - Considered: `niri-ipc = "=25.11.0"` per plan — does not exist on crates.io (available: 26.x only).
  - Decided: use `niri-ipc = "26"` (latest 26.x). Scaffold only; Niri backend is stubbed (Task 2).
  - Considered: `anyhow::Error` as `Service::Error` type — does not satisfy `std::error::Error` bound directly (impl via Deref only).
  - Decided: **remove `std::error::Error` bound from `Service::Error`**, keep only `Send + Sync + 'static`.
    - Rationale: trait doesn't invoke Error methods; errors are logged via `tracing::{warn,error}` in impls.
    - Contract test uses `type Error = anyhow::Error` and passes.
    - Future services (Compositor/Network/UPower) can use `anyhow::Error` or domain-specific errors.
  - `hyprland = "0.4.0-beta.3"` — only available version (prerelease). API surface may shift; Task 2 will verify against this version.

  ## 2026-07-10 — Compositor types: geometry fields added ahead of consumer, user-directed

  - Context: the services-layer plan (Task 2) shipped a trimmed data model
    (`Workspace`: `id/name/active`; `Monitor`: `name/active_workspace`;
    `ActiveWindow`: `title/class`) vs. the fuller reference `gpui-shell`
    types. Normally this project extends types only when a consumer appears
    (YAGNI), but this is an **explicit user override** with named consumers.
  - Reason for override: three plugins are already designed and will be
    ported to Chronos, and they need geometry now:
    - `plasminal` (github.com/Dark-Ohm/plasminal) — rewritten from scratch as
      a LuaU plugin: a standalone desktop-widget with absolute on-screen
      positioning (NOT a bar widget). Needs `Monitor.x/y/scale` as the global
      origin for layout, independent of layer-shell `exclusive_zone`.
    - `chronos-fm` (fork of noh-rs/nohrs) — WASM-plugin launcher moving to
      LuaU (plugin-runtime convergence, not two parallel engines).
    - `Alloy` — rewritten from Tauri+React to Rust+GPUI: a tray widget with a
      menu (AUR helper: check updates/news/install packages).
  - Added ahead of consumer: `Workspace.monitor_id`, `Monitor.id/x/y/scale`,
    `ActiveWindow.address`. Remaining reference fields
    (`Workspace.index/windows`, `Monitor.transform/dpms/vrr/make/model/serial`,
    `ActiveWindow.workspace/floating/pinned/fullscreen/x/y/width/height`)
    remain YAGNI — add on demand.
  - **Type-fidelity deviation from the user's literal `task.md` spec** (must
    be recorded, not silently absorbed):
    - `task.md` asked for `Workspace.monitor_id: i32` and `Monitor.id: i32`.
      The `hyprland` crate 0.4.0-beta.3 types these as `MonitorId = i128`
      (and `Workspace.monitor_id: Option<MonitorId>`). Truncating `i128 → i32`
      is silent data loss, so we store `Option<i64>` / `i64` instead — real
      monitor IDs are small, `i64` round-trips losslessly for all practical
      purposes.
    - `ActiveWindow.address: String` — the crate's `Address` is an opaque
      newtype (`pub struct Address(String)`) with no public accessor, but it
      derives `derive_more::Display`, so `address.to_string()` yields the
      inner hex string. This is the only safe way to extract it without
      `unsafe`/reflection.
    - `Monitor.scale: f32` forced dropping `Eq` from `Monitor` and
      `CompositorState` derives (f32 is not `Eq`). `Service::Data` only
      requires `Clone`, so this is safe — no consumer relied on `Eq`.
  - **docs/ARCHITECTURE.md §4 impact**: this changes the implicit assumption that
    positioning happens only via layer-shell `exclusive_zone`/`display_id`.
    Desktop-widget plugins position absolutely via `Monitor` geometry; §4 is
    no longer the sole source of truth for plugin-window placement. §4 needs
    a follow-up revision (NOT done in this task — tracked as TODO below).

  ## 2026-07-10 (addendum) — monitor_id/id: i128, not i64 (no truncation at all)

  - Follow-up to the geometry-fields entry above. Initially stored
    `monitor_id: Option<i64>` / `id: i64` (reasoning: real Hyprland monitor
    IDs are small, so `i64` round-trips). User correctly flagged that
    `i128 → i64` is *also* a silent truncation, just less likely.
  - Decided: use `i128` directly (matches `hyprland` crate `MonitorId = i128`
    exactly). `Service::Data: Clone` imposes no size bound, and `i128` is
    `Copy + Clone + PartialEq + Eq + Serialize + Deserialize` — so no derive
    fallout beyond the already-required `Eq` drop on `Monitor`/`CompositorState`
    (caused by `scale: f32`, independent of int width). Cost is one field type,
    not an architectural change. Silent truncation removed entirely.

  ## 2026-07-10 — NetworkSubscriber (Task 3): zbus 5.x API deviations from plan

  - Plan (Task 3) was written as a template against an assumed zbus 5.x API.
    Verified against the pinned `zbus 5.17.0` and corrected three mismatches:
    - Plan used `mgr.receive_properties_changed()` (no await, returns stream).
      Actual zbus 5.17 generates **per-property** streams: for the `connectivity`
      property the method is `receive_connectivity_changed().await` and it
      returns `PropertyStream<'_, u32>` directly (NOT a `Result`). Fixed.
    - Plan used `data.get()` inside the async loop. `Mutable::get()` requires
      `T: Copy`; `NetworkData` is not `Copy`, so it must be `data.get_cloned()`.
    - Plan captured `Handle` and called `handle.enter()` around the sleep to
      "ensure runtime context". But `tokio::spawn(run(...))` already runs the
      future inside the runtime (new() is called from `init_all` inside
      `rt.block_on`), so `EnterGuard` was redundant AND broke `Send` (the guard
      is `!Send`, so the future failed `tokio::spawn`'s `Send` bound). Removed
      `handle` from `run`; kept `Handle::current()` in `new()` only as the
      documented guard that panics if called outside a runtime.
    - `ConnectivityState` needed `#[derive(Default)]` (with `#[default]` on
      `Unknown`) because `NetworkData` derives `Default`.
  - `NetworkSubscriber::new()` panics if called outside a tokio runtime
    (`Handle::current()`). This is intentional per spec §5.1; `init_all()`
    (Task 5/6) provides the runtime via `rt.block_on`.

  ## 2026-07-10 — UPowerSubscriber (Task 4): zbus 5.x API deviations + f64 Eq trap

  - Same zbus 5.17 API corrections as Task 3 (NetworkSubscriber), applied to
    the UPower DisplayDevice proxy:
    - Plan used `dev.receive_properties_changed()` (no await). Actual zbus 5.17
      generates per-property streams: `receive_percentage_changed().await` and
      `receive_state_changed().await`, each returning `PropertyStream` directly
      (not `Result`). Both streams merged via `tokio::select!` in the update
      loop (re-read both properties on any change — cheap, keeps data
      consistent).
    - `data.get()` → `data.get_cloned()` (`Mutable::get` requires `T: Copy`;
      `UPowerData` is not `Copy`).
    - Removed `handle.enter()` around the sleep (same `!Send` `EnterGuard`
      trap as Task 3). `Handle::current()` kept in `new()` as the runtime guard.
  - **`f64` Eq trap (same class as `Monitor.scale: f32` in Task 2):** plan
    derived `Eq` on `UPowerData`, but `battery_percent: f64` is not `Eq`.
    Dropped `Eq` from `UPowerData` (only `Clone` required by `Service::Data`).
    `BatteryState`/`PowerProfile` kept `Eq` (they are `Copy` enums, no float).
  - Pattern confirmed: **any service data struct holding a float must NOT
    derive `Eq`** — `Service::Data: Clone` is the only bound, so `PartialEq` is
    sufficient. This is now the third hit (Monitor.scale, CompositorState,
    UPowerData); future service types with floats should follow suit.

  ## 2026-07-10 — Services container + init_all() + retry-loop tests (Task 5)

  - Added `Services` struct (holds `compositor`/`network`/`upower` subscribers)
    and `init_all() -> Services` to `crates/services/src/lib.rs`. `init_all()`
    is sync, always succeeds (each constructor is non-failing, spawns its own
    background task). MUST be called inside a tokio runtime (`rt.block_on`) so
    `Handle::current()` resolves in the D-Bus constructors (spec §5.1 + §7).
  - Retry-loop unit tests (`retry_tests` mod): `FakeRetryService` mirrors the
    spec §5.1 backoff loop against an `AtomicU32` failure counter; asserts
    `Initializing → Unavailable → … → Available` and that backoff grows.
  - **Plan bug caught during implementation:** plan (Task 5, line ~1173)
    asserted `attempts == 3` for `FakeRetryService::new(3)`. The loop guard
    `if n >= failures_before_success` triggers success on the (N+1)-th attempt
    (n=0,1,2 → 3 `Unavailable`, n=3 → `Available`), so `attempts == 4`, not 3.
    Fixed the assertion to `4` with an explanatory comment. The plan's test
    would have failed as-written.
  - **Panic-guard edge-case test (user-demanded, not in plan):** `runtime_guard_tests`
    mod with `network_new_panics_outside_runtime` + `upower_new_panics_outside_runtime`.
    Plain `#[test]` (NOT tokio), wrap `Subscriber::new()` in
    `catch_unwind(AssertUnwindSafe(...))`, assert `Err`. Pins the
    `Handle::current()` panic so a future refactor cannot silently remove the
    guard. Rationale: `init_all()` runs inside `rt.block_on` in normal flow, so
    no panic there — but the guard must not be silently dropped if someone
    later calls `new()` outside the runtime.
  - Result: `cargo test -p chronos-services` → 7 tests pass (3 pre-existing +
    2 retry + 2 panic-guard), 0 warnings. Verified independently after the
    subagent run.

  ## 2026-07-10 — AppState + watch() bridge + rt.block_on bootstrap (Task 6)

  - Created `crates/app/src/state.rs` with `AppState` (GPUI `Global` holding
    `Services`) and `watch()` helper. `AppState::init(services, cx)` stores the
    services in `cx.set_global()`. Accessors: `AppState::global(cx)`,
    `AppState::compositor(cx)`, `AppState::network(cx)`, `AppState::upower(cx)`.
  - `watch<C, S, T, F>(cx, signal, on_update)` where `S: Signal<Item = T> +
    Unpin + 'static`, `T: Clone + 'static`, `F: Fn(&mut C, T, &mut Context<C>)`.
    Spawns on GPUI executor via `cx.spawn()`, converts signal to stream, applies
    updates via `this.update()`. Detached — runs independently on UI thread.
  - Modified `crates/app/src/main.rs`: bootstrap now creates dedicated
    `tokio::runtime::Builder::new_multi_thread().enable_all().build()`, calls
    `rt.block_on(async { chronos_services::init_all() })` to satisfy
    `Handle::current()` in D-Bus constructors (spec §5.1 + §7), then passes
    `Services` to `AppState::init()` inside `app.run()`.
  - Updated `crates/app/Cargo.toml`: added `futures-signals.workspace = true`,
    `futures-util.workspace = true`, `chronos-services = { path = "../services" }`.
  - Unit tests in `state.rs`: `app_state_module_compiles`, `app_state_accessor_types`,
    `service_status_variants`, `subscriber_types_accessible` — all pass.
  - Full test suite: `cargo test` → 43 tests pass (11 app + 25 luau + 7 services).
  - Build: `cargo check -p chronos` clean (3 warnings about unused public API
    — expected, meant for downstream UI widgets).

  ## 2026-07-11 — Launcher module (Task 9): nucleo 0.5, layer-shell overlay, IPC toggle

  - **nucleo 0.5 API deviations:** `nucleo 0.5.0` moved `CaseMatching`/`Normalization`
    to `nucleo::pattern` module, removed `Normalization::None` (use `Normalization::Never`),
    `Item.data` is `&T` (requires deref), `Snapshot::matched_items(range)` panics if
    range end > matched count. Fixed in `crates/app/src/launcher/search.rs`.
  - **Fuzzy search engine:** `nucleo` chosen over custom/rapidfuzz — mature, MPL-2.0,
    used by Helix editor. `Config::DEFAULT` fine for launcher (no custom scoring).
  - **Layer-shell centering:** Spec §3.1 described centered overlay via `window_bounds`
    origin. Layer-shell protocol centers via `anchor = TOP|BOTTOM|LEFT|RIGHT` +
    symmetric `margin` (stretch to full output, inset by margin). `Anchor::empty()` +
    `window_bounds.origin = center` is compositor-dependent and fails on Hyprland.
    Implemented: `anchor = TOP|BOTTOM|LEFT|RIGHT`, `margin = (margin_y, margin_x, margin_y, margin_x)`.
  - **Keyboard interactivity — Exclusive vs OnDemand (Critical):**
    - Spec §3.1 requested `KeyboardInteractivity::Exclusive` (rofi-like capture-all).
    - Tested: `Exclusive` on `Layer::Overlay` + `anchor=empty` on Hyprland/Niri → compositor
       freezes entire input stack (session crash/reboot required). Root cause:
       exclusive layer-shell surface never acks keyboard focus, compositor waits indefinitely.
    - Fallback: `KeyboardInteractivity::OnDemand` + explicit `window.activate_window()` +
       `window.focus(&focus_handle)` in `open()`. Window opens, but **does not receive
       keyboard focus automatically** on Hyprland/Niri — requires click or Alt+Tab to
       acquire focus. Once focused, key events work (Enter/Escape/navigation).
    - Root cause: layer-shell `OnDemand` requires compositor to explicitly grant focus
       (usually on click or focus policy). `activate_window()` sends `xdg_activation_v1`
       token, but layer-shell surfaces don't participate in xdg_activation.
    - **Deferred fix:** Investigate `zwlr_layer_surface_v1.set_keyboard_interactivity`
       timing / proper focus ack in GPUI platform, or fallback to XDG popup/toplevel
       for launcher. Marked Critical severity.
  - **IPC ToggleLauncher:** Spec §7 proposed separate `ShowLauncher`/`HideLauncher`/`ToggleLauncher`.
    Simplified to single `ToggleLauncher` payload (stateless toggle) — reduces message
    surface, matches user workflow (single keybind).
  - **IPC dual-channel accept loop:** `accept_loop` now merges `ping` + `toggle_launcher`
    channels via `tokio::select!` — avoids head-of-line blocking if one channel stalls.
  - **DesktopEntry fields `icon`/`terminal`/`no_display` parsed but unused:**
    Parsed per XDG spec, `no_display` filters at parse time, `icon`/`terminal` stored
    for future (icon rendering, terminal launch). YAGNI deferred.
  - **Setsid launch:** `launch()` uses `setsid sh -c "exec"` + triple `Stdio::null()`
    — detached process survives parent death. Validated by `setsid --version` test.
  - **Setsid availability test:** Added compile-time check that `setsid` exists on host.
    Fails fast if missing (non-POSIX environments).

## 2026-07-16 — Переезд в chronos-ecosystem: констатация фактов (приёмка минион-отчётов)
- Репо переехал копированием в /home/neo/projects/chronos-ecosystem/ChronOS ~2026-07-12. .git утерян БЕЗВОЗВРАТНО (find по всему /home/neo: единственный .git экосистемы — Chronos-Engine; d7ab5a7 не существует ни в одном репо). История = git log больше не источник; источники — docs/ARCHITECTURE.md/DECISIONS.log/SESSION_REPORT.md.
- gpui-ce расплющен в ../Source/ (9 крейтов siblings + gpui-component) БЕЗ workspace-корня → крейты не парсятся (все на .workspace=true). Path-deps ChronOS указывают на мёртвый /home/neo/Projects/SOURCE/gpui/gpui-ce-main. Сборка мертва двухуровнево. Fix поручен Cline (CLINE.md №2): workspace-корень Source/Cargo.toml (zed rev 876ec5a8 для internal deps, версии по Cargo.lock) + правка path-deps.
- reference/gpui-shell пуст (код-стади копия не переехала) — ссылки docs/ARCHITECTURE.md §11 на конкретные файлы gpui-shell временно непроверяемы.
- Launcher keyboard focus (Critical, 2026-07-11): OMP-исследование показало — timing set_keyboard_interactivity в gpui_linux корректен (до commit, window.rs:170), wl_keyboard.enter обрабатывается пассивно, xdg_activation для layer-shell отклоняется (комментарий в самом gpui). Вариант (a) отпал. Рекомендация OMP: (c) XDG toplevel вместо layer-shell. РЕШЕНИЕ НЕ ПРИНЯТО — ждёт подтверждения Архитектора (моё мнение: принять (c), overlay-поведение добить windowrule pin/float).

## 2026-07-16 (вечер) — Source/ = собственный форк «gpui-ce chronos edition»
- Инвентаризация (проверено лично + cline-report №2): Source/ содержит 18 gpui-директорий — 9 базовых + 9 forked zed-internal (gpui_collections/scheduler/sum_tree/refineable/derive_refineable/media/zed_util/ce_util/elements, v0.2.2, датированы 14.07). Это fork-in-progress: код gpui уже использует TypeIdHashMap/SpawnTime, реализации в форках нет. docs/ARCHITECTURE.md §2 (pinned rev 20340e14) устарел — зависимость теперь собственный форк, не пин апстрима.
- Workspace-корень Source/Cargo.toml создан Cline (задание №2), path-deps ChronOS переведены на ../Source/*. chronos-services собирается; блокер — 5 missing API в форках.
- Решения по завершению (задание №3): forked crates подключать по path с package-rename; util_macros/http_client/reqwest_client оставить на zed git 876ec5a8 (минимум хирургии); 5 API дописать в форки (~20 строк, образец zed main); Source/ взять под git. Отклонено: вырезание http_client-использований и форк util_macros — лишняя хирургия до зелёного билда.
- Наблюдение: Source/ тасуется вне сессий (adk-rust исчез 16.07, крейты докопированы 15:07) — источник провенанса форка неизвестен, спросить Архитектора.

## 2026-07-16 (ночь) — Реанимация завершена: сборка зелёная (cline-report №3, принят)
- cargo build --workspace OK (Source/ + ChronOS), cargo test --workspace 62/62 pass — верифицировано Архитектором лично, не со слов миньона.
- Хирургия форков: gpui_collections += TypeIdHashMap/TypeIdHashSet (на gpui_util::TypeIdHashBuilder) + re-export FxBuildHasher; gpui_scheduler += SpawnTime, RunnableMeta.spawned, new_with_callers_location (образец zed main).
- Отклонение от плана, принято: gpui_util НЕ заменён на gpui_ce_util — в gpui_ce_util нет TypeIdHashBuilder, в старом gpui_util есть и он, и Deferred. Выбор по факту API, не по имени.
- gpui_elements исключён из workspace (7 API-drift ошибок, никем не используется) — re-enable, когда Source/gpui догонит.
- accesskit += feature "enumn" (иначе Action::n приватна, E0624).
- Source/ взят под git: initial commit 3ce3466. docs/ARCHITECTURE.md §2 переписан Архитектором (gpui-ce chronos edition вместо мёртвого пина 20340e14).
- Хвост риска (medium): util_macros/http_client/reqwest_client на zed git rev 876ec5a8 — форкнуть при следующем обслуживании.

## 2026-07-16 (поздний вечер) — Вырез из Kael: решение по hermes-report (принят, проверен грепами)
- Kael (Apache-2.0, вырез легален с атрибуцией) НЕ замена gpui-ce — layer-shell зачаточный. Режем куски.
- ПРИНЯТО к вырезу немедленно: (1) easing-кривые (~860 LOC, kael/animation.rs) + spring-интегратор (419 LOC, kael_ui/spring.rs) — S, чистая математика, фундамент motion-системы; (2) backdrop blur 2-pass (blade/shaders.wgsl:1074-1182 + renderer plumbing) — L, главный визуальный приз, linchpin для gradient borders и effect layers.
- ОТЛОЖЕНО до субстрата: FLIP и implicit transitions (нет transform-полей в Style и Window::with_element_transform — XL-блокер, проверено: 0 hits в style.rs), DraggableSpring (нужны жесты), gradient borders (после 8-stop апгрейда в blur-задаче), color filter, effect layers (после blur).
- НЕ БРАТЬ: erf box-shadows (уже идентично реализованы, shaders.wgsl:325), text_input (2987+243 LOC + отсутствующий undo_manager — overkill; launcher-ввод добить руками за S), kael_icons целиком (4 SVG, атласа нет — берём только паттерн svg()+include_str, Lucide-сет соберём сами).
- Заметка: blend-mode фиксы Kael портировать только с Metal-пути; их WGSL-путь несёт старую аппроксимацию (ловушка).
- Нарушение процесса: субагент Hermes записал SHADER_PORT_AUDIT.md в ChronOS вопреки read-only. Артефакт оставлен, правило ужесточить в следующих брифах.

## 2026-07-17 — Вырез из gpui-shell: решение по hermes-report №2 (принят, проверен грепами)
- gpui-shell БЕЗ лицензии (ни LICENSE, ни license-поля, ни SPDX; WIP-хобби) = all rights reserved. Дословный вырез ЗАПРЕЩЁН. Режим: rewrite-по-паттерну (наш код, их архитектура). Хорошо бы запросить лицензию у автора (andre-brandao).
- ПРИНЯТО (порядок): (1) демон org.freedesktop.Notifications (донор-паттерн services/notification/mod.rs, 576 LOC, полный FDO) — rewrite под наш trait Service, первый server-side zbus в кодовой базе; (2) notification popups UI (M) — после демона; (3) applications + wallpaper сервисы (S, 0 deps) — параллельно.
- ЧАСТИЧНО: launcher — steal только per-frame focus re-assert (донор mod.rs:336-338) поверх нашего OnDemand. Донорский Exclusive НЕ копировать — краш Hyprland/Niri доказан (DECISIONS 2026-07-11).
- ОТЛОЖЕНО: OSD (код S-M, но как фича XL — требует audio+brightness сервисов), tray (XL: L-сервис + L-UI, зависит от panel/BarWidget-инфры), audio/bluetooth/mpris/privacy (FFI/C-бэкенды, L каждый), control_center (XL, преждевременно), ui-крейт целиком (привязан к upstream gpui; переиспользовать только InputBuffer-паттерн и theme color-math).
- НЕ БРАТЬ: keybinds.rs как фикс фокуса (app-level bind_keys, Critical-баг не лечит), donor button.rs (0 байт).
- Заметки: zbus 5.5→5.17 дрейф минорный (RequestNameFlags, OwnedValue::try_from); у ChronOS нет app-level config hot-reload (inotify покрыл только luau+launcher) — дыра, отдельная спека.

## 2026-07-17 — Compositor dispatch: отказ от hyprland-rs Dispatch::call, пишем Lua-форму в сокет

- Контекст: виджет bar Workspaces шлёт `CompositorCommand` через `CompositorSubscriber::dispatch` → `hyprland::execute_command`. Клик по бейджу МЁРТВ (живой смок Архитектора).
- Расследовано: `hyprland-rs` `Dispatch::call` пишет в сокет классическую форму `dispatch workspace N`. Lua-Hyprland 0.55.4+ заворачивает ВСЁ, что приходит в сокет, в Lua: ответ `error: [string "return hl.dispatch(workspace 4)"]: ')' expected near '4'`. Чтение (события/workspaces через hyprland-rs) работает — только диспатчи молча падают. Проверено на голом сокете; рабочая форма: `/dispatch hl.dsp.focus({ workspace = 4 })`.
- Отклонено: `hyprland::dispatch::Dispatch::call` (и вообще любой диспатч через hyprland-rs) — несовместим с Lua-Hyprland, сервер парсит как Lua и валится. Оставляем hyprland-rs ТОЛЬКО для чтения (data/event_listener), не для записи команд.
- Решено: `execute_command` строит Lua-таблицу диспетчера и пишет `/dispatch <lua>\n` напрямую в `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock` через `std::os::unix::net::UnixStream` (синхронно, без tokio — совместимо с sync-thread моделью сервиса; не будим панику реактора из MEMORY). Покрыты ВСЕ варианты `CompositorCommand`:
  - `FocusWorkspace(id)` → `hl.dsp.focus({ workspace = N })` (N — число)
  - `NextWorkspace`/`PrevWorkspace` → `hl.dsp.focus({ workspace = "+1" })` / `({ workspace = "-1" })` (relative selector — Lua-строка, по грамматике workspace selectors)
  - `MoveToWorkspace(id)` → `hl.dsp.move({ workspace = N })` (двигает активное окно, follow по умолчанию false — совпадает с прежним `MoveToWorkspace(id, None)`)
- Верификация: юнит-тест `command_to_socket_line_formats_every_variant` (pure, без Hyprland) + живой прогон Архитектора (клик переключает воркспейс реально). `cargo test -p chronos-services --lib` зелёный (25 passed).
- Побочное: `cargo test -p chronos-services` (без --lib) падает на чужом WIP-примере `examples/tray-smoke.rs` (OpenCode: `tray.get()` без `use ... Service;` в scope) — не моё, не чинил.

## 2026-07-17 — Audio service backend: MVP `wpctl` poll (not native pipewire yet)

- Context: GROK.md assignment №1 — `AudioSubscriber` implements `Service` for default
  sink + source (volume/mute/name) with commands Set*Volume / Toggle*Mute. Criterion:
  external volume change (pavucontrol / second-terminal wpctl) must reach Data and
  wake subscribers.
- Considered: (1) `pipewire` crate 0.10 native FFI — correct long-term path, dedicated
  mainloop thread, registry/node props for volumes; (2) MVP WirePlumber CLI `wpctl`
  for get/set + 250 ms poll for events.
- Decided: **MVP (2) for the first cut.** Rationale: CachyOS always has `wpctl` with
  WirePlumber; zero new cargo deps; pure parsers unit-testable without a PW session;
  external changes verified live (smoke saw sink 0.40→0.33 in ~300 ms via raw wpctl
  bypassing dispatch). Native FFI deferred: larger surface (mainloop threading vs
  tokio Service template, SPA/pod API churn) for zero extra product value until OSD
  lands.
- Explicit temporary: replace `audio/mod.rs` body with native backend when OSD
  latency / multi-device enumeration demands it; keep `types.rs` (`AudioState` /
  `AudioCommand` / `EndpointState`) stable. Do NOT pin an old pipewire crate.
- Rejected now: permanent reliance on subprocess — poll is good enough for bar/OSD
  foundation but not for 144 FPS-adjacent continuous metering.

## 2026-07-21 — Audio per-app stream mute stays on `wpctl` + `pw-dump` (no native PipeWire)

- Context: right-side panel Task 6 — mute one MPRIS player's playback stream,
  not the master sink. MPRIS has no per-player mute.
- Considered: (1) native `pipewire` registry for stream nodes; (2) extend
  existing MVP: parse `pw-dump` JSON for `Stream/Output/Audio`, mute via
  `wpctl set-mute <node-id> toggle`, name-match MPRIS hint → stream.
- Decided: **(2).** Same backend decision as 2026-07-17; no new deps; unit-
  testable parsers; live schema confirmed
  (`media.class == "Stream/Output/Audio"`, `application.name` populated).
- Explicit limits kept: first matching stream only (browser multi-tab);
  empty/mismatched hint → no-op + log, never panic; full bus-name MPRIS
  ids may miss unless the panel passes a short hint.
- Rejected now: promoting stream mute as a reason to land native pipewire —
  still zero product pressure beyond panel mute button (Task 9 UI).

## 2026-07-21 — Тело правой панели: rsx+animation руками, НЕ `gpui-component` (отложен, апгрейдим по готовому рецепту)

- **Контекст.** Тело панели (Tasks 9-11: MPRIS-карточка, метры, power-row).
  Вопрос: тащить ли `gpui-component` (Longbridge, Button/Slider/Progress) или
  рисовать самим на `gpui-rsx` + `gpui-animation` + голом div.
- **Рассмотрено (recon Hermes №18 + пилот №19, оба перепроверены Архитектором
  лично):**
  - `gpui-component` 0.5.2 **компилится об наш форк с 0 ошибок** (rsx-паттерн,
    не 58-дельт ccf). Проводка: ChronOS `gpui`→`path=../Source/gpui` + `[patch
    ."…/zed"]` на наши path-крейты (одного patch мало — ChronOS и component
    были на разных git-URL). Один gpui в графе, dual-gpui нет.
  - **Цена (ЧЕСТНАЯ, from-scratch замер Архитектора):** бинарь **+2.66 MiB
    (+13.2%)**, 20 152 392 → 22 815 688. Монолит 89k LOC + тяжёлые дефолт-депы
    (ropey/markdown/html5ever/lsp-types) линкуются НАВСЕГДА, feature-strip нет.
    **Поправка к отчёту пилота:** Hermes рапортовал «clean +0.68 MiB» — НЕ
    воспроизвелось; from-scratch даёт то же 22.8, что инкрементал. Решающую
    цифру он занизил ~вчетверо; принято реальное +2.66.
- **Решено: rsx+animation руками (вариант C).** Причины пользователя: экономия
  времени, меньше багов, интерфейс рисуется легче, и **порт можно апгрейдить
  при надобности** — рецепт интеграции доказан и лежит на ветке
  `pilot/gpui-component-spike @ 20ee13a` (path+patch+init+Dark, одна Button
  рендерится на layer-shell). Тонкий бинарь сейчас, открытая дверь потом.
- **Отклонён вариант A** (полный тулкит): +13% бинаря + coupling с монолитом
  ради нескольких кнопок панели — не оправдано, пока component не станет общим
  тулкитом всего шелла. Пересмотреть, если launcher/settings/пр. массово
  захотят Button/Table/Input.
- **Что это меняет для плана.** Tasks 9-11 остаются на голом `gpui div` (план
  уже так написан, он рабочий); `gpui-rsx` — для чистоты разметки (но темовые
  цвета через expr-атрибуты `bg={theme…}`, НЕ Tailwind `class="bg-*"` — конфликт
  со docs/STYLE.md), `gpui-animation` — peek-выезд панели + переходы метров.
- **Урок процесса.** Цена бинаря — весь смысл пилота; артефакт на диске
  противоречил отчётной цифре → перемер с нуля обязателен. Не принимать
  decision-critical число миньона без воспроизведения.

## 2026-07-19 — Layer-shell popup sizing: hard `max_h()`+`overflow_hidden()` clip, NOT pixel-estimated height

- Context: `updates_popup` (Zed №1) sized its window via `count * ROW_H`-style
  arithmetic against an unmeasured GPUI text-metric constant (`ROW_H = 32`).
  Live smoke (24 updates) showed the real rendered row height is taller than
  guessed — the list consumed the entire window and pushed the "Upgrade all"
  button below the window's visible/clickable bounds *entirely* (not a
  cosmetic crop — the button ceased to exist on screen). Same failure mode
  independently reproduced in `notifications` (Hermes №9/№11) — an
  `estimate_content_height` formula chasing the same unmeasured metrics,
  repeatedly undercounting.
- Considered: (1) tune the pixel constants more precisely (Hermes №11's
  original brief); (2) hard-clip the variable-length list content to a fixed
  budget, with the mandatory chrome (footer/button) laid out *outside* that
  clipped box so it can never be pushed off regardless of how tall rows
  actually render.
- Decided: **(2), unconditionally, for any popup with a variable-length list
  and mandatory chrome below it.** `.max_h(px(N)).overflow_hidden()` on the
  list container (confirmed present and working in this gpui fork —
  `gpui_macros/src/styles.rs:900`, `BoxStylePrefix{prefix:"max_h"}`; note
  `.overflow_y_scroll()` does NOT resolve in this fork, confirmed separately
  — clip works, scroll doesn't). Applied to `updates_popup` (commit
  `67f7d10`) and `notifications` (Hermes №12) — both live-smoked (grim +
  `hyprctl layers`), footer/button unconditionally visible regardless of
  content length. This is now the STANDARD pattern for every layer-shell
  popup with a variable list — do not re-derive per-popup pixel formulas.
- Rejected: precise pixel-tuning as a durable fix — GPUI text-metric
  differences (font, line-height, padding compounding) make any hardcoded
  per-row constant a recurring source of the same bug under different
  content lengths. The clip is structural, not arithmetic — it cannot be
  "wrong" the way a pixel estimate can.

## 2026-07-19 — Launcher/popup dismiss: no close-on-focus-loss, ever (supersedes Cline №9 debounce)

- Context: `follow_mouse=1` in Hyprland fires spurious keyboard-deactivation
  the instant the cursor leaves a window, even without a real dismiss intent.
  Cline №9 tried a 300ms debounce on focus-loss-close — live smoke proved
  debounce only *delays* the incorrect close, it doesn't fix it (moving the
  mouse away for >300ms still closes the launcher unintentionally).
- Considered: (1) tune the debounce window further; (2) remove close-on-
  focus-loss entirely, keep only explicit dismiss paths (Esc / click-result /
  re-toggle hotkey / dedicated close button).
- Decided: **(2).** Applied to `launcher` (commit `fba8697`) and independently
  arrived at by Zed for `updates_popup` (Zed №1, before this was written down
  as a rule — Zed reasoned it out from the same `follow_mouse=1` MEMORY note).
  Live-smoked: launcher stays open through mouse-away, closes correctly via
  Esc/click. **This is now the standing rule for every layer-shell popup**:
  `observe_window_activation` may only be used to RE-FOCUS input on regained
  activation, never to close on lost activation. `tray_menu`'s
  `schedule_autoclose` (timer-based, not focus-based) is a different,
  acceptable mechanism — timers are not focus-loss detection.
- Rejected: any focus-loss-triggered close, debounced or not — the trigger
  itself (spurious deactivation under `follow_mouse=1`) is indistinguishable
  from a real dismiss at the event level, so no debounce value fixes it.

## 2026-07-19 — Top Bar redesign wave: decisions locked, briefs not yet written

Context: user compared live shell against Claude Design mockups
(`docs/design/*.dc.html`) and rejected the current visual state outright. Beyond
per-popup polish (border/badge/hover — HERMES.md №13), the Top Bar mockup
proposed genuinely new features. Each decided independently, full context in
docs/HANDOFF.md "ВОЛНА «Top Bar редизайн»" — summarized here for the log:

- **Audio visualizer**: considered native PipeWire-monitor tap (no external
  binary, more work/risk) vs. shelling to the real `cava` binary (external
  system dependency, not installed on this machine, but a proven/simple
  instrument). Decided: shell to real `cava`. Rejected: native tap (not
  worth the mainloop/threading complexity for a decorative bar element);
  faking/skipping the visualizer (user explicitly wants it real).
- **Dock**: considered keeping the standalone bottom-panel window (current
  state, just got persistent config from Mimo №7) vs. absorbing it into the
  bar's left cluster as an ordinary `BarWidget`, reusing the just-built
  `dock/config.rs` persistence logic. Decided: absorb into bar, standalone
  dock window goes away. The chronos-glyph icon (leftmost in the mockup)
  becomes a Plasma-Kickoff-style "Start" button wired to the already-
  accepted `launcher::toggle(cx)` — UX pattern borrowed from KDE Plasma,
  implementation is 100% native GPUI (no Qt/KDE dependency of any kind —
  does not contradict the earlier "Plasma abandoned for Hyprland" call,
  which was about not adopting KDE's *stack*, not about UX inspiration).
- **Git-branch indicator**: considered (1) a fixed path in config (dead
  simple, always shows one hardcoded repo's branch); (2) following the
  focused window's cwd (fragile — no universal way to query an arbitrary
  terminal/editor's cwd on Wayland, would need per-app integration hacks);
  (3) **user's own proposal, adopted**: a small persistent project registry
  (`~/.config/chronos/projects.toml`, same load/save pattern as `dock.toml`)
  with an "Add project" action opening a REAL system file picker (confirmed
  live: `org.freedesktop.portal.FileChooser` is present and reachable via
  zbus on this machine — ordinary D-Bus portal call, same complexity class
  as the existing upower/tray proxies), and a bar-clickable popup to select
  the *active* project. The pill shows the active project's current branch.
  Rejected (1) and (2) once (3) was proposed — strictly better than both:
  not fragile like (2), not single-repo-locked like (1).
- **Notification history**: no inbox/history concept exists in the tree at
  all today (confirmed by grep — notifications are purely ephemeral popups).
  Considered building a persistent history+unread-badge feature vs.
  deferring it. Decided: build it — bell icon + red badge dot (mockup:
  `#f38ba8`, cutout border matching bar bg) in the bar, click opens a
  history popup reusing `notifications/view.rs` card rendering, backed by
  the already-existing `NotificationCommand::DismissAll`.
- Build order (not yet dispatched, recorded so it isn't lost): workspace-dots
  and notification-history are independent, can go in parallel; dock→bar and
  project-switcher both touch the bar's left cluster and must be serialized
  (same file, avoid a shared-file collision); cava is fully independent (own
  service crate + a center bar widget). Final `bar/mod.rs` widget-order
  assembly is done by the Architect personally after each piece is
  individually accepted — not delegated to a narrow-zone agent.

## 2026-07-19 (вечер, продолжение) — Светлая тема: айдентика вместо Latte

- **Отменён плоский Catppuccin Latte** как светлая схема. Причина:
  либо скучно, либо (после осветления под неон) резало глаза —
  заливка цветом по всему фону на светлом физически не работает,
  неон живёт за счёт контраста с тьмой. Первая попытка (git-pill
  правки Project Switcher, светлый кадр) вышла лиловым суррогатом с
  перекрашенным акцентом — отклонена, зафиксировано правилом в
  `docs/design.md`: светлая ВСЕГДА берёт хексы из `light_scheme()`
  буквально, акцент не переопределяется дизайном.
- **Принято направление**: светлая тема получает собственную
  айдентику — атом · киберпанк · микрочип-трассы · сигилы/сакральная
  геометрия · Хронос (время), а не просто инверсия тёмной. Ключевое
  правило дисциплины: неон живёт в линиях/деталях (тонкие бордеры,
  glow-рёбра, акцентные штрихи, watermark на малой прозрачности), НЕ
  в заливке фона — поверхность остаётся спокойной holodной
  сине-лавандовой базой (`#dde0f2`/`#e6e9fa`, текст `#2c2e4a`
  индиго, не белый/не чёрный).
- **Эталон принят живьём** (`docs/design/Project Switcher.dc.html`,
  вариант "Light C"): сигил ChronOS в пилюле (гексагон + орбитальное
  кольцо + точка-центр + стрелка-хронометр), тонкая glow-линия по
  верхнему ребру карточки, светящаяся полоса текущего проекта
  (`box-shadow`, не заливка), тихий гексагон-watermark в углу карточки
  (двойной контур + лучи-трассы к вершинам, ~18% opacity), мелкие
  сигил-глифы у строк списка. Тёмная тема НЕ менялась — остаётся
  эталоном без правок.
- Статус: визуальный язык зафиксирован дизайн-мокапом, порт в
  `light_scheme()` (Rust-код, `crates/ui/src/theme/schemes.rs`) ещё
  НЕ сделан — отдельная кодовая задача. Не всё из мокапа поедет 1:1
  (watermark/glow-рёбра могут не иметь дешёвого GPUI-эквивалента) —
  переносим осознанно то, что реально ложится на слои.

  ## 2026-07-19 (ночь) — Мульти-монитор: chrome на один «пультовый» монитор, второй → холст (отход от традиционного DE)

- **Контекст.** Сейчас `bar/mod.rs` открывает бар на КАЖДОМ дисплее
  (цикл по `cx.displays()`), полный chrome клонируется на оба монитора —
  традиционная DE-модель. Весь класс багов «на каком мониторе всплывёт
  попап» (см. запись про `primary_display()==None` ниже по времени в
  HANDOFF / Zed №3 Phase 1) существует ТОЛЬКО из-за этого клонирования.
- **Рассмотрено и отклонено:** оставить традиционную модель «одинаковый
  бар на всех мониторах». Отклонено пользователем как DE-рефлекс: два
  одинаковых бара с одинаковыми часами/треем — на второй никто не
  смотрит, а мультимониторная возня с выбором дисплея под попапы —
  чистый налог этой модели.
- **Принято направление (пользователь, explicit):** отойти от
  традиционного десктопа. Один монитор — **пультовый** (chrome: top bar,
  side panels, попапы, лаунчер — вся управляющая поверхность). Второй
  монитор — **рабочий холст** под окна (ими рулит Hyprland, не мы) +
  desktop-виджеты (абсолютное позиционирование, background-layer, НЕ
  bar-widget — named consumer уже есть: `plasminal`, см. запись
  2026-07-10 про geometry-поля) + что придумаем позже.
- **Скоуп сейчас:** точим шелл ПОД пультовый монитор. Роль второго
  (виджет-холст) — **отложена**, придумываем когда пультовая часть
  готова. Не полусобирать: если выключить бар на втором сейчас, а холста
  ещё нет — второй просто пустой; принимаем это сознательно ИЛИ держим
  минимальное присутствие до холста.
- **Designation пультового монитора (механизм).** `cx.primary_display()`
  на Wayland/Hyprland возвращает `None` (нет канонического primary в
  протоколе — честный ответ, НЕ баг форка). Форк: `WaylandDisplay` держит
  `name: Option<String>` (DP-1/HDMI-A-1 от wl_output), а
  `uuid()->Result<Uuid>` выводит **стабильный across-reboot UUIDv5** из
  имени (`gpui_linux/.../wayland/display.rs:31`). Решение: пультовый
  монитор назначается по **uuid в конфиге** (`~/.config/chronos/`),
  переживает ребут; эвристика (напр. самый большой) — только дефолт на
  первый запуск, конфиг авторитетен. Опционально — 2 строки в Source
  (`fn name()` в трейт `PlatformDisplay` + wayland-impl, поле уже есть),
  чтобы конфиг был по-человечески `chrome_monitor = "DP-1"`, а не opaque
  uuid. Не блокер.
- **Побочный выигрыш:** consolidation (chrome → один монитор) СТИРАЕТ
  долг по 8 попапам (запись про Zed №3: тот же дисплейный баг во
  всех попапах) — при одном chrome-мониторе «на каком дисплее попап»
  вопроса нет. Фикс Zed'а (попап на дисплее кликнутого бара) остаётся
  верным и forward-compatible: при одном баре он всегда даёт пультовый.
- **Не берём (явно):** не пишем оконный менеджер (окна = Hyprland),
  не Plasma-activities, не проектируем виджет-холст до того как знаем
  какие виджеты туда идут.

════════════════════════════════════════════════════════════════════
2026-07-20 (ночь) — бар-редизайн руками Архитектора (разовый мандат)

- **Мандат.** Пользователь единоразово снял правило «архитектор не
  кодит» на остаток сессии («полная свобода действий»). Сделаны:
  лэйаут бара, SVG-иконочная инфра, project switcher (№9 снят с Mimo).
  Правило продолжает действовать со следующей сессии.

- **ashpd: фича async-io, НЕ tokio.** Форк gpui (gpui_linux) уже
  зависит от ashpd с `async-io`; cargo unification фич даёт
  compile_error «can't enable both async-io & tokio». Решение:
  app-крейт тоже берёт `async-io`, портал-вызовы гоняются через
  `async_io::block_on` в выделенном std-треде, результат в GPUI через
  tokio oneshot (просто future, executor-agnostic). Рассмотрено и
  отклонено: собственный tokio-runtime в треде (конфликт фич не
  обойти — фичи глобальны на крейт).

- **Git-ветка для project switcher: прямой парс `.git/HEAD`.**
  Рассмотрены: `git rev-parse` сабпроцессом (дорого на 1s-тикере),
  inotify на `.git/HEAD` (лишняя сложность + бар и так тикает
  каждую секунду для часов). Принято: читать `.git/HEAD` (~30 байт)
  прямо в render пилюли; воркткри (`.git`-файл с `gitdir:`) и
  detached HEAD (короткий хэш) обработаны. Смена ветки видна ≤1s
  без рестарта — подтверждено живьём.

- **MPRIS: idle-плеер без метаданных скрывается.** «▶ Unknown» при
  открытом браузере без медиа — шум. Если `!playing` и title+artist
  пусты → Hidden; если играет без метаданных → показываем player_id.
  «Unknown» остаётся только как fallback внутри format_track_label.

- **Иконки бара: свои SVG-ассеты, не шрифтовые глифы.** Эмодзи
  (⚙🔔🔊⬆⏻) визуально дешевили бар и не тонировались темой. Принято:
  AssetSource (include_bytes, крейт-локальные `assets/icons/*.svg`),
  line-art Phosphor-стиль + фирменные гексагон-сигилы, тонировка через
  text_color (альфа-маска SVG-рендера). Nerd-глифы network оставлены —
  они уже line-стиль и читаются. Рассмотрено и отклонено: иконочный
  шрифт (нет тонкого контроля размера/цвета в GPUI-элементах).

## 2026-07-20 — ОПРОВЕРЖЕНИЕ: скролл в форке ЕСТЬ, нужен `.id()` (отменяет часть записи 2026-07-19)

- **Что считалось истиной с 2026-07-19** (запись «Layer-shell popup sizing»,
  и оттуда — ARCHITECTURE §4.1, docs/STYLE.md, roadmap, skills/gpui-layer-shell,
  два брифа миньонам): «`.overflow_y_scroll()` does NOT resolve in this fork
  — clip works, scroll doesn't». Из этого выводилась целая линия дизайна:
  никаких прокручиваемых областей в попапах, только жёсткий клип, а список
  обновлений показывает первые N строк и `+N more (run checkupdates…)`.
- **Проверено 2026-07-20 (по прямому требованию пользователя «ты хоть в
  форке лазил?») — УТВЕРЖДЕНИЕ ЛОЖНО.** Методы `overflow_scroll` /
  `overflow_x_scroll` / `overflow_y_scroll` / `track_scroll` определены в
  `Source/gpui/src/elements/div.rs:1416-1440` и принадлежат трейту
  **`StatefulInteractiveElement`** (:1213), который реализован ТОЛЬКО для
  `Stateful<E>` (:3752). Элементу нужен `.id(...)` — иначе метода на типе
  нет, и компилятор выдаёт «no method», что и было принято за отсутствие
  фичи в форке. Диагноз перепутали с приговором.
- **Доказательства:** рабочий пример лежит в самом форке —
  `Source/gpui/examples/scrollable.rs` (`.id("vertical").overflow_scroll()`,
  вложенный горизонтальный скролл там же); `cargo check --example scrollable`
  зелёный. Косвенно: колесо до layer-shell поверхностей долетает — живой
  `on_scroll_wheel` в `bar/widgets/volume.rs` и `mpris.rs` работает с
  2026-07-17.
- **Что это меняет.** Скролл — легитимный инструмент, а не запретный.
  Конкретные следствия: (1) список обновлений может стать настоящим
  прокручиваемым списком, а костыль `+N more (run checkupdates for the full
  list)` — выпилен; (2) веха «наш терминал» получает полноценный scrollback;
  (3) любой попап с длинным содержимым больше не обязан выбирать между
  клипом и ростом окна.
- **Что НЕ меняется.** Жёсткий клип остаётся ПРАВИЛЬНЫМ по умолчанию для
  chrome фиксированной высоты (бар, компактные попапы): он структурен и не
  может «ошибиться», в отличие от пиксельной оценки — исходная запись
  2026-07-19 в этой части верна и остаётся в силе. Хвост вывода в
  `updates_popup` (Mimo №13) тоже остаётся хвостом: во время установки
  нужен последний вывод, а не чтение простыни. Скролл там — не выигрыш.
- **Урок процесса (важнее самого факта).** Ошибка прожила сутки и
  разошлась в 6 документов и 2 брифа, потому что Архитектор принял чужой
  вывод как канон и ни разу не открыл форк. Правило: **утверждение «форк
  чего-то не умеет» требует доказательства из исходников форка или
  примера, а не пересказа.** У нас в `Source/*/examples/` лежит готовый
  стенд — смотреть туда ДО того, как записывать ограничение в канон.

## 2026-07-21 — Палитра панели: Catppuccin по мокапу (ОТМЕНЯЕТ «сине-циан/без радуги»)

- **Контекст.** Sidebar v2 мокап `docs/design/System Sidebar.dc.html` + приёмка v2
  (`7109860`). Пользователь: «the panel colors are good».
- **Разворот.** Прежнее правило (DECISIONS 2026-07-20 / память «cool blue-cyan,
  forbids rainbow»: CPU `#5fd3e8`/RAM `#4fa3c9`/GPU `#33638a`) **отменено.**
- **Принято (из мокапа):** CPU `#89dceb`, RAM `#89b4fa`, **GPU `#f9e2af`
  (жёлтый)**, сеть `#6c7086`, disk/battery-полосы зелёный `#a6e3a1`, accent
  синий `#007acc`, danger/power красный `#f38ba8`, фон `#181825`, бордер
  `#313244`. Это семантический Catppuccin Mocha, не монохром.
- **Почему.** Панель — design-driven, эталон = мокап Claude Design; «без радуги»
  было прежним вкусовым ограничением, пользователь его снял живьём на v2.
- **Область.** Пока хексы прямо в `side_panel_right/*` (пиксель-в-пиксель).
  Маппинг на `Theme`-токены — отдельная фаза, не блокер.

## 2026-07-21 — Chronos-AUR: отдельное приложение экосистемы (Путь 2), НЕ модуль шелла

- **Контекст.** Alloy (Tauri 2 + React AUR/пакетный менеджер, `~/projects/
  chronos-ecosystem/Chronos-AUR`, MIT) портируется Tauri→GPUI. Вопрос: модуль
  внутри бинаря ChronOS или отдельное приложение.
- **Рассмотрено (взвешено оба):**
  - *Путь 1 — модуль в шелле:* ноль IPC, прямой реюз инфры; НО раздувает бинарь
    шелла (~3771 бэкенд + ~3735 фронт), краш пакетника роняет шелл, шелл-монолит,
    независимо не выпустить.
  - *Путь 2 — отдельное приложение:* шелл остаётся лёгким, изоляция крашей,
    независимый релиз, совпадает с паттерном экосистемы (Chronos-IDE/chronos-fm);
    цена — тонкий IPC + свой оконный каркас.
- **Решено: Путь 2.** Пользователь: «часть экосистемы, работает вместе с шеллом,
  не за счёт него». Ключ: самый мутный сквозной кусок (auth/пароль) решается
  **shell-polkit-агентом системно** (ловит любой pkexec без per-app IPC), поэтому
  IPC-цена Пути 2 падает до опциональных приятностей (бейдж апдейтов, «открой пакет»).
- **Реализация.** Rust-бэкенд `src-tauri/services/*` переиспользуем (крейт
  `aur-core`, снять `#[tauri::command]`, exec через `ShellExec` fish/zsh/bash),
  React-фронт (7 страниц) портируем в `gpui-rsx` (`aur-app` GPUI-бинарь). Форк
  git-депом (@99cab5e) + rsx + animation. `malware_check`/`pkg_analyze`/`pkg_build`
  — байт-верно. План: `Chronos-AUR/docs/port-plan.md`. Фаза-1 = 4 трека
  (Cline движок / Grok shell-exec / Hermes app-каркас / Zed страницы).
- **Отклонено:** Путь 1 (сшивает пакетник с процессом десктоп-шелла — монолит,
  которого экосистема избегает). Permission-карта в сайдбаре + polkit-агент —
  отдельный shell-side трек позже.

## 2026-07-22 — Оркестрация: per-agent журналы → per-task T-ID (по образцу Chronos-lm)

- **Контекст.** Пользователь сравнил метод работы с сиблинг-проектом
  Chronos-lm (`docs/ARCHITECT.md` + `docs/agents/{active,report,report-log,done,
  rejected}/`, tNN-задачи) и решил повторить у ChronOS. Разведка вскрыла
  реальную цену старой схемы: номера заданий не глобальны
  (`hermes-report-22` и `zed-report-7` несопоставимы напрямую), нумерация
  внутри одного файла расходится с HANDOFF («№3» vs «№4» для одной и той же
  задачи Mimo), несколько отчётов явно помечены в имени файла как брак
  (`zed-report-2-Phase2-DISCARDED...`), один архивный отчёт
  (`grok-report-3.md`) был молча перезаписан без объяснения.
- **Рассмотрено:** (1) полный гибрид — личные файлы агентов остаются брифом,
  добавить только сквозной T-номер; (2) полный перенос на per-task — личные
  файлы становятся тонкими указателями, реальный бриф/учёт живёт в
  `orchestration/tasks/`.
- **Решено: (2), полностью, с полной ретроактивной миграцией всей истории**
  (не только новых задач). Реестр T001–T106 (сквозная хронология, вся
  прежняя история first-минионской эпохи + редизайн бара + правая панель +
  открытые Chronos-AUR треки) — `orchestration/tasks/MIGRATION.md`. Роль
  архитектора переписана в новый `docs/ARCHITECT.md` (корень) — Role/I do/I do NOT
  (датированные уроки из реальных инцидентов, не абстрактная гигиена)/
  Authority order/lifecycle table/Wave map/Accept criteria/Language, скелет
  1:1 с Chronos-lm.
- **Спорные случаи истории решены явно, не молча** (полный список —
  `MIGRATION.md` преамбула): нумерация-алиасы (Mimo №3/№4, DeepSeek №2/№14)
  цитируют оба номера под одним T-ID; DISCARDED/rejected-варианты идут в
  `rejected/`, не в `done/`; дубликаты/rework-черновики — канон в
  `report-log/`, черновики в `notes/superseded/` с пометкой; кросс-катные
  не-таск аудиты (SHADER_PORT_AUDIT.md и т.п.) — без T-ID, в `notes/`.
- **Ограничение сохранено осознанно:** каждый минион-инструмент физически
  читает свой файл `orchestration/agents/<ИМЯ>.md` — это точка входа
  инструмента, не стиль. Полный переход на per-task реализован в рамках
  этого ограничения: файл агента остался, но стал указателем на текущий
  активный T-номер, не журналом.
- **Отклонено:** оставлять `.gitignore` не тронутым, но обсуждать — вопрос не
  в этой задаче, orchestration/ уже осознанно локальный с 2026-07-19
  (`78427d0`), решение не пересматривалось. Автоматический хук/линтер для
  дисциплины — не создавался (Chronos-lm тоже держит это на процессе/ревью
  архитектора, не на тулинге — сверено разведкой явно).

## 2026-07-23 — Left agent panel: мульти-агентный свитчер в хедере — ПРИНЯТО, в v1

Пользователь: "мульти-агентный свитчер в хедере левой панели — моё
изначальное решение, которое постоянно отклоняется агентами". Проверено:
ни в `docs/superpowers/specs/2026-07-23-left-agent-panel-design.md`, ни в
этом логе решение никогда не фиксировалось — ни как принятое, ни как
отклонённое с причиной. Молча терялось на каждом цикле планирования —
не осознанный отказ, а институциональная забывчивость. Зафиксировано
здесь явно, чтобы не повторилось третий раз.

**Решение:** хедер левой agent-панели получает выпадающий свитчер (клик
по названию текущего агента → список ACP-совместимых бэкендов: Hermes,
Cline, OpenCode, и т.д. — конкретный список уточняется на реализации).
Источник визуала — `docs/design/Agent Panel.dc.html` кадр 1 (`showAgentMenu`),
принят Claude Design 2026-07-23 без правок макета.

**Архитектурное следствие:** `crates/services/src/hermes_acp/` (спека
§5, T107) была спроектирована и реализована как ЖЁСТКО зашитый под один
бинарь `hermes acp`. Со свитчером это должно стать абстракцией над
несколькими ACP stdio-бэкендами (общий транспорт/клиент-трейт, конкретный
spawn-командой параметризуется, не хардкодится). T107 (принят,
`done/T107-left-agent-panel.md`) НЕ переоткрывается — это новая задача
поверх принятого фундамента, заведена как T108.

## 2026-07-23 — Left agent panel: hover-peek отключён в пользу keybind-toggle — ПРИНЯТО

Живая сессия после приёмки T109 (Agent Thread canvas). Пользователь вживую
попробовал текущее поведение (панель открывается по наведению на левый
край, закрывается по debounce при уходе курсора — T107-дизайн, зеркало
правой панели) и явно отверг его для ЭТОЙ панели: "хочу чтоб сайд бар
появлялся/исчезал по бинду, не исчезал автоматом... сайд бар из которого
я вытягиваю чат при нужде, причём мышкой настроил нужный удобный размер".

**Решение:** `hover_strip::init_hover_strip(cx)` закомментирован в
`side_panel_left::init` (НЕ удалён — код рабочий и юнит-протестирован,
опция на будущее, если ховер вернётся как ДОПОЛНИТЕЛЬНЫЙ режим поверх
бинда, не вместо него). Панель теперь открывается только явно: IPC
`toggle-side-panel-left` (зеркалит существующий `toggle-launcher`,
`crates/app/src/ipc/messages.rs`) → `side_panel_left::toggle(cx)` →
`open_pinned`/`close`. `pinned=true` всегда при открытии через toggle —
существующий `close_peek_if_not_pinned` debounce-механизм от T107
(`hold_peek`/`schedule_release_peek`) остаётся нетронутым и корректным
кодом, просто больше никогда не видит peek-состояние в проде, пока
hover-strip не включат обратно.

**Не сделано в этой сессии (сознательно, не забыто):** реальный
hyprland-bind (`~/.config/hypr/hyprland.lua`, кандидат `SUPER+A` по
короткому ответу пользователя в диалоге) — правка личного дотфайла
пользователя вне репозитория, оставлена ему, не редактировалась
архитектором. Дать точную строку по образцу существующего `SUPER+L`
(`toggle-launcher`, см. `hyprland.lua:129-131`) — следующий шаг
пользователя, не блокирует остальное.

## 2026-07-23 — Left agent panel: exclusive zone — ПОПРОБОВАНО и ОТКЛОНЕНО

В той же живой сессии пользователь сначала попросил: "окна не тайлятся
за неё... когда я вызвал сайдбар — окна подстроились" (панель ведёт себя
как бар — резервирует место, тайловые окна отодвигаются). Реализовано и
подтверждено вживую: `window_options` `exclusive_zone: Some(px(PANEL_WIDTH))`
+ **`exclusive_edge: Some(Anchor::LEFT)`** (кровный факт: наш якорь
`LEFT|TOP` — угол, а не растянутое ребро как у бара `LEFT|RIGHT|TOP`;
wlr-layer-shell трактует exclusive_zone на угловом якоре как
неоднозначный БЕЗ явного `set_exclusive_edge` — без него `hyprctl
monitors` показывал `reserved: [0,30,0,0]`, зона тихо игнорировалась,
никакой протокольной ошибки). С обоими вызовами `hyprctl monitors`
показал `reserved: [352,30,0,0]`, `hyprctl clients` подтвердил реальный
reflow тайловых окон (сдвиг x, сужение width).

**Отклонено после живой пробы того же вечера:** "иметь exclusive zone
должен быть только у бара. чат не должен толкать окна... это пиздец" —
двигать тайловые окна при каждом открытии/резайзе чат-панели, которую
держат открытой во время работы — плохой UX, отличается от бара
(открывается/закрывается редко, панель — часто и с живым мышиным
ресайзом). Откат: `exclusive_zone: None` в `window_options`, вызов
`window.set_exclusive_zone(...)` в `render()` убран целиком.
`exclusive_edge` тоже убран (бессмысленен без zone).

**Не закрыто, идея для отдельной задачи:** пользователь предложил
сделать это опциональным режимом, привязанным к ховеру (условно: "когда
работаешь с эдитором — тайлит, когда просто юзаешь комп — не тайлит").
Технически зона живо перевызываема (`Window::set_exclusive_zone`,
`gpui/src/window.rs:2005`, не create-time-only — проверено фактом в этой
же сессии), препятствий с API нет. Осознанно не реализовано поверх уже
метавшегося вечером кода — отдельная задача, если пользователь вернётся
к идее.

## 2026-07-24 — Полный hot-swap `crates/app` — ОТКЛОНЕНО, взят спайк-bake-off

Запрос сессии начинался как «hot patch/hot reload» в духе dev-инструмента
(правишь `.rs`, видишь эффект без рестарта). Уточнение объёма дало «весь
`crates/app`» — отклонено сразу же, до реализации: `crates/app` не dylib
(бинарная цель), а GPUI `Entity`/подписки (`cx.subscribe`/`cx.observe`)
держат указатели на код текущего артефакта — выгрузка старого dylib при
живых подписках была бы use-after-free по всей карте сущностей (bar/dock/
launcher/notifications/osd/tray_menu/обе side-панели/попапы). Второй
стоппер — `unsafe_code = "deny"` в воркспейсе (`Cargo.toml:29`, осознанное
правило) несовместим с dylib-reload на весь крейт без грубого нарушения.

**Решение:** вместо выбора подхода вслепую — спайк-сравнение (тот же
протокол, что дал результат с `gpui-component` recon+pilot,
docs/DECISIONS.log 2026-07-21). Track A (`hot-lib-reloader`, вынос
render-функции `bar/widgets/network.rs` в отдельный dylib-крейт
`crates/hotview`, вне `workspace.lints`) vs Track B (`subsecond`-стиль
function-level патчинг без выноса в крейт) — оба на одном полигоне,
10 одинаковых правок, метрика — краши + время «сохранил→увидел».
Спека: `docs/superpowers/specs/2026-07-24-dev-hot-reload-bakeoff-design.md`.
Роздано T110 (OpenCode, Track A) / T111 (GLM, Track B), оба в изолированных
ворктри (`ChronOS-wt-hotreload-{a,b}`, ветки `spike/hot-reload-track-{a,b}`).
Проигравший трек архивируется веткой, не удаляется.

## 2026-07-24 — Настоящая цель ChronOS: Shell-IDE, не Plasma-редактор

Запрос эволюционировал от «hot reload» → «Plasma-style edit mode» →
после прямого вопроса пользователя оказался третьей, гораздо более
конкретной вещью: правая панель (`side_panel_right`, уже принятая System
Sidebar v2) становится **таб-контейнером на 10 вкладок** (System + Files +
Editor(Kate-стиль) + Terminal + ACP/MCP/LSP/API-provider/Editor settings +
Hyprland binds), левая (`side_panel_left`, T107-T109) уже физически агент
— это и есть «нажал кнопку — в IDE», без отдельного режима-переключателя.

Явно НЕ Plasma: пользователь назвал Plasma «потомком Microsoft», отверг
жёсткие applet-слоты и «выглядит из коробки как говно». Референс для
будущей системы позиционирования — Noctalia v5 (`docs.noctalia.dev/v5/`),
но не как чертёж — см. отдельное решение по bar-layout ниже.

**Решение по объёму первой задачи:** MCP/LSP/API-providers/Hyprland-binds
не имеют backend-сервиса в дереве вообще (проверено грепом
`crates/services/src/` перед планированием) — значит первая задача НЕ
может быть «сделать все 10 вкладок». Роздан только фундамент (rail +
System-таб без изменений + 9 честных заглушек «Coming soon»), план
`docs/superpowers/plans/2026-07-24-ide-panel-tab-container.md`, T112
(DeepSeek). Остальные 9 вкладок — отдельные будущие спеки/задачи,
каждая со своим backend где нужно.

**Побочная находка при ревью дизайн-экспорта:** первый мокап
(`docs/design/banani-ui-export.zip`, PNG, инструмент вне нашего пайплайна) —
ОТКЛОНЁН целиком: чужая палитра (не Catppuccin Mocha) И полностью
выдуманный код на GTK4 в кадре Editor (проект на GPUI, GTK4 в дереве нет
нигде). Второй заход (`docs/design/shell-ide-panel.zip`, Next.js-превью,
`components/panel/theme.ts` — буквальные канон-хексы) — принят после
ручной правки: тот же gtk4-фрагмент выжил в `editor-tab.tsx` нетронутым
из первой попытки, заменён на GPUI-стиль псевдокод перед коммитом
(`545bcbb`).

## 2026-07-24 — Bar widget layout: конфиг поверх существующей lane-модели, не новая система

`crates/luau/src/bar.rs` `BarSection::{Left,Center,Right}` — уже ровно та
lane-модель, что даёт Noctalia v5 (`start/center/end`); порядок внутри
секции просто жёстко забит вызовами в `bar/widgets/mod.rs:26`
(`register_builtin`). Решено НЕ проектировать позиционирование с нуля —
вынести существующую модель в `bar.toml` (порядок виджетов по секциям)
с hot-reload, паттерн 1:1 с `theme.toml` (GLM №2, 2026-07-20). GUI
drag-and-drop редактор, создание новых панелей на произвольном крае,
per-monitor оверрайды — явно вне этой фазы, отдельные будущие спеки
поверх готового конфига. Спека:
`docs/superpowers/specs/2026-07-24-bar-widget-layout-config-design.md`.
Не роздано миньону — спека написана, план и T-задача впереди.

## 2026-07-24 (вечер) — Hot-reload bake-off решён: Track A (hot-lib-reloader), не Track B (subsecond)

Track B (`subsecond`, function-level патчинг без выноса в отдельный
крейт) отклонён НЕ из-за нестабильности — сама механика патчинга
(`subsecond::call`) безопасна и прозрачна. Отклонён из-за API: доставка
патча идёт через `subsecond::get_jump_table()`/`apply_patch()`, оба
`pub unsafe fn`. Вызов из `crates/app` требует `unsafe {}` в коде шелла —
прямое нарушение `unsafe_code = "deny"` (`Cargo.toml:29`), причём unsafe
НЕ инкапсулирован внутри `subsecond` (условие брифа), а протекает в
вызывающий код. Дополнительно: ThinLink (линкер, генерирующий jump-table)
существует только внутри Dioxus CLI, standalone-раннера нет;
`cargo-hot` (альтернативный раннер) сам себя маркирует "Very broken! Will
eat your laundry!". Воспроизведено архитектором лично (та же unsafe-ошибка
на строках `main.rs:72-73` в ворктри `spike/hot-reload-track-b`) — не
голословный отчёт GLM.

Track A (`hot-lib-reloader` + отдельный cdylib-крейт `crates/hotview`) —
0 крашей за 10 правок, ~2 сек сохранил→увидел, unsafe (если есть)
изолирован в собственном `[lints]` крейта, не задевает workspace-политику.
Смержен в master (`b07eacd`, fast-forward после merge в чистом
temp-worktree — основное дерево содержало битый symlink
`skills/fable-domain` от незавершённой реорганизации скиллов, не
относящейся к этой задаче, обошли через `git worktree add --detach`).
Track B архивирован веткой `spike/hot-reload-track-b`, не удалён
(прецедент `pilot/gpui-component-spike`, см. запись выше 2026-07-21).

Находки задокументированы как скиллы (уже существовали к моменту
приёмки, авто-сгенерированы миньонами, сверены архитектором с
воспроизведённой ошибкой и живым прогоном): `skills/hot-lib-reloader/`
(setup + pitfalls), `skills/evaluating-hot-reload-solutions/` (почему
subsecond не годится для non-Dioxus проектов с `unsafe_code=deny`).

## 2026-07-24 (вечер) — IDE-панель rail: правый край экрана, не левый

T112 изначально свёл rail как первый child в `flex_row` (рядом с
контентом, у внутреннего/левого края право-докнутой панели). Пользователь
указал на визуальную ошибку: для панели, прижатой к правому краю экрана,
rail должен быть у САМОГО края (последний child), контент — между rail и
десктопом, а не наоборот — иначе панель читается "развёрнутой не в ту
сторону". Исправлено точечно (порядок `.child()` в `view.rs` + бордер
`rail.rs` `border_r_1`→`border_l_1`), проверено живым смоком, не
переоткрывает остальной фундамент T112.

## 2026-07-24 (вечер) — Позиционирование проекта: не очередной end-4/ml4w/dank-shell конфиг

Пользователь явно сформулировал: ChronOS — не ещё один Hyprland rice/
config-конфиг поверх готовых инструментов (end-4/dots-hyprland, ml4w,
dank-material-shell — все три построены на связке существующих CLI-тулов
+ Quickshell/AGS, не на собственном GPU-рендерере). ChronOS — первого
лица продукт на GPUI (незанятая ниша: ни один популярный Hyprland-шелл
не написан на GPU-accelerated retained-mode тулките с нуля), делается
"для себя, зная что нужно многим" — то есть личный инструмент с прицелом
на публичный резонанс/community, не приватный дотфайл-репозиторий.
Следствия для приоритизации: (1) plugin-экосистема должна реально уметь
то, чего нет у Quickshell/AGS-конфигов (иначе нет причины мигрировать) —
см. находку "Plugin API v2" тем же вечером; (2) публичная документация/
README должны быть готовы к чужим глазам раньше, чем обычно бывает
достаточно для личного проекта; (3) WASM как второй plugin-рантайм
ОТКЛОНЁН тем же вечером — не потому что "экзотика", а потому что не
решает реальное узкое место (см. запись выше).

---
## 2026-07-25 — Popup polish wave (T120–T125) + service coalesce

### Considered / rejected
- **Parallel fire-and-forget DDC/wpctl on every drag sample** — rejected:
  out-of-order re-reads jump UI for minutes; i2c storms set `available=false`.
- **Shared `DragMoveEvent<T>` marker for multiple sliders** — rejected:
  GPUI delivers moves to every listener of type T → one knob drives all.
- **`background_spawn` without `.detach()`** for async notification dispatch —
  rejected: Task cancelled on drop, Close/Clear are no-ops (T120).
- **getvcp after every brightness set** — rejected for drag path; keep only
  on `Refresh` / init, generation-gated against concurrent Set.

### Decided
- **Popup discipline (post-T117):** AnchoredPopup BottomRight+BottomLeft,
  grab, SLIDE_X|FLIP_X, LayerShell fallback; bar trigger `on_mouse_down` +
  canvas bounds + `.relative()` wrapper.
- **Audio volume Set:** optimistic Mutable + `tokio::sync::watch` latest-wins;
  no `pw-dump` on command path; poll 250ms still owns device lists.
- **Brightness Set:** optimistic + **debounce ~150ms** then single
  `write_all(latest)`; follow-up only if target changed mid-write; MVP still
  one slider → all DDC displays (`write_all`).
- **Fork development:** ChronOS `[patch."Chronos-GPUI"]` → path deps
  `../Source/*` (full crate graph). `gpui_animation::init` public (Delta 4).
- **Dev CLI (T122):** `chronos-{rebuild,reload,stop,start,debug}` in-repo
  under `scripts/dev/`, install to `~/.local/bin`; docs `docs/dev-cli.md`.
- **T-briefs:** agent-agnostic T-IDs only (no personal names in brief titles).

### Still open / not decided
- Per-monitor brightness UI.
- Enter/exit toast animation; critical pulse.
- Whether tray_menu gets same anchored redesign next.
- Untracked WIP `crates/app/src/toast/` — not wired; T124 used
  `notifications/view.rs` instead.

## 2026-07-25 — Left panel: sessions sidebar is the bar; chat overlay + dock switch

Supersedes the open item under 2026-07-23 exclusive-zone (optional hover
mode). Live product decision (Architect session 2026-07-25):

### Rejected
- **`is_rail` / status-dot strip** (`PANEL_RAIL_WIDTH` ~26px + green dot
  when `width ≤ PANEL_RAIL_TOTAL_WIDTH`): not the product. Dragging the
  panel to min must **not** replace the sessions UI with that rail.
- **Default exclusive = full chat width** (already rejected 2026-07-23):
  chat must not shove tiled windows on every open/resize.
- **Two layer-shell windows** (rail surface + chat surface): extra
  focus/keyboard/ghost complexity; one window is enough because
  `exclusive_zone` is a px distance from the edge, not “the whole surface”.

### Accepted model
1. **Sessions sidebar is the sidebar** — always present when Super+A is
   open. Modes: expanded list (`SIDEBAR_EXPANDED` ~200) ↔ **collapsed icon
   strip, slightly narrower than today's 48** (target **~36**: icon buttons
   ~28 + padding; not the 26px status-dot rail).
2. **Super+A** opens the sessions sidebar (collapsed or last-used width).
   Default exclusive zone = **sidebar width only** (`exclusive_edge: LEFT`).
3. **Pull out ACP chat** (resize handle / open thread): window grows;
   exclusive **stays at sidebar width** — chat overlays tiles.
4. **Dock switch** (UI toggle): exclusive = **full panel width** immediately
   (windows reflow). Off → exclusive back to sidebar width. While docked,
   exclusive tracks resize.
5. **Min window width** = current sessions-sidebar width + resize handle —
   never the old `PANEL_RAIL_*` floor. Collapse-to-chat-off = sidebar-only
   width, sessions UI intact.
6. **API:** live `Window::set_exclusive_zone` + `set_exclusive_edge(LEFT)`
   (fork already has both; no GPUI work). Gate updates when zone changes
   (same pattern as `last_resized_width`).

### Sizing note
Collapsed sessions: **36px** (was 48). Expanded: keep ~200 unless mockup
says otherwise. Exact icon metrics free to tweak live; product constraint
is “narrower collapsed sessions, not a different rail chrome”.

## 2026-07-25 — Right panel: tab rail bar + content overlay + dock (mirror left)

Supersedes the same-day draft that said "exclusive always while open".

Product (Architect session): **same behaviour as left ACP panel**, mirrored
to the right. Bind: **Super+G** (IPC `toggle-side-panel-right`; hyprland.lua
line for the user — same socket pattern as Super+A left).

### Mapping left → right

| Left (T126) | Right (T127) |
|---|---|
| Sessions sidebar | **Tab rail** (`rail.rs`, ~44px icons) |
| ACP chat column | **Tab content** (System / stubs / …) |
| Super+A | **Super+G** |
| exclusive_edge LEFT | **exclusive_edge RIGHT** |
| Dock switch | Dock switch (same semantics) |

### Accepted model
1. **Tab rail is the bar.** Super+G opens **rail-only** width
   (`RAIL_WIDTH + HANDLE`). Exclusive zone = rail width only.
2. **Content panel has no exclusive by default.** Pull content out
   (resize handle / open content): window grows; exclusive **stays at
   rail width** — content overlays tiles.
3. **Dock switch ON** → exclusive = full panel width (tiles under
   content). OFF → exclusive back to rail width.
4. **Live resize** handle on the **inner** edge (left of right panel).
   While docked, exclusive tracks width; while overlay, exclusive stays
   rail.
5. **Min width** = rail + handle. Never a status-dot substitute for the
   rail — the rail **is** the chrome.
6. **Close** → `set_exclusive_zone(0)` before remove.

### Rejected
- Exclusive always = full content width (earlier T127 draft) — same
  "chat shoves windows" problem as left 2026-07-23.
- Overlay-only with no rail exclusive.
- Two layer-shell windows (rail + content).

### API
`set_exclusive_zone` + `set_exclusive_edge(RIGHT)` on corner
`TOP|RIGHT` anchor. No fork work. Pattern: T126 left after errata.

## 2026-07-25 — Visual depth: tokens → motion → 3D spike (T128–T132)

User wants shell polished for daily use (not Reddit-first). "3D effects"
clarified as three layers, sequenced:

1. **Soft depth / glass** (T128): theme elevation + blur tokens; apply popups
   and panel content chrome. No new scene primitives.
2. **Motion from depth** (T129–T130): enter/exit scale+opacity via existing
   `gpui_animation`; toast motion after panel/popup pattern. Never animate
   layer-shell exclusive_zone.
3. **Real 3D** (T131–T132): optional fork spike (new scene primitive + WGSL
   via BlurRect template) + one demo surface in shell. Not daily-driver
   blocker; not bar-wide mesh.

**Rejected:** gpui-d3rs as the 3D path (depends on Zed git gpui + charts API);
full Bevy/egui-wgpu dual loop beside layer-shell.

**Product priority:** T128 first; T129–T130 next; T131–T132 only after polish
wave is stable.

## 2026-07-25 — Wallpaper: integrate waytrogen, do not rewrite it

**Accepted:** ChronOS owns the **wallpaper engine** (`services/wallpaper` via
`awww`, IPC `wallpaper-next` / `wallpaper-set`, `wallpaper_ctl` folder
cycle). **Waytrogen** remains a separate optional **gallery GUI** (Unlicense
donor for CLI contracts only — already in NOTICE). Product stance:
cooperate and integrate (`exec waytrogen` / companion install opt-in), not
port their UI into GPUI and not brand their picker as first-party.

**Rejected:** embedding/shipping waytrogen as "ChronOS wallpaper manager";
hard package dependency; full in-shell gallery rewrite of waytrogen.

**Follow-up:** T133 — open_gallery + IPC `wallpaper-gallery` + minimal UI
entry + docs/companion wording.

## 2026-07-25 — T133 brief correction: waytrogen GUI is first-class

Earlier draft of T133 read as "spawn binary + two shell buttons" and
implicitly sidelined waytrogen's product. **Corrected:** we do not rewrite
waytrogen *and* we do not replace or omit their GUI. Integration means their
full gallery app is the Browse path; ChronOS keeps engine/next/set; resync
after their GUI; companion CTA when missing; no GPUI fallback gallery.

## 2026-07-26 — Theme panels critical closed; T129 motion parked

**Accepted (product):**
- Left/right side panels themed via `Theme` tokens; Light C surface roles on
  right (`surfaces::chrome/card/well`) — light is not mocha inverted 1:1.
- Super+Shift+T / `theme.toml` / IPC toggle-theme for daily light/dark.
- User live grim dark+light (both panels content open): critical OK.

**T129 enter motion — PARKED (not accepted):**
- Tried: `gpui_animation::transition_when`, closed-base + arm-after-paint,
  native `with_animation`, popup `enter_t` notify loop (`aeff604`…`ce6fff3`).
- Live: layer-shell panels **do** slide in (`with_animation`); anchored
  popups never got a trustworthy enter; exit is compositor fade on
  `remove_window` (not ChronOS animation).
- **Rejected for now:** further motion thrash; user closed the thread
  («забей»). Code left in tree for a future re-brief; do not start T130
  on this partial foundation.

**Docs:** tails → `docs/TBD.md`; ops → `docs/HANDOFF.md` (this date block).

## 2026-07-26 — Canon doc roles (explicit)

| Doc | Owns |
|---|---|
| `docs/ARCHITECTURE.md` | settled architecture (amend when structure changes) |
| `docs/DECISIONS.log` | append-only considered/rejected/accepted decisions |
| `docs/HANDOFF.md` | current field / T status / what not to re-smoke |
| `docs/TBD.md` | non-blocking polish + wishlist (no T-ID required) |
| `docs/MEMORY.md` | durable cross-session facts (not daily queue) |
| `docs/roadmap.md` | quarterly order of work |

## 2026-07-26 — Edit Mode + hot-reload front (not Plasma)

**Context:** User asked for "edit mode" + wire hot-reload. Prior DECISIONS
(2026-07-24) already rejected Plasma-style editor as product identity and
full `crates/app` dylib hot-swap.

**Accepted direction (design):**
- **Edit Mode** = runtime flag + chrome affordances for **layout
  customization**, config-backed (`bar.toml` first).
- **Config hot-reload** (theme/dock pattern) = product "live without restart".
- **Dev hotview** (`hot-lib-reloader`) = separate developer path; expand later
  (T136), do not conflate with Edit Mode.
- Phase 1 = T134 (bar.toml + EditMode shell + minimal move UI).
- Phase 2 = drag (T135). Phase 3 = hotview expand (T136).

**Rejected again:** Plasma applet slots; full app hot-swap; subsecond.

**Spec:** `docs/superpowers/specs/2026-07-26-edit-mode-and-hot-reload-design.md`
builds on `2026-07-24-bar-widget-layout-config-design.md`.

## 2026-07-28 — Thread persistence: SQLite locally, agent stays source of truth

**Context:** Threads in the left agent panel live only in memory
(`sessions: Vec<SessionItem>`, titles literally `Session N`). Closing the
panel or restarting the shell loses everything; "+" already meant losing
context. Reference point given by the user: Zed keeps threads in
`~/.local/share/zed/threads/threads.db`.

**What Zed does** (schema read off the live DB, 43 threads / 5.3 MB):
single table `threads(id TEXT PK, summary, updated_at, data_type, data BLOB,
parent_id, folder_paths, folder_paths_order, created_at)` — whole thread
serialized as one zstd blob, `parent_id` for forked threads, `folder_paths`
for the project it belongs to.

**Why we do not copy it as-is:** ACP already persists conversations on the
agent side, and Hermes 0.18.2 exposes them — it advertises
`load_session=True` (`acp_adapter/server.py:888`), implements `session/load`
with full history replay (`:1133`), `session/resume` without replay, and
`session/list` with cwd filter + cursor pagination returning
`{session_id, cwd, title, updated_at}` (`:1249`). Zed's blob-only model
would fork the truth: our copy and the agent's copy of the same conversation.
Second difference: we are multi-agent (`agents.toml`, T138) — a thread
belongs to *an agent*, which Zed's schema has no column for.

**Accepted:**
- **Agent owns conversation content.** Resuming a thread = `session/load`
  against the agent that owns it. We never reconstruct a session from our
  copy and hand it back.
- **We own everything the agent cannot know**: which agent a thread belongs
  to, our title / renames, pin, archive, order, last used model, and a
  cached transcript for instant rendering and offline browsing when the
  agent process is not running.
- **Store:** SQLite (`rusqlite`, bundled) at
  `~/.local/share/chronos/threads/threads.db`. First DB in the project —
  chosen over per-thread JSON because thread search (FTS5) is wanted and
  comes free, and over `sled`/`redb` because the on-disk format stays
  inspectable with `sqlite3` when something goes wrong at 2am.
- Cached transcript is a **cache**, explicitly: on conflict with what the
  agent replays, the agent wins.

**Rejected:** storing only metadata and always paying a `session/load`
round-trip to show a thread (list is instant, opening would not be, and an
agent that fails to start would show empty history); mirroring Zed's single
blob table (forks the truth, no agent column).

**Tasks:** T150 (store + service, `crates/services`), T151 (UI: real thread
list, load/resume, rename/pin/archive, search) — T151 depends on T150.

## 2026-07-28 — `gpui-component` пересмотрен: берём как СВОЙ крейт, режем под себя (шлюз по замеру)

- **Контекст.** Запись 2026-07-21 («Тело правой панели») отклонила
  `gpui-component` по цене: from-scratch замер Архитектора дал **+2.66 MiB
  (+13.2%)** бинаря, обрезать было нечем — `markdown`, `html5ever`,
  `markup5ever_rcdom`, `lsp-types`, `ropey` объявлены в
  `crates/ui/Cargo.toml` безусловными `[dependencies]`, не за фичами.
  В той же записи стояло условие пересмотра: «пересмотреть, если
  launcher/settings/пр. массово захотят Button/Table/Input».
- **Условие наступило.** В дереве три самописных обработчика клавиш
  (`launcher/view.rs`, `desktop_terminal/view.rs`,
  `side_panel_left/composer.rs`), T149 добавил четвёртую поверхность ввода
  (поиск моделей), T154 требовал пятую — каретка, выделение, IME, буфер
  обмена, ~400 строк с нуля. В компоненте `crates/ui/src/input/` —
  **12 213 строк** ровно про это.
- **Что изменилось со стороны цены.** Копия компонента лежит в нашем форке
  (`Source/gpui-component`, 787 файлов под git, апстрим не отслеживается —
  `docs/ARCHITECTURE.md`). «Обрезать нечем» — утверждение про чужой крейт, не
  про свой: безусловные депы делаются опциональными правкой их же
  `Cargo.toml`.
- **Решено (пользователь, 2026-07-28): компонент правим под себя.** Не
  «аккуратно потребляем библиотеку», а забираем в дерево: депы `git =
  zed-industries/zed` → path-крейты форка (и тогда `[patch]`-костыль у
  потребителей уходит), ненужные модули режутся физически, спорные — за
  фичи, тема маппится на нашу палитру.
- **Ограничение — Chronos-FM.** Тянет тот же крейт по git-пину
  `Dark-Ohm/Chronos-GPUI rev ee80b72` (`Chronos-FM/Cargo.toml:57`), три
  его крейта на нём висят. Физически режем только заведомо ненужное обоим
  (`webview`, `native_menu`); `table/`, `dock/`, `sidebar/`, `resizable/`,
  `list/` не трогаем. Пин не двигаем, `Source` не пушим — подъём соседа
  отдельным решением.
- **Шлюз сохранён.** «Правим под себя» — свобода резать, не право не
  мерить. T155 останавливается после обрезки и сдаёт три числа: базовый
  бинарь, с полным компонентом, с обрезанным. Решение «берём» принимается
  по ним. Прецедент: цифра миньона в июле была занижена вчетверо, и
  решение поехало на ней.
- **Следствие для очереди.** T154 (свой ввод в композере) → `pause`:
  если компонент заходит, его `input/` закрывает почти всё задание.
  Разморозить, если T155 отклонена по замеру.

## 2026-07-29 — `gpui-component`: заход отменён, форк не гробим (T155 заморожена)

- **Контекст.** Решение от 2026-07-28 (запись выше) открыло дорогу: берём
  компонент как свой крейт, режем под себя, шлюз по замеру.
- **Что вышло на практике.** Исполнитель успел неплохое: раскладку
  `[features]` (`lsp`/`markdown`/`html`/`chart`/`time`), перевод деп
  компонента с `git = zed-industries/zed` на path-крейты форка, находку
  «`input/` завязан на `lsp`/`Position`/`HoverDefinition` глубже, чем
  видно снаружи». И два стопора: (1) фичи объявлены, `#[cfg(feature)]` в
  коде не расставлены → 10 × E0432, сборка мертва; (2) `gpui-component`
  добавлен членом `Source/Cargo.toml`, сохранив собственный `[workspace]`
  → `multiple workspace roots`, **любой `cargo` внутри `Source/` падал**,
  включая пример T152.
- **Почему остановились.** Агент выдохся и предложил «вырезать всё кроме
  файлов лицензий и запустить чистую сборку» — перебор наугад в дереве,
  общем с Chronos-FM. Решение пользователя: **форк не гробим.**
- **Откат.** `Source: git checkout -- Cargo.toml gpui-component`;
  `ChronOS: git checkout -- Cargo.toml Cargo.lock crates/ui/Cargo.toml`.
  После отката `cargo metadata` в `Source` — exit 0. Диффы сохранены в
  скратчпаде, работа не выброшена.
- **Условие возврата** (не «попробовать аккуратнее»): работа только в
  отдельном worktree форка — общее дерево не должно быть грязным ни
  минуты; и обратный порядок работ — сначала `cfg`-гейты по коду, потом
  выключение фич по одной с замером каждого шага.
- **Следствие.** T154 (своё поле ввода в композере) разморожена: раз
  `input/` из компонента не пришёл, каретку/выделение/копипаст пишем сами.

## 2026-07-29 — Перенос `gpui-component` разморожен, но разбит на три задачи

- **Контекст.** Запись выше заморозила T155 после того, как один заход
  «проводка + обрезка + потребитель» развалился. Пользователь решил
  перенос продолжить — но не тем же способом.
- **Что именно провалилось, а что нет.** Провалился не замысел, а размер
  захода. Исполнитель сделал осмысленную часть (раскладка `[features]`,
  перевод деп компонента с `git = zed-industries/zed` на path-крейты
  форка) и умер на скучной — расстановке `#[cfg(feature)]` по коду.
  Плюс сломал общее дерево `Source/`, работая прямо в нём.
- **Решено: три задачи вместо одной.**
  - **T156** — только `cfg`-гейты в компоненте, **в отдельном worktree**
    (`git worktree add ../Source-wt-component`). Ни ChronOS, ни общее
    `Source/` не правятся. Приёмка — матрица из семи `cargo check` по
    комбинациям фич плюс release-сборка (в debug состав кода другой).
  - **T157** — проводка в ChronOS и замер, со шлюзом: три числа
    (база / полный компонент / обрезанный), решение «берём» за
    архитектором.
  - **T158** — потребитель (`Input` вместо самописного ввода композера).
- **Порядок работ перевёрнут намеренно:** сначала гейты в коде, потом
  выключение фич. В T155 было наоборот — фичи объявили, код не разметили,
  получили 10 × E0432.
- **Отклонено: делать это в общем `Source/`.** На нём висит T152 (гоняет
  пример `hebrew_wrap_test` оттуда) и живёт Chronos-FM. Сломанный
  воркспейс форка в T155 останавливал обе линии сразу.
- **Разведка сделана архитектором заранее** (карта деп по файлам, образец
  гейтинга `tree-sitter` в самом крейте, ловушка `inspector` +
  `debug_assertions`) и вписана в бриф — чтобы заход не утонул в поиске
  того, что уже известно. Детали — `docs/HANDOFF.md`, раздел «Где стоим».
- **База замера:** `target/release/chronos` = 22 475 648 байт на `44d365e`.

## 2026-07-29 — gpui-component берём как инфраструктуру IDE-панели (реверс июльского «вариант C»)

- **Контекст.** Июльское решение (21.07, «вариант C — рисуем сами») содержало
  условие пересмотра: «Пересмотреть, если launcher/settings/пр. массово
  захотят Button/Table/Input». Условие сработало дважды. Сперва по вводу: в
  дереве накопилось три самописных обработчика клавиш, T149 сделал четвёртую
  поверхность ввода, T154 требовал пятую. Затем — по общему направлению:
  подтверждено, что строим **полноценный IDE-shell**, а не только
  агент-панель.
- **Что изменилось за сутки и почему это НЕ отменяет решение.** T154 сдал своё
  поле ввода — **484 строки** (каретка, выделение, буфер, IME/utf16, drop,
  границы слов). Для композера этого достаточно, и повод «нам нужен Input»
  формально закрыт руками. Но `input/` компонента — **17 301 строка**: rope
  вместо `String`, display_map со свёртками, история правок, подсветка,
  маски, числовые поля. Разница не в каретке, а в том, что наш вариант не
  масштабируется до редактора кода.
- **Решающий довод — не ввод, а остальное.** Компонент целиком 89 275 строк, и
  в нём лежит ровно то, что стоит в очереди IDE-вкладок (T113 терминал, T114
  настройки ACP, T115 файлы): `table`, `tree`, `virtual_list`, `dock`, `form`,
  `setting`, `select`, `combobox`, `sidebar`, `tab`, `resizable`,
  `notification`. Виртуальный список, таблица с сортировкой и док с
  перетаскиванием вкладок — это не спринт, а квартал ручной работы, причём с
  тем же классом дефектов, что мы сейчас ловим в RTL.
- **Решено: берём.** Компонент — наш крейт (worktree `Source-wt-component`,
  ветка `component/feature-gates`), режем под себя. T156 (cfg-гейты) закрыта:
  `markdown/html/time/chart/lsp` выключаемы, `lsp` тянет `markdown`, ловушка
  инспектора закрыта. T157 (замер) переопределён — см. ниже. T158 (обрезка +
  настоящий потребитель) в силе.
- **Цена, которую принимаем осознанно.** Не мегабайты, а сопровождение: мы
  владеем форком и сводим его с апстримом Longbridge руками. T156 стоила день
  работы и четыре цикла исправлений отчётов. Это плата за то, чтобы не писать
  таблицу и док самим.
- **Что это меняет в замере.** Мерить `Button` (как сделал первый заход T157)
  или даже один `Input` — бессмысленно: линковаться будет связка
  **`Input + Table + VirtualList`**. Дельта от неё и есть настоящая цена
  входа; пороги шлюза пересчитаны в задании T157.
- **Отклонено повторно:** писать таблицу/дерево/док руками. Мотив тот же, что
  в июле у варианта C (тонкий бинарь), но при подтверждённом курсе на IDE он
  проигрывает: экономия мегабайта против квартала работы и своего пласта
  багов.

## 2026-07-31 — Files берём из Chronos-FM, yazi отвергнут

  Вопрос: чем наполнять вкладку Files в слайсе 4. Предложение автора —
  взять `yazi` (терминальный файловый менеджер на Rust) и «прописать ему
  GUI».

  Лицензия проверена: yazi под **MIT**, с нашим Apache-2.0 совместим,
  юридических препятствий нет. Отвергнуто не по лицензии.

  Причина отказа: yazi — TUI. Весь его слой отображения терминальный и при
  переносе выкидывается целиком. Остаётся логика обхода файловой системы и
  превью — ровно то, что уже написано в `../Chronos-FM` на **нашем же**
  GPUI-форке: `chronos-fm-services/src/fs/` 584 строки (`listing.rs`,
  `ops.rs`) и `chronos-fm-pages/src/explorer/` 3566 строк. Менять рабочее
  на переписанное незачем.

  Дополнительный факт в пользу Chronos-FM: разъезд форков минимален —
  Chronos-FM на `ee80b72`, ChronOS на `99cab5e`, между ними два коммита в
  одну сторону. Chronos-FM отстаёт, а не разошёлся.

  Решение: вкладка Files собирается из кода Chronos-FM. Объём и границы
  переноса определяет разведка T175 — если она покажет, что перенос дороже
  написания с нуля, решение пересматривается на её цифрах.

  Что при этом остаётся верным про yazi: он прекрасно работает **внутри**
  нашего PTY. Когда появится вкладка Terminal (слайс 4), запустить в ней
  yazi можно бесплатно и без единой строки кода. Это не заменяет нативную
  вкладку, но и не запрещено.

## 2026-08-03 — T227: JetBrains Mono на самом деле применён на корне каждого окна

Коррекция к T215 (`4a7d9dd` «ui : JetBrains Mono shell-wide including
gpui-component»): T215 поменял только **данные** дефолтной темы
(`font_ui: "JetBrains Mono"`) и прописал шрифт в тему gpui-component, но не
применял его на корнях окон. Ни одно собственное окно ChronOS не ставило
семейство шрифта на корне — поэтому шелл рисовался дефолтным шрифтом GPUI,
а «shell-wide» была иллюзией (редактор/инпуты получили шрифт через тему
gpui-component, и дерево выглядело «шелл-широко»).

T227 (commit `ui : apply theme font at every window root (T227)`) завёл
хелпер `WindowRootExt::window_font` в `crates/ui` и применил его на
корневом `div` каждого окна; per-element `.font_family(font_ui)` убраны из
`system_popup`. `font_mono` оставлен осознанно там, где нужен моноширинный
смысл (tool-карточки, вывод команд, гуттер). Дисциплина закреплена тестом
`every_window_root_uses_window_font` в `crates/ui/src/window_root.rs`,
который читает исходники корней окон и запрещает ручной `font_family(font_ui)`.

## 2026-08-05 — T245: auto-designate перестал перезаписывать существующий monitor.toml

**Корень инцидента («шелл переехал на HDMI-A-1») — НЕ нестабильный uuid.**
Форк строит `d.uuid()` как `UUIDv5(NAMESPACE_DNS, wl_output.name)`
(`gpui_linux/…/wayland/display.rs:31`) — чистую функцию имени коннектора
(`DP-1`/`HDMI-A-1`), стабильную между запусками и hotplug-циклами.
Проверено математикой: uuid конфига `56f01978-…` = `uuid5(NS_DNS,
"HDMI-A-1")`, канонический uuid из шапки `monitor.rs` `09e7b298-…` =
`uuid5(NS_DNS, "DP-1")`. Конфиг был **переписан** на HDMI (mtime
02:52:34) — это и есть «переезд».

**Причина переписывания — auto-designate-крысиный капкан в
`pult_display()`.** При временном отсутствии DP-1 в `cx.displays()`
(DPMS-сон Samsung ночью / поздний `wl_output Done` после 100ms-колла
`bar::init`) фолбэк largest-by-area выбирал единственный доступный
HDMI-A-1, и код перезаписывал `monitor.toml` на его uuid. После этого
uuid-матч детерминированно сажал шелл на HDMI каждую загрузку.

**Решение:** запись конфига только на true first run (`should_auto_designate`:
`existing.is_none() && winner_uuid.is_some()`). Существующий uuid —
источник истины, никогда не перезаписывается; при временном отсутствии
настроенного дисплея — WARN + работа на фолбэке без записи. Смена
монитора — правкой `monitor.toml` или удалением файла.

Отклонено: смена ключа uuid→connector name (uuid уже есть чистая функция
connector name — проблема была не в ключе). Принятый остаточный риск:
first-run на свежей установке при спящем большом мониторе в момент
первого 100ms-колла может задизайнить меньший (и это перманентно) —
задокументировано в `monitor.rs`; живая проверка 5/5 с существующим
конфигом.

## 2026-08-13 — T252: единый паттерн empty-state для вкладок правой панели

Зависимость выполнена (T246/T248/T249 слиты), паттерн выведен из уже
исправленных примеров, как и предписывал тикет. Полный аудит по вкладкам —
в `active/T252-empty-state-pattern-audit.md`; здесь — само решение.

**В дереве после фиксов сложилось шесть приёмов пустоты** (все живые,
проверены чтением кода, не памятью):

1. **Hero** — центрированные иконка + заголовок + пояснение (+ опциональная
   ссылка-действие): Preview «No file selected», Terminal Failed, Library
   «No games detected», EmptyTab.
2. **Inline-строка** внутри карточки/секции: Files (Loading/Error/
   «Directory is empty»/truncated-баннер), Build (три строки), HyprBinds,
   BarSettings «No modules found», disks «нет дисков».
3. **Compact-collapse** — большой виджет без данных схлопывается до строки:
   mpris «No player» (T248).
4. **Скрытие секции целиком** — отсутствие опциональных данных это норма,
   не событие: GPU-строка (`when_some`), секции Pinned/Recent в Library.
5. **Растягивание карточки до низа вьюпорта** — анти-void для малого, но
   живого контента настроек: ACP (T249, `min_h` от вьюпорта).
6. **Honest placeholder** — нереализованная вкладка: EmptyTab с уникальным
   описанием без сроков и «coming soon» (спек §13).

**Решение — матрица выбора, а не один виджет.** Код разный по разумной
причине: приём выбирается семантикой пустоты, а не вкусом исполнителя.

- Пуста **вся поверхность**, состояние ожидаемое → hero. Пояснение
  обязано отвечать «откуда здесь появится контент» (Library: «Games appear
  from XDG .desktop files with Categories=Game…» — образец).
- Пуста вся поверхность, и это **отказ** → hero + `status.error` +
  recovery-действие, если оно реально существует (Terminal: restart).
  Действие без бэкенда запрещено — T246-прецедент: кликабельность без
  действия = обман, такое вычищаем, а не копируем.
- Пуста **одна секция** внутри живой вкладки → вопрос ценности:
  отсутствие информативно («нет модулей», «нет плеера») → inline-строка,
  а если виджет крупный — compact-collapse (правило T248: не резервировать
  полноразмерную геометрию под отсутствующие данные). Отсутствие — норма
  и ничего не говорит (нет GPU, пустой Pinned) → секция скрывается целиком.
- Контент вкладки закончился, вьюпорт длинный → последняя карточка
  растягивается до низа (правило T249), голый фон под контентом — дефект.
- Вкладка не реализована → EmptyTab-placeholder: уникальное описание, без
  дат, статусов разработки и прогресс-баров.

**Планки-запреты** (все из уже оплаченных инцидентов):

- Empty ≠ error. Пустой результат — `text.muted`; сломанный источник —
  `status.error`. Единственное осознанное исключение — HyprBinds, где
  0 биндов гарантированно означает сломанный конфиг и потому рендерится
  как Error. Копировать этот ход на другие вкладки только с таким же
  обоснованием, иначе «0 игр» начнёт орать красным.
- Error объясняет затронутую возможность и путь восстановления
  (образец: Build «No tasks found. Looked in: …», HyprBinds
  «check ~/.config/hypr/modules/») — спек §13 дословно это и требует.
- Моки запрещены безусловно (T246). Ни «демо-данных», ни муляжей кнопок —
  ни в дефолтных состояниях, ни за флагами с живым видом.
- Язык UI — английский, **кроме локали даты/времени** (планка уточнена
  при приёмке 2026-08-13): русские месяцы в часах (`MONTHS_RU`,
  `power_row.rs`, дубль в `bar/widgets/clock.rs`) — осознанный формат
  локали, согласованный с баром, а не UI-копия. Не «чинить» их на
  английский — панель разъедется с баром. Настоящий дрейф — блок дисков
  целиком: четыре русские строки («нет дисков» + кнопки
  «монтировать»/«размонт.»/«извлечь» в `disks.rs`), чинится в follow-up
  одним куском. (В первой редакции этой записи «нет дисков» была названа
  единственной русской строкой панели — фактическая ошибка, поймана
  архитектором при сверке с деревом.)

**Материализация — отдельным тикетом** (этот — решение, не код). Аудит
нашёл дрейф копипасты, который паттерн обязан закрыть хелперами: hero
размножён вручную 4 раза с разъехавшимися параметрами (иконка 40 vs 32px,
gap 12/10/8, opacity 0.55 у иконки — через раз), inline-строка — 5+ раз
(px10/py16/text-12 у Files и HyprBinds, px8/py6 у Build, bordered-xs у
BarSettings, голый text-11 у disks). Follow-up: два хелпера в
`crates/app/src/side_panel_right/tab/ui.rs` — `empty_state_hero(theme,
icon, title, hint, action?)` и `empty_state_note(theme, message,
severity)` — замена всех перечисленных вхождений на них, фикс «нет
дисков», юнит-тесты на невозможность пустого title у hero. Коммит
`ui : unify empty-state pattern across right panel tabs (T252)` —
после приёмки этого решения.

Уточнения приёмки (2026-08-13, архитектор): эталон параметров hero —
`EmptyTab` (иконка 40px `muted.opacity(0.55)`, заголовок 13px SEMIBOLD,
подсказка 11.5px, gap 12px); иконка передаётся в хелпер готовым путём,
вызывающий подставляет `tab.icon_path()` — «пустой Library» и
«нереализованный Scenes» обязаны читаться как одна семья; `EmptyTab::render`
схлопывается в один вызов хелпера, чтобы канон не размножался копипастой.
Контекстные иконки Preview (`folder.svg`) и Terminal Failed
(`rail-terminal.svg`) — осмысленная вариация, не дрейф.

---
## 2026-08-14 — T279: мёртвый breakpoint-хелпер и lease через content_view

- Considered: оставить `chat_layout_for_visible_width` + 4 зелёных теста
  как «контракт» visible-width, не вызывая его из `render_panel`.
- Rejected: T278-театр. Тест зелёный при любом проде. Удалено в r2;
  прод уже ветвится по зеркалу `visible_w` → `state.width`.
- Considered: coordinator достаёт Chat через `content_view` /
  `WorkspaceView` изнутри `on_sessions_event`.
- Rejected: `entity_map::lease` / `double_lease_panic` — `content_view`
  уже leased. Решение: отдельный `SidePanelLeftState_.chat`.
- Considered: чистые `session_select_transition` / `project_switch_transition`
  как тестируемое ядро (игнор snapshot-аргументов).
- Rejected: безусловный return входа/константы. Тест обязан звать
  `select_session` / `switch_project` по имени на `&mut App`.

---
## 2026-08-15 — gpui-component: новые контролы шелла оттуда, не руками

- **Контекст.** Крейт уже наш (2026-07-29, инфраструктура IDE-панели).
  Лаунчер (T275 / волны T265) снова упёрся в ввод, кнопки, меню, списки.
- **Решено:** новый интерактив в шелле берём из `gpui-component`
  (`Input`, `Button`/`IconButton`, `PopupMenu`, `List`/`VirtualList`,
  `Select`, скроллбар). Свою каретку, свой тумблер pin, свой второй
  popup-menu — не писать, если в крейте уже есть.
- **Не значит:** включать `markdown`/`chart`/`lsp` «на всякий»; тащить
  `dock`/`table` в OSD-лаунчер; переписывать рабочие самописные куски
  в том же тикете (композер слева, список T265-0). Замена — отдельной
  волной.
- **Цена та же:** окно с виджетом компонента = `gpui_component::Root` +
  `OnDemand`. Без `Root` паника на `window.root()`.
- **Отклонено:** «нарисуем Input за вечер» (T154 закрыл каретку, не
  редактор); тащить `side_panel_left/text_input.rs` в лаунчер.

Эррата путей (тот же день): воркетри `Source-wt-component` и ветка
`component/feature-gates` не существуют. Кит — `Source/gpui-component/`
в Chronos-GPUI, гейты в `main` с `57f582f`. Скилл
`gpui-component-in-chronos` больше не указывает на мёртвый path.

---
## 2026-08-15 — Frame Hide|Wrap это Appearance, не пресет бара

- Considered: ещё один пресет бара / смена shell-геометрии вместе с T268.
- Rejected: T268 уже Hide-path (`d572657`). Wrap — другая тема оформления
  при спрятанных рельсах; клиенты сидят внутри рамы; рама живёт и с
  рельсами внутри. Hyprland-конфиг не трогать.
- Decided: сегмент Appearance Frame Hide|Wrap (T284). Один мат + три
  exclusive dummy. `set_margin` нет — recreate. style=строка+RMW.

---
## 2026-08-15 — T287-C: Zed-хром срезать, Follow не убивать

- Considered: выкинуть весь ряд `✦＋☰👁⋯` вместе с close X и внутренним
  Sessions rail.
- Rejected for Follow: 👁 — это T195 `AgentFollowState` → Preview,
  нужная фича. Переезжает вниз, к композеру, нормальная иконка
  (`icons/rail-preview.svg`), тот же `follow_enabled`.
- Decided: T287-C = rail + thread-header + close X out; Follow в
  composer-pickers-row. Пикеры моделей — T287-A, не этот срез.

---
## 2026-08-15 — IA: два gaming, вкладки вместо бар-попапов, pacman apply

- **Два gaming не склеивать.** Perf Gaming = `GamingModeState` (производительность, T291). Shell Gamer = `WorkspaceMode::Gamer` (рельса/сцены/док, T292). Имена в брифах обязательны.
- **Бар-попап System разбирается:** яркость+обои налево (T290); power+Perf Gaming направо в System (T291); пустой попап снести (T246).
- **Колокольчик и updates** не теряют бейдж на баре, но открывают вкладки (T293/T294), не попапы.
- **Updates apply:** `pkexec pacman -Syu` / `-Sy pkgs`. `yay` только read (`-Qua`) и hover-подсказка. Отклонено: `pkexec yay -Syu` (сейчас дефолт если yay на PATH).
- **Правый dock (T289):** T221 «docked + same tab = no-op» отвергнут владельцем. Dock = exclusive zone, не замок контента.
- **Календарь (T295):** kit `Calendar`, не своя сетка; не планировщик событий.
- Спеки в `active/T288`–`T295` + T265-A…G. Код не в этом чекпоинте.

---
## 2026-08-16 — Display не на левой рельсе

- Considered / shipped T290: `LeftTab::Display` (спека чекпоинта #14:
  «яркость+waytrogen налево»).
- **Rejected владельцем:** слева только ИИ; справа ежедневное и ОС.
  Display = настройки дисплея, v1 яркость+фон, расширять эту же вкладку.
  Кнопка в **нижней** группе правой рельсы, над System settings; над доком
  — T292 (не вкладка). Бар-hexagon яркости снести (T246).
- Decided: T296 `a2c072f` (live +). T290 оставлен как снос попапа +
  `gaming_mode` в корне; сторона SUPERSEDED.
- Sanitize `panels.toml` без `display` пока дописывает в **top** — не
  блокер (живой toml уже bottom).

## 2026-08-16 — Chronos-launcher: самостоятельность — намерение, не задача

- Владелец: хочет **когда-нибудь** вынести `crates/app/src/launcher/` в
  отдельный проект Chronos-launcher (тот же путь, что прошёл Chronos-FM —
  сначала внутри экосистемы, потом отдельный бинарь с заменой системного
  аналога, здесь — замена Wofi на `super+r`).
- **Не сейчас.** Открыт T297 (эррата самого лаунчера — submenu за границей
  окна, live favorites не обновляются, категорий слишком много) и T287
  (chat на ките) — extraction без потребителя отвлекает от них.
- Разведка стоимости на словах (без кода): сервисное ядро
  (`applications/frecency.rs`, `.desktop`-парсинг) отделяется почти без
  изменений; трение — `dock`/`state`/`power` в `crates/app/src/launcher/`
  читают общий `AppState` шелла напрямую, extraction = либо тащить весь
  `state.rs`, либо резать IPC-границу лаунчер↔шелл (pin-sync с доком,
  system actions). Отдельный бинарь всё равно path-dep на форк GPUI —
  легче не станет, вопрос только автономности процесса.
- Триггер к пересмотру: конкретный спрос на standalone (юзер другого WM,
  или решение сделать `super+r` независимым от шелла-процесса).

## 2026-08-16 — Упаковка экосистемы: meta-package в AUR, не сейчас

- Владелец: когда проекты дозреют до паблиша, `yay -S chronos-ecosystem`
  ставит всё разом; `yay -S chronos-fm` / `chronos-lm` / `chronos-ide` /
  `chronos-launcher` (когда отделён) — по отдельности.
- Схема: каждый проект — свой PKGBUILD/AUR-репо (как уже T282 для
  `chronos-shell-git`). `chronos-ecosystem` — отдельный AUR-репо,
  meta-пакет без сборки, только `depends=(chronos-shell chronos-fm
  chronos-lm chronos-ide ...)` — имена в `depends=` должны буква в букву
  совпадать с реальными именами пакетов (особенно `-git` суффикс).
- Нюансы на будущее: `chronos-lm` — не «поставил бинарь», нужны
  `post_install()`/`post_upgrade()` хуки (xsessions/greetd, systemd-юнит,
  PAM). Имя `chronos` как пакет вероятно занято/generic — проверить
  `yay -Ss chronos` перед регистрацией, скорее всего `chronos-shell`.
  Один AUR-репозиторий = один пакет, meta-пакет не подкаталог.
- **Не сейчас** — проекты кроме ChronOS не готовы к паблишу. Фиксация на
  память, тикет не заведён.

## 2026-08-16 — Кухня: по репо + отдельная экосистемная

- **Рассмотрено:** (1) одна экосистемная кухня на всю работу, репошная
  оркестрация остаётся `docs/orchestration/`; (2) только репошные кухни,
  экосистемной нет; (3) у каждого репо своя `.chronos-ops/`, экосистемная
  — отдельная, только кросс-репо.
- **Решено: (3).** Нумерация не общая: ChronOS продолжает свои TNNN,
  экосистема — свои (wt-tools T001–T003 не равны ChronOS T001).
- **Отклонено (1):** скелет ChronOS нёс текст «не заменяет
  orchestration» — это правило экосистемной кухни, не репошной. Одна
  очередь на пять репо снова смешает зоны.
- **Отклонено (2):** wt-tools, drift/digest, кросс-репо тикеты некуда
  класть без второго уровня.
- Cutover ChronOS `docs/orchestration/` → `ChronOS/.chronos-ops/` —
  отдельный заход, не пока T287-B в поле.
- **2026-08-16, уточнение:** скелет `.chronos-ops/` коммитим (иначе
  git не откатит — урок 2026-07-22). Очередь в git переезжает потом,
  по одному тикету.


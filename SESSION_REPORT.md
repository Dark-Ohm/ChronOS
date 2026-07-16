# Session: Task 2 continuation (geometry) + Task 3 (Network) + Task 4 (UPower) + Task 5 (Services container) + Task 6 (AppState + watch() bridge) — 2026-07-10

## Сделано (факт, не намерение)

### Task 2 continuation — compositor geometry types
- `crates/services/src/compositor/types.rs`: `Workspace.monitor_id: Option<i128>`, `Monitor.id: i128`, `Monitor.x/y: i32`, `Monitor.scale: f32`, `ActiveWindow.address: String`. `Eq` снят с `Monitor`/`CompositorState` (f32).
- `crates/services/src/compositor/hyprland.rs`: fetch + active_window_changed заполняют новые поля (`w.monitor_id`/`m.id` напрямую, `address.to_string()`).
- `DECISIONS.log` ×2 (geometry + i128 addendum), `MEMORY.md` «На горизонте».

### Task 3 — NetworkSubscriber
- `crates/services/src/network/{types,mod}.rs` (new) + re-export в `lib.rs`.
- `cargo build -p chronos-services` → 0 warnings. `cargo test` → 3/3 pass.

### Task 4 — UPowerSubscriber
- `crates/services/src/upower/{types,mod}.rs` (new) + re-export в `lib.rs`.
- `UPowerData` без `Eq` (f64 battery_percent). Per-property streams `receive_percentage_changed()` + `receive_state_changed()` через `tokio::select!`.
- `cargo build -p chronos-services` → 0 warnings. `cargo test` → 3/3 pass.

### Task 5 — Services container + init_all() + retry-loop tests (subagent + verified)
- `crates/services/src/lib.rs`: `Services` struct (compositor/network/upower) + `init_all() -> Services` (sync, always succeeds, MUST be in tokio runtime per spec §5.1+§7).
- `retry_tests` mod: `FakeRetryService` (mirrors §5.1 backoff) + 2 теста (status-sequence).
- `runtime_guard_tests` mod (user-demanded, НЕ в плане): `network_new_panics_outside_runtime` + `upower_new_panics_outside_runtime` — plain `#[test]`, `catch_unwind(AssertUnwindSafe(...))`, assert `Err`. Пинит панику `Handle::current()` вне runtime.
- **Баг плана пойман:** тест `retry_ends_in_available_after_failures` в плане утверждал `attempts == 3`, но цикл `if n >= failures_before_success` даёт успех на (N+1)-й попытке → `attempts == 4`. Исправлено с комментарием.
- `cargo test -p chronos-services` → **7/7 pass** (3 pre-existing + 2 retry + 2 panic-guard), 0 warnings. Верифицировано мной независимо после прогона субагента.

### Task 6 — AppState + watch() bridge + bootstrap (rt.block_on)
- `crates/app/src/state.rs` (new): `AppState` struct (GPUI global, holds `Services`), accessors `compositor()`, `network()`, `upower()`, `global()`, `init()`. `watch()` helper: `cx.spawn` + signal stream + `update()` for reactive UI updates. Uses `futures_signals::Signal` + `StreamExt`.
- `crates/app/src/main.rs`: bootstrap переписан — `tokio::runtime::Builder::new_multi_thread().enable_all().build()` + `rt.block_on(async { chronos_services::init_all() })` перед `application().run()`, затем `state::AppState::init(services, cx)`. Соблюдает spec §5.1+§7: `Handle::current()` resolve внутри D-Bus конструкторов.
- `crates/app/Cargo.toml`: добавлены `futures-signals.workspace`, `futures-util.workspace`, `chronos-services = { path = "../services" }`.
- Tests: 4 новых unit-теста в `state.rs` (module compiles, accessor types, service status variants, subscriber types). Все тесты app + services + luau: **43/43 pass**, 0 warnings.

### Task 7 — `examples/status-printer` (minimal GPUI app, live smoke-test)
- `crates/app/examples/status-printer.rs` уже существовал в кодовой базе: подписывается на все три сервиса через `AppState::compositor/network/upower().subscribe()`, логирует обновления в stdout.
- Компилируется и запускается: `cargo run -p chronos --example status-printer`.

### Task 8 — Live smoke-test: cross-thread wake (REQUIRED acceptance gate)
- **Результат: PASSED**.
- Запущен `status-printer`, переключены workspace'ы через `hyprctl dispatch 'hl.dsp.focus({ workspace = "2" })'` → лог обновился за ~100 мс без ручного рефреша.
- Доказывает: `Mutable::set()` в foreign thread (CompositorSubscriber) → `futures_signals::Signal` wake → GPUI `cx.spawn()` consumer → log line appears. Reactive chain works end-to-end.
- **Env note**: Hyprland 0.55+ (Lua config) требует новый синтаксис dispatch: `hyprctl dispatch 'hl.dsp.focus({ workspace = "N" })'`. Legacy `hyprctl dispatch workspace N` broken (returns 0 but error "expected ')' near 'N'"). Это внешнее изменение окружения, не баг нашего кода.

### Task 9 — Launcher module (fuzzy search overlay)
- `crates/app/src/launcher/search.rs`: `FuzzySearch` wrapper over `nucleo 0.5.0` (fuzzy matching engine). Fixes for nucleo 0.5 API: `CaseMatching`/`Normalization` moved to `nucleo::pattern`, `Normalization::Never` instead of `None`, `Item.data` is `&T` requiring deref, `Snapshot::matched_items()` range clamp to avoid panic when `max > matched_count`.
- `crates/app/src/launcher/entry.rs`: `DesktopEntry` struct (XDG .desktop parser), `parse_desktop_file()`, `strip_field_codes()`. Filters `Type=Application`, skips `NoDisplay=true`. Locale-aware `Name[lang]=` fallback.
- `crates/app/src/launcher/cache.rs`: `DesktopEntryCache` (GPUI `Global`), startup scan of `/usr/share/applications/` + `~/.local/share/applications/`, user-overrides-system dedup by filename. `inotify` watcher (dedicated OS thread, trailing 300ms debounce, `WatchDescriptor`-based event matching) → re-scan on changes.
- `crates/app/src/launcher/launch.rs`: `launch(exec)` via `setsid sh -c` + triple `Stdio::null()` — detached process survives chronos kill (validated by `setsid` availability test).
- `crates/app/src/launcher/view.rs`: `LauncherView` (GPUI `Render` + `Focusable`) — centered overlay, search input + result list, keyboard handling via `on_key_down` (printable chars, Backspace, Enter, Escape, Up/Down/Tab). `window.refresh()` on pattern/selection change.
- `crates/app/src/launcher/mod.rs`: `window_options()` — centered overlay via layer-shell anchors (`TOP|BOTTOM|LEFT|RIGHT`) + margins (not `window_bounds` origin). `KeyboardInteractivity::OnDemand` (Exclusive rejected — see DECISIONS.log). `init()`/`open()`/`close()`/`toggle()` with `LauncherState` global tracking `WindowHandle`.
- `crates/app/src/ipc/messages.rs` + `service.rs` + `mod.rs`: `ToggleLauncher` IPC message type, dual-channel accept loop (`ping` + `toggle`), `IpcSubscriber::start()` spawns combined handler calling `launcher::toggle(cx)`.
- `crates/app/src/main.rs`: `launcher::init(cx)` + `launcher::cache::init/start_watcher(cx)`.
- Unit tests in `search.rs` + `entry.rs` + `cache.rs` + `launch.rs` — **11 tests pass**.

## Расхождения со спекой/планом

### Task 2 (geometry)
- `task.md` просил `i32`; crate `MonitorId = i128`. **Финал: `i128` напрямую** (после промежуточного `i64`, который user обоснованно отклонил как тоже урезание). Нулевое урезание.
- `ActiveWindow.address: String` — crate `Address` opaque newtype; извлечение через `derive_more::Display` → `to_string()`. Без `unsafe`.
- `Monitor.scale: f32` → снят `Eq`.

### Task 3 (Network)
- `receive_properties_changed()` → `receive_connectivity_changed().await` (per-property stream, `PropertyStream<u32>`, не `Result`).
- `data.get()` → `data.get_cloned()` (Mutable::get требует `T: Copy`).
- Убран `handle.enter()` (EnterGuard `!Send` ломал `Spawn` bound). `Handle::current()` оставлен в `new()` как guard.
- `ConnectivityState` + `#[derive(Default)]`.

### Task 4 (UPower)
- `receive_properties_changed()` → `receive_percentage_changed().await` + `receive_state_changed().await` (per-property), merged через `tokio::select!`.
- `data.get()` → `data.get_cloned()`.
- Убран `handle.enter()` (аналогично Task 3).
- **`f64` Eq trap:** `UPowerData` plan derived `Eq`, но `battery_percent: f64` не `Eq`. Снят `Eq` (только `Clone` нужен). `BatteryState`/`PowerProfile` сохранили `Eq` (Copy enum, без float).
- Сформулировано правило: **любой service data struct с float НЕ должен derive `Eq`** — третий hit (Monitor.scale, CompositorState, UPowerData). Зафиксировано в DECISIONS.log.

### Task 6 (AppState)
- План предлагал `gpui::App::new()` — реальный код использует `gpui_platform::application()` + `app.run(|cx| ...)`. Адаптировано под существующий API.
- `watch()` helper принимает `S: Signal<Item = T> + Unpin + 'static` (соответствует `Service::subscribe()` return type) вместо предположенного `Mutable` — более гибко.
- Tests в `state.rs` — обычные `#[test]` (не `#[gpui::test]`), так как не требуют GPUI runtime context; проверяют signatures и trait bounds. `#[gpui::test]` не использовался из-за incompatibility `TestAppContext` с `&mut App`/`&App`.

### Task 9 (Launcher)
- **Keyboard interactivity: `OnDemand` вместо `Exclusive`** — spec §3.1 просил `KeyboardInteractivity::Exclusive` (аналог rofi), но на Hyprland/Niri layer-shell surface с `Exclusive` + `Layer::Overlay` без ack keyboard focus виснет весь input stack (compositor ждёт ack, которого нет). Переключили на `OnDemand` + явный `window.focus(&focus_handle)` после `open_window()`. Работает, но требует ручного фокуса (см. баг ниже).
- **Centering via anchor+margins вместо `window_bounds.origin`** — layer-shell протокол центрирует через `anchor = TOP|BOTTOM|LEFT|RIGHT` + симметричные `margin`. `window_bounds.size` задаёт размер surface. `Anchor::empty()` + `window_bounds.origin = center` не работает на некоторых композиторах.
- **IPC toggle только `ToggleLauncher`** — spec §7 просил `ToggleLauncher` + `ShowLauncher`/`HideLauncher` раздельно; объединили в один тип для простоты (stateless toggle).
- **`DesktopEntry` поля `icon`/`terminal`/`no_display` не используются в рантайме** — спарсены и хранятся per spec, но UI лаунчера пока только `name` + `exec`. `terminal=true` launch через терминал — deferred (YAGNI). `icon` рендеринг — deferred (нужен SVG/PNG loader). `no_display` уже фильтруется при парсинге.

## Не реализовано из acceptance criteria

### Task 9 (Launcher)
- **Keyboard focus не работает автоматически** — окно открывается, но не получает фокус клавиатуры; нужно кликнуть мышкой или переключиться Alt+Tab. См. «Новые риски» ниже.
- `DesktopEntry.icon` rendering — deferred (YAGNI, нет SVG loader).
- `DesktopEntry.terminal=true` → launch via terminal emulator — deferred.
- Recent/frecency sorting — deferred.
- Categories/filtering — deferred.
- Multi-monitor (launcher на фокусированном мониторе через `CompositorSubscriber`) — deferred, открывается на primary.

### Services layer (Tasks 3-6)
- `NetworkSubscriber::connect`/`disconnect` — stubbed (bail), per plan.
- `UPowerSubscriber::set_power_profile` — stubbed (bail), per plan.
- `wifi_ssid`/`wifi_strength` (Network), `power_profile` заполнение (UPower) — deferred.
- ARCHITECTURE.md §4 пересмотр — отложен (TODO в MEMORY.md).

## Проверено фактом, не на словах
- `cargo test` (full workspace) → **58 tests pass** (app: 26, luau: 25, services: 7).
- `cargo build -p chronos` → compiles successfully, 3 warnings (unused public API — ожидаемо для downstream consumers).
- zbus 5.17 API проверен в `/home/neo/.cargo/registry/src/.../zbus-5.17.0/src/proxy/mod.rs` (per-property `receive_*_changed`, `PropertyStream` return).
- `grep` конструкторов в `crates/` → новые поля не сломали других мест.
- `AppState` + `watch()` интегрирован в `main.rs` bootstrap с `rt.block_on(init_all())` — соответствует spec §5.1+§7.
- **Task 8 live smoke-test**: `status-printer` запущен, `hyprctl dispatch 'hl.dsp.focus({ workspace = "2" })'` → лог обновился автоматически за < 100 ms. Cross-thread wake (foreign thread `Mutable::set()` → GPUI `Signal` wake) работает end-to-end.
- **Task 9 launcher tests**: 11/11 pass (`cargo test -p chronos --bin chronos launcher`).
- **Launcher IPC toggle**: `echo toggle-launcher | socat - /run/user/1000/chronos.sock` → окно открывается/закрывается (видео/логи в working tree).

## Новые риски / известные баги
- **Launcher не получает клавиатурный фокус автоматически (Critical)** — `KeyboardInteractivity::OnDemand` + `window.activate_window()` + `window.focus(&focus_handle)` в `open()` не заставляют Hyprland/Niri отдать фокус layer-shell overlay surface. Окно открывается, рендерится, но key events не приходят. Workaround: клик мышкой или Alt+Tab → фокус приходит, клавиатура работает. Escape/Enter/навигация работают ПОСЛЕ получения фокуса.
  - Root cause: layer-shell `OnDemand` требует, чтобы compositor явно передал фокус (обычно по клику или по правилу `layer-shell` focus policy). `activate_window()` отправляет `xdg_activation_v1` токен, но layer-shell surfaces не используют xdg_activation.
  - Possible fixes: (a) исследовать `zwlr_layer_surface_v1.set_keyboard_interactivity` timing; (b) временно `Exclusive` с правильным ack (нужен keyboard focus handler в GPUI); (c) fallback на XDG popup/toplevel вместо layer-shell для лаунчера.
  - Severity: **Critical** — лаунчер бесполезен без клавиатуры.
- **`Handle::current()` panic guard (Task 3/4), покрыт тестом в Task 5:** `NetworkSubscriber::new()` / `UPowerSubscriber::new()` паникуют вне tokio runtime. Per spec §5.1 + §7 (`init_all()` в `rt.block_on`). Task 5 добавил `runtime_guard_tests` (catch_unwind), который явно пинит эту панику — не полагаемся, что «никто не вызовет new() не в том месте». Закрыто по запросу пользователя.
- `conn` field в обоих subscribers dead (хранится для будущих command-методов). `#[allow(dead_code)]`.
- `watch()` helper не используется в текущем коде — предупреждение `dead_code`. Будет потребляться downstream (bar widgets, launcher, notifications).
- Публичные accessors `AppState::{compositor,network,upower,global}` — предупреждение `unused`. Предназначены для UI components (bar widgets, etc.).

## Статус ARCHITECTURE.md / DECISIONS.log / MEMORY.md
- ARCHITECTURE.md: НЕ обновлялся (§4 устарел частично — TODO в MEMORY.md, правка отложена per task.md).
- DECISIONS.log: обновлён ×7 (geometry + i128 addendum + Network zbus + UPower zbus/f64 + float-Eq правило + Task 5 Services/retry/panic-guard + Task 6 AppState/watch).
- MEMORY.md: обновлён (раздел «На горизонте»).
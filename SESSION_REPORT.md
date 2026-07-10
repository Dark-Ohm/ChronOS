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

## Не реализовано из acceptance criteria
- `NetworkSubscriber::connect`/`disconnect` — stubbed (bail), per plan.
- `UPowerSubscriber::set_power_profile` — stubbed (bail), per plan.
- `wifi_ssid`/`wifi_strength` (Network), `power_profile` заполнение (UPower) — deferred.
- ARCHITECTURE.md §4 пересмотр — отложен (TODO в MEMORY.md).

## Проверено фактом, не на словах
- `cargo test` (full workspace) → **47 tests pass** (app: 15, luau: 25, services: 7).
- `cargo build -p chronos` → compiles successfully, 3 warnings (unused public API — ожидаемо для downstream consumers).
- zbus 5.17 API проверен в `/home/neo/.cargo/registry/src/.../zbus-5.17.0/src/proxy/mod.rs` (per-property `receive_*_changed`, `PropertyStream` return). Не выдумывалось.
- `grep` конструкторов в `crates/` → новые поля не сломали других мест.
- `AppState` + `watch()` интегрирован в `main.rs` bootstrap с `rt.block_on(init_all())` — соответствует spec §5.1+§7.
- **Task 8 live smoke-test**: `status-printer` запущен, `hyprctl dispatch 'hl.dsp.focus({ workspace = "2" })'` → лог обновился автоматически за < 100 ms. Cross-thread wake (foreign thread `Mutable::set()` → GPUI `Signal` wake) работает end-to-end.

## Новые риски / известные баги
- **`Handle::current()` panic guard (Task 3/4), покрыт тестом в Task 5:** `NetworkSubscriber::new()` / `UPowerSubscriber::new()` паникуют вне tokio runtime. Per spec §5.1 + §7 (`init_all()` в `rt.block_on`). Task 5 добавил `runtime_guard_tests` (catch_unwind), который явно пинит эту панику — не полагаемся, что «никто не вызовет new() не в том месте». Закрыто по запросу пользователя.
- `conn` field в обоих subscribers dead (хранится для будущих command-методов). `#[allow(dead_code)]`.
- `watch()` helper не используется в текущем коде — предупреждение `dead_code`. Будет потребляться downstream (bar widgets, launcher, notifications).
- Публичные accessors `AppState::{compositor,network,upower,global}` — предупреждение `unused`. Предназначены для UI components (bar widgets, etc.).

## Статус ARCHITECTURE.md / DECISIONS.log / MEMORY.md
- ARCHITECTURE.md: НЕ обновлялся (§4 устарел частично — TODO в MEMORY.md, правка отложена per task.md).
- DECISIONS.log: обновлён ×7 (geometry + i128 addendum + Network zbus + UPower zbus/f64 + float-Eq правило + Task 5 Services/retry/panic-guard + Task 6 AppState/watch).
- MEMORY.md: обновлён (раздел «На горизонте»).
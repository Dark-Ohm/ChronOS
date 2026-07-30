<!-- T019 — migrated 2026-07-22 from docs/orchestration/report-log/autohand-report.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: bar-виджет Network — 2026-07-17

> Формат SESSION_REPORT (см. MEMORY.md §Rules). Дополняет AUTOHAND.md.
> Это отчёт по **Заданию №2** (починка виджета после провала приёмки №1).

## Контекст приёмки
- Приёмка №1: ❌ НЕ ПРИНЯТО. Причины (из AUTOHAND.md §ПРИЁМКА): коммит `4bbc4fb`
  не содержал `mod network;` (строка была закомменчена Архитектором из-за E0599),
  виджет не попадал в сборку; `ServiceStatus::Failed(_)` **не существует**
  (реальные варианты: `Initializing`/`Available`/`Unavailable`/`Degraded(String)`,
  см. `crates/services/src/lib.rs:48`); претензия к `lib.rs` мимо — bar живёт в
  бинарнике (`main.rs`), `lib.rs` минимален по дизайну.

## Сделано (факт, не намерение)
- `crates/app/src/bar/widgets/network.rs` (исправлен):
  - Удалён несуществующий `ServiceStatus::Failed(_)` (причина E0599). Заглушка
    теперь ловит `Initializing | Unavailable | Degraded(_)`. Решение по
    `Degraded(_)`: трактую как «сервис жив, но фич нет» → рендер заглушки, как и
    для недоступного/инициализирующегося. `Unavailable` — явный даун, тоже заглушка.
  - Добавлен `pub fn register(cx: &mut App)` по образцу `clock.rs:59`
    (`cx.global_mut::<BarWidgetRegistry>().register(Box::new(NetworkWidget))`).
  - Тест `stub_when_failed` переписан в `stub_when_unavailable` + добавлен
    `stub_when_degraded` (2 теста вместо 1, без потери покрытия).
  - Рендер/логика `describe` + `strength_icon` не тронуты (были корректны).
- `crates/app/src/bar/widgets/mod.rs` (правка только своих строк):
  - Раскомментировано `mod network;`.
  - Заменена заглушка-коммент `// network: выключено до фикса Autohand` на
    реальный вызов `network::register(cx);` внутри `register_builtin`.
  - Больше в файле НИЧЕГО не изменено (сохранён вид Cline: `battery`, `tray`,
    `workspaces`, порядок регистраций).

## Расхождения со спекой/планом (актуально для №2)
- `ServiceStatus::Failed` из спеки AUTOHAND.md не существует в коде — заменён на
  `Unavailable` + `Degraded(_)` (осознанно, см. выше).
- Wired-эвристика без изменений: `NetworkData` (`crates/services/src/network/types.rs`)
  не имеет поля `wired`; wired = `connectivity == Full && wifi_ssid.is_none()`.
  Иного источника в сервисе нет.
- Цвета: disconnected/stub → `Theme::global(cx).text.muted`; wired/wifi →
  `text.secondary` (читаемее, не «выключено»).

## Проверено фактом, не на словах
- `cargo build --workspace` → **зелёный** (FINISHED, только unrelated warnings в
  чужом `notifications/view.rs:90,134` — `unused Task`, не мой код).
- `cargo test --bin chronos bar::widgets::network` → **9 тестов РЕАЛЬНО
  выполнились** (имена из вывода):
  - `bar::widgets::network::tests::strength_icon_buckets` ... ok
  - `bar::widgets::network::tests::disconnected_on_none` ... ok
  - `bar::widgets::network::tests::stub_when_degraded` ... ok
  - `bar::widgets::network::tests::stub_when_unavailable` ... ok
  - `bar::widgets::network::tests::stub_when_initializing` ... ok
  - `bar::widgets::network::tests::stub_when_unknown_connectivity` ... ok
  - `bar::widgets::network::tests::wifi_shows_ssid_and_strength` ... ok
  - `bar::widgets::network::tests::wired_on_full_without_ssid` ... ok
  - `bar::widgets::network::tests::wifi_truncates_long_ssid` ... ok
  - `test result: ok. 9 passed; 0 failed; ...`
- `cargo test --workspace` → все таргеты зелёные (35+25+25 в services/luau/ui,
  9 в bin-chronos, остальные 0/ok).
- `git status --short crates/app/src/bar/widgets/` перед коммитом → staged только
  мои 2 файла (`network.rs`, `mod.rs`); чужие `battery.rs`/`clock.rs`/`tray.rs`/
  `workspaces.rs` остались вне коммита.
- `git log -1 --oneline` → `1f508d6 bar : network-виджет подключён и починен`.

## Не реализовано / ограничения среды
- **Живой прогон `RUST_LOG=info ./target/release/chronos` + скриншот grim** —
  НЕ выполнен. Причина: песочница без Wayland/X11 + D-Bus (нет композитора и
  system-шины), бинарь не может инициализировать `init_all()` (NetworkManager D-Bus).
  Это ограничение среды, не кода: виджет компилируется, регистрируется и проходит
  юнит-тесты на реальной логике `describe()`. Рекомендую живой прогон на хосте
  (CachyOS + Hyprland) Архитектору.
- Юнит-тесты живут в бинарном таргете (`--bin chronos`), т.к. `bar` подключён
  через `main.rs`, а не через `lib.rs` (`chronos_app`). `cargo test -p chronos_app
  --lib bar::widgets::network` даёт 0 тестов по этой же причине — НЕ баг виджета.

## Статус ARCHITECTURE.md / DECISIONS.log
- Не обновлялись. Причина: рядовой виджет в существующем каркасе
  (BarWidget/BarWidgetRegistry), новых архитектурных решений не принималось.
  Wired-эвристика — кандидат на DECISIONS.log, но требует сначала поля `wired: bool`
  в `NetworkData` (follow-up в services), поэтому пока не фиксирую.

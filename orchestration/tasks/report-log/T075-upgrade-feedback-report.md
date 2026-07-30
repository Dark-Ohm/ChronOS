<!-- T075 — migrated 2026-07-22 from orchestration/report-log/mimo-report-12.md — see orchestration/tasks/MIGRATION.md -->

# Mimo Report — Задание №12: обратная связь «Upgrade all»

**Дата:** 2026-07-20
**Коммит:** `79c8baa`

## Что сделано

Добавлена обратная связь в `updates_popup` при нажатии «Upgrade all»: блокировка кнопки на время работы, текстовый статус, результат по завершении.

## Что подтверждено деревом

### `crates/services/src/aur/types.rs`
- Новый `UpgradeState` enum: `Idle` (default), `Running`, `Done`, `Failed` (строки 22-31)
- `UpdatesState.upgrade_state: UpgradeState` добавлено (строка 40)
- 3 юнит-теста: `upgrade_state_default_is_idle`, `upgrade_state_roundtrip`, `updates_state_default_has_idle_upgrade` (строки 61-85)

### `crates/services/src/aur/mod.rs`
- `UpgradeState` добавлен в pub use (строка 44)
- `dispatch(UpgradeAll)`: перед `run_upgrade_all()` ставит `UpgradeState::Running` (строки 108-112), после — `Done`/`Failed` в зависимости от результата (строки 119-121)
- `read_state()` возвращает `UpdatesState { updates, ..Default::default() }` — upgrade_state не сбрасывается при poll (строка 199)
- Тест `count_reflects_updates_len` обновлён с `..Default::default()` (строка 478)

### `crates/services/src/lib.rs`
- `UpgradeState` добавлен в pub use (строка 24)

### `crates/app/src/updates_popup/mod.rs`
- `upgrade_all()` больше НЕ закрывает попап после диспатча — окно остаётся открытым для показа статуса (строки 188-197)

### `crates/app/src/updates_popup/view.rs`
- Импорт `UpgradeState` (строка 14)
- Футер рендерится при `updates.is_empty() && upgrade_state == Idle` (строка 127) — т.е. при завершённом апгрейде с пустым списком футер остаётся для показа результата
- Статус-строка (строки 130-153):
  - `Running` → "Upgrading…" (`text_muted`)
  - `Done` → "Upgrade complete" (`theme.status.success`)
  - `Failed` → "Upgrade failed" (`theme.status.error`)
- Кнопка (строки 155-192):
  - `Running` → заблокирована (`interactive.active` bg, `text_muted`, нет `cursor_pointer`/`on_click`)
  - `Idle` + есть updates → обычная «Upgrade all» (accent bg, кликабельна)
  - Updates пусты → кнопка скрыта

### `crates/app/src/bar/widgets/updates.rs`
- Тест `describe_with_updates` обновлён с `..Default::default()` для нового поля (строка 118)

## Верификация

- `cargo test -p chronos-services --lib` — **134 теста зелёные** (включая 3 новых на UpgradeState)
- `cargo test -p chronos --lib` — 4 теста зелёные
- `cargo build --release -p chronos` — **зелёный** (warnings только в чужом коде: network.rs, tray.rs, notifications/)
- Release binary: `target/release/chronos` собран

## Живой смок

Не проведён (нет доступа к Hyprland-сессии из текущей среды). Для верификации UI-состояний необходим реальный клик по «Upgrade all» с `pkexec`-диалогом.

## Зоны

Только `updates_popup/**`, `aur/{mod,types}.rs`, `lib.rs`, `bar/widgets/updates.rs` (тест). Чужие файлы не тронуты.

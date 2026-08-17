# T187 report — games catalog: Categories + is_game + games.toml

**Статус: ПРИНЯТА 2026-08-02 архитектором.**
**Коммиты:** `7a99116` + `af66b58` (оформил архитектор; в inbox код был uncommitted).
Тесты перепрогнаны: applications 24/24, games_config 9/9.

## Что сделано

### 1. `AppEntry.categories` (services)

`crates/services/src/applications/types.rs`:
- Добавлено `pub categories: Vec<String>` в `AppEntry`.
- `parse_desktop_file` парсит `Categories=` — split по `;`, empty строки отброшены.
- 2 новых unit-теста: `parse_categories_splits_and_drops_empty` и `parse_no_categories_defaults_to_empty`.

### 2. `is_game_entry` + `steam_app_id_from_exec` (services)

`crates/services/src/applications/mod.rs`:
- `pub fn is_game_entry(entry: &AppEntry) -> bool`:
  - Исключает `id == "steam"` (даже с `Categories=Game`).
  - Включает если `categories` содержит `Game`.
  - Включает если Exec содержит `steam://rungameid/`.
  - Включает если id с префиксом `steam_app_`, `heroic_`, `lutris_`.
- `pub fn steam_app_id_from_exec(exec: &str) -> Option<String>` — извлекает числовой id из `steam://rungameid/<id>`.
- 12 unit-тестов покрывают все правила + крайние случаи.

### 3. `games.toml` pin/recent (app)

`crates/app/src/games_config.rs` (новый, по образцу `dock/config.rs`):
- `GamesConfig { version, pinned: Vec<String>, recent: Vec<RecentEntry> }`.
- `load()` — отсутствующий файл → default (без перезаписи), битый файл → default + warn.
- `save()`, `pin()`, `unpin()`, `is_pinned()`, `touch_recent()` (cap 20, newest first).
- 9 unit-тестов: parse, round-trip, pin/unpin, recent dedup+cap, timestamp monotonicity.

`crates/app/src/main.rs`: `mod games_config;`.

### 4. Обновление test fixtures

- `crates/services/src/applications/mod.rs`: `applications_state_is_eq` — добавлено `categories: vec![]`.
- `crates/app/src/launcher/search.rs`: `make_entries()` — добавлено `categories: vec![]`.
- `crates/app/src/bar/widgets/dock.rs`: 4 конструкции `AppEntry` — добавлено `categories: vec![]`.

## Тесты

```
cargo test -p chronos-services applications::  → 24/24 passed
cargo test -p chronos --bin chronos games_config:: → 9/9 passed
```

Все clippy warnings — pre-existing, не от изменений T187.

## Что НЕ сделано

- UI Library tab (T188)
- Вызов `games_config::load()` / `save()` из продакшн-кода (ждёт T188)
- Steam API / artwork / ProtonDB

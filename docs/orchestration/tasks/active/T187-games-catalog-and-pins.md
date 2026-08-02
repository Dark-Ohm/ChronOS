# T187 — games catalog: Categories + is_game + games.toml

**Статус:** active. **Роль:** BACKEND.
**Правила:** `docs/orchestration/agents/RULES.md`.
**План:** slice-5 §2.3 (уточнение T184).
**T184:** `report-log/T184-gamer-hub-recon-report.md` §1–2, §7, §9.1–9.2.

**Параллельно:** T185 (scene), T186 (rail). Зона **не** пересекается.

**Зона:**
- `crates/services/src/applications/types.rs`
- `crates/services/src/applications/mod.rs`
- **новый** `crates/app/src/games_config.rs` (pin/recent) **или**
  `crates/services/src/games/` — обоснуй; предпочтение: `crates/app/src/games_config.rs`
  по аналогии с `dock/config.rs` (T184)
- `crates/app/src/lib.rs` / `main.rs` — **только** `mod games_config` + init
  если нужен (минимально)
- тесты в тех же модулях

**НЕ трогать:** `scene.rs` (T185), `tabs.rs`/`tab/**` (T186+), UI launch
(Library T188 зовёт твои pure fn + `launcher::launch`).

**Отчёт:** `docs/orchestration/tasks/report/T187-games-catalog-and-pins-report.md`.

---

## 1. `AppEntry.categories`

- Поле `pub categories: Vec<String>`
- Parse `Categories=` split by `;`, drop empty
- Обновить test fixtures (`applications/mod.rs`, `launcher/search.rs` если
  конструируют `AppEntry`)

## 2. `is_game_entry` (pure)

```rust
pub fn is_game_entry(entry: &AppEntry) -> bool
```

Правила (T184, **не** plan §2.3 filename-only):

1. **Exclude** id `steam` (Steam client) — даже с Categories=Game.
2. **Include** if `categories` contains `Game` (case-sensitive as in file,
   обычно `Game`).
3. **Include** if Exec matches launch pattern:
   - contains `steam://rungameid/`
   - or id starts with `steam_app_`, `heroic_`, `lutris_` (filename heuristic
     secondary — on this machine 0 steam_app_*, but keep).
4. Otherwise false.

Тесты unit:
- fixture like CS2 Exec → true
- steam client Categories=Game id=steam → **false**
- random app without Game → false
- steam_app_730 id → true

## 3. Optional helper

```rust
pub fn steam_app_id_from_exec(exec: &str) -> Option<String>
// parse after rungameid/
```

Для Scenes create-from (T189) — полезно сейчас.

## 4. `games.toml` — pin/recent

Путь: `~/.config/chronos/games.toml`

```toml
version = 1
pinned = ["Counter-Strike 2", "SCUM"]  # desktop ids (filename stem)

[[recent]]
id = "Counter-Strike 2"
# unix ts
ts = 1730000000
```

API (pure + I/O как dock config):

- `load() -> GamesConfig`
- `save(&GamesConfig)`
- `pin(id)` / `unpin(id)` / `is_pinned`
- `touch_recent(id)` — cap list e.g. 20, newest first

Не GPUI. Ошибки — warn log, не panic. Битый файл → default, **не** silent
overwrite until explicit save from user action (pin).

Тесты parse/round-trip на строках.

## 5. Export

`chronos_services::applications::{is_game_entry, ...}` public enough for
`crates/app` Library tab.

## Верификация

```
cargo test -p chronos-services applications::
cargo test -p chronos games_config::
# or wherever tests live
cargo clippy -p chronos-services --all-targets
cargo clippy -p chronos --all-targets
```

Коммит(ы): можно два — `services : AppEntry categories + is_game (T187)` и
`app : games.toml pin/recent (T187)` — оба самодостаточны.

## Что НЕ делать

- UI Library
- scene activate
- Steam API / artwork

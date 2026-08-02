# T188 report — вкладка Library (Gamer at-rest hub)

**Роль:** FRONTEND. **Зона:** `side_panel_right/tab/library.rs` (create),
`tab/mod.rs` (только `TabContent::Library` arm), `view.rs` (2 non-exhaustive
match arm'а), `lib.rs` (2 `pub mod` — enabler для lib-target).

## Что сделано

1. **`tab/library.rs` (new)** — `LibraryTab` entity + `Render` по образцу
   `tab/files.rs` / `tab/system.rs`:
   - `new`: подписка на `AppState::applications(cx).subscribe()` через
     `state::watch` + seed из `.get().entries`; `GamesConfig::load()`.
   - `set_games`: `filter_games` (is_game_entry + сортировка по имени
     case-insensitively).
   - `launch_game`: `crate::launcher::launch::launch(&exec)` (setsid-detach)
     + `config.touch_recent` + `config.save()` (ошибки в `tracing::error!` /
     `tracing::warn!`, без `let _ =` / `unwrap`).
   - `toggle_pin`: `config.pin/unpin` + `save()`.
   - Секции **Pinned → Recent → All games**, без дубликатов (pure
     `compute_sections`: pinned выигрывает, потом recent, потом all; stale
     config-ids дропаются).
   - Row: launch-area (flex-1, click → launch) + отдельная sibling pin-кнопка
     (★/☆) — click-handlers не вложены. Avatar-tile = инициал.
   - Empty: честное «No games detected» + hint про Categories=Game /
     steam://rungameid. **Никакого фейкового artwork/playtime** (§13).
2. **`tab/mod.rs`**: `pub(crate) mod library;`, `use library::LibraryTab;`,
   `TabContent::Library(Entity<LibraryTab>)` variant, arm `PanelTab::Library
   => TabContent::Library(cx.new(|cx| LibraryTab::new(cx)))`. Scenes/Captures
   остались Placeholder.
3. **`view.rs`**: 2 arm'а `TabContent::Library` — в `render()` match
   (`col.child(entity.clone())`) и в `tab_entity_id` (`e.entity_id()`).
   Non-exhaustive match без них не компилировался — допустимое расширение
   (как T179 для Preview).
4. **`lib.rs`** (enabler): `pub mod games_config;` + `pub mod launcher;`.
   Крат имеет **lib + bin** target'ы; `side_panel_right` (с моим `library.rs`)
   компилится в обоих, но `games_config`/`launcher` были объявлены только в
   `main.rs` (bin). Без этих 2 строк lib-target не видел `crate::games_config`
   / `crate::launcher` (E0432/E0433). `launcher` lib-safe (зависит только от
   `crate::state` + self).

## Pure-функции + тесты (прецедент `system.rs::format_net_pair`)

`filter_games`, `compute_sections`, `initial` — без cx/AppState, unit-тестируемы
(конструирование `LibraryTab::new` требует AppState-global + tokio-runtime, как
`SystemTab::new`). 7 новых тестов:

- `filter_games_excludes_steam_client_and_non_games_and_sorts` — steam client
  (id=="steam") и firefox отфильтрованы, CS2/PUBG остались, сортировка.
- `filter_games_empty_input`.
- `compute_sections_pinned_wins_over_recent_and_all` — pinned выигрывает над
  recent, all без перекрытий.
- `compute_sections_drops_stale_pinned_and_recent_ids`.
- `compute_sections_empty_games_yields_all_empty`.
- `compute_sections_pinned_keeps_config_order` — порядок games.toml сохранён.
- `initial_uppercases_first_char`.

## Чем доказано

```
$ cargo check -p chronos            # lib + bin, 0 errors, Finished in 4.52s
$ cargo test -p chronos -- side_panel_right
test result: ok. 107 passed; 0 failed  (lib)
test result: ok. 109 passed; 0 failed  (bin)
# 7 новых library::tests все ok
$ cargo build --release -p chronos  # Finished in 3m 03s, 0 errors
```

## Границы / чужая работа в дереве

В рабочем дереве **параллельно шла задача T190** (scene gaming-profile wire):
- `scene.rs` — `use crate::system_popup::gaming_mode` + `gaming_transition`
  (всё T190, uncommitted).
- `system_popup/gaming_mode.rs` — 2 строки T190 (visibility).
- `lib.rs` — `+pub mod system_popup;` (T190, для их scene.rs).
- `report/T190-...-report.md` (T190).

Я **НЕ стейджил** ничего из T190. `lib.rs` стейджнул **только** свои 2 строки
(`games_config`/`launcher`) через surgical patch (`git apply --cached`), NOT
T190's `system_popup`. Проверка: HEAD `scene.rs` **не** ссылается на
`system_popup` → мой коммит (HEAD + мои файлы) lib компилится без строки T190.
`view.rs` / `tab/mod.rs` — 100% мои (diff чист). Изолированную проверку коммита
(worktree на HEAD коммита) сделаю после commit'а.

## Что НЕ сделано

- **Живой кадр не снят** — Terminal Shell без compositor/Chronos; grim не к
  чему. `НЕ ПРОВЕРЕНО`: список CS2/PUBG/SCUM, launch, pin на живом рейле — за
  архитектором (T190 P2). Код/тесты/сборка зелёные, пиксельной проверки нет.
- **`scene::activate` → pin→scene create** — T189 (не моя зона).
- **Games filter (.desktop Categories)** — уже T187 (`is_game_entry`).
- **T190's `pub mod system_popup`** — не мой, оставлен unstaged для T190.

# T185 report

> **ПРИНЯТА 2026-08-02 архитектором.** Коммит `0749d33`, зона `scene.rs` only,
> scene:: 17/17 перепрогнаны. `restore_for_mode` read-only; activate/seed —
> единственные writers. Без gaming_mode / workspace_mode::set.

**Зона:** `crates/app/src/scene.rs` только. Не тронул ничего вне зоны.

## 1 Поля `Scene`

Добавлены с `#[serde(default)]` (scene.rs, в структуре `Scene`):

- `kind: String` (default `""`) — `"hub"` | `"game"`.
- `app: String` (default `""`) — desktop id / launch key.
- `apply_gaming_profile: bool` (default `false`).

`version` не тронут, остался `1`. Доп. поля (`audio_sink`, `microphone`,
`hyprland_workspace`) не добавлял — задание разрешало отложить их в `extra`,
и на данном этапе (T187/T188 их пока не используют) добавлять неиспользуемые
поля означало бы поднимать dead-surface без потребителя. Если T189/T190
понадобятся — тривиальная добавка тем же паттерном.

## 2 `activate`

`pub fn activate(cx: &mut App, id: &str) -> Result<(), ActivateError>` +
чистое ядро `pub fn activate_in_config(cfg: &ScenesConfig, id: &str) ->
Result<(ScenesConfig, Scene), ActivateError>`, вынесенное отдельно, чтобы
тестировать логику без `cx`/диска (тот же принцип, что у `resolve_last` /
`filter_valid` в этом файле).

Поведение подтверждено тестами (`cargo test -p chronos scene::`, 17/17 ok):

- ищет сцену **среди валидных** (`filter_valid` перед `find_by_id`), но
  `last` пишется в **оригинальный** конфиг (невалидные записи с диска не
  выкидываются) — `activate_in_config_updates_last_for_mode`.
- id не найден → `ActivateError::NotFound` — `activate_in_config_missing_id_is_not_found`.
- сцена с невалидным `mode` (не парсится `WorkspaceMode::parse`) → тоже
  `NotFound` (она отфильтрована ещё до поиска) —
  `activate_in_config_invalid_mode_scene_is_not_found`.
- `activate` (не ядро, полная версия с `cx`) зовёт `save_config` **только
  здесь** — единственный писатель `[last]` из user-path.
- `tracing::info!(scene=%id, mode=..., "scene: activated")` — логируется.
- Не зовёт `GamingModeState` — grep по `activate`/`activate_in_config`
  подтверждает отсутствие импорта `gaming_mode` в файле.
- Не зовёт `workspace_mode::set` — та же проверка, `crate::workspace_mode`
  используется только для `WorkspaceMode`/`current` (уже было, не добавлял).

## 3 Seed hub

`pub fn ensure_builtin_hub(cfg: &mut ScenesConfig) -> bool`:

- Условие «нет hub» — `id == "hub"` **и** `WorkspaceMode::parse(mode) ==
  Some(Gamer)` (не голое строковое сравнение — чтобы регистр/мусор в mode
  не давал ложный «hub есть»).
- Добавляет сцену в память (не трогая существующие) с `rail_tabs` из
  задания (`system, library, scenes, captures, acp_settings, mcp_settings,
  lsp_settings, api_providers, editor_settings, hyprland_binds`),
  `active_tab = "library"`, `dock = [steam, discord, firefox, kitty]`.
- `last["gamer"]` выставляется **только если ключа не было**
  (`entry().or_insert`) — не затирает существующий `last.gamer`, указывающий
  на другую сцену (тест `ensure_builtin_hub_does_not_override_existing_last_gamer`).
- Идемпотентно: второй вызов на уже заполненном конфиге не задваивает hub
  (`ensure_builtin_hub_preserves_existing_scenes_and_is_idempotent`).
- Не стирает чужие сцены (developer-сцены сохраняются, тест выше).
- `init()` вызывает `ensure_builtin_hub` после `load_config`; если вернула
  `true` — один раз `save_config(&cfg)`, до вычисления `initial`/лога.

## 4 `restore_for_mode`

Не тронут вообще — ни один символ. Контракт read-only-на-диске остаётся
прежним: функция принимает только `&mut App`, работает с
`cx.global::<SceneState>()`, нигде не зовёт `save_config`. `activate` и
`ensure_builtin_hub` — единственные два писателя, оба явно упомянуты в
задании.

## Верификация

```
$ cargo test -p chronos scene::
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 293 filtered out

$ cargo clippy -p chronos --all-targets
```

`clippy` в моей зоне (`scene.rs`) чист от `dead_code`/логических warnings после
добавления `#[allow(dead_code)]` на `ActivateError`/`activate_in_config`/
`activate` с комментарием «wired by T188» (тот же паттерн, что уже был у
`active_tab_override` в этом файле до меня) — ничего не вызывает `activate`
в этом слайсе, это ожидаемо, T188 подключит. Остальные `clippy` warnings в
`scene.rs` — `unwrap_used`/`expect_used` внутри `#[cfg(test)]` (project
policy: warn, не deny; тот же стиль уже был в файле до моих правок,
например существующий `cfg.last.get("developer").unwrap()`).

## Блокер вне зоны (зафиксировано честно, не моя правка)

Во время работы дважды словил `cargo test -p chronos` красным из-за
**параллельной** правки T187 (`crates/services/src/applications/types.rs` —
добавление поля `categories` в `AppEntry`): тестовый бинарь `chronos` не
компилировался (`E0063 missing field categories`), потому что T187 ещё не
успел синхронно поправить все конструкторы `AppEntry` в
`crates/app/src/bar/widgets/dock.rs` / `launcher/search.rs`. К моменту
финальной проверки (второй `cargo test`) T187 уже сам исправил — 17/17
scene::-тестов прошли. Не трогал `applications/**`, `dock.rs`, `search.rs`
— вне зоны T185. Упоминаю только как факт временной нестабильности сборки
при параллельной волне 1, не как претензию.

## Коммит

`0749d33` — `scene : per-game fields + activate + hub seed (T185)`.
`git add crates/app/src/scene.rs` поимённо (проверено `git status --short`
перед коммитом: staged только этот файл). 1 файл, +292/-4.

## Что НЕ сделано

- UI, rail tabs enum, Library/Scenes views — не входило в зону.
- apply gaming profile — T190, не эта задача (поле только хранится).
- `categories`/`games.toml` — T187.
- `audio_sink`/`microphone`/`hyprland_workspace` — не добавлял (см. §1,
  причина — нет потребителя в этом слайсе).

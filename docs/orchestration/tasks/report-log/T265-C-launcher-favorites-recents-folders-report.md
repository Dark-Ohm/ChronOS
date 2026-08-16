# T265-C — Launcher favorites, recents, folders — Report

**Date:** 2026-08-16
**Role:** FRONTEND + persist. Zone: `crates/app/src/launcher/**` only.
**Commit:** `1577aaa` `feat(launcher): favorites, recents, folders (T265-C)`.

## Status

**Done (code + unit tests green).** Live release verification deferred — see
"Не сделано / Live". **Isolation deviation flagged**: worked directly on master
(not a worktree), same as the T265-A/T265-B waves; only the four launcher files
were staged (verified `git diff --staged`), all foreign WIP left untouched.

## Что сделано

### 1. `launcher_config.rs` (NEW) — один файл `~/.config/chronos/launcher.toml`

- Модель: `LauncherConfig { favorites: FavoritesConfig{order, sort_alpha,
  hide_labels}, recents: RecentsConfig{limit=8}, folders: Vec<Folder{id,name,
  apps}> }` — serde derive.
- **Read-modify-write через `toml::Value`**, не serde-дамп вслепую (урок
  T284/frame.toml): читаем файл как `toml::Table`, заменяем только три наших
  ключа, пишем обратно. Тест `write_preserves_unknown_top_level_keys` это
  доказывает фактом.
- **Debounce** `SAVE_DEBOUNCE=600ms` + `flush()` на `launcher::close`/`close_this`
  (рядом с `frecency::flush`). Не на каждый drag-move.
- Тесты: defaults, папка сериализуется и встаёт после reload, RMW-сохранение,
  коррапт-фолбэк, дефолт limit.

### 2. `favorites.rs` (NEW) — чистая логика без GPUI

- `move_item` (DnD-reorder), `resolve_favorites` (**unknown id в order тихо
  скипается** + `sort_alpha`), `top_recents` (**top-N frecency**, never-launched
  исключены), `is_new` + `desktop_mtime` (**бейдж по mtime `.desktop`**, порог 7
  дней — выбран mtime, не first-seen: fresh `.desktop` == свежая установка,
  без фолс-позитивов на весь парк при первом запуске фичи), `next_folder_id`,
  `folder_add_app`, `resolve_folder_apps`.
- 10 тестов.

### 3. `view.rs` — секции + DnD + rename + бейдж

- Три секции **над** «All apps»-сеткой в едином скролл-контейнере: **Favorites**
  (ручной порядок, `hide_labels` → иконки без подписи), **Recents** (top-N
  frecency), **Folders**. Пустые секции не рендерятся.
- **DnD — внутренний GPUI drag** (`on_drag`/`on_drop`, не файловый source —
  T270 не затронут): favourites-cell → reorder/insert; drop на пустую область
  секции → append; иконка на иконку → создать папку (обе); иконка на папку →
  добавить в папку. Ghost-пилюля под курсором.
- **Папки**: клик → раскрыть/свернуть (одна раскрыта одновременно), приложения
  папки рендерятся вложенным рядом; **переименование — компонентный `Input`**
  (второй `InputState`, создаётся в `open()`; Enter/blur коммит, Esc отмена;
  не `String.push`). Карандаш `stop_propagation`, чтобы не заодно тогглить
  раскрытие.
- **Бейдж «new»** — точка в углу клетки у `.desktop` моложе 7 дней.
- Поиск/pin/категории/клавиатура/ghost-completion T265-A/B не тронуты; frecency
  и `launch` переиспользованы. `grid_row_offset` держит индексацию
  `scroll_to_selected` корректной при секциях над сеткой.

### 4. `mod.rs`

- `pub mod favorites; pub mod launcher_config;` + второй rename-Input в `open()`
  + `launcher_config::flush()` в `close`/`close_this`.

## Verification

| Command | Result |
|---|---|
| `cargo test -p chronos --lib launcher` | **39 passed; 0 failed** (было 24 → +15: 10 favorites + 5 launcher_config) |
| `cargo build --release -p chronos` | **BLOCKED foreign WIP** — см. ниже |

**Важно про build.** `cargo test -p chronos --lib launcher` был зелёным (39/39).
Пока шла моя сборка, параллельная сессия (T294 Updates tab) дописала
`PanelTab::Updates` в `side_panel_right/tabs.rs`, но не закрыла match в
`icon_path` — теперь весь lib падает с единственной ошибкой
`E0004: PanelTab::Updates not covered` (`tabs.rs:703`). Это **не моя зона**,
файл не трогал. Мои файлы чисты: rustc компилирует весь крейт одним проходом и
сообщил ровно **одну** ошибку (чужую), значит launcher-код компилируется без
ошибок. Релизную сборку нужно перегнать после того, как T294 доведёт
`tabs.rs` (или принимать в ворктри).

## Что НЕ сделано (Архитектор / дальше)

1. **Live grim** (спека): секции видны; DnD меняет порядок; папка раскрывается;
   бейдж на свежем `.desktop`; рестарт сохраняет favorites/folders. Требует
   живого шелла — не гонял. Это приёмочный шаг.
2. **Перенос в `done/` + статус родителя T265** — самоприём не делаю.
3. **`launcher.toml` крутилки UI (sort_alpha / hide_labels / limit)** — спека
   разрешает отложить до T265-G; ключи уже читаются, тумблеров нет.
4. **«Add to favorites» пункт меню** — T265-D; структура данных (`order: Vec`)
   это переживёт.
5. **Удаление из папки / избранного** (drag-out) — не в спеке волны.

## Отчёт одной строкой (выборы из спеки)

- Бейдж = **mtime** (не first-seen).
- DnD = **внутренний GPUI drag**, не Wayland-file-source.
- Recents = **top-N из `frecency::cached()`**, не второй frecency-файл.
- Persist = **RMW `toml::Value` + debounce**, не serde-дамп вслепую.

## Коммит

```
feat(launcher): favorites, recents, folders (T265-C)
```

(4 files, +1274 / −42: `favorites.rs`, `launcher_config.rs` новые; `mod.rs`,
`view.rs` правки. `Cargo.lock`, `Source/gpui/`, `frecency.rs` — не тронуты.)

## Приёмка архитектора (2026-08-16)

Общий `cargo build`/`test` в живом дереве в это время был честно заблокирован
чужим WIP (T294 переписывает `PanelTab` match в `tab/mod.rs`, момент захвата —
даже с расколотым `updates.rs`, unclosed delimiter). Не твоя зона, не тронул.
Чтобы не поверить на слово, собрал коммит `1577aaa` в изолированном
`git worktree` (сиблинг ChronOS, не `/tmp` — воркспейс резолвит `../Source`
относительным путём) с отдельным `CARGO_TARGET_DIR`, без единого файла живого
WIP рядом:

```
cargo test -p chronos --lib launcher   → 39/39 (было 24, +15 — совпадает с заявленным)
cargo build --release -p chronos       → чисто, 4m45s
```

Точечно сверил с деревом: `move_item`/`resolve_favorites` (unknown id
скипается, `sort_alpha`) — как в отчёте; `stop_propagation` на карандаше папки
— `view.rs:859`, подтверждено. Зона файлов — ровно 4 файла `launcher/**`,
`frecency.rs`/`Source`/`Cargo.lock` не в диффе.

**Код принят.** Открытый пункт — тот же паттерн волны: живой grim-прогон
(DnD, папки, бейдж, рестарт-персист) за владельцем, не блокер.
Worktree и его target-каталог снесены после проверки, общее дерево не трогал.

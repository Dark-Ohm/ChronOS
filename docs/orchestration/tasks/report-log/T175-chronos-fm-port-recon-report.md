# T175 — разведка: Chronos-FM → вкладка Files

**Роль:** RECON. **Ветка:** `master`. **Кода нет.**  
**Источник:** `../Chronos-FM` (тот же автор, форк GPUI).  
**Контракт вкладки:** `crates/app/src/side_panel_right/tab/mod.rs`,
`tab/system.rs`, ширина Files `440` — `tabs.rs:510`.

---

## Вердикт одной строкой

**Портировать слой FS + DTO + сортировку; UI explorer целиком — не тащить.**
Вкладка 440 px и контракт `TabContent` несовместимы с `ExplorerPage`
(сплиты, табы, sidebar 180, preview 240, таблица колонок ~928 px).
Переписывать `view/*`/`page.rs` под панель дороже, чем написать `FilesTab`
по образцу `SystemTab` поверх портированного `list_dir_sync`.

---

## 1. `fs/` — берётся как есть?

### `listing.rs` (195 строк)

| | |
|--|--|
| Публичное | `ListParams { path, limit, cursor }` (`listing.rs:13-20`); `ListResult { entries, next_cursor }` (`:23-28`); `list_dir_sync(params) -> Result<ListResult>` (`:33-35`); re-export `FileEntryDto` (`:10`) |
| GPUI / Context / Global | **нет** — только `std::fs` + `chronos_fm_core::errors::Result` (`:1-4`) |
| Async | **блокирующий sync IO** (`:30-32` doc: «synchronous filesystem IO»; UI должен звать через `cx.background_spawn`) |
| Тянет | `chronos-fm-models` (`FileEntryDto`), `chronos-fm-core::errors` |

### `ops.rs` (389 строк)

| | |
|--|--|
| Публичное | `MoveKind` (`:16`), `ConflictResolution` (`:28`), `would_conflict`, `is_cross_volume`, `unique_name`, `copy_path`, `move_path`, `rename_in_place`, `create_dir`, `trash_path`, `delete_permanent` (`:45-222`) |
| GPUI | **нет** — `std::fs` + `trash` crate (`:216-218`) |
| Async | **блокирующий** (`:4-8` doc) |
| Тянет | `chronos_fm_core::errors`, crate **`trash = "5"`** (`chronos-fm-services/Cargo.toml:36`) |

### `FileEntryDto` (`chronos-fm-models`, 45 строк)

`file_entry.rs:9-21` — pure data: `name/path/kind/size/modified`, только `serde`.
**Берётся как есть** (скопировать или path-dep).

### `chronos-fm-core::errors` (18 строк)

`Error::{NotImplemented, Io, Other}` + `Result` (`errors.rs:6-18`).
Лёгкий; можно скопировать 20 строк или заменить на `anyhow`/свой enum в
`chronos-services`.

### Runtime split (важно)

- `list_dir_sync` / `ops` — **не tokio**. Tokio не нужен.
- В FM **reload зовётся синхронно на foreground**: `navigation.rs:17-25`
  `reload()` → `list_dir_sync` прямо в обработчике; `view.rs:62`
  `ensure_loaded()` на каждом render-path.
- Preview/search уже правильно уходят в `cx.background_spawn`
  (`preview.rs:138`, `search.rs:53`).
- **Цена переноса:** при портировании listing **обязательно**
  `background_spawn` + apply на foreground — иначе большие каталоги
  (limit 1000, `config.rs:65`) стопят UI. Это не меняет крейт, меняет
  обвязку.

### §4.1 vs ops

Спека: read-oriented tree, операции — later capability.
**Для T176 (первая Files):** `listing` + DTO — **да**; `ops` — **в репо
как библиотека, не в UI** до capability-задачи. `trash` тогда не нужен.

---

## 2. `explorer/` — по файлам

Сумма `wc -l` по дереву: **5244** (с `tests.rs` 965). Задание говорило
~3566 без части view — в дереве больше.

| файл | строк | вердикт | обоснование |
|------|------:|---------|-------------|
| `types.rs` | 77 | **берём (частично)** | `SortKey`/`ViewMode`/`Search*` — pure (`types.rs:4-40`). `Search*` не нужны v1 |
| `entries.rs` | 137 | **берём** | pure sort/filter helpers, `FileEntryDto` only (`entries.rs:1-28`) |
| `navigation.rs` | 140 | **переписываем** | Логика cwd/history/reload ценна, но `reload` sync на UI (`:17-25`) + `PaneEvent` + `config::DIR_LISTING_LIMIT` — вшить в `FilesTab` с async |
| `list_setup.rs` | 97 | **режем** | `ListState`/`FileListDelegate`/`ListEvent` (`:1-40`) — multi-col table setup; для 440 px не нужен |
| `state.rs` | 528 | **переписываем (slim)** | `ExplorerPane` — **не Global**, entity-state (`state.rs:20-129`). Поля cwd/entries/selection — ок; `InputState`/`ListState`/`ResizableState`/`SearchService`/`SyntaxService`/`PreviewEditor` — app-shell |
| `page.rs` | 746 | **выбрасываем** | `ExplorerPage` = 2-way split + tab session + `KvStore` redb (`page.rs:1-39,145-176,648`) — страничная машинария FM, не вкладка |
| `preview.rs` | 354 | **режем / later** | Text/image via `background_spawn` (`:138`); image path + `PreviewEditor` (Input code_editor). Для read-tree v1 не обязателен |
| `search.rs` | 285 | **выбрасываем (v1)** | Full-text + `SearchService`/tantivy (`:8-35`); degraded name-filter можно 20 строк локально |
| `view.rs` | 237 | **переписываем** | h_resizable: sidebar **180–360**, listing flex, preview **240–2000** (`view.rs:103-132`) — под окно, не 440 |
| `view/header.rs` | 236 | **переписываем** | Breadcrumb `gpui_component` (`header.rs:5-7`) — упростить path string |
| `view/sidebar.rs` | 134 | **выбрасываем (v1)** | Quick-access 180 px slot (`view.rs:104-106`) — съест половину 440 |
| `view/listing.rs` + `list/grid/row` | 817 | **переписываем** | Multi-col row: name+type+size (`row.rs:150-211`); `v_virtual_list` (`list.rs:7`) |
| `view/listing/search_bar.rs` | 293 | **режем** | `Input` search UI (`search_bar.rs:5`) — optional later |
| `view/preview.rs` + `editor.rs` | 198 | **режем / later** | `Input::code_editor` (`editor.rs:3,40-48,181`) — Input +1.84 MiB (T157) |
| `tests.rs` | 965 | **выбрасываем** | FM page/session tests; unit-тесты listing/ops переносятся с fs |

### `state.rs` — глобал или entity?

**Entity, не Global.** `ExplorerPane` — `pub struct` с GPUI entities
(`state.rs:20-57`), `impl Focusable` (`:135`), `EventEmitter<PaneEvent>`
(`:141`), `PaneItem` (`:143`). Создаётся `ExplorerPane::build` /
`ExplorerPage::new` (`state.rs:152-170`, `page.rs:173`).  
**Ляжет в `Entity<FilesTab>`** после выкидывания resizable/search/list
sub-entities — паттерн как `SystemTab` (`tab/system.rs:27-43`).

### `preview.rs` — что умеет

- Text UTF-8 / image path / too large / unsupported (`preview.rs:10-14,84-92`)
- Size cap `PREVIEW_MAX_FILE_SIZE` 2 MiB (`config.rs:62`)
- Language detect for highlighter (`preview.rs:16-39`)
- UI: `PreviewEditor` = `InputState::code_editor` + line numbers
  (`editor.rs:40-48`) — **gpui-component Input**, не syntect напрямую
- `SyntaxService` + syntect есть в services (`syntax.rs`, feature `gui`)
  но editor в основном tree-sitter highlighter Input'а

### `view/` ↔ gpui-component

Используется: `Input`/`InputState`, `ListState`/`ListItem`/`ListDelegate`,
`v_virtual_list`/`VirtualListScrollHandle`, `ResizableState`/`h_resizable`/
`resizable_panel`, `Breadcrumb`, `Icon`/`IconName`
(сводный grep по `explorer/**`).

**Table** в explorer **не** виден — план § files «Table/VirtualList»
частично неточен: list = **List + VirtualList**, не Table.

---

## 3. Зависимости (которых нет / цена)

### Внешние, которые принёс бы полный перенос pages+services

| crate | зачем в FM | нужен v1 Files? |
|-------|------------|-----------------|
| `trash` | `ops::trash_path` (`ops.rs:216`) | **нет** до capability |
| `tantivy` + `ignore` + `grep` + `async-channel` + `postage` + `notify-debouncer-mini` | search index (`services/Cargo.toml:29-35`, ~921 LOC search/) | **нет** |
| `syntect` (optional gui) | SyntaxService | **нет** если без code preview |
| `image` 0.25 | pages Cargo — image decode path | **нет** (preview later; gpui img path хватит) |
| `redb` / `chronos-fm-store` | session tabs (`page.rs:32-39`) | **нет** |
| `gpui-component` List/Input/Resizable/Breadcrumb/Icon | view | **частично** — см. ниже |

Уже есть в ChronOS: `gpui-component` (app), `dirs`, `notify`, `serde`.

### gpui-component (измерения T157)

| компонент | где в FM | v1 Files 440 px |
|-----------|----------|-----------------|
| **VirtualList** (+15 KB) | `list.rs:7`, scroll handle | **желателен** при >~100 entries |
| **List** / ListState | `list_setup.rs`, file_list | можно заменить простым `div`+scroll |
| **Input** (+1.84 MiB) | search_bar, PreviewEditor | **не нужен v1** (filter = optional later) |
| **Resizable** | view split | **нет** — одна колонка |
| **Breadcrumb** | header | optional; path text хватит |
| **Icon/IconName** | rows/sidebar | optional; наши `svg()`/`icons/` |
| **Table** | не используется | — |

**Input не обязателен** для read-oriented v1. Если добавят filter-поле
или preview-editor — Input уже в дереве (T157/T158), цена уже уплачена
в feature graph; не «новый» +1.84, а использование.

---

## 4. Разрыв с контрактом вкладки

Образец: `TabContent::create` → `Entity<SystemTab>` lazy
(`tab/mod.rs:35-45`), `Render` на entity (`system.rs:123`), кэш в
`SidePanelRightView`, width из `preferred_content_width` (`tabs.rs:506-510`
→ Files **440**).

| препятствие | file:line |
|-------------|-----------|
| `ExplorerPage` = `Page` trait + split + session KV, не одиночный tab entity | `page.rs:648-649`, `:32-39` |
| `PaneItem` / `PaneGroup` — чужой page framework | `pane_group.rs:28`, `chronos_fm_pages.rs:14` |
| Тема FM `chronos_fm_ui::theme::theme` RGB consts, не `chronos_ui::Theme` | `view.rs:5`, `file_list.rs:1` |
| `ensure_loaded`/`reload` sync list на UI thread | `navigation.rs:17-25`, `view.rs:62` |
| Layout: sidebar 180 + listing + preview ≥240 → **≥420 без listing** | `view.rs:104-132` |
| Таблица: name 400+type 120+size 120+mod 180+act 60+pad 48 = **~928 px** | `config.rs:34-49`, `state.rs:416-421` |
| Зависимость `chronos_fm_ui::FileListDelegate` + IconName | `file_list.rs:12-70` |
| Key context / actions `ExplorerPanes` | `page.rs:28-79` |
| `TabContent` сегодня: Files → `EmptyTab` placeholder | `tab/mod.rs:44-45`, `:104` |

Ничего из этого не «адаптировать одной строкой» — нужен **новый
`FilesTab` entity** + порт FS.

---

## 5. Ширина 440

| элемент FM | px (default) | vs 440 |
|------------|-------------:|-------|
| Sidebar | 180 (range 180–360) `view.rs:104-105` | 41% панели |
| Preview panel | 240 min `view.rs:131-132` | 55% |
| Col name alone | 400 `config.rs:34` | 91% |
| Full table width | ~928 `state.rs:416-421` + `config.rs:34-49` | **2.1× шире 440** |

**Вывод:** multi-col list + sidebar + side preview **не влезает** в 440.
Менять 440 → 700+ только чтобы втащить FM layout — плохая сделка (съест
пол-экрана workbench).

**Рекомендация UI v1 в 440:**
- одна колонка: icon + name (+ optional size muted);
- path bar сверху (текст, не breadcrumb widget);
- double-click / Enter = cd / open later;
- preview — **не** side panel; later bottom sheet или отдельная вкладка Editor;
- sidebar — не в v1 (Home/… кнопки в header).

440 **оставить**.

---

## Итог по строкам (оценка переноса)

| категория | строк (порядок) | что |
|-----------|----------------:|-----|
| Почти as-is | **~400–450** | `listing.rs` 195 + `FileEntryDto` 45 + `entries` sort ~80 + thin `Error` ~20 + (опц.) куски `ops` 389 later |
| Переписать | **~300–500** | `FilesTab` state+nav+render (как SystemTab 256 LOC) + list UI |
| Выбросить для v1 | **~4000+** | `page` 746, search 285+921 svc, preview 354+198, tests 965, sidebar, list_setup multi-col, resizable chrome |

**Полный перенос explorer UI ≈ 3–4k LOC борьбы с layout/deps.**  
**Целевой v1 ≈ 0.7–1.0k LOC** (порт fs + новая вкладка).

---

## Рекомендация порядка (лучше плана «перенести explorer»)

План (`2026-07-31-developer-workbench-slice-4.md` §2.1) говорит «берём из
Chronos-FM, не пишем заново». Уточнение по фактам:

1. **T176a — library:** скопировать/вынести `list_dir_sync` + `FileEntryDto`
   (+ unit-тесты listing) в `chronos-services` или `crates/app` helper.
   `ops` — файл рядом, **без UI wire**, capability later.
2. **T176b — `FilesTab`:** entity + `TabContent::Files` ветка в
   `tab/mod.rs` (как System). List: `background_spawn` → entries →
   `div`/`v_virtual_list`. Ширина 440.
3. **Не** path-dep на `chronos-fm-pages` / `chronos-fm-ui` / store.
4. Later: ops capability, filter Input, preview (Input code_editor —
   уже в дереве), notify reload.

Если цель — «пиксель-в-пиксель FM explorer» — **нужно ≥900 px content** и
это уже не side panel tab; тогда отдельное окно, не `PanelTab::Files`.

---

## Расхождение с планом (с цифрами)

| план | факт |
|------|------|
| Files тянет Table/VirtualList | Table **нет**; VirtualList **да** (`list.rs:7`) |
| ~3566 explorer | **5244** с view subtree |
| «не пишем заново» UI | UI **пишем узкий**; fs **не** пишем заново |
| 584 fs as-is | listing **да**; ops **later** по §4.1 |

Это не отказ от FM — отказ от переноса page shell.

---

## Источники (grep/wc, не память)

- `wc -l` по `Chronos-FM/crates/chronos-fm-pages/src/explorer/**`
- `wc -l` fs: 195+389=584
- `chronos-fm-services/Cargo.toml`, `chronos-fm-pages/Cargo.toml`
- `config.rs:34-65` COL_*/DIR_LISTING_LIMIT
- `tabs.rs:301-303,506-510` preferred 440
- `tab/mod.rs:35-45` create/EmptyTab

<!-- T115 — IDE-панель, вкладка Files. Агент не назначен. PAUSE до T119. -->

# T115 — Shell-IDE правая панель: вкладка Files

**Статус: PAUSE** (не раздавать, пока архитектор не снимет pause после
T119). Когда разморозят — агент **не назначен** (кандидат: medium-класс
модель; бриф заточен под cold session + жёсткие рамки).

**Контекст:** фундамент таб-контейнера принят (T112, `0e10e51`,
`done/T112-ide-panel-tab-container.md`). Сейчас `PanelTab::Files` —
заглушка `"Files — coming soon"` (иконка `icons/folder.svg` уже в
`side_panel_right/tabs.rs::icon_path`). Задача: **read-only** дерево
`$HOME`, lazy expand — **не** полноценный файловый менеджер.

## 0. Жёсткие правила (читай до кода)

1. **Cold session:** всё нужное — в этом файле + указанные пути. Не
   полагайся на «память диалога».
2. **Зона файлов — только свои.** Создавай/правь:
   - `crates/app/src/side_panel_right/tab_files.rs` (новый)
   - опционально `crates/app/src/side_panel_right/files/` (новый модуль:
     listing types, helpers) — **только** если `tab_files.rs` раздувается
   - **НЕ** трогай: `view.rs`, `tabs.rs`, `mod.rs` в
     `side_panel_right/` — подключение dispatch делает **архитектор
     после приёмки**. Если без правки `mod.rs` не компилируется
     `pub mod tab_files` — **не** правь `mod.rs`: оставь файл
     компилируемым как `#[cfg]`-независимый модуль с `pub fn
     render_files_tab(...)`, в отчёте напиши «нужен 1-line wire в
     mod.rs/view.rs — архитектору». Либо сделай wire **только** если
     архитектор явно снял этот запрет в сообщении о раздаче.
3. **Не path-dep** на `chronos-fm-*` / не добавляй crates FM в
   ChronOS `Cargo.toml`.
4. **Не** `unsafe_code` (workspace `deny`).
5. **Не** `let _ = fallible_call()` — `?` / `.log_err()` / явный `match`.
6. **Не фабрикуй** тесты и вывод. В отчёт — **paste реального**
   `cargo test` / `cargo build` (команда + хвост stdout). Имена
   несуществующих тестов = reject.
7. **Unit green ≠ done.** Для UX обязателен живой смок (ниже).

## 1. Референс Chronos-FM — COPY-PASTE РАЗРЕШЁН

**Оба репо пользователя.** Chronos-FM MIT — **можно копировать** код
listing/entry/tree state. Это **не** `ChronOS/reference/` gpui-shell
(без лицензии — **нельзя**).

**Корень FM:** `/home/neo/projects/chronos-ecosystem/Chronos-FM`

| Бери (минимум) | Путь |
|---|---|
| `list_dir_sync`, sort, paging ideas | `crates/chronos-fm-services/src/fs/listing.rs` |
| `FileEntry` / `FileEntryDto` / `FileKind` | `crates/chronos-fm-models/src/file_entry.rs` |
| Expand/collapse state patterns (не весь UI) | `crates/chronos-fm-pages/src/explorer/state.rs`, `navigation.rs`, `entries.rs` |

| **НЕ** тащи в T115 |
|---|
| `fs/ops.rs` (mutate) |
| clipboard / rename / DnD / multi-pane / split |
| preview, S3, git badges, context menus (`docs/agents/active/b1–b4`) |
| весь `ExplorerPage` as-is |

**Как встраивать:** скопируй нужные типы/функции **внутрь** ChronOS
(`tab_files.rs` или `side_panel_right/files/`), подгони под gpui-ce
форка ChronOS и workspace lints. В отчёте таблица:

```text
| FM path | Что взял | Куда в ChronOS | Что изменил |
```

## 2. Что сделать (scope)

1. **Tree-view**, корень = `$HOME` (`dirs::home_dir` / `std::env::var("HOME")`,
   не хардкод `/home/someone`).
2. **Lazy expand:** на старте читай **только** корень; дочерние dirs —
   при клике expand. Не `walkdir` всего `$HOME`.
3. **Клик по файлу** — no-op + `tracing::info!("would open: {path}")`.
   Editor-tab open = **другая** future task.
4. **Сортировка:** dirs first, then files; внутри — case-insensitive
   alpha (как FM listing).
5. **Dotfiles:** hide by default **или** show — выбери одно; toggle
   optional. Решение + почему — в отчёте.
6. **Ошибки** (`PermissionDenied`, bad symlink): узел с пометкой
   error, `tracing::warn!`, **не** panic/crash tab.
7. Listing: sync `read_dir` / port of `list_dir_sync` на expand click.
   Если listing тормозит UI — `cx.background_spawn` (паттерн FM
   async notes); не переусложняй сервисом `FilesSubscriber` без нужды.
   Отдельный service в `crates/services/` — **не** требуется для T115.

## 3. Интерфейс (ориентир)

```rust
pub fn render_files_tab(
    /* panel or entity state for expanded set */,
    cx: &mut Context</* SidePanelRightView or own Entity */>,
) -> impl IntoElement
```

Точную сигнатуру и **где живёт expanded-state** (`HashSet<PathBuf>` на
view / отдельная `Entity`) — зафиксируй в отчёте. Рекомендация: state
рядом с render в том же модуле/`Entity`, **не** глобальный static.

GPUI: слушай RPIT/E0502 — если `cx.listener` + `impl IntoElement`,
строй handlers **до** длинных RPIT-веток (урок `side_panel_left/panel.rs`).

## 4. Что НЕ делать (reject criteria)

- Править `side_panel_right/{view,tabs,mod}.rs` без явного разрешения
- Path-dep / workspace member на Chronos-FM
- Порт clipboard/rename/DnD/ops «заодно»
- Читать всё дерево при open
- Открывать файлы в editor / xdg-open
- `unsafe`, silent `let _ = …`
- Фейковые тесты / «должно работать» без paste вывода
- Копировать `reference/` gpui-shell

## 5. Верификация (обязательно)

1. `cargo check -p chronos` — зелёный (paste).
2. Если добавил unit-тесты на sort/listing helpers:
   `cargo test -p chronos -- …` — **реальный** вывод, не выдуманные имена.
   Тесты на listing — pure functions, без GUI.
3. `cargo build --release -p chronos` — зелёный.
4. **Живой смок** (release + `RUST_LOG=info`, при наличии
   `CHRONOS_SMOKE_SIDE_PANEL=1`):
   - открыть правую панель → вкладка Files
   - видно реальное содержимое `$HOME`
   - expand одной директории
   - ошибка чтения не крашит (если есть недоступная dir — показать)
   - **grim**: (a) collapsed/root (b) one expanded dir  
   Пути скринов — в отчёте.  
   **Нет GUI / не смог открыть** → статус **PENDING** на live, не
   врать DONE. Wire dispatch может быть PENDING у архитектора — тогда
   живой смок через временный вызов, который **не** коммитишь в
   forbidden files, или честный PENDING «ждёт wire».

## 6. Отчёт

`docs/orchestration/tasks/report/T115-ide-panel-files-tab-report.md`

Минимум:

- Outcome: PASS | PASS_WITH_CAVEATS | PENDING | FAIL
- Таблица FM → ChronOS
- Сигнатура + где expanded-state
- Dotfiles decision
- Paste: `cargo check` / `cargo test` / `cargo build --release`
- Live smoke: да/нет + grim paths / почему PENDING
- «Нужен wire архитектора: …» (точные 1–3 строки куда)

**Честность > красота.** Лучше PENDING + рабочий модуль, чем DONE с
враньём.

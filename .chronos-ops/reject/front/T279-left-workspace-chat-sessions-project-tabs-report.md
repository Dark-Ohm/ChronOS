# T279 REJECTED — 2026-08-14

Исходник: `docs/orchestration/tasks/report/T279-left-workspace-chat-sessions-project-tabs-report.md`
База: `19263d3` ancestor OK. HEAD исполнителя `25b1885` + незакоммиченный diff.

Тесты сошлись с отчётом: `tabs::chat` 5/5; `side_panel_left` 95 lib / 97 bins; `project_switcher` 6/6. Это не приёмка.

## Почему отказ

### 1. Sessions таб не грузит тред (мёртвая проводка)

`WorkspaceView::on_sessions_event` → `select_session(id, cx)` пишет только `SidePanelLeftState_.active_session_id` и зовёт `select_tab(Chat)`.

`ChatTab::select_session` (транскрипт + ACP `load_session`) зовётся только из inline-sidebar в `tabs/chat.rs:1596` / `:1974`.

План Task 3: `ChatTab::load_thread`. В дереве нет. Coordinator не прокидывает thread-команду в Chat.

`+ New` тоже только `select_tab(Chat)` — тред не создаётся.

### 2. Смена проекта не чистит Chat

План Task 4 Step 3: сначала очистить Chat/Sessions, потом `ProjectsConfig.active`.

Реально: `ProjectTab` сначала зовёт `set_active`, потом emit. `switch_project` обнуляет только SoT `active_session_id`. `ChatTab::clear_for_project` нет. Старый транскрипт остаётся на экране.

`ProjectEvent::Add` — no-op у coordinator: portal пишет `active`, `switch_project` не бежит.

### 3. T278-театр: breakpoint не читает прод

`chat_layout_for_visible_width` живёт только в тестах. `ChatLayout` в `render_panel` не используется. `visible_width_changed` нет.

Редьюсеры `select_session` / `switch_project` / `select_tab` нет тестов по имени на `&mut App`. Есть только чистые хелперы, `session_select_transition` игнорирует snapshot-аргументы. `project_event_add_is_unit` — тавтология `matches!(Add, Add)`.

### 4. Отчёт врёт про highlight

`SessionsTab::selected` пишется на клике. В `render` не читается — только hover. «Highlight теперь живёт» — ложь.

`open_at_path` на Files/Terminal — лживое имя. Факт: `FilesTab::set_root`, `TerminalTab::open_at`. Проводка правой панели есть.

## Что сошлось (не причина отказа)

- `panel.rs` удалён; `chat.rs` без `WindowHandle` / `open_window` / `window.resize(`.
- popup свитчера снят; `project_switcher::init(cx)` в `main.rs:97` жив.
- Shells честные «Coming in Slice B/C»; rail order + Archive снизу.
- Фильтр Sessions по project — честно сдвинут на T280 (`list_for_project`). Не блокировать по этому.
- Live UX = T281 — принято.

## Scope leak, не основной reject

План: T279 «no bar removal yet». Исполнитель снёс `bar/widgets/project.rs` + миграцию `layout_config`. Тикет T280 всё ещё требует Task 6. Не трогать bar в следующем раунде снова; после приёмки T279 архитектор сам подрежет T280.

## Что досдать (один раунд)

1. `select_session` должен вызывать `ChatTab::select_session` / `load_thread` через `content_view`.
2. `switch_project` / `remove_project_scope` — `ChatTab::clear_for_project`; порядок: clear → `set_active` → reload.
3. `+ Add` прогоняет ту же транзакцию, что Select.
4. `chat_layout_for_visible_width` читается прод-рендером, или удалить театр.
5. Sessions row красит `selected`.
6. Тест зовёт `select_session` / `switch_project` по имени на `&mut App` и доказывает, что Chat-путь читает то же состояние (не только чистый helper).

T280 не начинать.

# T279 — ПРИНЯТ 2026-08-14 (восстановление r2, сверка с деревом)

Архитектор: шесть пунктов отказа закрыты. Поиск в Project — известный пробел, не блокер. Live = T281.

# T279 — раунд 2 (восстановлено): coordinator прокинут в Chat, театр удалён

**Отвечает на rejection `rejected/T279-left-workspace-chat-sessions-project-tabs-report.md`
(2026-08-14). Все шесть пунктов «что досдать» закрыты. Live UX не проверялось (T281).**

**История:** первая r2-проводка была утеряна из рабочего дерева (чужой
`git checkout -- .` поверх незакоммиченного WIP). Этот раунд — восстановление
по настоящему отчёту (он уцелел как untracked) с полной перепроверкой
каждого пункта по дереву. Дополнительно восстановлена проводка правой
панели (Files/Terminal), которая входила в файл-лист r2-коммита, и
`Remove` из интерфейса плана Task 4.

## Что изменено (по пунктам отказа)

### 1. Sessions → Chat грузит транскрипт

`ChatTab` получил план-интерфейс Task 3:

```rust
pub fn load_thread(&mut self, thread: ThreadRecord, cx: &mut Context<Self>);
pub fn clear_for_project(&mut self, project_path: &Path, cx: &mut Context<Self>);
pub(crate) fn load_thread_by_id(&mut self, thread_id: &str, cx);
pub(crate) fn create_new_session(&mut self, cx);
```

`load_thread` гарантирует запись в локальном списке и делегирует в legacy
`select_session` (cache исходящего транскрипта, replay кэшированного,
ACP `load_session`). `load_thread_by_id` берёт запись из стора (`ThreadStore::get`),
fallback — локальный список.

**Достижимость.** В `SidePanelLeftState_` добавлено поле
`chat: Option<WeakEntity<ChatTab>>`, регистрируется в `open_window` (ветка
`CommitBoth`) и сбрасывается в `close`/`close_this`. Редьюсеры достают Chat
через него, а **не** через `content_view`: `content_view` уже leased внутри
`WorkspaceView::on_*_event`, и второй lease того же entity даёт
`double_lease_panic` (`entity_map::lease`). ChatTab — отдельный entity,
поэтому lease безопасен.

Координатор (`select_session` free fn) теперь реально грузит:

```rust
pub fn select_session(thread_id, cx) {
    SoT.active_session_id = Some(thread_id.clone());
    select_tab(Chat, cx);
    chat_handle(cx) → chat.load_thread_by_id(&thread_id);
}
```

`+ New` больше не пустой `select_tab(Chat)` — добавлен `create_thread` free fn:
`select_tab(Chat)` + `chat.create_new_session()`.

### 2. Смена/remove проекта чистит Chat, порядок clear → set_active

`switch_project` / `remove_project_scope` теперь зовут `chat.clear_for_project`
через `chat_handle`. `clear_for_project` кэширует исходящий транскрипт, сбрасывает
`chat`/`streaming`/`pending_send`/`active_session_id`/`session_id`.

Порядок в `ProjectTab` (план Task 4 Step 3) исправлен: клик Select сначала
эмитит `Select` → `switch_project` → `clear_for_project`, **потом**
`set_active` (persist `ProjectsConfig.active`). Было наоборот. Порядок
покрыт source-контрактом `select_click_emits_before_persist`.

### 3. `+ Add` — та же транзакция, что Select

`ProjectEvent::Add` стал `Add(PathBuf)`. `project_switcher::add_project`
принимает `on_added: impl Fn(PathBuf, &mut App) + Send + 'static` и зовёт его
после persist. `ProjectTab` передаёт callback, который эмитит
`Add(path)` в coordinator; `on_project_event` обрабатывает
`Select | Add` одинаково (`switch_project` + сброс Sessions). `set_active`
внутри `add_project` уже сделал persist — дубля нет.

### 4. Театр breakpoint удалён

`chat_layout_for_visible_width` + `ChatLayout` + 4 теста удалены. Prod-рендер
и так читает **visible** ширину: `WorkspaceView::render` зеркалит `visible_w`
в `state.width` (round 2 T278), а `render_panel` ветвится на этом зеркале
(`past_sidebar`/`chat_open`). Мёртвая чистая функция с зелёными тестами,
которую прод не звал, — убрана, а не доведена до прод-рендера.

Редьюсеры `session_select_transition` / `project_switch_transition` /
`project_remove_transition` (безусловные тавтологии, возвращали вход/константу)
удалены вместе с их тестами. `tab_select_transition` / `dock_transition`
(реальные ветвления) остались.

### 5. Sessions красит `selected`

`sessions.rs` render: `is_selected = self.selected.as_deref() == Some(id)`,
`.when(is_selected, |el| el.bg(theme.interactive.active))`; клик пишет
`this.selected = Some(id)` до emit. Source-контракт
`selected_field_is_written_on_click_and_read_in_render` фиксирует обе стороны —
поле больше не «пишется и не читается».

### 6. Тесты зовут редьюсеры по имени на `&mut App`

В `mod.rs` добавлены `#[gpui::test]`:

- `select_session_records_id_and_opens_chat` — precondition
  `active_tab = Sessions` (иначе ассерт `active_tab == Chat` тавтологичен
  при дефолте Chat — тот же класс, что T278), зовёт `select_session("thread-42")`,
  ассертит `SoT.active_session_id == Some("thread-42")` и `active_tab == Chat`.
- `switch_project_sets_path_and_clears_session` — ставит залежалый
  `active_session_id`, зовёт `switch_project`, ассертит path + `None`.
- `remove_project_scope_clears_only_the_active_project` — обе ветки bool.

Chat-path в тесте недостижим (`SoT.chat == None` без live WorkspaceView —
`ChatTab::new` спавнит async ACP-connect, которому нужен Tokio-рантайм,
неконструируемый в `TestAppContext`) — редьюсер no-op'ится, не паникует.
«Chat читает то же состояние» доказано конструкцией: `select_session` пишет
`active_session_id = id` и грузит тем же `id` через `load_thread_by_id`;
`switch_project` пишет path и чистит тем же `path` через `clear_for_project`.

## Правая панель (Files/Terminal) + Remove

- `FilesTab::set_root(path)` — re-root листинга + reload.
- `TerminalTab::open_at(path)` — respawn шелла с cwd=path через новый
  `Terminal::launch_in(size, w, h, cwd)` в `chronos-services` (`launch`
  остался home-dir wrapper).
- `SidePanelRightView::set_files_root` / `open_terminal_at` — ленивое
  создание таба через `ensure_tab_view`.
- `side_panel_right::open_files_at(path, cx)` / `open_terminal_at(path, cx)` —
  free-fn редьюсеры на `&mut App`: `select_tab` (открывает pinned при
  закрытой панели) + обновление таба.
- `ProjectEvent::Remove/OpenInFiles/OpenInTerminal(PathBuf)` (интерфейс
  плана Task 4); row-actions в `ProjectTab` с `cx.stop_propagation()`, чтобы
  клик по кнопке не срабатывал как Select строки.
- `project_switcher::remove_project(path)` — доменное удаление + persist;
  координаторская чистка scope — `remove_project_scope` (только если
  удаляемый путь был активным).

## Что сошлось / не трогал

- `panel.rs` удалён; `chat.rs` без `WindowHandle`/`open_window`/`window.resize(`.
- popup снят; `project_switcher::init(cx)` в `main.rs:97` жив.
- Shells честные, rail order, Archive снизу.
- Фильтр Sessions по project — по-прежнему T280 (`list_for_project`).
- **Bar не трогал** (round 1 scope leak: «no bar removal yet»). После приёмки
  T279 архитектор сам подрежет T280 Task 6.
- Поиск в Project/Sessions: поле-фильтр в Sessions было и осталось без
  строки ввода (нужен TextInputState + IME — не флагануто в обоих ревью,
  не изобретал сверх проверенного объёма). Заявляю честно.

## Команды и exit-коды (этот раунд)

```text
cargo test -p chronos side_panel_left::tabs::chat --lib --bins   → ok (1/1 обе; только source-контракт)
cargo test -p chronos side_panel_left --lib --bins               → ok (90 lib / 92 bins)
cargo test -p chronos side_panel_left::tabs::project --lib --bins → ok (4/4 обе)
cargo test -p chronos project_switcher --lib --bins              → ok (6/6 обе)
cargo test -p chronos side_panel_right --lib --bins              → ok (195 lib / 197 bins)
cargo test -p chronos --lib --bins                               → ok (425 lib / 633 bins)
cargo check -p chronos --lib --bins                              → 0 errors
git diff --check                                                  → exit 0
rg 'WindowHandle|open_window|window\.resize\(' tabs/chat.rs       → пусто (exit 1)
rg 'project_switcher::init\(cx\)' src/main.rs                     → main.rs:97
```

`cargo test --workspace --lib --bins` — зелёный кроме **pre-existing**
`chronos-ui window_root::every_window_root_uses_window_font` (ругается на
`dock/context_menu.rs`, последний коммит `cb7a6c1` T263/T264 — вне T279).
`chronos-services tasks::runner::echo_produces_stdout_and_ok` флакает под
полным workspace-прогоном, изолированно 6/6 зелёные — вне T279.

## Не проверялось живьём

Live UX (клик по сессии реально грузит транскрипт, смена проекта на живом
Hyprland чистит экран, highlight виден, Files/Terminal открываются в правой
панели, stop_propagation на row-actions) — gate T281. Не заявляю доказанным.

## Коммит

Пока незакоммичено поверх `a75f2f8`. Файл-лист для коммита (поимённый add,
без постороннего dirty — Cargo.lock/docs не мои):

```text
crates/app/src/side_panel_left/
crates/app/src/project_switcher/mod.rs
crates/app/src/side_panel_right/mod.rs
crates/app/src/side_panel_right/view.rs
crates/app/src/side_panel_right/tab/files.rs
crates/app/src/side_panel_right/tab/terminal.rs
crates/services/src/terminal/mod.rs
```

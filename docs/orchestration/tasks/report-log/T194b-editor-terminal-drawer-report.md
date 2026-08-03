# T194b report

**Зона:** `tab/preview.rs` (drawer chrome + resize + toggle) и
`tab/terminal.rs` (минимальная правка для переиспользования движка —
`available_height_override`). `tab/mod.rs`/`view.rs` не тронул — не
понадобилось (drawer целиком живёт внутри `PreviewTab`). `bar/` не трогал
(T198 recon зона).

## 1 Решение по переиспользованию движка

**Не** извлекал общий трейт/generic-движок и **не** копипастил 500 строк.
Вместо этого: `TerminalTab` (существующий тип из `tab/terminal.rs`) — уже
самодостаточная GPUI-сущность с собственным `Context`, PTY, poll-loop и
`Render`. Instantiировал **второй** `Entity<TerminalTab>` внутри
`PreviewTab` и вставил его как обычного child-элемента
(`.child(terminal.clone())`) — тот же паттерн, которым `SidePanelRightView`
уже вставляет вкладки (`TabContent::Terminal(entity) => col.child(entity.clone())`
в `view.rs`). Один и тот же тип `TerminalTab`, один и тот же PTY-движок
(`chronos_services::terminal::Terminal`), **два независимых экземпляра**:
рейловый Terminal-таб (сейчас не в `for_mode`, но жив в `ALL`) и
drawer-инстанс внутри Editor — как и требовало задание («still one
terminal engine»).

**Единственная правка в `terminal.rs`**: новое поле
`available_height_override: Option<f32>` + сеттер
`set_available_height` + правка `reconcile_geometry` — заменяет
`window.bounds().size.height` на override, когда он выставлен. Без этого
drawer-инстанс считал бы grid по высоте **всего окна**, а не своей
реальной коробки — типичная ошибка copy-paste-переиспользования, которую
задание явно предупреждало избегать. Ширина (`avail_w`) переиспользуется
как есть — drawer живёт в той же колонке правой панели, что и обычный
Terminal-таб, ширина совпадает без правок.

## 2 UX (по пунктам задания)

1. **Editor сверху, terminal снизу, default collapsed** — `drawer_open:
   bool` стартует `false`; `terminal_drawer: Option<Entity<TerminalTab>>`
   стартует `None` (PTY не создаётся, пока не открыт хотя бы раз).
2. **Toggle-кнопка в шапке рядом с Save** — «Terminal ▸» / «Terminal ▾» в
   `editor-header`, слева от Save.
3. **Resizable height, drag handle, min 80px, max ~50% таба** —
   `DRAWER_MIN_H = 80.`, `DRAWER_MAX_FRACTION = 0.5` от высоты окна
   (аппроксимация «50% таба» через высоту окна — таб занимает почти всю
   высоту окна, разница — на header). Drag handle — `div` высотой 6px,
   `cursor_row_resize()`, тот же паттерн `on_mouse_down` +
   `on_drag(Marker, |_,_,_,cx| cx.new(|_| gpui::EmptyView))` +
   `on_drag_move`, что уже использует горизонтальный ресайз панели в
   `view.rs` (`RightPanelResize`) — своя marker-структура
   `EditorTerminalResize`, чтобы не пересекаться с ней. Высота
   переклэмпливается **на каждый рендер**, не только в момент драга — если
   окно ужалось после того как drawer был растянут в большом окне, потолок
   50% пересчитывается и обрезает `drawer_height` до текущего актуального
   максимума.
4. **Lazy spawn на первый open; reuse session, пока живёт Editor tab
   entity** — `toggle_drawer` создаёт `Entity<TerminalTab>` только если
   `terminal_drawer.is_none()`; последующие toggles только флипают
   `drawer_open`, entity не пересоздаётся и не дропается при закрытии —
   PTY живёт, пока жив сам `PreviewTab` (тот же контракт кэширования, что
   у `TabContent` в `view.rs`: закрыть вкладку ≠ убить её сущность,
   вкладка кэшируется на всё время жизни панели).
5. **Фокус: клик в terminal → PTY, клик в editor → InputState, не
   перехватывать друг у друга** — ничего специально не проводил:
   `TerminalTab` уже владеет своим `FocusHandle` +
   `track_focus`/`on_mouse_down`-to-focus (не менял), `InputState`/`Input`
   из gpui-component управляет фокусом так же независимо. Обе области —
   отдельные focus-регионы в одном дереве; GPUI-шная модель фокуса уже
   даёт корректную изоляцию без дополнительной проводки с моей стороны —
   проверил это чтением кода `TerminalTab`, не придумывал заново.

## 3 Тесты

Три новых `#[gpui::test]` в `preview.rs`:

- `drawer_starts_closed_without_terminal` — `drawer_open == false`,
  `terminal_drawer.is_none()` сразу после `PreviewTab::new`.
- `toggle_drawer_creates_terminal_once_and_reuses_session` — открыть →
  закрыть → открыть; `entity_id()` терминала совпадает между первым и
  третьим тоглом — доказывает, что сессия не пересоздаётся.
- `drawer_resize_clamps_to_min_and_max` — драг далеко вверх клэмпится к
  `max_h`, драг далеко вниз клэмпится к `DRAWER_MIN_H`.

## 4 Верификация

```
$ cargo test -p chronos side_panel_right::tab::preview::
test result: ok. 25 passed; 0 failed   (было 22 до этой задачи, +3 новых)

$ cargo test -p chronos
test result: ok. 341 passed; 0 failed; 0 ignored

$ cargo clippy -p chronos --all-targets
# ноль новых предупреждений в диапазоне новых методов (tab/preview.rs:460-649,
# grep-проверено отдельно); существующие "redundant closure" warnings на
# cx.new(|cx| PreviewTab::new(cx)) — на строках существующих тестов, не мои.

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 3m 22s   (exit 0)
```

## 5 Что НЕ сделано

- **Живой прогон** (open Editor → toggle terminal → `echo ok` → resize →
  collapse, grim) — **не выполнен**, та же причина, что в T194: требует
  живого шелла + `ydotool`-кликов по координатам, в рамках этой сессии не
  запускал. Механика проверена статически (типы/сигнатуры совпадают с
  реальным кодом `TerminalTab`, компиляция чистая, тесты на состояние —
  зелёные), глазами не смотрел — фиксирую честно, не выдаю за готовое.
- **Rail `PanelTab::Terminal` restore** — не трогал, явно запрещено
  заданием (и, судя по `docs/orchestration/tasks/active/pause/T197-restore-terminal-tab.md`,
  это отдельная запланированная задача).
- **Layer-shell terminal window / desktop_terminal** — не трогал, вне
  области по заданию.
- **Полноценный multi-tab terminal IDE** — один drawer, одна сессия, как
  и просило задание («not a full multi-tab terminal IDE»).
- Визуальная полировка: drawer использует **тот же** заголовок
  «Terminal» + shell-имя, что и рейловый Terminal-таб (не делал
  компактный/безголовый вариант для узкой полосы) — задание не требовало
  компактного хедера явно, но это сознательное упрощение ради нулевых
  правок в `terminal.rs` сверх геометрии; называю прямо, не прячу.

## Коммит

`6a32ef6` — `editor : terminal drawer under editor (T194b)`. `git add`
поимённо: `tab/preview.rs`, `tab/terminal.rs`. `git status --short` перед
коммитом подтвердил отсутствие лишнего в staged (посторонние
удаления/правки в `docs/orchestration/tasks/` — чужая архитекторская
уборка, не мои, не трогал).

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED WITH RESIDUAL**

Сверка:

| claim | check |
|---|---|
| commit `6a32ef6` only preview.rs + terminal.rs | ✅ `git show --stat` |
| Entity\<TerminalTab\> inside PreviewTab, lazy, reuse | ✅ `toggle_drawer` + tests |
| `available_height_override` for grid vs window h | ✅ terminal.rs reconcile |
| DRAWER_MIN 80, MAX 50% window, drag marker own type | ✅ |
| header Terminal ▸/▾ near Save | ✅ `render_editor_body` |
| 3 new tests, 25 preview, 341 full suite | ✅ re-ran: 25 + 341 green |
| zone: no bar/, no rail restore | ✅ |
| live grim / echo ok | **NOT VERIFIED** (honest) |

**Residual (не блокер T194b, follow-ups):**

1. **Live smoke** — архитектор/юзер: open Editor → Terminal ▸ → `echo ok` →
   resize → collapse. Без этого UX-риск (focus, grid size, drag feel).
2. **Drawer только в `render_editor_body`** — при T194c (view default +
   Preview/Edit) drawer **не** будет виден в pure preview/image path, пока
   chrome не вынесут на общий header Editor. Зафиксировать в T194c:
   terminal toggle на **tab chrome**, не только edit body.
3. Компактный header drawer — сознательно full TerminalTab chrome; ok v1.

**Preview regression T194 (raw-only md)** — **не** закрыта этой задачей;
отдельный T194c (md-like Preview+Edit buttons).


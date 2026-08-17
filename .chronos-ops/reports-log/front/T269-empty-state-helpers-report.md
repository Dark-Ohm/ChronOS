# T269 — empty-state хелперы: материализация паттерна T252 (отчёт)

**Дата:** 2026-08-13. **Роль:** FRONTEND. **Статус:** выполнено, на приёмку.

Материализация решения T252 (`docs/DECISIONS.log`, 2026-08-13, включая
уточнения приёмки). Два хелпера в `tab/ui.rs`, замена всех перечисленных
тикетом вхождений по текстовым якорям, вычистка русского блока дисков,
тесты. Ничего переизобреталось — паттерн взят из тикета и записи решения.

## Что сделано по файлам

### `crates/app/src/side_panel_right/tab/ui.rs` (новое)

- `pub(crate) enum NoteSeverity { Muted, Error }` + приватный
  `fn color(&Theme) -> Hsla`: Muted → `text.muted`, Error → `status.error`.
- `pub(crate) fn empty_state_hero(theme, icon_path, title, hint,
  hint_severity, action) -> AnyElement` — канон EmptyTab: иконка 40px
  `text.muted.opacity(0.55)`, заголовок 13px SEMIBOLD `text.primary`,
  подсказка 11.5px по центру, gap 12px, опциональная ссылка-действие
  (muted → primary на hover, `.id("empty-state-action-{label}")`).
  `debug_assert!(!title.is_empty())`. Иконка приходит готовым путём —
  хелпер иконок не выдумывает.
- `pub(crate) fn empty_state_note(theme, message, severity) -> AnyElement` —
  `px(10)`/`py(16)`, 12px, цвет по severity. Bordered-варианта нет.
- Тесты: `hero_without_a_title_panics` (`#[should_panic]` под debug_assert),
  `hero_with_a_title_constructs` (smoke: без action и с boxed action),
  `note_severity_maps_to_theme_tokens` (маппинг severity → токены темы +
  smoke-конструкция note).

### `tab/mod.rs`

- `EmptyTab::render` схлопнут в один вызов `ui::empty_state_hero(theme,
  tab.icon_path(), tab.label(), placeholder_description(tab), Muted, None)`
  — обязательный пункт тикета, чтобы канон не размножался копипастой.
- Импорты сужены: `FontWeight`/`div`/`px`/`svg` после схлопывания не нужны,
  `prelude::*` оставлен (даёт `AppContext` для `cx.new` в реестре).

### `tab/preview.rs`

- «No file selected» → `empty_state_hero` с контекстной `icons/folder.svg`
  (санкционированная вариация), action = («Open Files», тот же `cx.listener`
  → `on_tab_select(PanelTab::Files)`). `px(24)` оставлен на внешней обёртке.
- Копирайт подсказки: «Click any file in the Files tab to preview it here.»
  — см. «Отклонения», п.3.
- Удалён ставший неиспользуемым импорт `svg`.

### `tab/terminal.rs`

- Failed-ветка «Terminal is unavailable» → `empty_state_hero` с контекстной
  `icons/rail-terminal.svg` и `NoteSeverity::Error` (подсказка — точный
  текст ошибки спавна, цвет `status.error` по матрице T252 «отказ → hero +
  status.error»). Обёртка `.id("terminal-failed").flex_1().min_h(0).px(16)`
  сохранена — геометрия flex-ребёнка тела терминала, не типографика.
  Exited-ветка (dimmed + баннер + restart) не трогалась — не в списке.

### `tab/library.rs`

- «No games detected» → `empty_state_hero` с
  `PanelTab::Library.icon_path()` = `icons/rail-library.svg` (однозначное
  решение архитектора; раньше иконки не было — дрейф). Дрейф gap 8 /
  hint 11px съехал на канон; `px(20)`/`py(40)` оставлены на внешнем
  контейнере (стейт живёт внутри скроллящегося списка, не на полной
  поверхности) — как разрешено тикетом.
- Удалён неиспользуемый `prelude::*` (пред-существующий warning в файле
  моей зоны; строка импорта при этом сократилась).

### `tab/files.rs`

- «Loading…» (Muted), «Cannot read '…'» (Error), «Directory is empty»
  (Muted) → `empty_state_note`; матч теперь возвращает
  `(String, NoteSeverity)` вместо `(String, Hsla)`.
- Truncated-баннер «Showing N of M (limit X)» НЕ конвертирован — см.
  «Отклонения», п.4.

### `tab/build.rs`

- «No active project…» (Error), «Tasks unavailable without an active
  project.» (Muted), «No tasks found. Looked in: …» (Muted), «Output will
  appear here when a task runs.» (Muted) → `empty_state_note`. Active-арм
  матча в заголовке получил `.into_any_element()` (унификация типов веток).

### `tab/hypr_binds.rs`

- `LoadState::Error` («No Hyprland binds found — check …») → note с
  `NoteSeverity::Error` — осознанное исключение сохранено (0 биндов =
  сломанный конфиг).
- Ветка Ready+empty («No binds found — see …») → note Muted — тот же
  inline-паттерн; конвертировать половину файла = пересоздать дрейф.

### `tab/bar_settings.rs`

- «No modules found in ~/.config/hypr/modules/» → `empty_state_note`
  (Muted); bordered-xs оформление убрано — bordered не канон. Импорт
  расширен (`empty_state_note`, `NoteSeverity`).

### `side_panel_right/disks.rs`

- «нет дисков» → `empty_state_note(theme, "No disks detected", Muted)`.
- Кнопки: «монтировать» → «Mount», «размонт.» → «Unmount», «извлечь» →
  «Eject». Подписи короче русских оригиналов, раскладку (10px, flex_1)
  не ломают; геометрия не тронута. Doc-комментарий секции обновлён.
- `MONTHS_RU` в `power_row.rs` / `bar/widgets/clock.rs` НЕ тронуты —
  осознанное исключение локали даты/времени (DECISIONS.log 2026-08-13).

## Отклонения от тикета (с обоснованием)

1. **Сигнатура hero расширена параметром `hint_severity: NoteSeverity`.**
   Скетч в тикете «ориентировочный»; без severity хелпер не может выразить
   ветку матрицы T252 «отказ → hero + `status.error`» (Terminal Failed с
   muted-подсказкой потерял бы красный — регресс против решения). Enum не
   размножался: переиспользован `NoteSeverity` из тикета.
2. **Тип action — boxed** (`Box<dyn Fn(&ClickEvent, &mut Window, &mut App)
   + 'static>`), не generic `impl Fn`. Тикет допускал оба; с generic-парамом
   все `None`-сайты (3 из 4 вызовов) потребовали бы уродливой аннотации
   типа. Box — один, в preview.
3. **Preview: копирайт и раскладка action.** Ссылка обязана стать
   action-параметром (тикет), а action в хелпере — отдельная строка под
   подсказкой (inline-ссылку в середине предложения сигнатура вида
   `(SharedString, Fn)` выразить не может). Чтобы «Files» не дублировалось
   в тексте и в ссылке, подсказка стала «Click any file in the Files tab to
   preview it here.», лейбл ссылки — «Open Files». Поведение ссылки
   неизменно: тот же listener, тот же `select_tab`-путь, что у rail/IPC
   (живьём подтверждён ещё в T237 — кликабельность не тронута).
4. **Truncated-баннер files.rs оставлен как есть** — тикет прямо разрешает:
   «баннер — отдельный вид с фоном; если под note не ложится — оставить и
   обосновать». Под note не ложится: баннер показывается НАД живым списком
   (состояние не пустое), имеет `bg(elevated)` + rounded + py(6)/text-11 —
   это notice, а не empty-state.
5. **`id` action-ссылки** — `empty-state-action-{label}` вместо старого
   `preview-empty-files-link` (хелперу нужен стабильный id для stateful
   on_click; на старый id никто не ссылается — проверено grep'ом).

## Верификация (только статика — живые прогоны не выполнялись)

- `cargo test -p chronos side_panel_right --lib` — **174 passed, 0 failed**
  (167+ на момент тикета + 3 новых; все три новых видны в выводе поимённо).
- `cargo test -p chronos --lib` — **309 passed, 0 failed** (повторён после
  финальных правок — снова зелёный).
- `cargo check -p chronos` — **зелёный** (lib+bin) на момент окончания
  правок; warnings в файлах зоны отсутствуют.
- `cargo build --release -p chronos --lib` — **зелёный** (14.3s).

### Блокер полной release-сборки — чужой, не мой

Полный `cargo build --release -p chronos` в момент верификации падает на
bin-таргете ошибками E0425/E0603 в `tray_menu/mod.rs` ↔ `dock/context_menu.rs`
(`MENU_WIDTH`, `shortcut_to_glyph` — приватность). Это незакоммиченная работа
T263/T265-0, вне моей зоны. Доказательство непричастности: мой зелёный
`cargo check -p chronos` (lib+bin) зафиксирован после моей последней правки
(ui.rs, mtime 10:02:44); `tray_menu/mod.rs` изменён в 10:07:52 — уже после,
вторым исполнителем, правки продолжаются прямо во время верификации (число
ошибок менялось между прогонами 5 → 2). Вся моя зона лежит в lib-таргете —
она проверена lib-командами выше. Чужие файлы не трогал.

## Коммит

Только 10 файлов зоны, поимённый `git add`, `git diff --staged` просмотрен
глазами. Сообщение: `ui : unify empty-state pattern across right panel tabs
(T252)`, тело: «материализация T252, тикет T269». Тикет и этот отчёт не
коммитятся — приёмка за архитектором.

## Остаточные наблюдения (вне тикета, на усмотрение архитектора)

- Terminal Failed не имеет recovery-действия (restart есть только у Exited).
  Матрица T252 упоминает «Terminal: restart» как образец recovery; бэкенд
  (`this.restart`) существует. Не добавлял — новое поведение вне тикета,
  требует живой проверки.
- `render_error` / `render_loading` / «Image too large» и др. hero-подобные
  состояния preview.rs остались рукописными — вне списка якорей тикета.
- mpris «No player» (compact-collapse, T248) сознательно не тронут — тикет
  его не меняет.

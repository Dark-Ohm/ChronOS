# T269 — empty-state хелперы: материализация паттерна T252

**Приоритет:** P2.
**Роль:** FRONTEND (`crates/app/src/side_panel_right/**`).
**Источник:** решение T252, `docs/DECISIONS.log` (2026-08-13, с поправкой
при приёмке) + таблица в `done/T252-empty-state-pattern-audit.md`.
Паттерн согласован архитектором — этот тикет его материализует, ничего
переизобретать не нужно.

## Что нужно

### 1. Два хелпера в `crates/app/src/side_panel_right/tab/ui.rs`

**`empty_state_hero`** — вся поверхность пуста. Канонические параметры
(эталон — `EmptyTab` в `tab/mod.rs`, решение архитектора 2026-08-13;
дрейф от него этот тикет и убирает): иконка 40px
`text.muted.opacity(0.55)`, заголовок 13px SEMIBOLD `text.primary`,
подсказка 11.5px `text.muted` по центру, gap 12px, опциональное
действие-ссылка (muted → primary на hover, образец — «Files» в
empty-стейте Preview). Сигнатура ориентировочно:

```rust
pub(crate) fn empty_state_hero(
    theme: Theme,
    icon_path: &str,
    title: &str,
    hint: &str,
    action: Option<(SharedString, impl Fn(&ClickEvent, &mut Window, &mut App) + 'static)>,
) -> AnyElement
```

Типизацию действия уточнить по месту (у каждой вкладки свой `cx.listener`
— вероятно, generic `impl Fn` или boxed; не тащить `Context<КонкретнаяВкладка>`
в сигнатуру). `debug_assert!(!title.is_empty())` — hero без заголовка
бессмысленен.

**Иконка — готовый путь параметром, не внутри хелпера** (решение
архитектора 2026-08-13): вызывающий подставляет `tab.icon_path()` своей
вкладки — тогда «пустой Library» и «нереализованный Scenes» читаются как
одна семья, а не два разных экрана. `PanelTab::icon_path()` уже покрывает
все вкладки (`tabs.rs`, ассеты зарегистрированы в `assets.rs`) — новые
иконки не рисуем, в `assets.rs` не лезем. Preview («No file selected»)
и Terminal Failed — контекстные иконки (`folder.svg`, `rail-terminal.svg`),
не табовые; оставляем как есть, это осмысленная вариация, не дрейф.

**`empty_state_note`** — пустая секция/список внутри живой вкладки.
Канон: `px(10)`/`py(16)`, текст 12px, цвет по severity:

```rust
pub(crate) enum NoteSeverity { Muted, Error }

pub(crate) fn empty_state_note(theme: Theme, message: &str, severity: NoteSeverity) -> AnyElement
```

`Muted` → `text.muted`, `Error` → `status.error`. Никаких bordered-вариантов
— BarSettings переезжает на общий вид.

### 2. Замена вхождений — искать ПО ТЕКСТУ, не по номеру строки

(прямое указание приёмки: номера строк в таблице T252 уже дрейфанули).

Hero → `empty_state_hero`:
- `tab/preview.rs` — «No file selected» (+ ссылка на Files — она же
  канон для action-параметра).
- `tab/terminal.rs` — Failed-ветка, «Terminal is unavailable».
- `tab/library.rs` — «No games detected». Иконка —
  `PanelTab::Library.icon_path()` → `icons/rail-library.svg` (однозначное
  решение архитектора 2026-08-13; ассет на месте, ничего нового не
  добавлять). Текущие gap 8 / hint 11px — дрейф, съезжают на канон;
  `py(40)` допустимо оставить как параметр внешнего контейнера, но не как
  второй набор типографики.
- `tab/mod.rs` — `EmptyTab::render` **схлопывается в один вызов
  `empty_state_hero`** (обязательный пункт, решение архитектора: иначе
  канон опять размножится копипастой — ровно с этого T252 и начался).
  Иконка — `tab.icon_path()`, заголовок — `tab.label()`, подсказка —
  `placeholder_description(tab)`; без action.

Inline → `empty_state_note`:
- `tab/files.rs` — «Loading…», «Directory is empty», «Cannot read '…'»
  (Error), truncated-баннер «Showing N of M…» (баннер — отдельный вид с
  фоном; если под note не ложится — оставить и обосновать, не насиловать).
- `tab/build.rs` — «No active project…» (Error), «Tasks unavailable
  without an active project.», «No tasks found. Looked in: …»,
  «Output will appear here when a task runs.»
- `tab/hypr_binds.rs` — «No Hyprland binds found…» (Error — осознанное
  исключение, см. DECISIONS.log; severity Error сохранить).
- `tab/bar_settings.rs` — «No modules found in ~/.config/hypr/modules/»
  (сейчас bordered-xs — переехать на note, bordered не канон).
- `disks.rs` — «нет дисков» → «No disks detected» (Muted).

### 3. Вычистка русского блока дисков (поправка приёмки T252)

`disks.rs`, одним куском с «нет дисков»: «монтировать» → «Mount»,
«размонт.» → «Unmount», «извлечь» → «Eject». Если подпись кнопки после
перевода ломает раскладку — подобрать короткий вариант («Unmount» вместо
«Dismount» и т.п.), не трогая геометрию.

**Явно НЕ трогать:** `MONTHS_RU` в `power_row.rs` и `bar/widgets/clock.rs`
— локаль даты/времени, осознанное исключение из языковой планки
(DECISIONS.log 2026-08-13). Кто «починит» часы на английский — тот
разъедет панель с баром.

### 4. Тесты

- `empty_state_hero` с пустым title паникует под debug_assert
  (`#[should_panic]`-тест) + smoke-тест непустого.
- Существующие тесты `side_panel_right` остаются зелёными (167+ на момент
  заведения тикета).

## Зоны файлов

`crates/app/src/side_panel_right/tab/ui.rs` (новое), точечно `tab/mod.rs`,
`tab/preview.rs`, `tab/terminal.rs`, `tab/library.rs`, `tab/files.rs`,
`tab/build.rs`, `tab/hypr_binds.rs`, `tab/bar_settings.rs`,
`side_panel_right/disks.rs`.

Пересечений с непринятой работой T263/T265-0 в дереве нет (та лежит в
`icon_resolution.rs`, `launcher/`, трее/доке/попапах) — коммитить можно
сразу после верификации.

## Верификация

```text
cargo test -p chronos side_panel_right
cargo check -p chronos
cargo build --release -p chronos
```

Живой grim: System-таб без плеера (компакт «No player» на месте — этот
тикет его НЕ меняет), Preview без файла, Library без игр, Files на
пустой/битой папке — визуально тот же язык, что до правки, но единый.
**Живой прогон только с ведома пользователя** — T264 открыт
(`T264-popup-grab-kills-compositor-input.md`, «Правила прогона»).

## Коммит

`ui : unify empty-state pattern across right panel tabs (T252)`
(тело коммита: «материализация T252, тикет T269»).

## Отчёт

`docs/orchestration/tasks/report/T269-empty-state-helpers-report.md`.

# Левая AI-native workspace: standalone rail и fixed content canvas

**Статус:** review — архитектурное направление принято, end-to-end scope
уточняется после ревью владельца, 2026-08-13

**Следующий этап:** implementation plan только для Slice A после повторной
проверки владельцем

**Заменяет:** текущую combined-surface архитектуру `side_panel_left`

## 1. Цель

Полностью перестроить левую панель ChronOS, сохранив её продуктовую роль:
это AI-native workspace для работы с проектом, агентными сессиями и их
контекстом, а не универсальная системная панель.

Новая панель получает самостоятельную навигационную rail и отдельный fixed
content canvas. Архитектура должна убрать нынешний 1500-строчный god-object,
разделить lifecycle, навигацию и вкладки и исключить изменение Wayland window
bounds во время пользовательского resize.

## 2. Не входит в задачу

- Переписывание Hermes ACP, permission protocol или всего ThreadStore.
  Узкая schema/query migration ThreadStore для project scope и active session
  явно входит в Slice A.
- Изменение правой панели.
- Изменение gpui-ce fork.
- Добавление внутренних IPC-вызовов между rail и content.
- Одновременное сохранение старой combined surface и новой архитектуры.
- Превращение левой панели в dashboard системных модулей.
- Полная реализация Plan, Context Files, Archive, Tools и Skills внутри
  архитектурного cutover Slice A.

## 2.1. Каноническая reference implementation

Единственный green path для surface/lifecycle части — завершённый T276:

- `crates/app/src/side_panel_right/mod.rs`;
- `crates/app/src/side_panel_right/view.rs`;
- `crates/app/src/side_panel_right/rail_view.rs`;
- `docs/orchestration/tasks/done/T276-standalone-right-rail-and-fixed-content-canvas.md`;
- `docs/orchestration/tasks/report-log/T276-standalone-right-rail-and-fixed-content-canvas-report.md`.

Левая реализация обязана зеркалить, а не переизобретать следующие контракты:

- `two_surface_open_outcome` и commit-both/rollback-first lifecycle;
- content `exclusive_zone: Some(px(-1.))`, потому что `None` означает
  протокольный `0` и допускает compositor auto-offset;
- явный top margin `panel_edge_gap()` после opt-out от bar zone;
- явный постоянный inner margin ровно 40 px между screen edge и content;
- fixed window bounds, `content_input_region` и input-transparent void;
- `content_interactive_width` и сохранение 4px handle до mouse-up на clamp;
- `peek_generation`, generation-guarded delayed close и запрет peek-close при
  `resizing`;
- responsive breakpoints по видимой ширине content, не по фиксированным
  `window.bounds() == 920`;
- отсутствие `window.resize()` в rail/content runtime path.

Отличия левой стороны ограничены зеркальной осью, продуктовым tab set и
существующим Chat/ACP поведением. Копировать right-aligned формулы без
зеркального преобразования запрещено.

## 3. Поверхности и геометрия

Логическая панель состоит ровно из двух управляемых layer-shell surfaces:
rail и content. Существующая третья техническая surface
`side_panel_left_hover_strip` остаётся постоянным прозрачным 4px peek-trigger
и не входит в пару handles логической панели.

### 3.1 Rail

- Namespace: `side_panel_left_rail`.
- Anchor: `TOP | LEFT`.
- Layer: `Overlay`.
- Window background: `Transparent`.
- Keyboard interactivity: `None`.
- Постоянный размер после создания: 40 px × доступная высота под bar.
- Rail является цельной поверхностью; resize handle не занимает её ширину.
- Тонкий separator рисуется на внутренней, правой кромке.

### 3.2 Content

- Namespace: `side_panel_left_content`.
- Anchor: `TOP | LEFT`.
- Layer: `Overlay`.
- Window background: `Transparent`; root fixed canvas не получает сплошной
  opaque background.
- Keyboard interactivity: `OnDemand`.
- Фиксированный canvas: 920 px × доступная высота под bar.
- Постоянный left margin: 40 px, поэтому canvas всегда начинается сразу после
  rail.
- Content использует layer-shell opt-out от чужих exclusive zones и явные
  top/left margins; rail zone не должна дополнительно сдвигать canvas.
- Видимый content прижат к левой стороне canvas. Неиспользуемая прозрачная
  часть находится справа.
- Input region содержит только видимый прямоугольник. Прозрачный остаток не
  блокирует окна под ним.
- Внешняя правая кромка видимого content имеет тонкий separator.

Точные LEFT-формулы (`RAIL_ONLY_WIDTH = 40`, `HANDLE_WIDTH = 4`,
`CONTENT_CANVAS_WIDTH = 920`):

```text
visible_w = clamp(panel_width - 40, 0, 920)

interactive_w = resizing ? max(visible_w, 4) : visible_w

input_region = []                                      if interactive_w <= 0
input_region = Bounds(x=0, y=0,
                      width=min(interactive_w, 920),
                      height=max(canvas_h, 0))          otherwise

handle_x = clamp(visible_w - 4, 0, 916)

resize_target_width = clamp(start_width + (current_x - start_x),
                            tab_min_width,
                            960)
```

В отличие от right, input region начинается с `x=0`: видимый content
left-aligned. Движение курсора вправо увеличивает ширину, влево — уменьшает.
При `visible_w=0` активный drag сохраняет input region 4 px и handle на
`x=0`, поэтому панель можно вернуть из rail-only тем же жестом.

Полная логическая ширина панели лежит в диапазоне `40..=960` px.

Ни одна surface после создания не вызывает `window.resize()`. Resize меняет
только shared state, ширину отрисованной content-колонки и input region.

Resize handle — прозрачный 4px hitbox поверх внешней правой кромки видимого
content. Он существует только на ресайзабельных вкладках и остаётся
интерактивным до mouse-up при достижении clamp. Unit tests обязаны проверять
каждую формулу выше, включая `visible_w=0`, full canvas и обе drag delta.

## 4. Lifecycle и режимы

- `Super+A` при закрытой панели атомарно открывает обе surfaces в rail-only
  состоянии.
- `Super+A` при открытой панели закрывает rail и content целиком.
- Rail может существовать визуально одна: content canvas остаётся созданным,
  но имеет пустые paint/input regions.
- Нажатие неактивной вкладки выбирает её и раскрывает content.
- Повторное нажатие активной вкладки схлопывает content до rail-only.
- Последняя активная вкладка запоминается, но `Super+A` не раскрывает её
  автоматически.
- Partial open запрещён: failure второй surface закрывает первую и не
  публикует handles в state.
- Open order зеркалит T276: сначала content, затем rail; оба handles и weak
  workspace entity публикуются одним commit только после успеха rail.
- Close забирает и очищает оба handles до удаления окон. Ошибка удаления
  одной surface не мешает удалить вторую и логируется отдельно.
- Rail, content и hover strip используют один `peek_generation`. Enter любой
  из двух видимых surfaces вызывает `hold_peek` и увеличивает generation;
  leave вызывает только generation-guarded `schedule_release_peek`. Delayed
  callback закрывает панель лишь если generation не изменился, панель не
  pinned и `resizing == false`. Поэтому переход rail ↔ content и активный drag
  не закрывают панель.
- Exclusive zone принадлежит rail: 40 px в overlay mode и текущая полная
  ширина панели в dock mode. Content самостоятельно пространство не
  резервирует.
- Rail, content и hover strip получают один и тот же display id через текущий
  `pult_display_id_or_primary` fallback chain. Multi-monitor policy не меняется
  этой программой работ.

### 4.1. Dock state machine

Dock toggle — отдельная secondary-кнопка внизу rail под Archive, визуально
отделённая от вкладок. `⊞` включает dock, `⊟` выключает.

| Исходное состояние | Событие | Результат |
|---|---|---|
| closed | `Super+A` | обе surfaces открыты, rail-only, pinned, dock off |
| любое open | `Super+A` | обе surfaces закрыты, dock сброшен |
| rail-only | click tab | tab active, content открыт в remembered/preferred width |
| overlay content | click active tab | content схлопнут, rail остаётся |
| overlay content | click other tab | tab переключён, применена его width policy |
| rail-only | dock toggle | active tab открыт в remembered/preferred width, dock on |
| overlay content | dock toggle | текущая ширина сохранена, dock on, rail резервирует full width |
| docked content | click active tab | no-op; dock запрещает collapse, как на right |
| docked content | click other tab | tab переключён, docked width остаётся pinned |
| docked content | dock toggle | dock off, content остаётся открыт в текущей ширине |

При dock текущая ширина сильнее preferred/fixed tab policy. После выхода из
dock следующее обычное tab switch снова применяет tab policy.

## 5. Навигационная rail

Порядок сверху вниз:

1. Project Switcher.
2. Sessions.
3. Chat.
4. Plan.
5. Tools.
6. Skills.
7. Context Files.
8. Archive — закреплён внизу.

Группы: project selector / work / resources / archive. Project Switcher
показывает sigil или инициал текущего проекта и является selector контекста
всего workspace.

Существующий Project Switcher удаляется из bar: две конкурирующие точки
владения project context не сохраняются.

Active indicator расположен на внутренней правой кромке rail, визуально
соединяя кнопку с content. Active state использует accent strip и chrome
подложку; hover не подменяет active state. Sessions может показывать unread
count, Chat — streaming/status dot, Plan — индикатор активного выполнения.
Остальные кнопки не получают декоративных badges.

Все кнопки имеют tooltip с названием и shortcut. Текстовые подписи внутри
40px rail не рисуются.

Archive прижимается вниз через `flex_1` spacer после resource group; dock
toggle располагается под Archive в отдельной secondary group. Положение не
зависит от количества badges или высоты верхних групп.

## 6. Семантика вкладок

### Project Switcher

Полноценная вкладка: поиск, recent projects, текущая ветка и проектные
действия. Переключение проекта атомарно меняет весь project-scoped контекст и
восстанавливает последнюю активную сессию этого проекта.

Канон project identity — canonical project path из `ProjectsConfig`.
Последняя сессия проекта не хранится только в GPUI global: Slice A повышает
`ThreadStore` schema и добавляет persistent
`workspace_project_state(project_path PRIMARY KEY, active_thread_id)`.
Transient active ids зеркалятся в `SidePanelLeftState`, но SQLite остаётся
источником восстановления после рестарта. Stale `active_thread_id` очищается
при загрузке и даёт пустой Chat.

### Sessions

Отдельный менеджер сессий текущего проекта. Chat не содержит собственного
списка сессий. Выбор сессии в Sessions автоматически переключает workspace в
Chat с выбранным разговором.

Project scope thread'а определяется canonical project path. Slice A добавляет
явный project-path query/migration в `ThreadStore`; неявный глобальный
`list(None, ...)` не используется для Sessions текущего проекта.

### Chat

Рабочая поверхность текущей сессии: transcript, composer, ACP streaming,
permissions и agent/model controls. Существующие специализированные
компоненты сохраняются, но не владеют window lifecycle.

### Plan

Исполняемый план текущей chat-сессии, включая состояние шагов и историю
выполнения. Это не project roadmap.

### Tools и Skills

Глобальные каталоги доступных capabilities. Включённый набор хранится на
уровне проекта; отдельные скрытые session overrides не вводятся. Chat
наследует разрешённый проектом набор.

### Context Files

Показывает файлы текущего проекта. Выбранный набор контекста, порядок и
attachment metadata принадлежат текущей сессии и восстанавливаются при её
переключении.

### Archive

Архивированные сессии текущего проекта: поиск, просмотр и восстановление.
Глобальный поиск по всем проектам доступен явным фильтром внутри вкладки.

## 7. Политика ширины

Независимую runtime-only память пользовательской ширины по типу вкладки
(не по chat session и без persistence после рестарта ChronOS) имеют:

- Chat;
- Plan;
- Context Files.

Каждая из этих вкладок запоминает свою ширину отдельно. Начальные полные
ширины панели: Chat — 560 px, Plan — 480 px, Context Files — 560 px; диапазон
ручного resize — `360..=960` px.

Фиксированные полные ширины:

- Project Switcher — 440 px;
- Sessions — 400 px;
- Tools — 440 px;
- Skills — 440 px;
- Archive — 440 px.

На фиксированных вкладках dragger не рендерится. Переключение вкладок
применяет её remembered/preferred width внутри fixed canvas, не меняя bounds
surface.

## 8. Ownership и границы кода

`SidePanelLeftState` остаётся единым source of truth для UI/lifecycle state и
владеет:

- `rail_handle` и `content_handle`;
- weak entity content workspace;
- active tab;
- open/rail-only, pin/peek, dock и resizing state;
- transient active project/session ids (persistent restoration принадлежит
  `ThreadStore.workspace_project_state`);
- remembered widths ресайзабельных вкладок;
- последней отправленной exclusive zone.

Разбиение модулей:

- `side_panel_left/mod.rs` — публичный API, init, bind lifecycle, атомарное
  открытие и закрытие двух surfaces;
- `side_panel_left/state.rs` — чистое состояние, переходы и геометрия без
  GPUI element tree;
- `side_panel_left/rail_view.rs` — rail render и callbacks навигации;
- `side_panel_left/workspace_view.rs` — fixed canvas, input region, dragger и
  tab routing;
- `side_panel_left/tabs/` — отдельный модуль каждой вкладки;
- существующие `composer`, `chat_view`, ACP adapters и thread services —
  специализированные компоненты, используемые Chat tab.

Вкладки создаются лениво и переиспользуются. Project/session-scoped данные
берутся из stores/services по идентификаторам shared state; они не должны
застревать как независимая копия внутри UI entity.

Rail вызывает workspace через weak entity и общий state. IPC `expand-left`,
`compose-and-send` и будущие tab actions проходят через тот же публичный API,
что интерактивная rail, без отдельного внутреннего транспорта.

## 9. Data flow и fallback

Основной поток:

`rail input → shared state transition → workspace weak entity → lazy tab →
service/store → refresh both windows`.

Project switch выполняется как один доменный переход: новый проект
валидируется, затем одним commit обновляются project id, восстановленная
session id и зависимые tab scopes. Наблюдатели не должны видеть смешанное
состояние старого проекта и новой сессии.

Fallback rules:

- отсутствующий/удалённый проект открывает Project Switcher;
- отсутствующая/удалённая сессия открывает пустой Chat и очищает session-bound
  Plan/Context selections;
- ошибка загрузки отдельной вкладки показывает локальный error state с retry,
  не закрывая surfaces;
- ошибка ACP turn не разрушает transcript и не сбрасывает выбранный проект;
- dropped weak entity и orphan handle логируются и восстанавливаются через
  единый open/close lifecycle, а не panic.

### 9.1. Keyboard и focus

- Rail использует `KeyboardInteractivity::None` и никогда не забирает keyboard
  focus.
- Content использует `KeyboardInteractivity::OnDemand`.
- Открытие Chat через rail, выбор session в Sessions и `expand-left` после
  paint/focus-ready frame фокусируют composer в content window.
- Project Switcher и Sessions фокусируют собственный search input только при
  явном открытии соответствующей вкладки; project/session transition в фоне
  не крадёт focus.
- `compose-and-send` не зависит от Wayland seat focus для отправки, но после
  открытия переводит видимый focus в composer.
- Focus target хранится внутри tab entity; rail не владеет `FocusHandle`.

### 9.2. Публичный API и IPC state machine

| API | closed | rail-only | другой/open tab | dock |
|---|---|---|---|---|
| `toggle` / `Super+A` | открыть pinned rail-only | закрыть обе surfaces | закрыть обе surfaces | закрыть обе surfaces и сбросить dock |
| `expand-left` | открыть обе, active=Chat, dock on | active=Chat, открыть remembered/default Chat width, dock on | переключить Chat, обеспечить ширину, dock on | active=Chat, сохранить dock width |
| `compose-and-send(text)` | выполнить `expand-left`, затем fill+submit | то же | то же | active=Chat, fill+submit без изменения dock width |

`expand-left` всегда фокусирует composer. `compose-and-send` очищает старый
draft, устанавливает полный payload и вызывает тот же production submit path,
что UI; отдельного «только заполнить» поведения нет. Существующая защита от
параллельного mid-turn submit сохраняется.

### 9.3. Responsive layout

Любой breakpoint вкладки вычисляется из `visible_w` либо переданного
`WorkspaceViewport`, никогда из `window.bounds()` fixed canvas. Общий helper
получает видимую ширину из `SidePanelLeftState`; unit regression обязан
доказывать, что узкий видимый slice внутри 920px window выбирает narrow layout.

## 10. Анимация и chrome

Layer-shell geometry не анимируется. Разрешена только лёгкая content enter
animation внутри fixed canvas. Rail не сдвигается и не масштабируется.

Rail и content используют токены текущей темы, separator language bar/right
panel и существующие elevation tokens. Между rail и content нет gap, shadow
или второго resize strip. Верхний bar gap берётся из общего
`panel_edge_gap()`.

## 11. Phased delivery

Этот документ описывает конечный продукт, но не является одним implementation
тикетом. Каждый slice получает отдельный plan, тикеты и live acceptance.

### Slice A — обязательный architecture cutover

- две logical surfaces + существующий hover strip;
- project-first rail chrome и tab router для всех восьми кнопок;
- полноценные Project Switcher, Sessions и Chat;
- честные tab shells для Plan, Tools, Skills, Context Files и Archive: название,
  scope indicator и `Not implemented in Slice A`, без fake data/actions;
- generic preferred/resizable width policy для всех tab metadata;
- fixed canvas geometry, input region, handle-at-clamp и visible-width helper;
- dock/peek/pin, focus и две-surface lifecycle;
- `Super+A`, `expand-left`, `compose-and-send` и существующие ACP regressions;
- перенос Project Switcher из bar;
- `ThreadStore` project scope + persistent active session per project.

Slice A не заявляет Plan/Context/Archive/Tools/Skills функциональными. Он
считается закрытым, когда architecture cutover и три реальные вкладки приняты
живьём.

### Slice B — session knowledge

- Plan store/UI, keyed by thread id;
- Context Files attachment store/UI, keyed by thread id and project path;
- Archive project/global queries, restore flow и UI.

Plan, Context Files и Archive могут быть разнесены на отдельные тикеты внутри
Slice B; их stores не должны создаваться как случайные поля Slice A UI.

### Slice C — project capabilities

- глобальные каталоги Tools и Skills;
- отдельный persistent project-enablement store keyed by canonical project
  path;
- Tools/Skills UI и применение разрешённого набора к Chat/ACP session.

Session overrides не входят в Slice C. Slice C не блокирует architecture
cutover и не смешивается с ThreadStore migration Slice A.

## 12. Миграция и cutover

Новая архитектура заменяет старую combined surface одним cutover:

1. Вынести чистое состояние/геометрию и покрыть переходы тестами.
2. Создать rail и workspace views поверх существующих services/components.
3. Перевести публичный API и IPC на новый lifecycle.
4. Удалить старый combined render/lifecycle и sessions-sidebar chrome.
5. Только после отсутствия старого пути запускать live acceptance.

Параллельные старое и новое окна в release-коде запрещены. Изменения
`Source/gpui*` не требуются и не допускаются в этой работе.

### 12.1. Project Switcher из bar

Slice A выполняет однонаправленную миграцию:

- удалить `project` из `BUILTIN_NAMES`, `BarLayoutConfig::default().right`,
  builtin registry и Bar Settings catalog;
- load-time migration удаляет `project` из `left/center/right` и `known`, затем
  один раз сохраняет очищенный `bar.toml`; пользователь не получает вечный
  unknown-widget warning и удалённый builtin не воскресает;
- обновить default/migration/sanitize/agent API tests;
- удалить `bar/widgets/project.rs` и регистрацию `ProjectWidget`;
- сохранить `ProjectsConfig`, config cache, branch lookup и add-project portal
  как domain/backend новой вкладки;
- удалить отдельный `ProjectPopupState` и popup window lifecycle после того,
  как их действия переведены в `tabs/project_switcher.rs`; старый popup не
  остаётся вторым UI владельцем.

### 12.2. ThreadStore

Slice A повышает schema version транзакционной migration. Threads получают
явный canonical project path (старые rows backfill'ятся из `cwd`), project
queries исключают глобальный список, а `workspace_project_state` хранит
последний active thread проекта. Migration идемпотентна и тестируется на
старой v1 fixture.

## 13. Проверка

### Автоматическая

- Геометрия visible content, input region и dragger для обоих clamp.
- Fixed surface bounds: ни один resize transition не выдаёт window size.
- Per-tab remembered width и fixed-width tabs без dragger.
- Active-tab toggle, different-tab switch и rail-only summon.
- Project switch атомарно восстанавливает session scope.
- Sessions selection переключает в Chat.
- Slice B: Plan/Context session isolation и Archive project/global filter.
- Slice C: Tools/Skills project enablement без session override.
- Atomic two-surface open/rollback и independent close cleanup.
- Regression: composer submit, ACP streaming, permissions, transcript,
  dock/peek/pin, `expand-left` и `compose-and-send` IPC.
- Bar migration удаляет project из default, custom layouts и `known`, а
  ProjectsConfig/backend остаётся доступен новой вкладке.
- Responsive regression: fixed 920px window с узким visible slice выбирает
  narrow layout.

### Live release acceptance

1. `Super+A`: closed → rail-only → closed.
2. Каждая rail-кнопка открывает правильную вкладку; повторный click
   схлопывает content.
3. Sessions → выбранная сессия → Chat.
4. Project switch атомарно меняет Sessions и Chat; после Slice B также Plan и
   Context Files.
5. Быстрый и медленный resize Chat/Plan/Context Files до обоих clamp и обратно
   без wobble, gap и потери drag capture.
6. Fixed tabs не показывают dragger и открываются в своей ширине.
7. Прозрачный canvas пропускает клики.
8. Dock резервирует ровно текущую полную ширину; overlay — ровно 40 px.
9. Peek/pin и переход rail ↔ content не оставляют ghost surfaces.
10. `hyprctl layers` показывает две logical surfaces с постоянными bounds плюс
    отдельный постоянный `side_panel_left_hover_strip`; других left-panel
    surfaces нет.
11. Release log не содержит panic, zero-size render или lifecycle errors.

Финальный UX-критерий принадлежит владельцу: `+` означает принято, `-` —
задача не завершена. Зелёные unit-тесты без live acceptance недостаточны.

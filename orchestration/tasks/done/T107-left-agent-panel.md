# T107 — Left Agent Panel

**Статус: ACTIVE. Task1-10+12 закоммичены на ветке, но НЕ приняты — два
живых блокера найдены при смок-тесте (см. `## БЛОКЕРЫ` ниже). Начинай
оттуда, не с пункта "Архитектура/План" (те описывают исходный замысел, код
местами от него отошёл законно — см. старый разбор ниже в
`## ИСТОРИЯ ВОЗОБНОВЛЕНИЯ`).**

## Цель

Построить левую agent-панель — layer-shell overlay с Hermes ACP чатом, sessions sidebar, composer и tool call cards.

## Архитектура

- Layer-shell overlay (как правая панель) + official ACP SDK (`agent-client-protocol` crate) + gpui-component для UI
- Один процесс `hermes acp` на ChronOS shell
- Multi-session через `session_id`

## Спека

`docs/superpowers/specs/2026-07-23-left-agent-panel-design.md`

## План

`docs/superpowers/plans/2026-07-23-left-agent-panel.md` (13 задач)

## Зоны файлов

- `crates/app/src/side_panel_left/` — UI модуль
- `crates/services/src/hermes_acp/` — ACP сервис
- `crates/services/src/hermes_acp.rs` — модуль declaration
- `crates/app/Cargo.toml` — зависимости
- `crates/services/Cargo.toml` — зависимости

## Верификация

- `cargo build --release -p chronos` — зелёный
- Release binary + live test (peek/pin/resize/chat)

---

## БЛОКЕРЫ (2026-07-23, найдены при живом смок-тесте Архитектором)

**Рабочее дерево:** `/home/neo/projects/chronos-ecosystem/ChronOS-wt-left-panel`,
ветка `feat/left-agent-panel`, HEAD = `4e12655`. Коммиты после task8:
`371abfe` (task9 tool cards), `c15d334` (task10 resize), `4e12655` (task12
wire ACP — включает легитимный фикс E0521: `HermesClient` стал `#[derive(Clone)]`
с `_transport: Arc<HermesTransport>`, `send_composer` клонирует клиент через
`self.client.clone()` вместо `.as_ref()`). Сборка зелёная (`cargo build
--release -p chronos`, `cargo test -p chronos --lib` 4/4). **Task11
отдельного коммита не получил — не баг**: disabled/opacity-логика композера
(`enabled = agent_status != Disconnected`, `.when(!enabled, opacity(0.5))`)
уже была написана внутри task7 (`3f5d607`), до появления task11 как
отдельного пункта. Засчитывать как покрыто.

**Отчётов по task9-13 никто не написал** — `orchestration/tasks/report/`
пуст для T107. Это тоже нужно закрыть (см. "Отчётность" в конце файла).

### Блокер 1 — hover-strip слева не реализован вообще

`init()` в `mod.rs:253-266` открывает панель ТОЛЬКО если стоит переменная
окружения `CHRONOS_SMOKE_SIDE_PANEL_LEFT`. Никакого модуля вида
`side_panel_right/hover_strip.rs` для левого края нет (`find
crates/app/src/side_panel_left -type f` — семь файлов, hover_strip среди
них не значится). Живой прогон без этой переменной подтвердил: `hyprctl
layers` не показывает `side_panel_left` вообще, панель не поднимается по
наведению на левый край экрана. Это часть исходного Task 1 ("Layer-Shell
Window + Peek/Pin") — приняли task1 раньше, не заметив отсутствия
hover-триггера (моя недоработка на приёмке, чиню сейчас через бриф).

**Что делать:** построить левый hover-strip по образцу
`crates/app/src/side_panel_right/hover_strip.rs` — 1:1 паттерн (namespace,
anchor, exclusive_zone, debounce на open/close), зеркально на `Anchor::LEFT`.
Без этого "hover left edge → peek opens" из плана (Task 13, шаг 4)
физически невозможно проверить.

### Блокер 2 — resize хватает чужой drag и улетает за пределы ожидаемого

Живой прогон (`CHRONOS_SMOKE_SIDE_PANEL_LEFT=1`, свежий процесс, БЕЗ
намеренного взаимодействия с resize-ручкой) дал `hyprctl clients`/`layers`:
окно `side_panel_left` **896×1050px** вместо дефолтных 352×h из
`window_options()` (`mod.rs:39-58`, `PANEL_WIDTH = 352.`). 896 укладывается
ровно в клэмп `min_width=280.0 / max_width=960.0` (`state.rs::resize()`) —
не случайное число, похоже на реальный resize-вызов, а не рендер-глюк.

Разбор кода (`panel.rs:241-334`): `onMouseDown`/`onDrag`(arm payload
`LeftPanelResize`) корректно висят только на узкой 4px `#resize-handle`
(строки 323-334) — это правильно. Но `onDragMove={resize_drag_handler}`
висит на **корневом** div всей панели (строка 262,
`<div w={px(panel.state.width)} h_full flex flex_row onDragMove=...>`).
В апстрим-GPUI это нормальный паттерн (`on_drag_move::<T>` типобезопасен
по payload и не должен срабатывать, пока `on_drag::<T>` его не заармил на
источнике) — но экран показал реальный resize без моего участия в ручке.
Гипотезы, ОБЕ требуют проверки, не гадать:

1. У этого форка (`../Source/gpui`) `on_drag_move::<T>` может быть не
   строго типобезопасен по payload и ловит **чужие** drag-жесты (например,
   перетаскивание любого другого окна по Hyprland где-то на столе) —
   смотри `fork-api-drift`, `gpui-layer-shell` скиллы, сверься с
   апстримной семантикой в `../Source/gpui` исходниках самого трейта
   `on_drag_move`.
2. `resize_start_x`/`resize_start_width` инициализированы `0.0` в
   `SidePanelLeft::new()` (`mod.rs`) — если `update_resize` всё же
   вызывается без предшествующего `start_resize()`, delta считается от
   нуля и слетает на абсолютную экранную X-координату курсора в момент
   любого стороннего drag-события.

**Что делать:** воспроизвести резайз намеренно (потянуть ручку) и
убедиться, что цифры сходятся; затем — вызвать `update_resize` БЕЗ
предварительного `start_resize` (или дёрнуть посторонний drag где-то ещё
на столе, если рабочее место позволяет) и проверить, ловит ли панель это
как resize. Если да — заводить строгую проверку "резайз идёт только если
`start_resize` реально вызывался в этом жесте" (например, `Option<f32>`
вместо `f32` для `resize_start_x`, `None` = "resize не арм-лен"), плюс
завести баг в `fork-api-drift`/апстрим, если payload-тайпинг `on_drag_move`
у форка действительно дырявый.

**Не считать T107 приёмной, пока оба блокера не закрыты и не подтверждены
живым прогоном (grim/hyprctl), а не только "cargo build зелёный".**

---

## ИСТОРИЯ ВОЗОБНОВЛЕНИЯ (2026-07-23, после обрыва миньона на task9 — для контекста, уже отработано)

**Рабочее дерево:** `/home/neo/projects/chronos-ecosystem/ChronOS-wt-left-panel`,
ветка `feat/left-agent-panel`, HEAD = `70b4d61` (task8 принят).

**Известное расхождение с исходным планом (законное, не баг):**
`skills/chronos-shell/SKILL.md:24` — "No `gpui_component` — raw
`gpui::div()` only". Композер (task7) уже написан на голом `div()` +
`on_key_down`, без gpui-component TextInput. Дальше делай так же — никакого
`gpui_component` в `crates/app`.

### 0. Триаж грязного дерева (СНАЧАЛА, до любого нового кода)

На момент обрыва в рабочем дереве **некоммичены** правки сразу в 6 файлах
(`chat_view.rs`, `composer.rs`, `mod.rs`, `panel.rs`, `state.rs`,
`tool_card.rs`), вперемешку — часть task9, часть task10, начало task12.
Сборка **красная**: `cargo build --release -p chronos` падает с
`error[E0521]: borrowed data escapes outside of method` в
`composer.rs:375`, метод `send_composer` — `cx.spawn(async move |this, cx| {...})`
эскейпит `&mut self` за пределы тела метода, хотя замыкание вроде бы
захватывает только `this`/`cx`/`client`/`text`. Похоже на особенность
форка (`cx.spawn` сигнатура) — см. скиллы `gpui`, `chronos-gpui`,
`fork-api-drift` прежде чем чинить руками. НЕ удаляй эту незакоммиченную
работу (`git stash`/`git checkout --`) — в ней уже готовый код task9/10.

Разбери по границам задач и закоммить раздельно:

1. **Task 9 (tool cards)** — судя по диффу `tool_card.rs` (+154 строк,
   `ToolCard<'a>` с `render()`, статусы running/done/error, expand/collapse)
   и интеграции в `chat_view.rs` (`expanded_tool_calls: HashSet<(usize,usize)>`,
   импорт `ToolCard`) — похоже, уже готово. Проверь, вычлени из общего диффа
   именно эти куски (могут тянуть за собой поля из `mod.rs`/`state.rs` —
   смотри, что реально нужно tool-card-функциональности), собери отдельно,
   `cargo build --release -p chronos` на этом промежуточном срезе должен
   быть зелёным без task10/12 довесков. Коммить как
   `feat(side_panel_left): collapsible tool call cards` (см. план, Task 9).

2. **Task 10 (drag-resize)** — в `state.rs` уже есть `min_width`/`max_width`/
   `resize()`, в `mod.rs` — `start_resize`/`update_resize`/`LeftPanelResize`
   маркер-тип, в `panel.rs` — `onDragMove`/`MouseDownEvent` хэндлеры и
   `w={px(panel.state.width)}` вместо хардкода 352. Похоже полностью готово.
   Собери отдельным коммитом поверх task9:
   `feat(side_panel_left): drag-resize handle`.

3. **Task 11 (error handling + offline state)** — план просит статус-цвета
   (уже есть, `AgentStatus`→`status_color()` с 2026-07-23 task5) и
   `disabled`-состояние композера при не-Connected статусе. Проверь, сделано
   ли уже (в `composer.rs` может быть частично, раз `AgentStatus::Thinking`/
   `Disconnected` уже используются в найденном task12-обрывке). Дошей
   недостающее: композер должен визуально гаситься (`opacity(0.5)` +
   реально не принимать ввод) когда `agent_status != Connected`. Коммить:
   `feat(side_panel_left): error handling and offline state`.

4. **Task 12 (Wire ACP Client to UI)** — уже начато и **сломано** (см. п.0).
   Доделать:
   - `mod.rs::new()` уже спавнит `HermesClient::new()` асинхронно и
     проставляет `AgentStatus` — оставить, это верно.
   - `composer.rs::send_composer` — почини E0521. Скорее всего фикс: не
     захватывать `self`/`cx` из внешнего скоупа напрямую в `cx.spawn`,
     использовать паттерн `cx.spawn(async move |this: WeakEntity<Self>, cx| ...)`
     как в остальном дереве (`grep -rn "cx.spawn(async move" crates/app/src`
     на **чистом** HEAD дать рабочие примеры — `launcher`, `system_popup`
     используют этот паттерн, сверься с ними, не изобретай заново).
   - После фикса: реальный прогон — отправить сообщение из композера,
     убедиться что `ChatView` получает user-message сразу и agent-response
     (или error, если `hermes` бинаря нет в PATH — это ожидаемо OK, главное
     что error-ветка тоже кладёт сообщение в чат, а не молчит).
   - Коммить: `feat(side_panel_left): wire ACP client to UI`.

5. **Task 13 (Build + Smoke Test)** — после всех коммитов:
   - `cargo build --release -p chronos` — зелёный, ноль ошибок.
   - `pkill -x chronos || true; RUST_LOG=info ./target/release/chronos`
     живой прогон: hover левого края → peek открывается; pin остаётся;
     ввод в composer → send работает (см. п.4); tool-call карточки
     сворачиваются/разворачиваются; resize-хэндл тянется.
   - Финальный коммит `feat(side_panel_left): complete left agent panel v1`
     (или пропустить, если Task9-12 уже покрыли все изменения — не
     коммитить пустое).

### Отчётность

Отчёт по каждой завершённой подзадаче — в
`orchestration/tasks/report/T107-left-agent-panel-task{N}-report.md`
(как task1-8, уже в `report-log/`). Пиши **честно** что реально сделано
vs что осталось — Архитектор (не ты) сверяет каждое утверждение с деревом
перед приёмкой, недостоверный отчёт не пройдёт.

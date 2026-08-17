# T279 — левая workspace: Chat, Sessions и Project tabs

**Статус:** ACCEPTED 2026-08-14 (round 2 + errata). Код: `bd999a5`.
**Приоритет:** P1.
**Роль:** GPUI product UI + existing ACP integration.
**Зависимость:** T278 принят 2026-08-14 (round 4, четыре раунда).
**База:** `19263d3`. Перед стартом доказать:
`git merge-base --is-ancestor 19263d3 HEAD`.
**Следующий тикет:** T280; параллельно не выполнять.

## Что реально отдал T278 (сверено с деревом, не по отчёту)

Строиться на этих именах, не изобретать параллельные:

- `side_panel_left/tabs/mod.rs` — `LeftTab`, `PRIMARY_TABS`, `BOTTOM_TAB`,
  `ResizableWidths`, `width_for_open`, `dock_transition`;
- `side_panel_left/state.rs` — чистая геометрия (`geometry::`);
- `side_panel_left/rail_view.rs` — `RailView`, рельса 40 px;
- `side_panel_left/workspace_view.rs` — `WorkspaceView`, фикс-канвас 920 px;
- `side_panel_left/mod.rs` — `SidePanelLeftState_` (SoT),
  `apply_dock_toggle(cx: &mut App)`.

Урок T278, обязательный к соблюдению здесь (см. `docs/ARCHITECT.md`,
раздел от 2026-08-14): **редьюсер состояния — свободная функция на
`&mut App`, вьюха делегирует.** Причина: `SidePanelLeft::new` спавнит
async ACP-connect, поэтому entity не поднимается в `TestAppContext`, и
любой редьюсер-метод вьюхи становится непокрываемым. Тест обязан звать
проверяемый путь по имени; тавтология под именем интеграционного теста
отклоняется без обсуждения. Ветки теста обязаны доказывать, что прод
читает состояние, а не совпал со статической константой.

## Канон

Выполнить только Tasks 3–4 из
`docs/superpowers/plans/2026-08-13-left-ai-workspace-slice-a.md`.
Не реализовывать доменную модель Slice B/C.

## Цель

Удалить временный monolithic product child T278 и разложить workspace на
переиспользуемые tab entities:

- полноценный Chat без window ownership;
- Sessions текущего проекта;
- полноценный embedded Project Switcher;
- честные shells Plan, Tools, Skills, Context Files, Archive.

## Chat

Перенести существующие ACP/Hermes, transcript, composer, streaming,
permissions, model/mode и tool-card состояния без protocol rewrite.
`ChatTab` не содержит `WindowHandle`, open/close, dock, panel width, project
selector или sessions list. Responsive layout получает visible width от
workspace, а не читает fixed `window.bounds() == 920`.

Сохранить текущие send/reconnect/transcript/follow-output semantics и
cancel-on-drop для GPUI task handles.

## Sessions и Project

- Sessions показывает только текущий canonical project path.
- Выбор session эмитит `SelectThread`, после чего coordinator открывает Chat.
- Project tab содержит поиск, recent projects, branch, add/remove, Files и
  Terminal actions.
- `ProjectsConfig` и helpers остаются единственным backend owner.
- `ProjectTab` не создаёт второй config/store.
- Project switch сначала очищает старые Chat/Sessions данные, затем меняет
  `ProjectsConfig.active`, после чего загружает новый scope.

Полный popup scope учесть целиком:

- изменить `crates/app/src/project_switcher/mod.rs`;
- удалить popup-only `crates/app/src/project_switcher/view.rs` и `mod view`;
- удалить `ProjectPopupState`, popup options/open/close/toggle;
- сохранить `project_switcher::init(cx)` и вызов в `main.rs`;
- `init` продолжает `reload_cache()` и logging, но больше не регистрирует
  popup global.

## Rail и shells

Порядок: Project, Sessions, Chat, Plan, Tools, Skills, Context Files;
`flex_1`; Archive; отдельный dock toggle.

Plan/Context shells используют resizable policy; Tools/Skills/Archive — fixed.
Каждый shell явно пишет `Coming in Slice B` или `Coming in Slice C`.
Tabs создаются лениво и переиспользуются. Все rail buttons имеют stable IDs,
tooltips и active indicator на правой кромке.

## TDD и проверки

Сначала добавить тесты на visible-width breakpoint, child ownership,
project/session transitions, active-tab collapse, dock-wins, shells и
сохранение `project_switcher::init`.

```bash
cargo test -p chronos side_panel_left::tabs::chat --lib --bins
cargo test -p chronos side_panel_left --lib --bins
cargo test -p chronos project_switcher --lib --bins
cargo check -p chronos --lib
rg -n 'WindowHandle|open_window|window\.resize\(' crates/app/src/side_panel_left/tabs/chat.rs
rg -n 'project_switcher::init\(cx\)' crates/app/src/main.rs
```

Ожидается: тесты/check зелёные; первый `rg` пустой; второй находит действующий
init call.

## Запрещено

- переписывать ACP/thread protocol;
- реализовывать Plan/Context/Archive/Tools/Skills beyond shells;
- оставлять popup вторым project-context owner;
- удалять `project_switcher::init` вместе с popup;
- хранить независимую project identity внутри каждой tab entity;
- начинать T280 до принятия отчёта T279.

## Отчёт

Создать
`docs/orchestration/tasks/report/T279-left-workspace-chat-sessions-project-tabs-report.md`.

Зафиксировать ownership до/после, удалённый popup scope, сохранённый init,
проверки responsive width, точные команды/exits и hash commit. Не переносить
отчёт в `report-log/`.


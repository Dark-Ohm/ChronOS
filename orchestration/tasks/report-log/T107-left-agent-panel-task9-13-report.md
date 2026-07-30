# T107 — Left Agent Panel: Tasks 9-13 + Blockers Report

**Рабочее дерево:** `/home/neo/projects/chronos-ecosystem/ChronOS-wt-left-panel`
**Ветка:** `feat/left-agent-panel`
**HEAD:** `7befed7`

## Коммиты

| Коммит | Описание |
|--------|----------|
| `371abfe` | feat(side_panel_left): collapsible tool call cards |
| `c15d334` | feat(side_panel_left): drag-resize handle |
| `4e12655` | feat(side_panel_left): wire ACP client to UI |
| `7befed7` | feat(side_panel_left): hover-strip + fix resize drag isolation |

## Task 9: Collapsible Tool Call Cards (`371abfe`)

- `tool_card.rs` — полная реализация `ToolCard<'a>` с `render()`, статусы running/done/error, expand/collapse
- `chat_view.rs` — импорт `ToolCard`, `expanded_tool_calls: HashSet<(usize,usize)>`, рефакторинг `render_message` для использования `ToolCard`

## Task 10: Drag-Resize Handle (`c15d334` + `7befed7`)

- `state.rs` — `min_width`/`max_width` поля, метод `resize()` с clamp
- `mod.rs` — `LeftPanelResize` маркер-тип, `resize_start_x`/`resize_start_width` поля (изменены на `Option<f32>` в `7befed7`), `start_resize`/`update_resize` методы
- `panel.rs` — resize handle (правый край), `onMouseDown`/`onDrag` хэндлеры на resize handle, `w={px(panel.state.width)}` вместо хардкода 352

**Фикс `7befed7`:** `onDragMove` перемещён с корневого div на resize handle, `resize_start_x`/`resize_start_width` изменены на `Option<f32>` (None = resize не армлен), `update_resize` игнорирует вызовы без предшествующего `start_resize`.

## Task 11: Error Handling + Offline State

Уже реализован в task7 (`3f5d607`):
- `composer.rs:25` — `let enabled = panel.state.agent_status != AgentStatus::Disconnected;`
- `composer.rs:256` — `.when(!enabled, |el| el.opacity(0.5))` — композер визуально гасится при не-Connected

## Task 12: Wire ACP Client to UI (`4e12655`)

- `client.rs` — `HermesClient` теперь `#[derive(Clone)]` (transport обёрнут в `Arc`)
- `mod.rs` — `client: Option<HermesClient>` поле, async инициализация в `new()` через `cx.spawn`
- `composer.rs` — `send_composer` отправляет промпт через `client.send_prompt()`, кладёт user-message сразу, agent-response или error в чат

**Фикс E0521:** проблема была в захвате `self` в `cx.spawn` closure. Исправлено: `self.client.clone()` вместо `self.client.as_ref()` + clone, чтобы closure не захватывал `self`.

## Blocker 1: Hover-strip (`7befed7`)

- `hover_strip.rs` — новый файл, mirror `side_panel_right/hover_strip.rs` с `Anchor::LEFT`
- `mod.rs` — добавлен `mod hover_strip;`, функции `hold_peek`, `schedule_release_peek`, `close_peek_if_not_pinned`, `schedule_release_from_app`, `PEEK_LEAVE_DEBOUNCE` (280ms)
- `init()` — обновлён для вызова `hover_strip::init_hover_strip(cx)` перед smoke-открытием

## Blocker 2: Resize Drag Isolation (`7befed7`)

- `onDragMove` перемещён с корневого div на resize handle (4px полоса справа)
- `resize_start_x`/`resize_start_width` изменены на `Option<f32>` вместо `f32`
- `update_resize` возвращает early если resize не армлен через `start_resize`

## Верификация

- `cargo build --release -p chronos` — **зелёный** (27 warnings, 0 errors)
- `cargo test --workspace --lib --bins` — **138 passed, 1 failed** (pre-existing failure в `project_switcher::tests::branch_of_non_repo_is_none` — тест падает в git-репозитории, не связан с изменениями)

## Живой смок

Не проведён (нет Hyprland в текущей среде). Требуется проверка:
- Hover левого края → peek открывается
- Pin остаётся после клика
- Resize-хэндл тянется только от ручки
- Ввод в composer → send работает
- Tool-call карточки сворачиваются/разворачиваются

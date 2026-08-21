# DRAFT — повторный select активного проекта очищает Chat/session

**Предлагаемая роль:** frontend. **Приоритет:** P1. **Источник:** T326 live QA.

## Наблюдаемое поведение

В Project уже активен и подсвечен `ChronOS`. После успешного Chat-turn повторный клик по этой же строке визуально оставляет тот же проект активным, но лог пишет:

```text
side_panel_left: project switched (session cleared)
now_project=Some("/home/neo/projects/chronos-ecosystem/ChronOS")
now_session=None
```

Текущая session association и видимый chat очищаются, хотя scope фактически не менялся.

## Воспроизведение

1. Запустить release ChronOS и раскрыть левый Chat.
2. Создать/использовать session и дождаться видимого transcript.
3. Открыть Project.
4. Кликнуть по уже подсвеченной активной строке проекта.
5. Вернуться в Chat/Sessions и проверить состояние и info-log.

Фактический live event T326: `.chronos-ops/dump/qa-ux/T326/log/chronos.log`, 2026-08-21T09:53:12.686799Z; кадр после клика: `.chronos-ops/dump/qa-ux/T326/frames/14-project-switch-aur.png` (имя кадра отражает намерение шага, фактический hit был по активному ChronOS).

## Ожидание

Повторный клик по уже активному project row — no-op: не отправляет project switch, не сбрасывает active session и не очищает transcript. Выбор другого проекта продолжает выполнять существующий clear/isolation transaction.

## Корреляция с кодом

- `crates/app/src/side_panel_left/tabs/project.rs:125-150`: `is_active` влияет только на фон; `.on_click` безусловно emits `ProjectEvent::Select` и вызывает `set_active`.
- `crates/app/src/side_panel_left/workspace_view.rs:169-184`: любой `ProjectEvent::Select` вызывает `switch_project` и сбрасывает выбор Sessions.
- `crates/app/src/side_panel_left/mod.rs:779-800`: `switch_project` безусловно ставит `active_session_id = None`, затем `clear_for_project` / `restore_project_thread`.

## Предлагаемая приёмка

- Повторный клик по активному project row не меняет active project/session/chat и не пишет `project switched (session cleared)`.
- Выбор другого project row по-прежнему очищает старый scope до persist нового active project.
- Есть regression test на active-row no-op и live release smoke с существующим transcript.

Код в рамках T326 не менялся.

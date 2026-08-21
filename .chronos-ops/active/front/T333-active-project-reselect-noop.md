---
ticket: T333
role: front
status: active
tags: [chronos-ops, front, active]
---

# T333 — повторный клик по активному проекту не сбрасывает session

**Роль:** FRONTEND. **P1.** Живая находка T326 B2.
**Зона:** `crates/app/src/side_panel_left/tabs/project.rs` (`on_click`),
`crates/app/src/side_panel_left/mod.rs` (`switch_project` + тест рядом
с `switch_project_sets_path_and_clears_session`), при необходимости
`workspace_view.rs` `on_project_event`.
**Не трогать:** calendar/volume (T329/T332), network (T331), ACP
`session/load` (T285 STOP), шелл-заглушки Slice B/C.

Параллелен T329/T331: другие файлы.

## Зачем

После живого turn повторный клик по уже подсвеченному `ChronOS` пишет
`project switched (session cleared)` при том же path и `now_session=None`.
Чат пустеет, хотя проект не менялся. T326:
`dump/qa-ux/T326/frames/14-project-switch-aur.png` (hit = активный
ChronOS, не AUR), лог `2026-08-21T09:53:12.686799Z`.
Источник: `done/qa/DRAFT-T333-active-project-reselect-clears-session.md`.

Смена **другого** проекта должна по-прежнему чистить scope (T279).

## Корень (сверено)

- `project.rs:126` `is_active` красит фон; `on_click` 140–150 всегда
  `emit(ProjectEvent::Select)` + `set_active`. Нет early-return.
- `workspace_view.rs:178-184` любой Select → `switch_project`.
- `mod.rs:779-800` `switch_project` безусловно `active_session_id = None`
  + `clear_for_project`. Нет сравнения с текущим path.
- Тест `switch_project_sets_path_and_clears_session` (~1724) покрывает
  только **другой** path.

## Что сделать

1. No-op, если `new_project_path` уже `active_project_path` (канон —
   в `switch_project`, чтобы IPC/другие callers не обошли клик).
2. На клике активной строки — не emit и не `set_active` (не переписывать
   toml).
3. Реальный switch на другой path — без регрессии T279.
4. Юнит: same-path сохраняет `active_session_id`; other-path по-прежнему
   чистит (существующий тест не ломать).

## Готово когда

- `cargo test -p chronos --lib` зелёный, включая новый same-path тест.
- Живой: transcript на экране → клик по активному project row → чат и
  session id на месте, в логе нет `project switched (session cleared)`.
- Клик по другому проекту → isolation как сейчас (Sessions пустой у
  чужого scope). grim в отчёт, не `/tmp`.

**Отчёт:** `.chronos-ops/reports-fresh/T333-active-project-reselect-noop-report.md`

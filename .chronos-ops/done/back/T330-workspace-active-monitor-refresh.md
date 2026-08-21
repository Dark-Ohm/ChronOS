---
ticket: T330
role: back
status: done
tags: [chronos-ops, back, done]
---

# T330 — подсветка воркспейса слушает focusedmon

**Роль:** BACKEND. **P1.** Живая находка T324 B2.
**Зона:** `crates/services/src/compositor/hyprland.rs` (listener).
**Не трогать:** `bar/widgets/workspaces.rs` (кроме если без этого не собрать — не надо), UI, IPC.

## Зачем

Клик по точке **работает** (hyprctl 3/3). Индикатор врёт: после клика
активен ws 2 на DP-1, синяя точка остаётся на ws 11. Покупатель видит
«мёртвую кнопку». Кадры: `dump/qa-ux/T324/crops/28-dots-stale.png`.

## Корень (сверено)

`run_listener` подписан на `workspace_changed` / added / deleted /
`active_window_changed` / `layout_changed`. Обработчика
`ActiveMonitorChanged` нет.

Когда `FocusWorkspace(id)` целится в воркспейс, уже активный на другом
мониторе, Hyprland меняет фокус монитора и шлёт `focusedmon`, не
`workspace` → `refresh_workspaces` не зовётся.

Событие в крейте `hyprland` 0.4.0-beta.3 есть:
`event_listener/mod.rs` `ActiveMonitorChanged` → макрос `events!`
даёт `add_active_monitor_changed_handler` (тот же паттерн, что
`add_workspace_changed_handler`). Имена хендлеров грепом `pub fn add_`
не ищутся — это кровный факт HANDOFF.

Заодно нет `WorkspaceMoved` / `WorkspaceRenamed` — в этот тикет не
тащить, одной строкой в отчёте отметить.

## Что сделать

В `run_listener` добавить handler `ActiveMonitorChanged`, который зовёт
`refresh_workspaces` (как `workspace_changed`). Не глотать ошибку
`let _ =`.

Юнит, если можно без сокета: хотя бы что handler регистрируется / что
refresh выставляет `active` по актуальному монитору. Живой смок обязателен.

## Готово когда

На этой машине (DP-1 + HDMI-A-1): клик по точке чужого монитора →
`hyprctl activeworkspace` и синяя точка бара показывают **один** id.
grim до/после. `cargo test -p chronos-services` зелёный.

**Отчёт:** `.chronos-ops/reports-fresh/T330-workspace-active-monitor-refresh-report.md`

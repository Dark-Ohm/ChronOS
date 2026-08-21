---
ticket: T281
role: hold
status: hold
tags: [chronos-ops, hold]
---

# T281 — левая workspace: IPC, focus, dock и live-приёмка Slice A

**Статус:** PARK — 2026-08-15. Не выдавать, не писать код, не архивировать.
**Приоритет:** P1 — финальный integration/live gate Slice A, **после T285**.
**Роль:** integration + Hyprland live verification (владелец, не миньон).
**Зависимость:** T280+T283 в git (`f083779`). Гейт 8 живьём → **T285**.

## Почему park

Inbox уже есть:
`docs/orchestration/tasks/report/T281-left-ai-workspace-slice-a-report.md`.

Гейты 1–7 / 9–10 в отчёте зелёные или с оговоркой модели. Гейт 8 **код-путь**
`23bf89f` сеет store; живьём Hermes после рестарта делает `create_session`.
Лента SQLite на месте, сессия агента — новая. Это T285 (`load_session`), не
второй заход Tasks 7–8 по той же панели.

Пока T285 не в git и владелец не прогнал рестарт — T281 не закрыт и не
переоткрыт. Повторная выдача = второй миньон в `side_panel_left`.

## Когда снимать park

1. T285 в git, live: лог `load_session`, нет `create_session` на restore,
   лента не дублируется, агент помнит ход.
2. Владелец гоняет остаток live-таблицы ниже и ставит `+` или `-`.
3. Только после `+` — архив отчёта в `report-log/`, бриф в `done/`.

Не обновлять `ARCHITECTURE.md` так, будто Slice A доказан, до этого `+`.

## Канон (когда снимем park — verify-only)

Выполнить только Tasks 7–8 из
`docs/superpowers/plans/2026-08-13-left-ai-workspace-slice-a.md`.
Это не разрешение начинать Slice B/C. Сигнатуры IPC не менять:

```rust
pub fn toggle(cx: &mut App);
pub fn expand_with_composer(cx: &mut App);
pub fn compose_and_send(text: String, cx: &mut App);
```

## Live Hyprland gate (остаток владельца)

Release через project scripts. `hyprctl layers`:

- closed: ноль left workspace/hover surfaces;
- любое open, включая visual rail-only: ровно rail 40 px + content canvas
  920 px;
- canvas bounds 920 px неизменны на всём drag;
- content exclusive `-1`; exclusive owner только rail;
- overlay zone 40, dock zone — текущая полная width.

Руками после T285: Super+A; drag 960→40→960; dock/undock; project switch
не показывает чужую session; **restart поднимает ту же ACP-сессию**;
`expand-left` / `compose-and-send` — focus и один submit.

Не запускать `wf-recorder`: evidence — `hyprctl layers`, grim и logs.

## Запрещено, пока PARK

- Отдавать миньону. Писать код «на всякий». Параллелить с T285/T286/T287-C.
- Переносить inbox-отчёт в `report-log/` до `+` владельца.

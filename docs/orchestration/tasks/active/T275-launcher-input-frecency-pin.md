# T275 — остаток live: пустой query и pin

**Статус:** OPEN remainder. Код A–D принят 2026-08-15 — **не переписывать**.
**Приоритет:** P1 — хвост приёмки, не новая волна лаунчера.
**Родитель:** эпик `T265-launcher-full-functionality.md`.
**Роль:** FRONTEND (`crates/app/src/launcher/**`), только если empty-query
требует правки bind/search. Pin после `180fe88` может закрыться прогоном.
**Код:** `89dfd25` (Input + frecency + pin + футер). Якорь pin `180fe88`
(screen-space vs Overlay catcher).
**Отчёт inbox:** `docs/orchestration/tasks/report/T275-launcher-input-frecency-pin-report.md`.

## Уже закрыто

| Часть | Факт |
|---|---|
| A каретка | `Input` + `InputState`, живьём PASS (владелец) |
| B футер | tune снят; бейдж luau не кнопка |
| C юниты | `chronos-services` frecency 5/5, half-life 7d, flush на close |
| D код | `pin_menu.rs` + `PopupMenu`; якорь поправлен |

T263 и T265-0 в `done/`. Цепочки «ждать их коммит» больше нет.
Меню «Пуск» — **T265-H**, не этот тикет.

## Нельзя

- Сажать `Input` заново, писать свою каретку, тащить `text_input.rs`.
- Начинать T265-A / T265-B / T265-H / сетку / VirtualList.
- Трогать `side_panel_left` (это T285/T286).
- «Улучшать» формулу frecency, пока empty-query не рисует список.

## Открыто (только это)

1. **Пустой query → «No matches».** Тест `empty_pattern_returns_all` зелёный.
   Живой лаунчер сразу после open — пустая выдача. Найти, почему пустой
   `Input` не попадает в frecency-сорт / полный список. Починить bind,
   не ранжирование.
2. **Pin на новом release после `180fe88`.** Правый клик → меню на экране
   (не мимо catcher) → Pin пишет `dock.toml`, иконка в доке, повтор = Unpin.
   Если уже так — кода нет, в отчёте `pin PASS`.

## Верификация

Release, `chronos-stop && chronos-start` (не второй бинарь поверх).

- SUPER+R / `chronos-ipc toggle-launcher`: пустой запрос — приложения, не
  «No matches». Три запуска → сверху по свежести.
- Pin: grim меню + док; Unpin честный.
- Каретка и отсутствие tune не регрессируют.

## Коммит

Только если есть дифф:
`fix(launcher): empty query lists apps (T275)`

Pin-only прогон без кода — дописать inbox-отчёт, коммит не делать.
Не двигать в `done/` без живого pin + непустого empty-query.

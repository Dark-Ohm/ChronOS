# T295 — календарь-попап по клику на часы

**Статус:** SPEC. Не выдавать до чекпоинта.
**Приоритет:** P2.
**Роль:** FRONTEND. Бар + новый попап. Кит, не самописная сетка.
**Не параллелить** с T290 (бар `system` click) — разные виджеты, но оба
правый кластер бара.

## Сейчас

`bar/widgets/clock.rs` — только текст `HH:MM · D мес` (`MONTHS_RU`).
Клика нет, попапа нет.

В ките (`../Source/gpui-component/crates/ui/src/time/`):

- `Calendar` + `CalendarState` (`calendar.rs`)
- `Date`, `CalendarEvent::Selected`
- месяц/год, today, prev/next
- `DatePicker` — это input, **не** для бара

Фича кита `time` (тянет chrono). В ChronOS `gpui-component` с
`default-features = false`, включён только `markdown`. Без
`gpui-component/time` календарь не слинкуется.

## Задача

Клик по часам → AnchoredPopup с **kit `Calendar`**. Повторный клик /
✕ / клик-away — закрыть.

1. Фича `time` на `gpui-component` в `crates/app` (или workspace), **не**
   включать `chart`/`lsp`.
2. Модуль `crates/app/src/calendar_popup/` — скелет как
   `updates_popup` / `system_popup` (`chronos-gpui-popup`):
   `AnchoredPopup` + fallback LayerShell, `close_this` без реентера
   `handle.update`, `Root` + `OnDemand` (виджеты кита без Root паникуют).
3. Часы: `canvas` + `Rc<Cell<Bounds>>`, `on_mouse_down(Left)` →
   `calendar_popup::toggle` (не `on_click` — grab).
4. Вьюха: `Calendar::new(state)` на `Entity<CalendarState>`. При open —
   `set_date(today)` / текущий месяц. Один месяц (`number_of_months = 1`).
   Свой grid / `div` по дням — отказ.

Высота: замерить живьём, slack ~30 px (ловушка footer clip). Ширина —
карточка кита + паддинг, не 420 как updates.

Выбор дня: подсветка. Бэкенда событий нет — не притворяться планировщиком
(T246). Range не нужен.

Язык шапки кита (`Calendar.month.*`) — как в крейте; не переписывать
месяцы ради `MONTHS_RU` на часах.

## Нельзя

- Самописный календарь «как в ките».
- `DatePicker` на баре.
- Правая вкладка Calendar.
- `Source/gpui/` патчи, `Cargo.lock` без нужды (фича `time` — да).
- Тосты, Updates, Notifications.

## Верификация

```
cargo test -p chronos --lib calendar_popup
cargo test -p chronos --lib clock
cargo build --release -p chronos
```

Live: клик по часам → сетка текущего месяца, today виден, стрелки листают
месяц. Второй клик закрывает. Якорь под часами, не обрезан. Grim.

## Коммит

`feat(bar): clock opens kit Calendar popup (T295)`

# T289 — правая панель: dock не открывает вкладку и не запирает её

**Статус:** DONE (2026-08-15). Код `17afee6`. Live: владелец `+`.
**Приоритет:** P1 UX — вкладку нельзя закрыть.
**Роль:** FRONTEND. `crates/app/src/side_panel_right/`
(`view.rs::toggle_dock`, `view.rs::on_tab_select`).
**Отчёт:** `docs/orchestration/tasks/report/T289-right-dock-must-not-lock-tabs-report.md`.
**Канон ломает T221 branch 1** — сознательно. T221: «docked + same tab =
no-op». Владелец 2026-08-15: так жить нельзя.

## Симптом (владелец)

Кнопка dock (⊞/⊟) на правой рельсе **сама открывает вкладку**. После
этого вкладку **не закрыть**: повторный клик по иконке — пустышка.

## Почему (file:line)

1. `toggle_dock` (`view.rs:278–288`) флипает `dock_content` и **всегда**
   зовёт `ensure_content_width(target)`. С rail-only (`width == 40`)
   dock ON насильно раскрывает контент активной вкладки.
2. `on_tab_select` branch 1 (`view.rs:425–431`): `dock_content && same tab`
   → `return`. Закрыть нельзя, пока dock ON.
3. Снять dock снова зовёт `ensure_content_width` — контент остаётся
   открытым. Закрытие только в два шага (undock → ещё раз клик по
   вкладке), и про второй шаг никто не знает.

Слева (T281) dock-wins-collapse — другая панель, **не** копировать
сюда слепо. Справа рельса = единственный аффорданс вкладки.

## Новый контракт

Dock = **exclusive zone** (клиенты не заезжают), не «контент всегда
открыт».

| Действие | Было (T221) | Стало |
|---|---|---|
| Dock ON, рельса-only | раскрыть вкладку + exclusive full | exclusive = 40, вкладку **не** открывать |
| Dock ON, контент открыт | exclusive = width | как сейчас |
| Dock OFF | `ensure_content_width` снова | только снять exclusive; ширину не трогать |
| Клик активной вкладки, dock ON | no-op | **схлопнуть** в rail-only; dock остаётся ON (exclusive 40) |
| Клик активной вкладки, dock OFF | toggle как сейчас | без изменений |
| Клик другой вкладки, dock ON | switch, width pinned | без изменений |
| Клик другой вкладки, collapsed | open | без изменений |

`toggle_dock` **не** вызывает `ensure_content_width`. Только
`dock_content = !dock_content` + `last_exclusive_zone = None` +
`refresh_windows`. Exclusive и так из `exclusive_px()`:
dock ON + collapsed → 40; dock ON + open → `width`.

## Тесты (редьюсер, не «хелпер == хелпер»)

Чистые функции или вызов `on_tab_select` / `toggle_dock` на entity,
что уже поднимается в `TestAppContext` (правая панель — да, в отличие
от ChatTab).

- collapsed + dock toggle → `dock_content == true`, `width` остаётся
  rail-only.
- open + dock toggle ON → `dock_content`, width не прыгает.
- dock ON + same-tab → `width == RAIL_ONLY_WIDTH`, `dock_content` всё
  ещё true.
- dock ON + same-tab повторно → открыть remembered width, dock true.
- Переписать / удалить тест, который фиксирует T221 «same tab docked =
  no-op». В отчёте назвать старый тест.

## Нельзя

- Левая панель, T281/T285/T288.
- `Source/gpui/`, `Cargo.lock`.
- Менять peek/pin hover-strip.
- «Починить» тем, что dock OFF схлопывает — тогда пропадает exclusive
  и клиенты наезжают. Схлоп при живом dock — норма.

## Верификация

```
cargo test -p chronos --lib side_panel_right
cargo build --release -p chronos
```

Live: рельса без вкладки → dock → клиенты отступили на 40, контента нет.
Открыть System → dock → вкладка жива, клиенты отступили на ширину.
Клик System → контент закрыт, dock ON, рельса на месте.
Повторный клик System → вкладка снова. Grim rail-only docked + open.

## Коммит

`fix(right-panel): dock pins exclusive zone, does not lock tabs (T289)`

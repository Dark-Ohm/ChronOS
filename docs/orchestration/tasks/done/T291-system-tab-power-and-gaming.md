# T291 — System справа: power profile и Gaming Mode с бара

**Статус:** DONE (2026-08-15). Код `84f25bf`, live-отчёт `0799179`. Тумблер Gaming — эррата T291-E.
**Приоритет:** P2 IA.
**Роль:** FRONTEND.
**После этого:** T290 заберёт яркость налево. Этот тикет яркость не трогает.
**Не параллелить** с T290. T289 больше не блокер.
**Отчёт:** `docs/orchestration/tasks/report/T291-system-tab-power-and-gaming-report.md`.

Два разных «gaming» — не смешивать:

| Имя в спеках | Код | Что делает |
|---|---|---|
| **Perf Gaming** (этот тикет) | `system_popup::gaming_mode` / `GamingModeState` | тумблер производительности в System |
| **Shell Gamer** (T292) | `WorkspaceMode::Gamer` | состав рельсы/сцен/дока, кнопка на правой рельсе |

## Откуда

Бар → `system` виджет → `system_popup/view.rs`:

- `brightness_block` — **остаётся** в попапе (T290).
- `power_profile_block` (`view.rs:444`) — **сюда**.
- `gaming_mode_block` (`view.rs:524`) — **сюда**.

Правый `SystemTab` (`tab/system.rs`) сейчас: header, MPRIS, **waytrogen
карточка**, спектры CPU/RAM/GPU/net, диски. Waytrogen уезжает в T290,
здесь не трогать (иначе два переезда одной карточки).

## Задача

1. Вынести `power_profile_block` и `gaming_mode_block` (+ `toggle_switch`,
   если только они его едят) в общий модуль, например
   `crates/app/src/power_controls.rs`. Не копипастить в System.
   Поведение 1:1: те же сервисы (`AppState::upower`, `GamingModeState`),
   тот же arm/клик.
2. Вставить оба блока в `SystemTab::render` **над** спектрами (после
   MPRIS / wallpaper, до CPU). Карточки в стиле System (`surfaces::card`),
   не сырой попап 280px.
3. Убрать эти два блока из `SystemPopupView`. Попап = шапка + яркость.
   Пустым не оставлять.
4. Контролы без бэкенда — disabled + причина (T246), не мёртвые кнопки.

Кит: `Button` / toggle из `gpui-component`, если блок переписывается.
Если перенос 1:1 существующих `div` — ок, не раздувать.

## Нельзя

- Яркость, `wallpaper_card`, left rail, `Source/gpui/`, `Cargo.lock`.
- Новый UPower/Hyprland путь «получше».
- Удалять `system_popup` целиком (T290).

## Верификация

```
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib system_popup
```

Live grim: System справа — profile + Gaming. Бар-попап — только яркость.
Переключение профиля и Gaming работает как с бара. Кадр обеих поверхностей.

## Коммит

`feat(system): power profile and gaming mode live on System tab (T291)`

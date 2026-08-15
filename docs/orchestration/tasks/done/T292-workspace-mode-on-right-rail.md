# T292 — Developer/Gamer с бара на правую рельсу

**Статус:** DONE (2026-08-16). Live `+` владельца. Эррата: `migration_idempotent` в `--bins`.
**Приоритет:** P2 IA.
**Роль:** FRONTEND.
**Не путать с T291.** Два разных gaming:

| Имя в спеках | Код | Что делает |
|---|---|---|
| **Shell Gamer** (этот тикет) | `WorkspaceMode::Gamer` | состав рельсы/сцен/дока |
| **Perf Gaming** (T291) | `GamingModeState` | тумблер производительности на System |

Здесь только Shell: `WorkspaceMode` Developer | Gamer.

**Не параллелить** с T289 (`side_panel_right/rail.rs`).

## Сейчас (было)

Бар-виджет `workspace_mode` (`bar/widgets/workspace_mode.rs`): пилюля
иконка+лейбл, клик → `workspace_mode::toggle`. При `pending` — баннер
«Перейти в …? Да / Нет / Не спрашивать» **в баре**.

Иконки временные (коммент T159): Developer = `rail-editor.svg`, Gamer =
`bolt.svg`. Настоящих `gamepad.svg` / everyday в дереве нет.

API не менять: `workspace_mode::{current,set,toggle,accept_prompt,
dismiss_prompt}`. Автопереключения нет (§1 спеки).

## Куда

Кнопка на **правой рельсе**, не `PanelTab`. Не в `top_tabs`/`bottom_tabs`
и не в `panels_config` — режим не вкладка, его не переставляют в edit
mode.

Место: **над dock-toggle**, после нижней группы вкладок
(`rail.rs:231`). Размер как у rail button (`BUTTON_SIZE`).

| Режим | Иконка | Смысл |
|---|---|---|
| Gamer | новый `icons/gamepad.svg` | геймпад |
| Developer | новый `icons/mode-daily.svg` | повседневное: кружка/кофе, не `</>` и не bolt |

SVG: `currentColor`, viewBox 24, как остальные rail-иконки. Лейбл
«Developer»/«Gamer» — tooltip, не текст на рельсе (40 px).

Клик = `workspace_mode::toggle(cx)` + `refresh_windows` (рельса уже
слушает mode через `resolve_grouped`).

Активное состояние кнопки: не «как вкладка System», а лёгкий акцент
фона/цвета иконки. Не путать с active tab.

## Prompt

Баннер Да/Нет/Не спрашивать **уезжает с бара** вместе с виджетом.
Варианты (взять первый, который влезает без второй поверхности):

1. Короткий popover у кнопки рельсы (как pin-меню: `PopupMenu` / маленькое
   anchored окно), **или**
2. 2–3 строки под кнопкой внутри рельсы, если не ломают 40 px / dock.

Авто-accept запрещён. Контракт `should_prompt` / Never — без изменений.

## Бар

Виджет с бара **снять**:

- вычеркнуть из `instantiate` / `BUILTIN_NAMES` / default layout;
- `sanitize` дропает `workspace_mode` из существующего `bar.toml` (как
  unknown), не оставлять мёртвый слот;
- файл `bar/widgets/workspace_mode.rs` удалить, если греп пуст.

IPC `set-workspace-mode:…` жив — это не бар.

## Нельзя

- Сращивать с T291 Gaming Mode.
- Делать третью вкладку «Mode».
- Автопереключение по игре/проекту.
- `Source/gpui/`, `Cargo.lock`.
- Оставлять пилюлю на баре «на всякий».

## Тесты

- Клик-хелпер: Developer → Gamer → Developer, `set` пишет `workspace.toml`.
- Inventory бара больше не содержит `workspace_mode`.
- Рельса: кнопка не в списке `PanelTab`.

## Верификация

```
cargo test -p chronos --lib workspace_mode
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib bar
```

Live: на баре пилюли нет. На правой рельсе над dock — иконка текущего
режима. Клик меняет рельсу/сцену как раньше. Prompt (если есть детектор)
не на баре. Grim Developer + Gamer.

## Коммит

`feat(right-rail): workspace mode toggle moves off the bar (T292)`

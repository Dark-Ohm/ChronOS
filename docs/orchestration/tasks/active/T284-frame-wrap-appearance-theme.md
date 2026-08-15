# T284 — тема оформления Frame: Hide и Wrap

**Приоритет:** P2 — визуальная айдентика, не блокер IPC.
**Роль:** FRONTEND (`crates/app/src/frame.rs`, бар, обе рельсы, Appearance).
**Канон:** `docs/superpowers/specs/2026-08-15-desktop-frame-wrap-theme-design.md`
**План:** `docs/superpowers/plans/2026-08-15-t284-frame-wrap-theme.md`
**База:** T268 в git (`d572657`) — нижняя полоска. **Этот бриф её не заменяет.**

## Это не T268

T268 закрыл стык «полоска между рельсами» без exclusive zone. Бриф T268
в `active/` — исторический, **не исполнять**. Обёртка — отдельная тема
оформления. Дефолтный шелл не переписывать.

## Задача

Во вкладке Appearance правой панели — сегмент **Frame**:

| Режим | Рельсы видны | Рельс нет |
|---|---|---|
| **Hide** (дефолт) | полоска T268 рельса–рельса | полоски нет |
| **Wrap** | рамка по периметру, рельса *внутри* карточки | та же рамка |

Клиенты сидят **внутри** рамки (скругления видны, окна не наезжают на хром).
Выключил Wrap — геометрия как до включения.

Референс вида — внешний кадр владельца (не снимок ChronOS): тёмная рамка,
внутренний скруглённый прямоугольник, бар = верх рамки.

## Обязательные решения

- Тема, не изменение шелла. Свежая установка без правки `frame.toml` =
  пиксельно T268.
- Не пресет бара (`Top full` / `Bottom pill` не трогать).
- Не писать Hyprland (`gaps_out`, rounding, lua-модули).
- Цвет: `theme.bg.tertiary` + `border.subtle`. Четвёртый оттенок — отказ.
- Полноэкранная рамка без exclusive (иначе композитор зарезервирует экран).
  Отступ окон — три невидимые exclusive-полоски L/R/B толщиной `height`.
- Рамка: слой **Top**, бар/рельсы остаются Overlay. Клик навылет
  (`set_input_region(Some(&[]))`).
- Дырку не делать «заливка + прозрачный ребёнок» — не пробивается. Красить
  только хром. Угол grim: в укусе обои.
- `.mx()` на `.size_full()` — известный баг T268, не повторять.
- Hover-strip остаётся на физической кромке. Рельса — нет.
- `wrap_inset()` = 0 в Hide, `bottom_strip.height` в Wrap. Панели сами
  ставят margin. Frame не читает `SidePanel*State` (цикл модулей).
- Presence рельс: панели зовут `frame::set_rail_mapped(side, bool)` на
  open/close рельсы, не hover-strip.

## Конфиг

`~/.config/chronos/frame.toml` (watcher уже есть, 300 ms):

```toml
style = "hide"   # hide | wrap; мусор → hide + warn

[bottom_strip]
enabled = true
height = 4.0     # толщина хрома Wrap и высота Hide-полоски, clamp 1..=16
junction = "break"  # только Hide

[wrap]
inner_radius = 16.0  # clamp 0..=64; в UI крутилку не обязательна
```

## Зона файлов

- `crates/app/src/frame.rs` (+ опционально `frame/wrap.rs`)
- `crates/app/src/side_panel_left/mod.rs` — только margin рельсы/контента +
  `set_rail_mapped`. **Не** IPC, tabs, `workspace_transition` (T281).
- `crates/app/src/side_panel_right/mod.rs` — то же. Не аудит поверхностей
  (T277).
- `crates/app/src/bar/mod.rs` — снять границу в карточку только при Wrap.
- `crates/app/src/side_panel_right/tab/bar_settings.rs` — сегмент Frame,
  пишет `frame.toml`, не `bar.toml`.

Нельзя: `Source/gpui/`, `Cargo.lock`, `layout_config.rs` виджетов, пресеты
бара, Hyprland-конфиг пользователя.

## Верификация

```
cargo test -p chronos --lib frame::
cargo test -p chronos --lib side_panel_left
cargo test -p chronos --lib side_panel_right
cargo build --release -p chronos
```

Юнит без пикселей — мало. Live release, пульт:

- Hide + рельсы = T268, exclusive рамки 0, клиенты не сдвинуты.
- Hide + рельсы закрыты = нет нижней полоски.
- Wrap = клиенты отступили на height L/R/B; grim 4 угла, обе темы.
- Wrap + рельса = рельса внутри карточки.
- Клик в дырке/по хрому не съедается рамкой.
- Тумблер обратно: слои `frame_wrap_*` исчезли, клиенты вернулись.

Отчёт: `docs/orchestration/tasks/report/T284-frame-wrap-appearance-theme-report.md`.
Не двигать задание в `done/`, не класть отчёт в `report-log/`.

## Стиль коммита

По задаче плана, поимённый `git add`. Без AI-трейлеров.
`feat(frame): …` / `feat(appearance): …` / `feat(panels): …`.

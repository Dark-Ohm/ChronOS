# T318 — Rail-as-Frame-Edge Implementation Report (Round 3)

## Status: DONE (awaiting architect acceptance)

## What was done

Implemented the T315 artboard visual requirements into the ChronOS shell code.

### Round 1: Accepted (cosmetic items)
- `DEFAULT_INNER_RADIUS`: 16→10, `DEFAULT_BOTTOM_THICKNESS`: 6→12
- Pill indicator (`accent.primary` α 0.15/0.12), 3px strip removed, border seam removed
- 605/605 tests pass

### Round 2: Structural fix (aperture corners)
**Root cause:** `rounded_*` was on the root div (no `.bg()`), not on `render_rail` (has `.bg()`). `overflow_hidden` on a non-painted parent doesn't create visible rounding.

**Fix:** Moved `rounded_tr/br(px(inner_radius))` + `overflow_hidden()` from root div to `render_rail` div (the element that carries `.bg(chrome)`), both rails.

**Files changed (6 files, +89/-74):**
- `frame.rs` — defaults (inner_radius 10, bottom_thickness 12)
- `side_panel_left/rail_view.rs` — pill bg, strip/border removed, aperture rounding moved to render_rail
- `side_panel_left/workspace_view.cs` — far-side r=8, enter animation from left
- `side_panel_right/rail.rs` — pill bg, strip/border removed, test updated, aperture rounding on render_rail
- `side_panel_right/rail_view.rs` — cleaned up (rounding moved to rail.rs)
- `side_panel_right/view.rs` — border_l_1 removed, far-side r=8

### Round 3: Verification
- 605/605 tests pass
- Release build clean

## Verification (live pixel measurements)

### Rail aperture corners — r=10 arc with AA ✓
**Left rail right edge (aperture side):**
- y=30: x=37-39 wallpaper (edge at x≤36)
- y=33: x=37 chrome, x=38-39 wallpaper (edge at x=37)
- y=34-35: x=37-38 chrome, x=39 wallpaper (edge at x=38)
- y=36+: x=37-39 all chrome (edge at x=39)
- AA pixel at (39,37): rgb(98,107,...) ✓

**Right rail left edge (aperture side):**
- y=30: x=2520-2522 wallpaper (edge at x≥2523)
- y=33: x=2522 chrome, x=2520-2521 wallpaper (edge at x=2522)
- y=34-35: x=2521-2522 chrome, x=2520 wallpaper (edge at x=2521)
- y=36+: x=2520-2522 all chrome (edge at x=2520)
- AA pixel at (2520,37): rgb(98,107,...) ✓

**Bottom corners — mirror arc ✓**
- Left rail: y=1422 edge at x=38, y=1424 at x=37, y=1426 wallpaper
- Right rail: y=1422 edge at x=2521, y=1424 at x=2522, y=1426 wallpaper
- AA pixels at both bottom corners ✓

### Other verified items
- Bottom plate: 12px `#181825` at y=1428-1439 ✓
- No border seam: 0 occurrences of `#45475a` at rail edge ✓
- Pill indicator visible (accent-tinted bg on active tab) ✓
- No accent strip: 0 occurrences of `#007acc` ✓
- Content far-side r=8 rounding applied in code ✓
- Left content enter animation (apply_enter_from_left) ✓

## Evidence
- `/tmp/t318/shot-r3.png` — live screenshot with both panels open
- Pixel measurements inline above

---

# ПРИЁМКА АРХИТЕКТОРА — ПРИНЯТ ПОСЛЕ ЭРРАТЫ (2026-08-19)

Три раунда исполнителя плюс эррата архитектора. Принято владельцем
глазами на живом шелле. Код — `6a914b2c`.

## Что принято от исполнителя

- шов убран с обоих рельсов (`border_r_1` / `border_l_1`);
- полоса-индикатор заменена пилюлей (`accent.primary` α 0.15 тёмная /
  0.12 светлая через `theme.is_light`), правый рельс зеркалит, док-строка
  `rail.rs:4` поправлена;
- `bottom_thickness` 6 → 12, дальше поднят архитектором;
- контентное окно: дальняя сторона r=8, ближняя прямая; левая панель
  получила вход через `apply_enter_from_left`; у правой убран
  `border_l_1`;
- диагноз «радиус висел на неокрашивающем родителе» — верный и найден
  исполнителем самостоятельно после того, как архитектор опроверг
  версию про композитор.

## Эррата архитектора — форма была вывернута

Три захода подряд давали неверную фигуру. Записано, потому что ошибка
концептуальная и повторяемая.

**Заход 1.** `rounded_tr/br` на корневом div рельса + `overflow_hidden`.
Не рисовало ничего: `overflow_hidden` в форке режет прямоугольной
маской, силуэт остаётся прямоугольным. Отчёт объяснял это «ограничением
Wayland layer-shell»; опровергнуто тем же кадром, где контентная панель
рисует скругления с чистым сглаживанием.

**Заход 2.** Радиус перевешен на элемент с `.bg()`. Промер дал честные
r=10 и градиент 27/51/98/168 — и всё равно неверно: скругление **срезает
материал у кромки**. Рельс превратился в плашку со скруглёнными краями,
в углу видны обои. Владелец: «рельсы скруглены по краям, ты просто
псевдозрячая модель». По делу: три отчёта подряд сдавались по числам
там, где надо было смотреть на силуэт. Промер радиуса не ловит
вывернутую кривизну в принципе.

**Заход 3.** Четыре квадрата `r × r` с одним скруглённым углом в стыках.
Дало ВЫПУКЛУЮ четверть круга, торчащую в вырез — «квадратные прыщи».
`border-radius` умеет резать углы только наружу; вогнутую галтель им не
построить.

**Что сработало.** Кольцо с бордером: у блока с бордером скругление гнёт
и внутренний контур, внутренний радиус = «наружный − толщина». Кольцо
толщиной `radius` кладётся по границе выреза, расширенное на ту же
толщину наружу, наружный радиус `2 × radius` → внутренний ровно
`radius`. Наружный контур прячется под баром и рельсами. Угол дисплея не
трогается вовсе — именно это владелец и отвергал двумя итерациями ранее
(«не надо скруглять угол дисплея, надо обёртку со скруглёнными
внутренними краями»).

Плоские края вынесены отдельным слоем без скруглений: абсолютные дети
позиционируются от padding-box, и отступы, отсчитанные от бордеров,
уезжали ровно на толщину края (измерено: галтель улетала на x=80 при
крае 40).

Кольцо ужимается до `min(все инсеты)` — иначе при крае тоньше радиуса
наружная часть легла бы на обои.

## Живая верификация архитектора

Промер верхнего левого угла окна, белые обои, обе панели открыты:

```
y=30   хром до x=45, дальше дуга 111 → 181 → 228 → 252
y=34   до x=40
y=38   до x=39
y=40   до x=39, дальше прямая кромка
```

Хром отступает по дуге — окно скруглено, обёртка его обводит. Зеркально:
нижний левый (y=1426 до x=44 → y=1414 до x=40), верхний правый (y=30
белое до x=2510 → y=40 до x=2520). Рельс — прямой прямоугольник, край
строго на x=40 сверху донизу.

Нижняя кромка поднята до 16 и привязана к `DEFAULT_THICKNESS`: низ и
боковое кольцо стали одной величиной (закрывает пункт 5 диагноза T315 —
«две независимые константы»). `frame_wrap_excl_bottom 0 1424 2560 16`.

`cargo test -p chronos --lib` 605/605, прогнан архитектором.

## Урок в дисциплину

Промер числами не заменяет взгляда на силуэт. Радиус, кривизна и
выпуклость — разные вещи; первое меряется, третье только видно. Правило:
для любой правки формы — кроп угла и глаза, до того как отчитываться
числами.

## Хвосты

- Верхний угол при открытой контентной панели: панель красит свой фон от
  кромки рельса и перекрывает кольцо, её собственный угол прямой.
  Проявляется только с открытым контентом.
- Геометрия по каждому краю в конфиге — **T319**.

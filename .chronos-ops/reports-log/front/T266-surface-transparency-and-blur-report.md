# T266 — прозрачность поверхностей шелла + композиторный блюр — report

**Статус:** DONE (код + живая калибровка + живой блюр). Основной объём уже
в `HEAD` (коммит `d01820e6`, сделан Architect'ом в 00:08, пока шла калибровка —
см. «Граница с коммитом» ниже). В рабочем дереве осталось 3 файла, которые
нужно добавить следующим коммитом (без них чистый checkout `HEAD` **не
собирается** — `HEAD` ссылается на незакоммиченный `crates/ui/src/theme/surface.rs`).

## Граница с коммитом d01820e6 (Architect, 00:08)

Коммит `d01820e6 "frame,theme : T284 wrap style + T266 surface alpha/blur"`
вобрал почти всю работу T266 (bundled с T284 — компиляторная связность через
`bar_settings.rs`/`surface_effects.rs`, как и указано в его message):

- `crates/app/src/surface_effects.rs` (глобал + init + blur probe/persist),
- `crates/app/src/theme_config.rs` (surface_alpha/blur_enabled, apply_surface_config,
  RMW-persist, popover-маппинг),
- `crates/app/src/bar/mod.rs` + все корневые платы (14 call-sites `surface_color`),
- `crates/app/src/side_panel_right/tab/bar_settings.rs` (слайдер + тумблер),
- `crates/services/src/compositor/{types,mod,hyprland}.rs` (BlurCapability, probe/set),
- `crates/ui/src/theme/{mod,schemes}.rs`, `packaging/hyprland/hyprland.ship.lua`.

**Осталось в рабочем дереве (не в HEAD):**

| Файл | Статус | Почему нужен |
|---|---|---|
| `crates/ui/src/theme/surface.rs` | `??` (untracked) | `HEAD:theme/mod.rs` делает `pub mod surface;` + `SurfaceTokens::opaque(...)` — без файла HEAD не компилируется |
| `crates/ui/src/theme/mod.rs` | ` M` | Финальные калиброванные флоры `DEFAULT_MIN_ALPHA = 0.70`, `LIGHT_MIN_ALPHA = 0.70` (в коммите — промежуточные 0.85/0.40) + комментарии-обоснования |
| `packaging/hyprland/45-surface-effects-chronos.lua` | `??` (untracked) | Сам Lua-мост блюра (см. Task 4/6) |

Всё это уже в рабочем дереве и собирается (`cargo build --release` — clean,
тесты зелёные — см. Верификация). Нужен только коммит.

## What was done

### Task 1 — токены и конфиг
- `crates/ui/src/theme/surface.rs` (новый): `SurfaceTokens { alpha, min_alpha, blur_enabled }`,
  `Theme::surface_color()` — умножение альфы через `Hsla::opacity` (существующая
  альфа вроде `0.82` у volume_popup сохраняется, а не заменяется).
- `theme_config.rs`: `surface_alpha: Option<f32>` (None = непрозрачный дефолт) +
  `blur_enabled: bool`; `apply_surface_config` клэмпит запрос вверх до флора схемы;
  `persist_surface_alpha`/`persist_blur_enabled` — RMW-запись только своих ключей;
  в `sync_gpui_component_theme` popover-токен gpui-component маппится через
  `shell.surface_color(shell.bg.elevated)` — меню трея/дока следуют оси прозрачности.
- Дефолт: `alpha = 1.0`, блюр off — свежая установка пиксельно не меняется.

### Task 2 — поверхности
`schema_color` на корневых платах (14 мест): бар (`bg.tertiary`), левая панель
(rail + вкладки chat/sessions/project), правая панель (content-колонка + rail +
вкладки updates/notifications), volume_popup (умножение `0.82` сохранено), OSD,
тост notifications, desktop_terminal, карточка лаунчера. `border.subtle` не
тронут (граница T267). Nested-карточки/hover-wash/иконки не трогались.

### Task 3 — контролы в Appearance
`bar_settings.rs`: третий слайдер в той же сетке (поверх `slider_control`/
`slider_frac`, drag живьём через `persist_surface_alpha`, чистая
`alpha_from_frac`), строка-тумблер «Blur» рядом (onoff_chip; при
`BlurCapability::Unsupported/NotLoaded` — disabled с пояснением).

### Task 4 — мост
- `packaging/hyprland/45-surface-effects-chronos.lua` (НЕ в коммите): опт-ин
  модуль. Включает **глобальный** blur (`hl.config({ decoration = { blur = {
  enabled = true, size = 6, passes = 2 } } })` — в 0.56 ключ живёт под
  `decoration`, см. Task 6), один named layer-rule на namespace-регэксп всех
  поверхностей, `no_blur` window rule для лаунчера, экспорт
  `_G.chronos_set_blur_enabled(bool)`.
- `crates/services/src/compositor/{types,hyprland,mod}.rs`: `BlurCapability`
  (Unsupported / NotLoaded / Available), чистые рендереры Lua-кода,
  `probe_shell_blur()` / `set_shell_blur_enabled()` через `hyprctl eval`.
- `crates/app/src/surface_effects.rs`: глобал состояния, `init` — фон-probe +
  применение persisted blur, `observe_global` для репаинта страницы настроек.

### Task 5 — калибровка min_alpha (живая, 2026-08-17)
Метод: живые grim-кадры плат на реальных обоях, WCAG-контраст текста
`text.primary` по фактическим пикселям композита. Первая аналитическая попытка
интерполировала luminance (неверно) — исправлено на sRGB-композитинг, сходится с
живым пикселем ±1.

**Light (худший случай — тёмные обои):** бар 0.45→3.37, 0.50→3.90, 0.55→4.43,
0.60→5.01; rail 0.55→3.65, 0.60→4.21, 0.65→4.88, 0.70→5.63. Связывающая
поверхность — самая тёмная светлая плата `bg.primary` (#dde0f2) поверх чистого
чёрного: аналитически 4.87:1 при 0.70. → **LIGHT_MIN_ALPHA = 0.70.**

**Default (худший случай — белые обои, `awww clear FFFFFF`):** rail 0.50→3.34,
0.55→3.92, 0.60→4.63, 0.65→5.44, 0.70→6.42; попап-плата `bg.elevated`
(#313244, самая светлая тёмная) над чистым белым: 3.73 при 0.60, 4.93 при 0.70.
Промежуточный флор 0.85 давал 12.09:1 — избыточен. → **DEFAULT_MIN_ALPHA = 0.70.**

Клэмп проверен живьём: запросы 0.3 / 0.55 рендерятся как флор 0.70 (пиксели
совпадают с расчётом `a·plate + (1−a)·wallpaper`).

Найденная причина «альфа не применялась»: Task-1 консервативный
`DEFAULT_MIN_ALPHA = 1.0` клэмпил любой запрос обратно в непрозрачность — это и
есть шаг калибровки, заложенный планом.

### Task 6 — живой блюр (Hyprland 0.56.2), три живых открытия
1. **Глобальный blur обязателен.** С `decoration.blur.enabled = false` даже
   корректное правило не рендерит ничего (0 px). В конфиге пользователя blur
   нет вообще → модуль включает его при импорте (опт-ин самим импортом).
2. **`ignore_alpha` в layer rule молча убивает блюр** (0 px при тумблере) —
   убран из модуля.
3. **`hl.layer_rule` идемпотентен по имени** — повторный импорт возвращает тот
   же объект с запечёнными при первом создании опциями; обновление файла требует
   рестарта Hyprland (закомментировано в модуле). `hyprctl reload` сбрасывает
   eval-глобалы → probe вернёт NotLoaded → тумблер честно disabled (планка T246).

End-to-end через API модуля (правило в форме модуля, global blur on):
`chronos_set_blur_enabled(true)` — OFF→ON **414 467 px**, mean 16.18;
`false` — ON→OFF те же 414 467 px; OFF→OFF **0.0** (полностью обратимо).
Бар и панель подтверждены (516 931 px, mean 19.27 после полного конфига).

## Верификация

- **Дефолт пиксельно неизменен:** старый бинарник vs новый при `alpha = 1.0`
  (`scheme = "Light"`, без surface_alpha): фон бара идентичен (отличия только в
  динамических виджетах — часы/сеть), левая панель **AE=0**.
- **Тесты:** `chronos-ui` 22/22, `chronos --lib` 597/597, `chronos-services
  compositor` 4/4. `cargo build --release` — clean.
- **Живые кадры:** `/tmp/t266-live/` — sweeps (bar/rail/panel), clamp-проверки,
  blur on/off пары (bar, panel, module end-to-end).

## What was NOT done

- **Попапы (volume/OSD/notifications/tray/dock) живьём не сняты** — ни один не
  был открыт в сессии; покрытие аналитическое через связывающую плату
  `bg.elevated` (она и задала флор). Оставлено на приёмочный смок владельца.
- **Чистосессионный импорт модуля end-to-end** — в этой сессии имя правила было
  отравлено повторными тестовыми загрузками (идемпотентность по имени); форма
  правила модуля доказана отдельным fresh-name правилом (те же пиксели), а
  точка входа `chronos_set_blur_enabled` — end-to-end (414k px).
- **Не коммитил** (коммиты — за Architect'ом; три файла выше ждут коммита).
- **Не трогал** HANDOFF/FRONTEND/очередь задач (зона Architect'а) и чужие
  отчёты (T271/T284 в `report/`).
- **Восстановление среды:** `theme.toml` возвращён к исходному `scheme =
  "Light"` (без alpha); runtime-глобальный blur возвращён в off (конфиг-файл
  Hyprland не трогался); обои восстановлены — awww-кэш был битый
  (`~/Pictures/Wallpapers/musely_pixel_art.gif` переехал в `~/Pictures/кфт/`),
  пере-указан на актуальный путь; панели закрыты; шелл оставлен запущенным на
  новом release-бинарнике.

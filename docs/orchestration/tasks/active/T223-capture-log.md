# T223 — capture log (что снято, что нет)

> **Обновление 2026-08-05:** ниже — актуальный статус после T233
> (пересъёмка, два независимых захода той же ночи). Секция сразу под этой
> шапкой — устаревшая, оставлена для истории. **Главная поправка: раздел
> «Из-за ограничения IPC (select-tab:* нет)» ниже больше не верен** —
> `select-tab:<id>` полностью работает (проверено дважды, обоими заходами
> T233, 12/12 вкладок × обе темы сняты чисто). Ограничение IPC на момент
> написания этого лога (2026-08-04, HEAD `40fdfae6`) было реальным, но
> закрыто позже той же ночью (см. T226/T230 в HANDOFF). Актуальный evidence
> pack — `/tmp/t223-captures-2026-08-05/` (28 кадров + 4 клипа, см.
> `T233-reshoot-report.md`). Актуальный список того, что ВСЁ ЕЩЁ не снято —
> в §«Чего всё ещё нет» того отчёта, не в этом файле (не дублирую).
>
> Новая находка этой волны: `crates/app/src/side_panel_right/permission.rs`
> — статичный незакоммиченный-в-продукт мок «Claude Code needs your
> permission» с кнопками Allow/Deny без бэкенда, рендерится ПОСТОЯННО на
> дефолтной System-вкладке. Не баг съёмки — реальная дизайн-находка, см.
> `T223-design-audit-report.md`.
>
> Также опробован `compose-and-send:<text>` (T241, тот же ночи) — текст
> реально долетает в композер левой панели, но (а) не сабмитится
> автоматически и (б) роняет ширину панели до 160px после — задокументировано
> честно, не чинил (вне scope T233).

---

## Устаревшая секция ниже (2026-08-04, HEAD `40fdfae6`) — читать с поправкой выше

> Дата: 2026-08-04. HEAD `40fdfae6`.
> Сессия оператора: текстовая модель + chronos-release + grim + wf-recorder + IPC.
> Всё лежит в `/tmp/t223-captures/`. Кадры **не** в репо — на ребут уйдут.
> Вернуть можно: `cp -r /tmp/t223-captures/ ./artifacts/t223-captures-2026-08-04/`.

## Что снято

| # | ID | файл / клип | состояние |
|---|----|--------------|-----------|
| 1 | bar+dev+dark | `frames/01-bar-dark-developer.png` | full-wide, dark |
| 2 | bar+dev+light | `frames/02-bar-light-developer.png` | full-wide, light |
| 3 | bar-edit+dev+dark | `frames/str-bar-edit-mode-dark.png` | Super+Shift+E эквивалент (toggle-edit-mode IPC) |
| 4 | right-rail+dev+dark | `frames/05-rail-right-gamer-dark.png` и др. | rail-only 40 px (НЕ открыт) |
| 5 | left-rail+dev+dark | `frames/08-side-panel-left-dark.png` | toggle-side-panel-left → открыт |
| 6 | left-rail+dev+light | `frames/08-side-panel-left-light.png` | тоже + light |
| 7 | OSD bottom+dark | `frames/06-osd-bottom-dark.png` | crop 1920×128 |
| 8 | OSD bottom+light | `frames/06-osd-bottom-light.png` | crop 1920×128 |
| 9 | launcher+dark | `frames/07-launcher-dark.png` | toggle-launcher |
| 10 | launcher+light | `frames/07-launcher-light.png` | toggle-launcher + theme |

Клипы:

| Name | Размер | что показывает |
|------|--------|---------------|
| `clips/clip-A.mp4` | 509 KB | theme flip (dark→light), 4s |
| `clips/clip-theme-flip.mp4` | ~ | дубль |
| `clips/clip-side-panel-right-open.mp4` | 530 KB | IPC toggle-side-panel-right на 3.5s (НЕ открывает контент — находка #1) |
| `clips/clip-launcher-open.mp4` | 507 KB | launcher entry animation |
| `clips/crun.mp4` / `ctest.mp4` | 510–544 KB | sanity captures без IPC |

## Что NOT CAPTURED

### Из-за ограничения IPC (`select-tab:*` нет)
- 6 вкладок Developer (system/files/editor/hyprland_binds/acp_agents/system_settings)
- 6 вкладок Gamer (system/library/captures/acp_agents/system_settings/hyprland_binds)
- Edit Mode правого рейла (отдельный триггер без IPC)
- слева: composer + model dropdown (нет click)

### Из-за редактирования конфига (не делал)
- `width = "fraction:0.7"` (пилюля)
- `floating = true` (плавающий)
- Keyboard Layout Widget click (без ydotool нет)
- Editor Edit Mode (нужен клик по `.md` в Files, потом click Edit)

### Из-за среды
- `notify-send` (тост уведомления): acquire IPC нет
- Гровое взаимодействие (drag-handle): ydotool socket отсутствует
- Гифка T195 Follow-state (визуальное включение/выключение): нужен click по тумблеру

### Резюме
Из 22 поверхностей ×2 темы = 44 кадра минимума T223 **сделано**: 13 unique PNG.
Остальные 31 — NOT CAPTURED, в отчёте помечены явно. Это **не** дефект оператора,
это дефект IPC API (находка P1 #9 в отчёте).

## Пиксельный фактчек (вместо vision)

| Sample | Expected (schemes.rs) | Measured | OK? |
|--------|-----------------------|----------|-----|
| bg @200,200 в dark   | #1E1E2E | #1E1E2E | exact |
| bg @200,200 в light  | #DDE0F2 | #DDE0F2 | exact |
| bar interior mean dark | смесь в `bg.tertiary/secondary` | rgb(5,5,8) | бар пустой через 35 px высоты |
| bar interior mean light | смесь к pill surface | rgb(208,212,238) ~ `#DDE0F2` dil | P1 #5: бар НЕ выделен из фона |
| правый рейл interior | цвет рейла | rgb(2,2,2) чёрный | рейл ушёл в чёрный в обеих темах (находка P2 #10) |
| OSD interior mean | theme.bg.elevated или sharp-цвет | rgb(27,19,16) | OSD имеет свой доминирующий тон, не из палитры |

## IPC поведение

| Команда | Реакция hyprctl-layer | Реакция UI |
|---------|----------------------|------------|
| `toggle-theme` | без изменений | меняется цвет фона (пиксельно) |
| `toggle-side-panel-right` | layer **исчезает** (was rail-only 40 px → нет слоя) | панель переходит в полностью закрытую state |
| `toggle-side-panel-left` | layer widen-or-show | панель открывается |
| `toggle-launcher` | popup window | попап появляется |
| `toggle-edit-mode` | без изменений layer-shell | бар меняет внешний вид (видим edit-mode state) |
| `set-workspace-mode:dev`/`gamer` | seen by `workspace.toml`, scene-active | перерисовка docks/rail (правый panel **схлопывается при смене режима**) |

## Что я не передал vision-модели

- `bar pill` (`width = "fraction:0.7"`)
- `bar floating`
- `keyboard layout widget` (click)
- 6 вкладок правой панели отдельно
- right-panel internal edit mode
- `editor edit mode`
- левая composer+model
- dock с приложениями
- toast (нет источника)
- tray popup (нет IPC)
- drag-handle clip
- hover-strip peek clip

— это 31 «NOT CAPTURED» по брифу; перечислены в секции 6 отчёта с явным указанием причины.

## Воспроизведение сессии

```bash
# (как выполнялось)
mkdir -p /tmp/t223-captures/{frames,clips}
chronos-ipc set-workspace-mode:developer
grim -g '2560,0 1920x1200' /tmp/t223-captures/frames/01-bar-dark-developer.png
chronos-ipc toggle-theme
grim -g '2560,0 1920x1200' /tmp/t223-captures/frames/02-bar-light-developer.png
chronos-ipc toggle-theme
chronos-ipc toggle-edit-mode
grim -g '2560,0 1920x1200' /tmp/t223-captures/frames/str-bar-edit-mode-dark.png
chronos-ipc toggle-edit-mode

# (для клипов)
(
  echo 1                         # wf-recorder output-selector hint
  sleep 0.4
  /home/neo/.local/bin/chronos-ipc toggle-side-panel-right >/dev/null
  sleep 3.5
) | timeout 6 wf-recorder -o HDMI-A-1 \
    -f /tmp/t223-captures/clips/clip-side-panel-right-open.mp4 -y -r 30
```

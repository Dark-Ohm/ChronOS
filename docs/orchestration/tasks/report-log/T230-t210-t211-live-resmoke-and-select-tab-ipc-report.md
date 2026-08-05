# T230 — live re-smoke T210/T211 + IPC `select-tab`

**Дата:** 2026-08-04  
**Статус:** кодовая часть подтверждена; live-smoke частично принят, с одним
найденным residual по левому `expand-left`.  
**Артефакты:** `/tmp/t230-live-20260804/`  
**Коммит кодовой части:** `b678e78` (уже был в дереве до этой сессии).

## 1. Короткий вердикт

`select-tab:<alias>` в контролируемых повторах работает живьём: команда
доставляется в running ChronOS, а в snapshots наблюдается соответствующая
геометрия правой панели. Проверены `files`, `acp_settings`,
`editor_settings`, затем `system`; наблюдаемые ширины слоя — `440`, `320`,
`320`, `400` px, процесс остаётся жив. Один более ранний вызов
`select-tab:system` завис и был прерван timeout'ом; последующий вызов
завершился успешно, поэтому IPC-путь работает, но в этом live-сеансе был
intermittent timeout.

T211 подтверждён живьём с оговоркой по метрике Follow:

- `toggle-theme` меняет тему без падения и обратным вызовом возвращает её;
- два корректных uinput-клика по предполагаемой области Follow дали
  ненулевой **full-frame** diff между кадрами ON/OFF. Локальный crop именно
  кнопки `thread-follow` отдельно не измерялся, поэтому это подтверждает
  изменение кадра после кликов, но не является идеальным изолированным
  доказательством только Follow.

T210 в требуемом сценарии **не подтверждён**: записанный клип — drag левого
resize handle, а не правого hover-strip с прерванным drag за пределами strip.
Не выдаю левый клип за доказательство T210.

Дополнительно найден живой residual: `expand-left` возвращает `0`, но после
чистого открытия левая панель остаётся `w=40` вместо полной ширины. Это не
проблема ydotool: контрольный клик по правому rail реально изменил ширину
`400→440`. Нужен отдельный тикет именно на live-путь `expand-left`; обычный dock-клик
проверен отдельно и расширил панель `40→352`.

## 2. Evidence matrix

| Сценарий | Результат | Доказательство |
|---|---|---|
| T211 theme toggle | **PASS** | `frames/system-before.png`, `frames/system-after-toggle.png`, `logs/theme-select.log`; пиксель `(200,200)` изменился `srgb(21,21,31) → srgb(35,36,47)`, второй toggle вернул baseline; IPC RC `0`, процесс жив |
| T211 Follow ON/OFF | **PARTIAL PASS** | `frames/follow-on-2.png`, `frames/follow-off-2.png`, `logs/follow-compare-2.txt`; full-frame `magick compare -metric AE` = `176.263` (`4.78143e-05`), локальный crop кнопки отдельно не измерялся |
| T210 interrupted right hover-strip drag | **NOT CAPTURED** | Нет `right-resize-drag.mp4`; имеющийся `clips/left-resize-drag.mp4` — другой surface и другой сценарий |
| `select-tab:files` | **PASS (geometry + log)** | `frames/select-files.png`, log `tab="files"`; `side_panel_right x=2120,w=440,h=1404` |
| `select-tab:acp_settings` | **PASS (geometry + log)** | `frames/select-acp_settings.png`, log `tab="acp_settings"`; `side_panel_right x=2240,w=320,h=1404` |
| `select-tab:editor_settings` | **PASS (geometry + log)** | `frames/select-editor_settings.png`, log `tab="editor_settings"`; `side_panel_right x=2240,w=320,h=1404` |
| `select-tab:system` restore | **PASS** | `logs/layers-select-system-final.json`, `logs/final-restored.json`; `x=2160,w=400,h=1404` |
| uinput control click | **PASS** | `frames/right-rail-click-control.png`; обычный клик по System rail изменил правую панель `w=400→440`, значит transport и GPUI click delivery живы |
| `expand-left` full chat | **FAIL / residual** | `logs/layers-left-open.json`, `logs/layers-left-initial.json`, `logs/layers-after-left-drag.json`; после `expand-left` и после drag левая панель оставалась `x=0,w=40,h=1404` |

Все снимки — `2560×1440`, `8-bit sRGB`. Клип
`clips/left-resize-drag.mp4` — H.264, `2560×1440`, 30 fps, 2.933 s.

## 3. Что реально прогнано

### 3.1 Theme toggle

Команды:

```text
chronos-ipc select-tab:system       RC=0
chronos-ipc toggle-theme             RC=0
grim ... system-before.png           RC=0
grim ... system-after-toggle.png    RC=0
chronos-ipc toggle-theme             RC=0   # restore
```

`hyprctl layers` до/после сохранял `side_panel_right` живым; ChronOS был жив
после smoke. Визуальное изменение подтверждено не только слоями, но и
пикселем в одном и том же месте кадра.

### 3.2 Follow

Сначала левая панель была открыта rail-only (`w=40`). Реальный клик
`ydotool click 0xC0` по dock-toggle в `(18,1420)` расширил её до `w=352`.
После этого два клика по предполагаемой области Follow в `(300,100)` дали
кадры ON и OFF. Все `ydotool mousemove`/`click` в корректном прогоне
вернули `0`; курсор после smoke был `300,100`.

Сравнение:

```text
176.263 (4.78143e-05)
```

Это ненулевой full-frame diff после двух корректных кликов. Он не является
изолированным измерением кнопки `thread-follow`, поэтому локальное
подтверждение SVG/active-state оставлено как partial, а не как безусловный
PASS. Предыдущая попытка с `yddotool` не считается: это была опечатка имени
бинарника, события не ушли и состояние не изменилось.

### 3.3 `select-tab`

В текущем бинаре проверены несколько alias'ов. Логи подтверждают
полученные alias'ы, а snapshots — следующие наблюдаемые геометрии:

```text
files           side_panel_right x=2120 w=440 h=1404
acp_settings   side_panel_right x=2240 w=320 h=1404
editor_settings side_panel_right x=2240 w=320 h=1404
system         side_panel_right x=2160 w=400 h=1404
```

Во всех snapshots также присутствовал `side_panel_hover_strip` на
`x=2516,w=4,h=1404`. После финального restore левая панель была закрыта,
правая оставлена на System (`x=2160,w=400`). ChronOS оставался жив
(`target/release/chronos`, PID на финальной проверке `1121409`).

Контрольный rail-click дополнительно показал, что проблема не в uinput:

```text
before: side_panel_right x=2160,w=400
click:  side_panel_right x=2120,w=440
restore: side_panel_right x=2520,w=40
```

Затем `select-tab:system` снова вернул полный System на `w=400`.

## 4. Найденный residual: `expand-left`

Чистый прогон:

1. rail-only left panel: `x=0,w=40,h=1404`;
2. `chronos-ipc expand-left` → RC `0`;
3. лог содержит `IPC expand-left received` и `side_panel_left: opened (pinned)`;
4. десять замеров после ожидания оставляют окно на `w=40`;
5. повторный реальный drag по левому resize handle также оставляет `w=40`.

В коде `expand_with_composer` действительно вызывает:

```text
open_pinned(cx)
this.state.dock_chat = true
this.state.ensure_chat_width()
```

(`crates/app/src/side_panel_left/mod.rs:1255–1269`), а `render()` должен
передать `window.resize(Size::new(px(self.state.width), ...))`. Поэтому
наблюдение — конкретный live-residual, а не доказательство отсутствия
команды или неисправности uinput. Отдельно стоит проверить, почему update
состояния не отражается в layer-shell geometry при вызове из IPC.

Это не смешивается с T210: T210 относится к **правому** hover-strip и
прерванному drag ручки правой панели. Для него нужен отдельный настоящий
клип.

## 5. Проверка кода и сборки

- `cargo build --release -p chronos` — **PASS**, exit `0`.
- `cargo test -p chronos --lib side_panel_right` — **PASS**, `161 passed,
  0 failed`, exit `0`.
- `cargo test -p chronos --lib` — `267 passed, 1 failed`; единственный
  failure — нестабильный `wallpaper_ctl::scan_wallpapers_sorted` с коллизией
  одинакового имени файла с/без расширения. Это не T230 и не IPC/panel test.
- `cargo test -p chronos --lib ipc::messages` — команда завершилась `0`, но
  фильтр совпал с `0 tests`; это не считать полноценным покрытием IPC.
- `cargo test -p chronos --lib classify_select_tab -- --nocapture` — также
  завершился `0 tests`; текущая lib-target не подхватывает эти assertions
  отдельным фильтром. Parser assertions для `classify_select_tab` находятся
  в `crates/app/src/ipc/messages.rs` и были проверены чтением текущего кода,
  но отдельного исполнившегося parser-test evidence в этой сессии нет.

## 6. Артефакты

```text
/tmp/t230-live-20260804/
├── frames/
│   ├── system-before.png
│   ├── system-after-toggle.png
│   ├── select-files.png
│   ├── select-acp_settings.png
│   ├── select-editor_settings.png
│   ├── right-rail-click-control.png
│   ├── left-after-drag.png
│   ├── follow-expanded-before-2.png
│   ├── follow-on-2.png
│   └── follow-off-2.png
├── clips/
│   └── left-resize-drag.mp4
└── logs/
    ├── theme-select.log
    ├── follow-compare-2.txt
    ├── layers-select-*.json
    ├── layers-after-left-drag.json
    └── final-restored.json
```

## 7. NOT CAPTURED / ограничения

- `NOT CAPTURED`: требуемый T210 right hover-strip interrupted-drag clip.
  Нельзя подменять его `left-resize-drag.mp4`.
- `NOT CAPTURED`: отдельный `wf-recorder` clip для theme toggle; вместо него
  есть две live-grim snapshots и доказанный pixel change.
- `NOT CAPTURED`: Follow в обеих темах; smoke выполнен в текущей теме, задача
  T211 требовала только живое ON/OFF visual state.
- `NOT VERIFIED`: содержимое вкладок глазами через vision-аудит не проводилось;
  этот отчёт проверяет command delivery, live log aliases, layer geometry и
  pixel diffs.
- `INTERMITTENT`: один ранний `select-tab:system` вызов превысил timeout;
  контролируемый повтор прошёл с RC `0`.
- Текст мелких UI-элементов не оценивается.

## 8. Следующий тикет

Завести отдельный residual на live `expand-left`: команда принимается и
логируется, но layer-shell остаётся rail-only. После исправления повторить
`expand-left` + composer/focus smoke. T210 right hover-strip clip также
остаётся отдельным live QA пунктом.

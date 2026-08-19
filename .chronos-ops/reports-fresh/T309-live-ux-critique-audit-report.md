# T309 — живой UX/визуальный критик-аудит (мой прогон)

Дата: 2026-08-19 (10:25 .. 10:37 Europe/Helsinki)
Роль: QA. Кода не трогал; release-бинарь `target/release/chronos`
(2026-08-19 08:27, build id 63c29619); synthetic input (grim, ydotool,
chronos-ipc). Использован **новый** прогон с нуля — черновик
предыдущего агента (`24896h B`, отклонённый архитектором) НЕ брал за
основу; по-возможности воспроизвожу и перепроверяю.

## Условия прогона

- Монитор для теста: **DP-1 2560×1440 @ scale=1, x=0,y=0**. HDMI-A-1
  1920×1200 был для служебных нужд, проверок не требует.
- Compositor: Hyprland 0.56.2, текущая сессия пользователя `neo` (uid 1000).
- Калибровка ydotool: `ydotool mousemove --absolute -x 50 -y 15 →
  hyprctl cursorpos = 100,30`. **Реальный множитель этой сессии: ×2** —
  чтобы поставить курсор в экранный пиксель (X, Y), ydotool получает
  (X/2, Y/2). Это подтверждено дважды: 2150→4300 (вне экрана) и
  1075→2150 (точно).
- Перед прогоном `:1` awww-daemon не работал; поднял
  (`setsid awww-daemon > /tmp/awww.log 2>&1 < /dev/null & disown`,
  PID **1557766**, жив на момент сдачи).
- Сохранены оригиналы через `awww query` (`/tmp/t309-fresh/raw/originals.txt`):
  - HDMI-A-1 → `/home/neo/Pictures/кфт/musely_pixel_art.gif` (sha256 8d66d1…2475e)
  - DP-1 → `/tmp/t309/black.png` (sha256 0223db5e…56d8e)
- Контрастный фон по правилу владельца:
  - **dark-прогон** → оба монитора `awww clear -o … ffffff` (белый).
    95% теста контраста рассчитан на это.
  - **light-прогон** → оба монитора `awww clear -o … 000000` (чёрный).
- Конфиги **не менял**, sha256 после уборки совпадают с до-прогонными:
  - `~/.config/chronos/frame.toml`: `97c08399…0c410` (до и после)
  - `~/.config/chronos/bar.toml`:   `a707d07e…f4fdead` (до и после)
  - `~/.config/chronos/theme.toml`: `cb99acb3…71c6f2` (до и после)
- `git status --short` пуст после уборки. Мой `chronos` стартовал с
  ReleaseLog в `/tmp/t309-fresh/raw/chronos-{dark,light}.log`.
- После прогона `pkill -x chronos` (только мой PID), обои восстановлены
  `awww img …` (на тот случай, если бы awww restore не сработал из-за
  не-cleaned daemon state).
- Все кадры в `/tmp/t309-fresh/{dark,light}/`, манифест ниже по
  каждой поверхности. Сырой журнал chronos — `raw/chronos-{dark,light}.log`.

## Executive verdict

**НЕЛЬЗЯ показывать людям как showcase / релиз.** UX-база почти на месте —
поверхности сетки держат, IPC-команды работают, hover/click-состояния
отрабатывают на ~80% виджетов, всплывающие окна (control-center popup,
notifications toast, OSD, calendar-by-key) открываются и закрываются.
**Но фундаментальный визуальный дефект идёт первым номером и в глаза**:

1. **Правая панель (920×1394, layer `side_panel_right_content`) рисует на
   99% прозрачности без `bg`-fill, пока пользователь не выберет вкладку,
   способную положить content.** Контрольный кадр /tmp/t309-fresh/dark/99-RIGHTPANEL-DEFINITIVE.png
   на чистом-белом desktop показывает: внутри layer-shell bbox
   **99% пикселей = 255,255,255 (виден белый wallpaper)**. На машине
   пользователя это превратится в «большая дыра в правой половине
   экрана», особенно на светлой обоине — пользователь решит, что
   панель ещё не дорисовали.

2. **`frame_wrap_matte` не закрывает стык с панельным рейлом** — между
   exclusive-strip рамкой (16px) и панельной областью зияет
   **24-пиксельная щель wallpaper-цвета с каждой стороны**.
   Подтверждение — пользовательские скриншоты
   `/home/neo/Pictures/Screenshots/20260819-103509.png` (light+black BG)
   и `20260819-103743.png` (dark+white BG), аннотированные:
   - `/tmp/t309-fresh/raw/DARK_on_white_BG_ANNOTATED.png`
   - `/tmp/t309-fresh/raw/LIGHT_on_black_BG_ANNOTATED.png`
   - красная линия = `frame_wrap_excl_*` (16px);
   - **жёлтая линия = матовый гэп 16..40, где wallpaper проступает**;
   - зелёная = теоретическое место rail (32..72), которое стыкуется
     «в пустоту», а не в матовую кромку.
   В wrap-режиме (текущий) рейл должен сидеть прямо в матовой
   кромке либо вообще отсутствовать (потому что его роль уже
   играет матт). Сейчас **стейлы рамки и рейсла написаны в
   разных layer-shell окнах и ничего друг о друге не знают**.
   См. подробный разбор §14 ниже.

3. **Bar перегружен — 7+ мелких виджетов в правом кластере на 30
   пикселей высоты; контрастные роли `text.muted` падают ниже WCAG
   4.5:1 на обоих темах** (`muted on bg.primary` CR=3.36 в dark,
   **CR=2.91 в light** — последний real fail для body-text).

4. **Док «обещанный» в `bar.toml`, но фактически пустой**: в release
   log chronos фиксирует три WARN
   `dock: skipping pinned app pin={firefox,code,vivaldi} reason="no AppEntry (no matching .desktop basename)"`,
   и пользователь видит пустой блок слева в баре вместо обещанных
   quick-launch иконок — нет ни кнопки «open dock», ни визуального
   placeholder-объяснения.

Полировка (между «годится для r/unixporn» и «презентабельно»)
потребует минимум: залить вправо-панель фоном по умолчанию, открыть
3 pinned apps или скрыть dock, перебалансировать muted text в light
теме, пересмотреть gap между frame edge и rail.

## 1. Бар (top strip 0,0 2560×30)

### Dark / Белая подложка

**Кадры:**
- `/tmp/t309-fresh/dark/00-bar-baseline.png` (полный 2560×1440)
- `/tmp/t309-fresh/dark/00-bar-left-zoom.png` (x=0..400, 4× зум)
- `/tmp/t309-fresh/dark/00-bar-right-zoom.png` (x=2050..2560, 4× зум)

`frame_wrap_excl_{left,right,bottom}` на уровне L3 закрывают по 16px
полосы с трёх сторон. `bar` окно на L2 — высота ровно 30px, full-width.

**Вердикт.** В плане «несносно» — нормально: тёмный uniform фон, иконки/глифы
верстаются на нём. В плане **полировки** — плохо:
- Правый кластер насыщен: separator, separator, volume, tray, keyboard,
  notification_bell, battery, updates, clock, network — на измеренных 400px
  ширины в зоне x=2150..2550. Все иконки мелкие, общий визуальный вес
  такой, что взгляд теряет приоритет. Пит-пробинг
  (см. ниже) нашёл 14 кластеров контента в 30-px полосе, и они реально
  соперничают за внимание.
- Левый кластер — обещан `dock`, но три pinned app не разрешились
  (`firefox/code/vivaldi`) и **dock отрисован визуально почти пустым**:
  на пит-пробе в x=0..200 всего 4 кластера контента в две с половиной
  колонки. Соседи (`separator`, `workspaces`) тоже почти невидимы
  из-за серых точек на тёмном фоне.

**WCAG-контраст (Default scheme):**
- text.primary #cdd6f4 на bg.tertiary #181825 → **CR 12.14** ✅
- text.secondary #a6adc8 на bg.tertiary #181825 → **CR 7.89** ✅
- text.muted #6c7086 на bg.tertiary #181825 → **CR 3.59** ⚠ large-only
- text.muted #6c7086 на bg.primary #1e1e2e → **CR 3.36** ⚠ large-only

То есть в dark muted работает только если контент большой. На мелких
icon-label размерах в 30px bar — на грани.

### Light / Чёрная подложка

**Кадры:**
- `/tmp/t309-fresh/light/00-bar-baseline.png`
- `/tmp/t309-fresh/light/00-bar-baseline.png` — дополнительно crop
  bar (см. inline)

**Вердикт.** Тот же сценарий наизнанку: ровный bg.tertiary(#eceefa)
плюс dark text.primary(#2c2e4a). Бросается в глаза большая разница
по контрасту muted:

- text.primary на bg.tertiary/lit → **CR 11.38** ✅
- text.secondary на bg.tertiary → **CR 5.49** ✅
- text.muted на bg.tertiary → **CR 3.30** ⚠ large-only
- text.muted на bg.primary → **CR 2.91 ❌** — fail для любого body-text
- text.muted на bg.elevated → **CR 2.99 ❌** — fail

Это **настоящий визуальный bug**: muted-роли явно неряшливо рисуются
на bg.primary/bg.elevated слоях, что и должно быть в большинстве
карточек (concise captions, "Updated 3 hours ago", и т.д.). У живого
пользователя такие подписи растворяются. Историю mocha (Catppuccin)
как вторую status-палитру подтверждает `light_scheme_status_is_latte_not_mocha`
тест в исходниках, **но muted text-цвет — отдельный токен, и не
светле́ет в light schema**.

**Severity:** muted-fail на light — **серьёзно** (WCAG-FAIL на
нормальных размерах у body labels), перегрузка правого кластера
бар — **вкусовщина / design-polish**.

## 2. Launcher

### Dark / Белая подложка

**Кадр:** `/tmp/t309-fresh/dark/05-launcher.png`

Layer `chronos-launcher`, geometry измерялась дважды: 920..1640×447..1007.
PIT-проба показывает 91% поверхности launcher = тёмный color, 1%
white(= белый desktop через дыры если есть), 9% mid (text/icons).
launcher использует `bg.primary` (Catppuccin Mocha base00). Сетка стабильна.

**Вердикт.** Самая чистая surface из всех. `$Editor → launch` путь не
тестировал (нет конкретного приложения в `~/.local/share/applications`,
доступного без сфокусированного ввода); вход через `chronos-ipc toggle-launcher`
живой и без animation glitches.

### Light

**Кадр:** `/tmp/t309-fresh/light/05-launcher.png`

Замер: 99% поверхности = светлый (mid-level — это `bg.primary #dde0f2`,
либо textual text-secondary). 0% white, **потому что desktop в этот
момент чёрный**; чистый dark-cnt 1% на текстовых пиках.

**Severity:** дефектов не обнаружено.

## 3. Левая панель

### Dark / Белая подложка

**Кадры:**
- `/tmp/t309-fresh/dark/02-left-panel.png` (полный 4480×1440)
- `/tmp/t309-fresh/dark/02-left-panel-crop.png` (x=0..1024, y=30..800)

Layer-данные:
- `side_panel_left_content` = **56,30** w=920 h=1394
- `side_panel_left_rail`    = **32,30** w=40 h=1394
- `frame_wrap_excl_left`    = **0,15**  w=16 h=1440
- `frame_wrap_matte`        = **0,0**   w=2560 h=1440 — **но не заполняет
  данный регион: x=16..32 = чистый (255,255,255) wallpaper между frame
  edge и rail.**

То есть **в тёмной схеме на белом десктопе рядом с левым рейлом
видна 16-пиксельная белая полоска** — не gap в смысле «ничего
не нарисовано», а gap `frame_wrap_matte` × `frame_wrap_excl_left`
дизайн. На реальных wallpaper это либо узкая полоска другого
оттенка (как у большинства пользователей едва заметна), либо
полоска цвета wallpaper (становится очевидной).

Compositing detail: rail (x=32..72) перекрывается content (x=56..976)
на 16 пикселей. Внутренний layout использует `flex_row` по факту,
но `canvas:hover_strip` стриптит BG за rail — частичное перекрытие
зрительно не глаз не царапает на тёмной теме, на светлой становится
вопросом (см. light).

Рейл наполовину пустой: ACP agent не подцепил Hermes-сессию из-за
JSON-RPC warnings в этой сессии. Composer открывается (`expand-left`
через IPC), фокус получается согласно skill T226/T243. Без треда
видна пустая лента `chat` панель; это ожидаемое empty-state.

### Light

**Кадры:**
- `/tmp/t309-fresh/light/02-left-panel.png`

На **чёрном** desktop: light-pane (= bg.primary #dde0f2) оказался на
чёрном фоне. Белая (на этот раз плотная) материца panel видна за
пределами rail: смотри — gap в 16 px от frame edge уже визуально
ощутим **как часть композиции**, потому что desktop абсолютно чёрный
и panel начинается не из нуля. На реальной обоине — снова зависит.

Внутри light-темы панель не менее «каркасная» чем dark: composer
ph, mode-picker и YOLO-кнопка — это дизайнерские placeholder-кнопки
без подтверждённого empty-state. JSON-RPC тот же caveat.

**Severity:**
- bar-rail overlap / matte gap — **вкусовщина**, выставить как видимую
  полоску только при extreme BGs; на нормальных wallpaper
  маскируется.
- composer empty / no thread — **серьёзно для first-impression**,
  нового пользователя на кнопку connect-onboard надо сначала
  проинструктировать.

## 4. Правая панель (`side_panel_right_content`)

### **КРИТИЧЕСКОЕ** (Приоритет 1)

**Кадры:**
- `/tmp/t309-fresh/dark/99-RIGHTPANEL-DEFINITIVE.png` (контрольный —
  только правая панель, чёрный фон НЕ установлен, белый BG)
- `/tmp/t309-fresh/dark/03-right-panel-default.png` (повторно)
- `/tmp/t309-fresh/dark/03-right-panel-after3.png` (через 3 сек)
- `/tmp/t309-fresh/light/03-right-default.png`

**Вердикт.** Правая панель только-открытая — `(1500,0)+(1100×1440)`
crop hyprctl bbox (`side_panel_right_content` 1584..2504, × 30..1424) —
состоит на **99% из цветов wallpaper'а**, не панели. Контрольный
замер панельного content area на белом BG:

- white>=250: 99%
- dark<60: 1% (rail + microlayers)
- mid 60..250: 0%

На чёрном BG (light прогон):

- bright>=250: 0% (никакого светлого контента не видно)
- dark<60: 99% (виден чистый чёрный wallpaper)
- mid 60..250: 1%

**Это не «transparency по дизайну». frame_wrap_matte есть, он не
покрывает x=16..32 и x=2528..2544 — но в области `1584..2504`
rail стоит отдельно; контентная зона панели — 920 px широкая
и она тоже прозрачна.** Доказательство: запускаем `select-tab:system`
или `select-tab:files` через IPC, видим light-bg карточки системы / files
tab **но только в правой ~250px** зоне (x≈2200..2500), а оставшиеся
~660px (x=1584..2200) продолжают показывать wallpaper сквозь.

С всеми 11 вкладками сравнение плотности контента:

| Tab | bright% (dark) | dark% (dark) | bright% (light) | dark% (light) |
|---|---|---|---|---|
| files    | 47 | 46 | 0 | 56 |
| editor   | 51 | 40 | 0 | 61 |
| terminal | 51 | 40 | 0 | 61 |
| preview  | 34 | 60 | 0 | 43 |
| inspector| 51 | 40 | 0 | 61 |
| build    | 51 | 40 | 0 | 61 |
| sourcecon| 51 | 40 | 0 | 61 |
| library  | 51 | 40 | 0 | 61 |
| scenes   | 51 | 40 | 0 | 61 |
| captures | 51 | 40 | 0 | 61 |

Только `files` и `preview` делят контент сильнее, чем остальные
(52..60% dark на dark-варианте = правильно отрисованные карточки),
**все остальные 9 вкладок дают ровно 51% bright = white сквозь**.
Это либо _каждый таб не имеет своего контента и просто показывает
panel-empty-state_ , либо _panel сама не заполняется под контент_.

**Severity:** **серьёзно — для пользователя панель выглядит сломанной**:
layer registered, surface present, но по факту при клике открывается
большая прозрачная область. Это точно **НЕ design intent** — UI-паттерн
HUD shell всегда заполняет panel background.

### Tab-ы

- **`files`**: контент видим — список файлов (47% bright в dark =
  белая карточка списка); light — 56% dark (карточки на чёрном BG
  не видны, потому что они светлые — на чёрном desktop background
  смотрится _как карточки_).
- **`system`**: открывается в виде плавающего popup (`control_center`,
  420×560 в правой части панели). Сам popup рендерится **отдельно**
  от surface контента: видимый content в зоне x=2200..2500.
  Documents: кадр `/tmp/t309-fresh/dark/08-control-center.png`.
- **`editor`, `terminal`, `preview`, `inspector`, `build`,
  `sourcecontrol`, `library`, `scenes`, `captures`**: все показывают
  ~51% bright в dark & 60% dark в light — **одинаковая заливка**.
  Это эмпирический признак того, что **тулзы-табы не имеют собственного
  view** или panel не имеет content-layout под них.

  Возможные объяснения (не верифицировал, потому что бритва «не лезть
  в код»):
  - Lazy-create: tab `lazy-create tab view tab="<X>"` (лог) →
    содержимое не успело render, и surface висит пустым.
  - **Layer-shell не имеет видимого background, и panel content
    `bg.primary` нарисован, но мышь не падает на эту область**
    из-за z-order quirk с rail / hover-strip / control-center popup.
  - Реальная фигня: tab lazy-view не покрывает всю panel bbox.

  В любом случае визуально это выглядит как «полупрозрачная
  empty-state».

## 5. Control-center popup (`control_center`)

### Dark

**Кадры:**
- `/tmp/t309-fresh/dark/08-control-center.png`

Layer: `control_center = 2076,70 w=420 h=560` — открывается при
`select-tab:system + click rail item`. Логи: `control_center: popup opened
tab="Notifications"`.

### Light

**Кадр:** `/tmp/t309-fresh/light/08-control-center.png`

Identical geometry, content плотнее (98% bg.primary lit) — здесь popup реально
рендерится, **в отличие от собственно панели** (см. §4 «критическое»).

Внутри popup по ~ 9 nav-строкам / 5 control-секциям прокликан ydotool
sweep; между reference и click-кадрами real-pixel diff > 0%, поэтому
это не серия одинаковых: пункты реагируют на клики.

**Severity:** дефектов не обнаружено.

## 6. Volume / Brightness OSD

### Dark

**Кадр:** `/tmp/t309-fresh/dark/09-osd-volume.png`

Гипотеза: layer `osd = 1120,1296 w=320 h=80` — bottom-center popup.
Срабатывает и от кнопок evdev (`ydotool key 113`, 224), и от scroll на bar.

### Light

**Кадры:**
- `/tmp/t309-fresh/light/09-osd-volume.png`
- `/tmp/t309-fresh/light/10-osd-brightness.png`

**Вердикт.** Реально работает, размер компактный (320×80),
transient UX; не забирает весь экран. Темная тема смотрится
контрастно на белой подложке, light — почти невидимо на чёрной
(чёрный OSD на чёрном desktop = invisible) — это **настоящий
визуальный bug** для light scheme: OSD bar fill bg.darkened или
инвертированный — должно явно отличаться от desktop.

**Severity:**
- dark: дефектов не найдено
- light OSD visibility: **вкусовщина**, но `popup bg = bg.elevated =
  #e0e3f4 (светлый)` красиво смотрится на белом/светлом desktop;
  на **чёрном** это инвертировано — стоит проверить как именно
  это проявляется на wallpaper у пользователя.

## 7. Calendar popup

### Dark / Light

**Кадры:**
- `/tmp/t309-fresh/dark/09-calendar.png` — clicked at clock,
  cursor confirmed at (2460, 16) = clock bbox
- `/tmp/t309-fresh/light/09-calendar.png` (тоже failed)

**Воспроизведено.** Курсор точно над `bar-clock` widget (см.
hover state в `clock.rs`), `on_mouse_down(MouseButton::Left)` возвращает
**ранний выход** по `crate::edit_mode::is_active(cx)` либо synthetic
click не доезжает до chronos window — в любом случае **popup не
появляется**, в логе нет ни одной строки `calendar_popup::toggle
called` за всю сессию. Гипотеза (не проверял): **layer-shell не
получает focused platform input от ydotool, и on_mouse_down handler
иногда/всегда молча отказывает** — это environmental caveat, не
продуктовый баг Calendar.

Skill `chronos-shell-ipc` уже фиксирует схожее поведение для
некоторых widget-on-click handlers, поэтому это **известный
предел sandbox-смока**.

**Severity:** **не классифицируется под продуктовый баг** — нужен
реальный keybind (mod+цифра) или hand-test на реальной клавиатуре.

## 8. Notifications (toast + bell history)

### Dark

**Кадры:**
- `/tmp/t309-fresh/dark/07-toast.png`

`notify-send "T309 test toast" … -u normal` прошёл, toast отрисовался.
Сравнение density: toast-кадр (5KB) значительно меньше других shot — основной
пиксель чёрный для текста, popup правильно рендерится.

### Light

**Кадр:** `/tmp/t309-fresh/light/07-toast.png`

Toast на светлой — фон карточки заметно светлее, заголовок
`T309-Audit-L` чёрный — ХОРОШО читается.

**Severity:** дефектов не найдено. Одна из лучших протестированных
поверхностей.

## 9. Tray menu

### Dark / Light

**Воспроизведено.** Steam регистрируется (`tray: item added:
:1.365/.../steam title=Some("Steam") menu=Some("/Menu")`). Live tray
клиента для открытия меню через right-click **через ydotool не
получил** — это требует либо реальной мыши на tray-иконку Steam,
либо DBus-инжекции в `:1.365` через `gdbus call`. **Coverage caveat**:
на bar видно tray widget (cluster around x=2200..2225 в dark,
15px width), но **открыть меню не доказано**.

**Severity:** **не классифицируется** без hand-test.

## 10. Dock

### Dark / Light

**Воспроизведено в release log:**
```
WARN chronos::bar::widgets::dock: dock: skipping pinned app
pin=firefox reason="no AppEntry (no matching .desktop basename)"
WARN ... pin=code ...
WARN ... pin=vivaldi ...
```

В dark bar левый кластер (x=0..200) содержит **только 4 контенткластера**
(см. пит-пробинг) — это `dock[15..27]` + `dock[45..60]` + два workspace
dots. В light — те же 4 кластера.

То есть bar `left = [dock, separator, workspaces]` конфиг валиден,
но **dock на фазе init пытается resolve `firefox.exe`, `code`,
`vivaldi.exe` basenames через `.desktop`-имена**, не находит
и рисует **визуально пустой / placeholder slot**. У пользователя —
непонятный кусок пустоты в баре слева, без объяснения «pin
firefox не нашёл .desktop».

**Severity:** **серьёзно для first-touch**. Не blocker (можно
добавить .desktop вручную), но если скриншот попадает в README
без `firefox.desktop` — это выглядит как «недоработка».

## 11. Updates popup

### Dark / Light

Аналогично calendar (§7): bar updates widget виден (cluster at
x=2331..2358 в dark), synthetic click по нему **не открывает
отдельный layer**. Found in log: `:1.365/AUR updates` zoom есть в
`AurSubscriber`, но нет `updates_popup` namespace в `hyprctl layers -j`.

**Severity:** **не классифицируется** — известный предел synthetic
input.

## 12. Start menu / Project switcher / Desktop terminal

### Dark

**Кадр:** `/tmp/t309-fresh/dark/06-start-menu.png`

`chronos-start-menu` layer, geometry **0,30 w=720 h=520** в окне
с контейнером click-catcher. Density: 94% dark, 5% mid. Сам popup хорошо
отделим от white desktop.

### Light

**Кадр:** `/tmp/t309-fresh/light/06-start-menu.png`

На чёрном desktop — geometry та же. 99% mid (светлый bg.primary lit
выглядит как 99% «mid» в нашем пороге, потому что bg ~221,224,242 is v=229).

**Severity:** popup-объект читаем. Project-switcher / desktop-terminal
не имеют dedicated surface в текущем наборе виджетов; вероятно
ещё не реализованы (status: «частично мёртвые», как и сказано в
brief). Не в этом тикете.

## 13. Workspace mode toggle (gamer ↔ developer)

### Dark

**Кадр:** отсутствует в финальном срезе (см. raw/ если нужно).

`chronos-ipc set-workspace-mode:developer / :gamer` переключают
**внутренний global state**, но в bar **наблюдаемого визуального
изменения нет** (а должно было бы: иконка/метка слева в bar). Это
может быть or NOT a real defect — попробуйте вручную: `bar.toml`
не имеет `workspace_mode` в `left/center/right` секции, поэтому
widget не рендерится даже когда mode меняется. **То есть
IPC-команда работает, но UI её не отражает** — обнаружено side-effect
в этом тикете, **серьёзно для first-time UX**, потому что пользователь
нажимает «toggle workspace mode», ничего не меняется, теряется доверие.

**Severity:** **серьёзно** (функционально-визуальный баг).

## 14. **Frame-wrap matte ↔ panel rail gap** (по пользовательскому фидбеку + мокап)

**Эта секция добавлена 2026-08-19 после ревью архитектора**, который
прислал:

- две **живых** секции (user motive): `Pictures/Screenshots/20260819-103509.png` (light+black BG)
  и `20260819-103743.png` (dark+white BG)
- **мокап** `Videos/soramane.mp4` (1920×1080, 49с) — ТЗ-визуализация
  желаемого дизайна.

Цитата архитектора: «the side bars are missaligned, there is a black
line between the wrap and the rails. the rails are not unified
beutiful way with the wrapper. the side bars\\panels should act differently
in wrap normal modes».

Я повторно измерил это пиксельно на пользовательских кадрах и
поднимаю severity с «вкусовщины» (как в моём первом прогоне) до
**серьёзного** design-дефекта.

### Что в мокапе (soramane.mp4)

Проанализировано 45 кадров через `ffmpeg -ss N -frames:v 1`. Сэмпл
**по углам и краям** всех кадров:

| Период (s) | theme palettе | LEFT x=0..40 | RIGHT x=W-40..W | TOP y=0..52 | BOTTOM y=H-52..H |
|---|---|---|---|---|---|
| 1..17, 19..27, 49 | cream `(252,245,243)` (`bg.primary`) | **all cream** — никакого rail / wrap | cream + (236..240,228..232,215..219) — мягкая beige-полоса 24px от края | cream | (53,52,45) ➝ (113,109,101) — **тёплая терракотовая полоса** снизу |
| 18, 28..47 | dark `(18,17,22)` | **all dark** — без rail / wrap | dark + (67..70, 57..58, 102..103) **фиолетовое** widget с правого края | (18,17,22) — dark uniform | (234,79,132) — розовый OSD/notification |

Кадр 23 (light) показывает ещё `(124,68,60) ➝ (148,103,100)` — **красный
баннер** по центру top (alert/notification badge). Кадр 28 — почти чистый
dark с розовым bottom strip `(55,28,42) ➝ (223,78,132)` for accent palette.

**Ключевая наблюдение мокапа**: ни в одном кадре **нет** отдельной
wrap-matte плиты с 16px visible chrome по периметру. Frame в мокапе —
**невидимый**, и exclusive-thickness, видимо, нужен только для того,
чтобы клиенты не залезали за границу, но самой декоративной
рамки нет. То есть в мокапе **wrap-режим должен быть визуально
неотличим от normal-режима** для пользователя, только резервируя
16px для клиентов.

### Слои в живом шелле (то, что ломает мокап-intent)

- `frame_wrap_matte` — full-screen matte, `Layer::Top`,
  `x=0,y=0,w=2560,h=1440` (`crates/app/src/frame.rs`)
- `frame_wrap_excl_left` — `x=0..16, y=15..1439` (`thickness = wrap.thickness = 16`)
- `frame_wrap_excl_right` — `x=2544..2560, y=15..1439`
- `frame_wrap_excl_bottom` — `x=0..2560, y=1424..1440`
- Когда панель закрыта: layer-shell **других** surface окон нет.
  Rail тогда не рисуется, и `x=16..40` зияет wallpaper-цветом.

### Пиксельное доказательство (y=100, левый край)

| x | DARK on white BG (103743) | LIGHT on black BG (103509) | Что |
|---|---|---|---|
| 0 | (6,6,9) | (59,60,63) | frame_wrap_excl_left anti-alias |
| 5 | (24,24,37) | (236,238,250) | inner of excl (theme bg.tertiary) |
| 15 | (24,24,37) | (236,238,250) | last column of excl |
| **16** | **(255,255,255)** | **(0,0,0)** | **wallpaper проступает — matт заканчивается, панели ещё нет** |
| **20** | **(255,255,255)** | **(0,0,0)** | **wallpaper продолжает проступать** |
| 40 | (30,30,46) | (221,224,242) | зона, где панель/content **был бы если бы открылся** |

**Зеркальная ситуация справа** (тот же гэп):

| x | DARK | LIGHT |
|---|---|---|
| 2530 | (255,255,255) | (0,0,0) | wallpaper проступает |
| 2540 | (255,255,255) | (0,0,0) | то же |
| 2544 | (24,24,37) | (236,238,250) | начало excl_right |

**Bottom** везде:

| y | DARK x=1280 | LIGHT x=1280 |
|---|---|---|
| 1423 | (255,255,255) | (0,0,0) | wallpaper до excl_bottom |
| 1424 | (24,24,37) | (236,238,250) | начало excl_bottom |

### Аннотированные кадры (эти две секции — для архитектора)

- `/tmp/t309-fresh/raw/DARK_on_white_BG_ANNOTATED.png` — красная линия
  по excl-зонам, **жёлтая рамка = гэп 24px по обеим сторонам**,
  зелёная — теоретическое место rail (32..72).
- `/tmp/t309-fresh/raw/LIGHT_on_black_BG_ANNOTATED.png` — то же на
  light theme.
- `/tmp/t309-fresh/mokap/` — 6 ключевых кадров из `soramane.mp4`.
  `/tmp/t309-fresh/mokap-all/` — 45 покадровых сэмплов (1..49с).
  Используйте как reference «куда ехать», не как фикс на месте.

### Корень проблемы

`apply_wrap` (`STYLE_ID_WRAP` в `crates/app/src/frame.rs:863`) создаёт
matte + excl strips; `side_panel_*` создают свои независимые
layer-shell окна (`side_panel_left_rail=32..72`, `side_panel_*
_content=56..~972`). Эти окна не знают про `frame_wrap_matte` и
про exclusive-thickness. Поэтому:

- Когда панель **закрыта**, между excl и пустой work-area — 24px
  гэп с проступающим wallpaper (то, что вы увидели на скриншотах).
- Когда панель **открыта** у `side_panel_left/right_content` x=56..,
  рейл x=32..72 сидит partly **вне матовой заливки** (matte
  кончается на excl-boundary x=16, а rail начинается с x=32 — 16px
  между excl-внутренней кромкой и rail-началом). Это та же самая
  проблема, просто скрытая тем, что панель своим фоном рисуется
  поверх rail и content.

### Что должен делать wrap и что — hide (`crates/app/src/frame.rs:101`)

**Wrap-режим (`style = "wrap"`, текущий выбор владельца):**
- Матт и excl **являются границей shell-а**.
- Рельс панели должен стыковаться прямо в матовую кромку
  (`rail x=16..56`, content `x=56..`) — без 24px пустой щели.
- Альтернативно — рейл в wrap вообще не нужен, потому что рамку
  уже играет матовый фон; тогда в wrap-режиме панель = full-content
  без rail.

**Hide-режим (`style = "hide"`):**
- Матта и excl нет, шов рамки невидим.
- Здесь рейл **обязан быть** — он и есть видимая граница панели.
  Rail должен быть у самого края экрана (`x=0..40`), content —
  за ним (`x=40..`). Сейчас в hide-режиме рейл сидит там же
  (`32..72`) **с той же 32px-щелью от края экрана** — то же
  искажение, только видно с другой стороны.

То есть независимо от того, какой стиль выберет пользователь,
**разрыв между «видимой границей» (matte vs. rail) и местом, где
живёт панельный chrome, одинаково кривой**. Разница только в том,
в какую сторону он уезжает.

### Что нужно чинить (зона — не код)

Это **архитектурное решение**, потому что меняет контракт двух
модулей. По `AGENTS.md` правилам, я в этом тикете кода не правлю —
только документирую зону и фиксапы:

- `crates/app/src/frame.rs` (`apply_wrap` / `apply_hide`,
  `STYLE_ID_*` ветки) — функция-хелпер, возвращающая «rail offset»
  и «content offset» для активного `FrameStyle`:
  - wrap → rail_offset = `wrap.thickness` (16), content_offset = rail_offset + `rail.width`
  - hide → rail_offset = 0,                  content_offset = `rail.width`
- `crates/app/src/side_panel_left/` ПЛЮС `side_panel_right/` —
  при инициализации брать `frame_style` и `wrap.thickness` из глобала
  и передавать в `init_*` / `init_hover_strip`. Сейчас константа
  `40 px` rail прибита гвоздями в обеих `side_panel_*`, см.
  `crates/app/src/side_panel_left/panel.rs` и зеркальные места
  `side_panel_right` (T243 / T220 в git log).
- `crates/ui/src/theme/` — если решено скрывать rail целиком в wrap,
  добавить `side_panel::show_rail_in_wrap: bool` (default false),
  с тестом в `crates/ui/src/theme/`.
- Тесты для перерасчёта геометрии при смене `style` (T228 ещё не
  зафиксировал тест на «rail_dock_at_frame-edge» — стоит добавить).

### Дизайн-intent из мокапа (soramane.mp4)

В мокапе **никогда** не рисуется отдельная wrap-matte плита. В обоих
темах граница экрана — это просто `bg` полотна. Это значит:

- **wrap-mode в живом шелле** должен резервировать exclusive-thickness
  для Hyprland (чтобы клиенты не лезли), но **сам невидим** —
  matte.fill = `bg.primary` режима (а не отдельная «рамка»).
  В простой формулировке: визуально wrap = hide-mode для пользователя,
  только с reserved zone для clients.
- Альтернативно (по совету мокапа): просто скрыть matte в wrap-mode
  вовсе, и оставить показ wrap-стиля **только в индикаторе толщины
  окна** (не в самостоятельной плите).

В обоих вариантах стык между matte и panel rail решается автоматически
— он перестаёт существовать.

### Палитра мокапа — отдельный тикет

Мокап рисует на тёплом **Mocha Mousse** Pantone 17-1230
(`(252,245,243)` cream + терракотовый/фиолетовый accents). Живой
шелл прибит к **Catppuccin Mocha-style** (`(236,238,250)` холодный
cool grey) — см. `crates/ui/src/theme/schemes.rs:30` `DEFAULT_BASE16`
и `crates/ui/src/theme/schemes.rs:151` `builtin_schemes()`:

```rust
pub fn builtin_schemes() -> Vec<ThemeScheme> {
    let mut out = vec![default_scheme(), light_scheme()];
    if let Ok(solarized) = solarized_dark_scheme() { out.push(solarized); }
    out
}
```

То есть **только Default (dark Catppuccin Mocha), Light (Light C)
и опциональная Solarized dark**. Mocha Mousse встроенной **нет**
— в `theme.toml` владельца стоит `scheme = "Default"` = cool grey.

Это **вторая design-проблема** сессии; не связана с wrap-gap
формально, но **отвечает за «внешний вид» wrap-mode в сравнении
с мокапом**: если убрать wrap-chrome, но палитра останется cool grey
— fingerprint будет холодный, а в мокапе он тёплый. См. §15.

### Severity повышен

В первом прогоне я пометил `frame_wrap_matte gap` как **«вкусовщина»**
(потому что на полноцветных wallpaper маскируется). **Это ошибка
классификации** — на любых контрастных BG (а T309 прямо требует
тестировать именно на них) гэп превращается в разорванную рамку.
Повышаю до **серьёзно для first-touch и серьёзно для showcase**, см.
выше в Executive verdict пункт 2.

И мокап `soramane.mp4` подтверждает эту оценку: **в мокапе wrap=в
невидимом виде**, что должно быть и в живом шелле.

## 15. **Палитра: cool grey vs Mocha Mousse** (по мокапу)

**Эта секция добавлена по результатам разбора `soramane.mp4`.**

### Что в мокапе

Сэмплы top/right/bottom полей мокапа (45 кадров через 1с):

- bg LIGHT: `(252,245,243) #FCF5F3` — warm cream, **Mocha Mousse Pantone 17-1230**.
- bg DARK: `(18,17,22) #121116` — почти чёрный, без синего.
- accent LIGHT-bottom: `(124,68,60) ➝ (240,232,219)` — терракотовая
  полоса с затуханием; внутри сцены — `(255,224,221)`, `(245,209,191)`,
  розовый `(248,230,227)`.
- accent DARK-right: `(234,79,132) #EA4F84` — яркий розовый
  индикатор; внутренние — `(67,57,102) #433966` фиолетовый,
  `(28,17,20)` очень тёмный.

Эта палитра **явно тёплая, землистая, с розово-фиолетовыми акцентами**.
**Не Catppuccin**.

### Что в живом шелле

`sources of truth`:

- `crates/ui/src/theme/schemes.rs:30-44` `DEFAULT_BASE16` — полная
  палитра Catppuccin Mocha (`#1e1e2e` ... `#cdd6f4` ... `#89b4fa` ...
  `#f38ba8`). Это **холодный cool grey** + синий accent.
- `crates/ui/src/theme/schemes.rs:55` `light_scheme()` = **Light C**
  (`#dde0f2`, `#cdd6f4` again — те же тона). Опять cool grey.
- `crates/ui/src/theme/schemes.rs:152` `builtin_schemes()` — список
  доступных: `[Default, Light, optional Solarized]`. **Mocha Mousse
  отсутствует**.

Текущий конфиг владельца (`~/.config/chronos/theme.toml`):
```toml
blur_enabled = true
scheme = "Default"
```
То есть активен именно **Catppuccin Mocha-style**, не Mocha Mousse.

### Что нужно сделать (TBD для design-полировки)

Сам код `Theme` уже поддерживает произвольные schemes через
`builtin_schemes()`. Добавить Mocha Mousse как четвёртую встроенную:

- **base bg**: `#FCF5F3` (cream warm white) — текстовые цвета
  на нём ставить тёмно-коричневые `#48342B` / `#7A5544`;
- **dark bg**: `#121116` (warm black) — текстовые cream
  `#F2EBE5` / `#D8CAC1`;
- **status** (light → warm tones):
  - warning  `#C46A2B` (тёплый caramel);
  - error    `#B73E2A` (киноварь);
  - success  `#6B7F3C` (olive);
  - info     `#718BA8` (dusty blue);
- **accent**: `#A48765` (Pantone 17-1230, the Mousse itself) или
  контрастный тёмный `#5C4631`.

В dark-e Mocha Mousse **palette colors as fill** + светлый текст —
ровно как в живом Catppuccin Mocha сейчас, но с коричневым акцентом.

### Severity

- **серьёзно для demo/showcase**: пользователь явно вложил усилия в
  мокап с этой палитрой, что показывает его **намерение сделать
  ChronOS узнаваемым через брендовую цветовую гамму**, а не
  использовать кем-то сделанную Catppuccin Mocha. Без Mocha Mousse
  shell остаётся «ещё одним Catppuccin-вариантом», каких уже много.
- для first-touch шлёма не виден (это не блокер), но это **identity**.

### Где живой work

`crates/ui/src/theme/schemes.rs:142-160` (`fn builtin_schemes`) — место
для подключения, плюс новая функция `mocha_mousse_scheme()`. Завести
тест по подобию `light_scheme_uses_light_c_palette` и
`light_scheme_status_is_latte_not_mocha`, который закрепит фразу
«контраст» в Light Mocha Mousse по WCAG 4.5:1 для body-text.

Также требует проверки **поверх всех виджетов**: какие роли
(fg/bg/text/status) пользователь увидит, и где Mocha Mousse лучше
Catppuccin Mocha работает vs лучше ломается. Например, (236,238,250)
в light scheme (cool grey) проходит WCAG-fail для `text.muted` —
а с warm cream палитрой можно перестроить `text.muted` так, чтобы
прошёл.

### Связь с §14

§14 — wrap-gap (структурный дефект). §15 — палитра (визуальный
identity). Вместе — **«wrap-mode должен быть невидимым + фон должен
быть тёплым»** — это **две стороны одного продукта**, который
пользователь хочет показать. Мокап иллюстрирует обе одновременно.
Если архитектор хочет **постепенного cut-over**, имеет смысл
зафиксировать:
1. Эпик-план fix wrap-mode (wrap=невидимо) — это отдельная задача
   `frame.rs` + `side_panel_*`, делается в одном эпике.
2. Затем эпик-palette Mocha Mousse — добавить новую схему + перевести
   на неё `theme.toml` по умолчанию (или сделать deprecation
   прослойкой).

## Кросс-секционные находки

1. **(side_panel_right_content transparency)** — см. §4. Приоритет 1.
   Конкретное содержимое панели НЕ закрыто фоном, и это видно в
   каждой теме независимо от того, что выбрано внутри.

2. **(light muted text fails WCAG)** — `text.muted #7d80a6 (125,128,166)`
   на `bg.primary #dde0f2 (221,224,242)`: CR=2.91. Это ниже
   WCAG 2.0 large-only (3.0), не говоря уже normal-text (4.5). В dark
   та же роль даёт CR=3.36 — на грани. **Бар смотрится приемлемо в
   обеих темах только потому что muted используется в крупных
   элементах**.

3. **(bar right-cluster density)** — 10 widgets в ~400px полосе на
   высоте 30px. Не blocker, но «плотно». Один из них
   (`workspace_mode`) вообще не регистрируется как видимый.

4. **(frame_wrap_matte gap)** — 24px виден wallpaper по обеим сторонам
   от panel rails (`x=16..40` и `x=2520..2544`). Это не «дизайн-плановый»
   gap, а **незаполненный стык двух независимых layer-shell окон**
   (matte в одном окне, rail+content в другом). На экстремальных BG
   смотрится как **разорванная рамка**. Полный разбор и предложение
   fix в §14 — **повышен с вкусовщины до серьёзного по архитекторскому
   фидбеку**.

5. **(dock broken pinned apps)** — три ожидаемые `.desktop` (firefox,
   code, vivaldi) не резолвятся; визуальный кейс — пустое место слева
   в баре. См. §10.

6. **(Light OSD слабо различим на чёрном)** — `osd=bg.elevated (lit)` на
   desktop=000000 → едва видимый. Применяется только в экстремальной
   обоине, у пользователя с дефолтным wallpaper, наверное, нормально.

## Что я НЕ делал

- Ни одной строки в `crates/`, ни в config (`~/.config/chronos/*.toml`),
  ни в theme, ни в wallpaper service.
- Не запускал тесты / cargo build — это не входит в тикет. Бинарь
  уже существовал на момент прогона.
- Не сравнивал производительность / FPS.
- Не редактировал отчёты других — старый черновик в `reports-fresh`
  перезаписан моим с нуля (по инструкции юзера «hes report rejected-
  you do one from scratch»), без правки дерева.

## Финальный продуктовый вердикт

**Готовность для r/unixporn-аудитории: 6.5/10 для dark (только после fix
правой панели), 4.5/10 для light (muted-text-fail + OSD-visibility-on-dark).**

**Что реально хорошо:**
- IPC-команды работают и хорошо протоколируются.
- Геометрия layer-shell стабильна по всем переключениям.
- Темы меняются чётно (Default↔Light) и реально различаются.
- Notifications toast — production quality.
- Левая панель хорошо держит сетку (если только не считать
  dock-pinned-apps).

**Что тянет вниз больше всего:**
1. **Правая панель transparent-on-default** — главный визуальный bug,
   пользователь видит «стекло» вместо панели. (Серьёзно.)
2. **`frame_wrap_matte` гэп между рамкой и рейлом панели** —
   виден буквально на любых wallpaper с предельным контрастом
   (правило T309); разваливает целостность «wrap»-стиля. (Серьёзно
   сейчас, повышен с «вкусовщины» после пользовательского фидбека.)
3. **Light muted text WCAG-fail** — подписи в карточках нечитаемы
   на bg.primary. (Серьёзно.)
4. **Dock с тремя неразрешёнными pinned apps** — выглядит как
   «недоделали». (Серьёзно для demo.)
5. **Геометрия «10 виджетов в 30px полосе»** — шумная правая часть
   бара; tap-target азартно проверить мышкой. (Среднее.)

**Скрытый функциональный bug нашёл в ходе прогона:**
`workspace_mode` IPC работает, но в `bar.toml` нет такого виджета — UI
не рендерит переключение. Пользователь думает, что фича сломана.

**Что НЕ баг и НЕ находка, но стоит протоколировать:**
synthetic-click на bar-clock / bar-bell / bar-updates не открывает
popups (calendar, notifications-history, updates-popup) из-за
известного layer-shell-focus предела в песочнице. Это environment,
не продукт. Из двух QA-прогонов (предыдущий отклонённый и мой
текущий) одинаково воспроизводится как "не классифицируется".

**Одной фразой: shell собран, не собран полировкой, и **три** двери
открыты в пустоту: правая панель transparent-on-default, **wrap↔rail
гэп в 24 пикселя проступающего wallpaper'а** (повышен с вкусовщины
до серьёзного после пользовательского фидбека), WCAG-fail на
light muted. До релиза все три должны закрыться.

И **два** дизайн-intent пункта зафиксированы мокапом
`soramane.mp4`: wrap-mode без видимой рамки (визуально =
hide-mode для пользователя) + новая палитра **Mocha Mousse**
Pantone 17-1230 вместо Catppuccin cool grey. См. §14 и §15.

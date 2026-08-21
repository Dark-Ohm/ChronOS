# T328 — рама, схемы, alpha/blur, обои. Отчёт

**Оболочка и тема продаются? — НЕТ.**

Рама сделана хорошо: апертура в `wrapped` держит радиус, стыки чистые, на
белом и на тёмном столе. Продажу валит не она. Валит всё, что вокруг темы:
одна из четырёх схем в пикере нечитаема, `surface_alpha` красит ровно один
элемент из пяти, blur — мёртвая серая кнопка, а подсистема обоев устроена
так, что оболочка при каждом старте сносит стол пользователя и не ставит
ничего взамен. Покупатель за $100 получает шелл, который на первом же
запуске съедает его обои.

- **Улики:** `.chronos-ops/dump/qa-ux/T328/` — `frames/` 36, `crops/` 42,
  `log/chronos.log` 502 строки, `config-backup/` 11 tomlов + SHA256
  BEFORE/AFTER, `fixtures/white.png`, `fixtures/black.png`.
- **Прогон:** release-бинарь `target/release/chronos` (20 авг 23:59),
  `RUST_LOG=info`, три перезапуска шелла за заход (все в логе).
- **Код продукта не тронут:** `git status --porcelain crates/ Source/
  packaging/` — пусто.

---

## Состояние стенда на входе (я это застал, я этого не делал)

Обоев на машине не было **до** начала аудита:

- `awww query` → `failed to connect to socket: Connection refused`,
  `awww-daemon` мёртв, сокет `wayland-1-awww-daemon.sock` от 11:23 висит.
- `waytrogen` — `<defunct>`, `mpvpaper` не запущен.
- `hyprctl layers`: `Layer level 0 (background)` **пуст на обоих мониторах**
  (`log/layers-before.txt`).
- На столе — дефолтные обои Hyprland (`force_default_wallpaper = -1`,
  `10-look.lua:20`) со сплэшем «How do I exit vim????».
  Кадр `frames/00-desk-before.png`, пробы (300,300) `#0B0F26`.

Записано в `wallpaper-before.txt`. Это же состояние — на выходе, см. «Стол».

**Загрязнение по ходу:** в 14:00:30 местного `bar.toml` `appearance.height`
переписался **20.0 → 28.39999771118164** (ровно `HEIGHT_MIN + 0.14*(HEIGHT_MAX
- HEIGHT_MIN)`, т.е. драг ползунка Height в открытой вкладке System
settings — `bar_settings.rs:158-166`). Ни один мой клик в тот момент не шёл
в ту область (все координаты в логе сессии). Скорее всего — твоя рука.
Я восстановил файл из бэкапа дословно; если 28.4 была осознанная правка —
верни её сам.

---

## Блокеры

### B1 — оболочка убивает стол пользователя и не ставит свой

Цепочка воспроизведена целиком, живьём:

1. ChronOS на каждом старте поднимает свой бэкенд:
   `chronos_services::wallpaper: WallpaperSubscriber: starting awww-daemon`
   (`log/chronos.log`, каждый из трёх запусков).
2. Обои при этом **не восстанавливаются никакие** — ни последние, ни
   дефолтные. `awww query` сразу после старта: `currently displaying:
   color: 000000`, слои `awww-daemon` с `a: 0`. Стол проваливается в
   дефолт Hyprland.
3. Единственный штатный способ поставить обои из шелла — кнопка
   **Open waytrogen** во вкладке Display. Она запускает чужое приложение,
   которое при применении `mpvpaper`-обоев **убивает `awww-daemon`**:
   после клика — `614346 [awww-daemon] <defunct>`, `657919 mpvpaper …`.
4. С этого момента управление обоями в шелле мертво до перезапуска:
   `chronos-ipc wallpaper-refresh` →
   `WARN WallpaperSubscriber::refresh failed: `awww query` failed: Error:
   "failed to connect to socket: Connection refused (os error 111)"`.
   Единственный след — строка в логе; в UI карточка Display продолжает
   показывать старый путь.
5. Обратное направление тоже проверено: после `waytrogen --restore`
   (`mpvpaper` жив, `hyprctl layers` → `namespace: mpvpaper` на обоих
   выходах, проба (300,300) `#0D263D`) перезапуск шелла поднимает
   `awww-daemon` — и `mpvpaper` умирает, стол снова дефолтный Hyprland.

Кадры: `frames/18-gallery-open.png`, `frames/20-gallery-applied.png`,
`frames/24-wallpaper-restored.png`, `frames/35-final-desk.png`,
`frames/36-final-desk-restored.png`.

Два демона обоев дерутся за background-слой, и ChronOS — один из бойцов,
который при этом сам обои ставить не умеет. Именно так стол и оказался
голым **до** моего захода.

### B2 — «Next» — мёртвая кнопка, отказ виден только в логе

В `~/Pictures/Wallpapers` **34 файла, все `.mp4`**, ни одной картинки.
`wallpaper_ctl` сканирует только изображения (`is_image`).

- `chronos-ipc wallpaper-next` →
  `WARN wallpaper_ctl: no wallpapers found in ~/Pictures/Wallpapers`.
- Кнопка **Next** во вкладке Display (`display.rs:445`) →
  `INFO wallpaper_card: Next clicked` + тот же WARN.

В интерфейсе — **ничего**. Ни тоста, ни надписи в карточке, ни смены
подписи (`fixtures/white.png` как стояла, так и стоит). Кадры
`crops/17a-next-hover.png` (наведение) и `crops/17b-after-next.png`
(после клика) отличаются только подсветкой кнопки.

Владелец шелла у себя на машине жмёт кнопку «следующие обои» и не получает
ни обоев, ни объяснения. Формально «в папке нет картинок» — но папка
называется Wallpapers и в ней 34 обоины.

### B3 — Solarized Dark: подписи выделенных чипов нечитаемы (1.19:1)

Схема из штатного пикера. В System settings после переключения:

| элемент | заливка | глиф | контраст |
|---|---|---|---|
| чип «Top» (выбран) | `#5B7D90` | `#268BD2` | **1.19 : 1** |
| он же, Default | `#293D5A` | `#007ACC` | 2.44 : 1 |
| он же, Light | `#BCD2EE` | `#007ACC` | 2.92 : 1 |
| он же, Mocha Mousse | `#403538` | `#A47864` | 3.05 : 1 |

Замеры — гистограммы по одному и тому же прямоугольнику `120x34+1905+336`
на четырёх кадрах (`frames/06,10,11,12`), формула WCAG 2.1.

В Solarized Dark одновременно теряются: `Top`, `Full`, `Soft`, `on`,
`Wrapped`, `no module` и подпись самой выбранной карточки «Solarized Dark».
Кадр 1:1 — `crops/11f-solarized-appearance-1to1.png`, зум ×5 —
`crops/11c-solarized-toppill-5x.png`.

Паттерн «акцентный текст на 20%-акцентной заливке» не проходит 4.5:1
**ни в одной** схеме; в Solarized Dark он проваливается до неразличимости.
T317 мерил контраст на Default — на остальных трёх схемах это не проверялось.

### B4 — blur: мёртвый контрол, конфиг врёт

- `theme.toml`: `blur_enabled = true`.
- Лог на каждом старте: `chronos::surface_effects: surface_effects: blur
  bridge not available, persisted state untouched capability=ModuleMissing`.
- UI (`crops/07-appearance-blur-row.png`): строка **Blur**, подпись
  `import 45-surface-effects-chrono…` (обрезана) и **выключенная серая
  кнопка «no module»**.
- Модуль лежит в репозитории — `packaging/hyprland/45-surface-effects-chronos.lua`,
  а в `~/.config/hypr/modules/` его нет (00,05,10,15,20,25,30,40 — и всё).

То есть фича заявлена в конфиге как включённая, в интерфейсе показана как
недоступная, а положить недостающий файл шелл не предлагает и не умеет —
кнопка не кликается, подпись обрезана и без пути. Проверить blur вкл/выкл
на этой машине физически нечем: разницы нет и быть не может.

### B5 — `surface_alpha` красит только бар (гипотеза T323 подтверждена)

Mocha Mousse, `surface_alpha = 0.7`, белые обои
(`frames/13-mocha-alpha07-white.png`, `frames/14-startmenu-alpha07.png`):

| поверхность | проба | цвет при 0.7 | цвет при 1.0 |
|---|---|---|---|
| бар | (300,10) | `#49464A` | `#18141A` |
| левый рельс | (8,700) | `#18141A` | `#18141A` |
| правый рельс | (2552,700) | `#18141A` | `#18141A` |
| нижняя планка | (1000,1432) | `#18141A` | `#18141A` |
| правая панель | (1800,800) | `#2D2830` | `#2D2830` |
| Start Menu, фон | (600,500) | `#241F23` | — |
| Start Menu, сайдбар | (60,430) | `#18141A` | — |

Полупрозрачный бар, приваренный к абсолютно глухой раме того же цвета, —
это не «прозрачность», это дефект. Ползунок Surface opacity стоит в
Appearance рядом с Height и Radius и выглядит как общая настройка
поверхностей; по факту он управляет одной поверхностью из шести.

Побочно: при alpha < 1 и открытом Start Menu полоса y=20..27 под баром
остаётся глухой `#18141A` — бар просвечивает, а полоса под ним нет.

---

## Находки (не блокеры)

**F1. Бар теряет нижнюю границу после любого живого изменения высоты
(`normal`).** Свежий старт, `style = "normal"`, Default, height 20:
y=19 = `#45475A` (`border_b_1()` + `theme.border.subtle`,
`bar/mod.rs:142-147`). Дальше `height 20 → 24 → 20` через hot-reload —
и y=19 = `#18141A`, граница исчезла по всей ширине (пробы x=300/800/2000),
дальнейшие перезагрузки её не возвращают, помогает только перезапуск.
Кадры `frames/29-normal-real-freshboot.png` (есть) против
`frames/33-normal-after-height-roundtrip.png` (нет), зумы
`crops/29a-bar-border-3x.png` / `crops/33a-bar-noborder-3x.png`.
Round-trip самой рамы (`wrapped→normal→wrapped`), схемы и alpha границу
**не** ломают — ломает только правка высоты. То есть штатный ползунок
Height в настройках необратимо портит отрисовку бара.

**F2. Спор про «1px seam под баром» закрыт: это не баг, это канон.**
`normal` на белом: y=19 `#45475A`, y=20 `#FFFFFF` —
`frames/03-normal-white.png`. `wrapped` на белом: y=19 `#181825`,
y=20 `#FFFFFF`, шва нет — `frames/02-wrapped-white.png`. Код прямо это и
делает: в wrap-раме бордер бара снимается как «режущий замкнутый контур»
(`bar/mod.rs:138-147`, T284 §5.3). Оба отчёта T323 были правы — каждый про
свой режим. С учётом F1 добавляется третье состояние: `normal` **без**
границы после правки высоты.

**F3. Light + светлые обои = рамы нет.** Wrapped, Light, белый стол: бар
`#ECEEFA` против обоев `#FFFFFF` — граница около 1.06:1, рельсы того же
тона, бордер в wrapped снят по канону. Оболочка растворяется целиком,
остаётся висящий в воздухе текст. `crops/10a-light-white-bar.png`.
На тёмных обоях Light выглядит нормально — но пикер про это не
предупреждает.

**F4. `[bottom_strip]` в `frame.toml` — молчаливый no-op в `normal`.**
`enabled = true, height = 4.0` — в `normal` ни планки, ни слоя
(`hyprctl layers`: только `bar` и `side_panel_hover_strip`), обои доходят
до низа (пробы y=1420..1439 = `#FFFFFF`). Секция остаётся в файле и ничего
не делает; в UI её состояние никак не отражено.

**F5. Галерея — чужое приложение без единой точки соприкосновения.**
`Open waytrogen` открывает Iced-окно на весь экран: свои виджеты, свои
синие кнопки, свои комбобоксы, своя типографика, ноль темы ChronOS
(`crops/18a-gallery-55.png`). Плюс: шелл спавнит её синхронно
(`wallpaper_ctl: opened waytrogen gallery` — ветка `open_waytrogen_gallery`,
не `_async`), поэтому после закрытия галереи **автоматического
`refresh` не происходит** — в логе после `pkill -x waytrogen` нет ни одной
строки wallpaper. Карточка Display остаётся врать до ручного
`chronos-ipc wallpaper-refresh`, о котором покупатель не знает.

**F6. Вкладка Display почти пустая.** 440 px ширины на «Brightness» и
карточку «Wallpapers» с двумя кнопками; ниже ~1200 px пустоты
(`crops/16a-tab-1250.png`).

---

## Что хорошо

- **Апертура в `wrapped`.** Внутренние радиусы одинаковые на всех четырёх
  углах, рельсы 16 px, стык бар/рельс без щели, обои обрезаны по радиусу
  чисто — на белом это видно идеально: `crops/02a-wrapped-white-TL-4x.png`,
  `crops/02b-wrapped-white-BR-4x.png`. Это лучшая часть продукта.
- **Hot-reload темы и рамы работает молча и без артефактов.** Схема
  меняется за ~300 мс (`theme: selected` → `theme: hot-reloaded`), рама —
  с корректным закрытием четырёх wrap-поверхностей
  (`frame: closed wrap surface Matte/ExclLeft/ExclRight/ExclBottom`).
  Ни одной потери слоя, ни одного protocol error за весь заход.
- **Пикер схем честно показывает палитру** — по пять образцов цвета на
  карточку, выбранная обведена акцентом. Mocha Mousse и Light выглядят
  как законченные темы.
- **Блок About не врёт**: `0.1.0`, `Apache-2.0`, «offline by design ·
  no network · no telemetry».
- **Blur-кнопка честно disabled** вместо тумблера, который делает вид, что
  что-то включил. Претензия в B4 не к честности, а к тупику.

---

## Гипотезы T323 — вердикты

| гипотеза | вердикт |
|---|---|
| `surface_alpha` красит бар, а Start Menu и кольцо рамы — сырой `bg.*` | **подтверждена**, B5, замеры по шести поверхностям |
| 1px seam под баром в `normal` на белом («есть»/«нет») | **закрыта**: в `normal` есть по канону, в `wrapped` снят по канону; третье состояние — F1 |
| `waytrogen --restore` убивал `awww-daemon` | **подтверждена и расширена**: убивает не `--restore`, а применение mpvpaper-обоев (в т.ч. из галереи, открытой кнопкой шелла); обратно `awww-daemon` со старта шелла убивает `mpvpaper` — B1 |

---

## Стол на выходе

Честно: **стол не восстановлен**, и это не в моей власти.

`waytrogen --restore` отработал дважды (`log/waytrogen-restore.log`,
`waytrogen-restore2.log`): `mpvpaper` поднимался, слой `namespace: mpvpaper`
появлялся на обоих выходах, проба (300,300) давала кадр видео (`#0D263D`).
Через несколько секунд `mpvpaper` умирает сам — процесса нет, background-слой
снова пуст, на столе дефолт Hyprland. Командная строка, которую строит
waytrogen, выглядит битой: `mpvpaper -o  --auto-pause * /home/neo/Pictures/
Wallpapers/kafka-by-moonlight.3840x2160.mp4 -f` (пустой `-o`, голая `*`).

Итог: **состояние стола на выходе идентично состоянию на входе** — дефолтные
обои Hyprland, `awww-daemon`/`mpvpaper` не живут. Чёрного экрана нет.
Если хочешь видео обратно — подними waytrogen руками и выбери
`kafka-by-moonlight`, у себя из GUI оно живёт дольше, чем из моего
`setsid`.

---

## Конфиги: до / после

Дословно, из `config-backup/`:

```toml
# frame.toml (до = после)
style = "wrapped"

[bottom_strip]
enabled = true
height = 4.0
junction = "break"

# theme.toml (до = после)
blur_enabled = true
scheme = "Default"
surface_alpha = 1.0
```

Трогал за заход: `frame.toml` (`style` — 6 раз туда-обратно),
`theme.toml` (`scheme` через пикер: Default→Light→Solarized Dark→Mocha
Mousse→Default; `surface_alpha` 1.0↔0.7↔0.95), `bar.toml`
(`appearance.height` — восстановление после чужой правки и два
round-trip'а под F1). Всё возвращено копированием из бэкапа.

```
$ sha256sum ~/.config/chronos/*.toml | diff - SHA256-BEFORE.txt
CONFIG SHA: MATCH
```

**Все 11 хэшей совпадают с бэкапом.**

Состояние оболочки на сдаче: `style = "wrapped"`, scheme `Default`,
`surface_alpha = 1.0`, `blur_enabled = true` (как было), панели закрыты
(`hyprctl layers`: `bar`, `frame_wrap_matte`, три `frame_wrap_excl_*`,
`side_panel_hover_strip` — и ничего больше), шелл перезапущен на
восстановленных конфигах.

---

## Проверяемые числа

```
кадров:                 36   (dump/qa-ux/T328/frames/)
кропов:                 42   (dump/qa-ux/T328/crops/)
лог:                   502 строки
grep -cE "panic|Protocol error"   0
grep -c "ERROR"                   0
конфигов сверено:       11/11 sha256 = backup
схем проверено:          4/4 через пикер
режимов рамы:            2/2 (normal, wrapped) × белый стол + живой стол
IPC обоев:               3/3 (next, refresh, gallery)
git status crates/ Source/ packaging/   пусто
```

---

## Вынести из сферы

- Дефолтный пресет «Top full» в Presets: подпись рвётся как
  `Full-width bar on top (default \n )` — одинокая скобка на строке.
  Правая панель, T327 закрыт.
- `side_panel_right_content` не следует за высотой бара: при height 28.4
  рельс уехал на y=28, контент остался на y=20. Уже известно по T327.
- Правая панель, открытая по `toggle-side-panel-right` на свежем старте,
  показывает **пустое** содержимое (лог: `lazy-create tab view tab="System"`),
  до первого клика по рельсу — `crops/05-right-panel.png`. T327.
- `hyprctl dispatch` на Lua-Hyprland 0.56.2 больше не принимает строковые
  диспетчеры (`hl.dsp.workspace.focus` нет). Сырой сокет ChronOS
  (`switchxkblayout all next`, `dispatch workspace N`) продолжает работать.
  Кровный факт для будущих смоков, не находка продукта.


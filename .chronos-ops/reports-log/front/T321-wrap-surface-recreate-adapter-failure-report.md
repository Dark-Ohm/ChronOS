# T321 — wrap-поверхность падает на адаптере: отчёт исполнителя (FRONTEND)

**Коммит:** не сделан — правки в рабочем дереве для приёмки (я не
самокоммичу). Бинарь release пересобран и запущен (pid в конце).

**Зона по брифу:** `crates/app/src/frame.rs`. Затронут только он.

## Итог коротко

Падение **воспроизведено** — но не так, как написано в брифе. Оно не
«wrap-поверхность», а **глобальный отказ адаптера**: при открытых обеих
боковых панелях быстрый рекрейт поверхностей роняет не только wrap, но и
bar, панели, hide-полосу. Корень — гонка жизненного цикла поверхностей в
форке (`check_compatible_with_surface` отдаёт пустые форматы, дальше флуд
`Protocol error on wp_viewport`). В `frame.rs` починил то, что лечится из
`frame.rs`: **геометрия теперь мутируется живьём, без рекрейта** (тот самый
штатный путь «владелец крутит толщины/радиус живьём»). Сам style-переход
`hide↔wrap` при открытых панелях остаётся гонкой — это баг форка + каскад
панелей, вне зоны брифа. Детали ниже.

## Воспроизведение (дословный лог)

Не воспроизводится при закрытых панелях (20 переходов на шаге 400 мс —
0 ошибок). Воспроизводится **с открытыми обеими панелями** — панели через
`apply_frame_inset` закрывают+переоткрывают по 2 поверхности на каждый
style-переход, и каскад `frame(4+4) + panels(2+2+2+2)` за ~секунду роняет
адаптер. Первый отказ:

```
frame: failed to open bottom strip: Adapter "NVIDIA GeForce RTX 3070" (backend=Vulkan, device=0x2484) is not compatible with the display surface for this window.
frame: failed to open bottom strip: Adapter "NVIDIA GeForce RTX 3070" (backend=Vulkan, device=0x2484) is not compatible with the display surface for this window.
side_panel_left: content surface failed to open: Adapter "NVIDIA GeForce RTX 3070" ... not compatible ...
side_panel_right: content surface failed to open: Adapter "NVIDIA GeForce RTX 3070" ... not compatible ...
frame: wrap surface Matte failed to open: Adapter "NVIDIA GeForce RTX 3070" ... not compatible ...
```

Дальше флуд `Protocol error 1 on object wp_viewport@1702` (счётчик — до
миллионов за секунды), все слои теряются (`hyprctl layers` — только
awww-daemon). Shell-процесс жив, поверхностей нет.

Дословный лог: `/tmp/t321/style20p.log` (20 переходов шаг 400 мс, панели
открыты: `failed to open` = 14, `not compatible` = 14, `Protocol error` =
5 343 366), `/tmp/t321/recipe100.log` (рецепт брифа 100 мс, панели открыты:
`Protocol error` = 1 956 205, `frame_wrap` живых = 0).

Важно: падает **не ExclBottom** (как в брифе), а первым же открытием после
накопления каскада; порядок ролей в логе случайный. Гипотеза «всегда
ExclBottom» не подтвердилась.

## Корень (факт, не гипотеза)

Ошибка «Adapter … not compatible with the display surface for this window»
приходит из `gpui_wgpu/src/wgpu_context.rs:194` (`check_compatible_with_surface`):
`surface.get_capabilities(&adapter)` возвращает пустой `formats`. То есть
свежесозданная wgpu-поверхность при быстром рекрейте не проходит проверку
совместимости с общим адаптером. Сопутствующий `Protocol error 1 on object
wp_viewport` — это wayland-объект `wp_viewport`, к которому форк обращается
после уничтожения его поверхности. Это гонка жизненного цикла поверхностей
в форке (`Source/gpui`), а не логика `frame.rs`. Гипотеза «исчерпание
адаптера» не подтвердилась — это не лимит, а гонка close/open.

## Что сделано в `frame.rs`

1. **Геометрия мутируется живьём, рекрейт убран** (`sync_wrap_excl_zones` →
   `sync_wrap_surfaces`, `frame.rs:1152`). Раньше любая правка
   `wrap.left/right/bottom/inner_radius` делала `close_wrap_windows` +
   `open_wrap_windows`. Теперь у полос есть живые сеттеры — `window.resize`
   (тем же сеттером hide-полоса уже меняет высоту) и `set_exclusive_zone`
   (механика T314) — а матте просто перерисовывается (`cx.notify`, рендер
   читает `cached_config()`). Полосы ничего не красят и не берут ввод, их
   футпринт и резервация описываются этими двумя значениями.
2. **`apply_wrap` открывает wrap-набор ДО сноса hide-полосы** (`frame.rs:1255`).
   При неудачном открытии hide-полоса остаётся на месте — «честный откат в
   предыдущий стиль», а не frame-less полу-состояние (п. 4 брифа).
   `open_wrap_windows` теперь возвращает `bool` (`frame.rs:1035`).
3. **Ошибка не глушится**: `failed to open` логируется как было + новый
   `wrap open failed — previous frame style kept`.
4. `LAST_WRAP_GEOMETRY`/`LAST_RAIL_MAPPING` остались как сигналы «нужен ли
   живой sync» — чтобы не спамить `set_exclusive_zone` на каждый apply.

## Верификация

### Юнит
`cargo check -p chronos --bins` → ok (только pre-existing warnings).
`cargo test -p chronos --lib` → **609 passed; 0 failed**.

### Живой — геометрия (главный фикс), release-бинарь

Старт с дефолтом (16px), правка `[wrap] left=32 right=32 bottom=20
inner_radius=8`:

```
frame: ExclLeft geometry synced zone=32
frame: ExclRight geometry synced zone=32
frame: ExclBottom geometry synced zone=20
```

Дословный промер слоёв после правки (рекрейта НЕТ — те же pid, ни одного
`closed wrap surface`):

```
frame_wrap_excl_left   xywh: 0 21 32 1440   (было 16)
frame_wrap_excl_right  xywh: 2528 21 32 1440 (было 16)
frame_wrap_excl_bottom xywh: 0 1420 2560 20  (было 16)
```

До фикса этот же промер давал 4× `closed wrap surface` + пересоздание.
`Protocol error` при правке геометрии — 0.

### Живой — style-переход при закрытых панелях

20 переходов hide↔wrap (шаг 400 мс, панели закрыты): `failed to open` = 0,
`Protocol error` = 0, все 4 `frame_wrap_*` живы. Рекрейт здесь остаётся
(поверхности hide и wrap — разные наборы), но не роняет адаптер.

## Что НЕ сделано / осталось

- **style-переход при открытых панелях по-прежнему роняет адаптер.** Это
  гонка форка (`check_compatible_with_surface` + `wp_viewport`), которую
  дёргает каскад `frame` + `apply_frame_inset` (панели). `frame.rs` этого
  не лечит: панели вне зоны брифа, а баг живёт в `Source/gpui`. Нужен
  тикет на форк (сериализация close→open или переиспользование поверхностей
  в wayland-бэкенде), либо на панели (убрать рекрейт `apply_frame_inset` в
  пользу живого инсета — там margin/height запекается так же, как раньше
  запекалась геометрия frame).
- Гипотеза «всегда ExclBottom» из брифа опровергнута — падает глобально.
- `apply_hide` (wrap→hide) не переставлен «open-до-close» — hide-полоса
  открывается простым `open()` и редко падает; для симметрии можно, но риск
  выше пользы, оставил.

## Состояние окружения

`~/.config/chronos/frame.toml` возвращён к застанному (`style = "wrap"` +
`[bottom_strip]`, без `[wrap]`). Shell перезапущен на бинаре с правкой
(pid 3852344): 4 `frame_wrap_*` + bar на месте, `Protocol error` в свежем
логе = 0.

---

# ПРИЁМКА АРХИТЕКТОРА — ПРИНЯТ С ПЕРВОГО ЗАХОДА (2026-08-19)

Код `bbf61f02` (`frame.rs`, +83/−50).

## Главный фикс проверен архитектором живьём

Правка `[wrap] left=32 right=32 bottom=20 inner_radius=8` на поднятом
шелле:

```
до:    excl_left w=16  excl_right w=16  excl_bottom y=1424 (16)
после: excl_left w=32  excl_right w=32  excl_bottom y=1420 (20)

closed wrap surface = 0     geometry synced = 3     Protocol error = 0
```

Pid не менялся. Геометрия мутируется на живых поверхностях, рекрейта нет
— ровно то, ради чего тикет и заводился, и ровно тот путь, которым уже
дважды лечилось это же семейство (T314, теперь T321).

## Корень подтверждён по исходнику форка

`Source/gpui_wgpu/src/wgpu_context.rs:194-200`,
`check_compatible_with_surface`: `surface.get_capabilities(&adapter)` →
`caps.formats.is_empty()` → ровно та строка про адаптер, что в логах.
Это гонка жизненного цикла поверхностей в форке, а не логика
приложения. Заявлено фактом со ссылкой, а не гипотезой — как и просил
бриф.

## Две гипотезы брифа опровергнуты — правильно

- «падает всегда `ExclBottom`» — нет, падает первое же открытие после
  накопления каскада, порядок ролей в логе случайный;
- «исчерпание адаптера» — нет, это не лимит, а гонка close/open.

Плюс уточнена сама постановка: отказ не «wrap-поверхности», а
глобальный — валятся и bar, и панели, и hide-полоса.

## Границы соблюдены

Исполнитель починил то, что лечится из `frame.rs`, и не полез в панели
и в форк, хотя корень там. Заодно сделал сверх брифа и по делу:
`apply_wrap` теперь открывает wrap-набор ДО сноса hide-полосы, а
`open_wrap_windows` возвращает `bool` — при неудаче остаётся предыдущий
стиль, а не frame-less полу-состояние (пункт 4 брифа про «не оставлять
полуживым»).

`cargo test -p chronos --lib` 609/609 прогнан архитектором, конфиг
владельца возвращён к застанному дословно.

## Остаток вынесен в T322

Style-переход при ОТКРЫТЫХ панелях по-прежнему роняет адаптер: каждая
панель через `apply_frame_inset` закрывает и переоткрывает по две
поверхности, и каскад `4 + 4 + 2×2 + 2×2` за секунду дёргает гонку
форка. Замеры исполнителя: 14 `failed to open` и **5 343 366**
`Protocol error` за 20 переходов; при закрытых панелях — ноль.

**T322** требует от панелей того же приёма: менять инсет живьём, а не
пересозданием. Отдельным пунктом — сверить с исходником форка, есть ли
живой сеттер для `margin` (в T307 зафиксировано, что геометрия
запекается в `window.open`), и если нет — сериализовать каскад вместо
одновременного рекрейта.

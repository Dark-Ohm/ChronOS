Попапы ведут себя как у платного шелла? НЕТ.

# T325 — попапы, оверлеи, start, launcher, click-catcher

## Итог

Живой аудит выполнен на release-бинарнике `target/release/chronos`, PID
`193962`, с `RUST_LOG=info`. Код продукта не менял. Снято 30 кадров в
`.chronos-ops/dump/qa-ux/T325/frames/`; append-only release log скопирован в
`.chronos-ops/dump/qa-ux/T325/log/chronos.log`, начало текущего среза указано
ниже.

Отдельные поверхности в основном отрисовываются и работают: Launcher и Start
ищут приложения, toast и OSD появляются, tray/app context menu открываются,
shared click-catcher закрывает Start и контекстное меню. До поведения платного
шелла не дотягивает общий lifecycle anchored popup: Sound и Calendar не имеют
click-away и остаются висеть поверх других режимов.

## Блокер

### B1 — anchored popup не закрываются кликом мимо и складываются друг с другом

- Sound → Calendar оставляет открытыми обе поверхности:
  `frames/14-volume-then-calendar.png`.
- Подтверждённый левый клик в `(1400,100)` вне обеих поверхностей не закрывает
  ни одну: `frames/15-volume-calendar-outside-click.png`.
- Sound также остаётся при открытии Start и edit-mode:
  `frames/16-volume-then-start.png`, `frames/18-volume-then-edit-mode.png`.
- Calendar в изоляции также переживает левый клик мимо `(1400,100)`:
  `frames/30-calendar-alone.png`, `frames/31-calendar-outside-click-stays.png`.
- Статика совпадает с живым симптомом: `volume_popup/mod.rs:131` и
  `calendar_popup/mod.rs:101` задают `grab: false`; среди найденных
  употреблений `popup_click_catcher` нет ни `volume_popup`, ни
  `calendar_popup`. У обоих
  остаётся только явный toggle/close.

Пользователь получает несколько независимых плавающих карточек без единого
правила dismiss/singleton. Это не косметика: привычный клик по рабочему столу
не возвращает чистое состояние.

## Покрытие

| Поверхность | Как открыта / проверена | Результат и улика |
|---|---|---|
| IPC `ping` | `chronos-ipc ping` | В логе `IPC ping received` и `Received ping from a secondary instance`. |
| Launcher | IPC `toggle-launcher`, ввод `vivaldi`, Escape | Окно `chronos-launcher` 720×560 в `(920,442)`; поиск фильтрует, Escape удаляет client: `01`, `04`–`06`. |
| Launcher app context | Right click по Podman Desktop | Меню Launch/Favorite/Pin/Hide/Properties открылось; click-away закрыл только меню, Launcher остался: `27`, `28`. |
| Start menu | IPC `toggle-start-menu`, клик в search, ввод `vivaldi` | Layer `chronos-start-menu` `(0,20) 720×520`; поиск работает: `07`, `29`. |
| Start click-catcher | Левый клик `(1400,100)` | `chronos-popup-click-catcher` и Start исчезли: `10`; точные layer snapshots в `log/layers-10-start-outside-left-click.txt`. |
| Bar edit-mode | IPC `toggle-edit-mode` дважды | Контролы перестановки появились внутри 20px бара и снялись: `11`; coexistence с Sound — `18`. |
| Sound | Левый клик по volume `(2120,10)` | Карточка целая, footer не обрезан, output/mic controls видны: `13`; close X работает (`17`). Lifecycle — B1. |
| Calendar | Левый клик по clock `(2400,10)` | Открывается и календарная сетка читается: `30`; заливки нет — это уже T329, новую находку не заявляю. Lifecycle — B1. |
| Updates | Левый клик по updates `(2336,10)` | Открывается list в правой панели, 23 строки и `Upgrade all` видимы: `19`; отдельного popup layer нет. |
| Notification toast | `notify-send 'ChronOS T325' 'toast'` | Layer `notifications` `(2188,32) 340×480`, карточка toast видима: `21`. |
| Notification history | Левый клик bell `(2302,10)` | Отдельной history-поверхности нет; открывается правая панель Notifications с сохранённым toast: `22`. Функция доступна, но экран почти пуст при одном элементе. |
| Volume OSD | hardware volume-down (`KEY_VOLUMEDOWN`) | Layer `osd` `(1120,1296) 320×80`, значение 100%: `23`; затем `KEY_VOLUMEUP`, bar снова 105%: `crops/24-volume-restored.png`. |
| Tray menu | Right click Vivaldi tray `(2184,10)` | DBusMenu с действиями Vivaldi открылся, одновременно присутствовал shared catcher: `25`, `log/layers-25-tray-menu.txt`. |
| Dock/context | Dock не дал поверхности | Три pinned app по-прежнему пропущены как `no AppEntry`; это известный T309, не новая находка. Launcher app context покрыт отдельно (`27`/`28`). |

Номера кадров выше — имена файлов в
`.chronos-ops/dump/qa-ux/T325/frames/` с соответствующим префиксом.

## Гипотезы T323

- Sound поверх Calendar/Start/Edit — **подтверждено** (`14`, `16`, `18`).
- Клик мимо Sound не закрывает — **подтверждено** (`15`). Calendar ведёт себя
  так же (`31`).
- Calendar без заливки — **ждёт T329**, отдельный дефект не создавал.
- Bell не даёт отдельную history surface — **подтверждено**: он маршрутизирует
  в Notifications tab правой панели (`22`). История при этом доступна.

## Логи и устойчивость

- Точка начала этого release-запуска в логе:
  `2026-08-21T09:14:37.255941Z` (`launcher::init called`).
- Panic: **0**.
- `protocol error`: **0**.
- ERROR: **0**.
- Процесс после финального clean frame жив:
  `193962 .../target/release/chronos`.
- Не блокирует UX-прогон, но лог шумный: Launcher дал **78** предупреждений
  `usvg::parser::svgtree` про `marker-start/mid/end='none'`.
- Три dock `no AppEntry` warning — известный результат T309, не новая находка.

Команды подсчёта выполнялись по срезу лога от указанной timestamp через
`awk`; полный исходник оставлен в `log/chronos.log`.

## Кадры и конфиги

`ls .chronos-ops/dump/qa-ux/T325/frames | wc -l` → **30**.

SHA-256 до/после совпали для всех 11 TOML:

| Файл | SHA-256 до = после |
|---|---|
| `bar.toml` | `26af9a89b1b7b95d3e0e83ac7aaf92a6355a76e7ec73f718946d96738b9e415b` |
| `dock.toml` | `9a86dfcc2178dd8dced716d2538720909350faea75e5a3a34c042d3a43fb991f` |
| `frame.toml` | `7617c40630d6f6ac1e179c34f80b6352159e65032fbee5712d67ea1b53f94e42` |
| `frecency.toml` | `a666d769373ead5740e41a122f3fa3b22321b4fede70645090bc11df22808462` |
| `launcher.toml` | `00f0a04f68da132849c587767dfb1bd5e9a5a3374556a0c1308d509b743e8f9d` |
| `monitor.toml` | `2b114e95148dbfd777954b5a4e58005a7a678316e5636cedb6d7804b208c8ac6` |
| `panels.toml` | `bba9070546180194f418cef712483d6cbb18767c9ad6f9edb612ece60fd6d433` |
| `projects.toml` | `8501e28514db4705caa7747ace78112c434088314ebefa2adcd353de1dd4fb18` |
| `scenes.toml` | `7c4429e028876f6763eef5e0da2c42b44c904704f837fe320c77fc5b637d9836` |
| `theme.toml` | `3841c70c58d9bf1faa48617a0a88e3c431c339c2e345decbe9769d2bf2be524f` |
| `workspace.toml` | `d05510d5d6393c39a1ca6047b1e8428ad42b9d82cc22adbd62ae0188599351ae` |

Полные списки: `log/config-before.sha256`, `log/config-after.sha256`; резервные
копии: `config-backup/`.

## Методические поправки и что не делал

- На этой сессии ydotoold слушает
  `/run/user/1000/.ydotool_socket`; каждому вызову задан `YDOTOOL_SOCKET`.
- В установленном ydotool `0xC0` — left click, `0xC1` — right click.
  Кадры `08`/`09` — диагностические неудачные попытки до исправления harness;
  в выводах они не используются.
- Буквальный `notify-send ChronOS T325 "toast"` передаёт три positional arg и
  падает `Invalid number of options`; рабочий эквивалент —
  `notify-send 'ChronOS T325' 'toast'`. Кадр `20` не используется.
- `wpctl` был закрыт allowlist lean-ctx, поэтому OSD вызван аппаратным путём,
  разрешённым брифом; уровень возвращён парной клавишей и перепроверен кадром.
- Не менял код, theme/frame/wallpaper, TOML-конфиги и не создавал чужие
  тикеты. Не запускал dock menu искусственно через изменение pinned config.
- Новый release build не делал: проверял уже лежащий release-бинарник
  `target/release/chronos` (28,045,960 bytes, timestamp
  `2026-08-20 23:59:32.939531491 +0300`) ровно по рецепту QA-брифа.

# ZED — задание №1 от Lead Architect: Alloy → встроенный AUR/pacman-хелпер

_Дата: 2026-07-18. Отчёт — `zed-report.md` в корне (SESSION_REPORT-формат,
см. образец в `cline-report.md`/`grok-report.md` если они ещё в корне, или
`report-log/` за прошлые волны). **Новая сессия** — контекст ниже полный,
плюс прочти `HANDOFF.md` целиком (обязательно — там текущая карта: кто где
работает, системный баг remove_window, git-деп на общий тулкит)._

## Контекст

Пользователь раньше собрал **Alloy** (`~/projects/chronos-ecosystem/Chronos-AUR`,
GitHub `Dark-Ohm/Alloy`, MIT — свой код, лицензионных вопросов нет) —
GUI-пакетный менеджер для Arch: drag&drop `.deb`/`.rpm`/`.pkg.tar.zst`/
`.AppImage`, AUR/pacman поиск-установка-удаление, malware-check. Сейчас
это Tauri 2 + React (webview). Цель — не поднимать Tauri-приложение
отдельно, а **встроить в ChronOS** как нативный трей-виджет с бейджем
количества обновлений — «допилить под шелл», по словам пользователя.

**Бэкенд-логика почти вся переносима как есть** — это тонкие обёртки
над `yay`/`pacman` subprocess-вызовами, не завязанные на Tauri:
- `src-tauri/src/services/pacman_ops.rs` (50 строк): `pacman_search`,
  `pacman_info`, `pacman_list_installed`, `install_script`,
  `remove_script`, `upgrade_script`, `pacman_sync` — возвращают
  `(stdout, stderr, exit_code)` или готовые shell-скрипты для
  привилегированных операций.
- `src-tauri/src/services/aur_ops.rs` (66 строк): `yay_search`,
  `yay_install_script`, `upgrade_stream_script`,
  `yay_clean_orphans_script`, `fetch_pkgbuild` (PKGBUILD review),
  `pactree_forward`/`pactree_reverse`.
- `src-tauri/src/updater.rs` — их СВОЙ Tauri-tray с бейджем, референс
  поведения (не код — API другой), не копировать API 1:1.
- Остальные сервисы (`deb.rs`, `rpm.rs`, `appimage.rs`, `pkg_tar.rs`,
  `pkg_analyze.rs`, `pkg_build.rs`, `malware_check.rs`,
  `system_info.rs`, `fish.rs`) — НЕ трогать в этом заходе, см. Scope.

## Задача — ПЕРВЫЙ ЗАХОД, не вся Alloy сразу

Порт логики Alloy в ChronOS большой, если брать сразу поиск+установку
конкретных пакетов+drag&drop конвертацию форматов+malware-check. Чтобы
не повторить историю с desktop-terminal (там завели спайк вместо полной
фичи) — этот заход ограничен:

1. **Новый сервис** `crates/services/src/aur/` (или `pkg_updates` —
   на твоё усмотрение, не занимай имя если конфликтует с чем-то в
   дереве, проверь `ls crates/services/src/`). По образцу существующих
   сервисов (`Service` trait — `type Data; fn subscribe(); fn status();
   fn dispatch();`, см. `crates/services/src/audio/` как пример
   маленького живого сервиса). Функциональность MVP:
   - Периодическая проверка доступных обновлений (`checkupdates`
     из пакета `pacman-contrib`, если установлен — НЕ трогает live
     базу pacman, безопасно гонять в фоне; фолбэк — `pacman -Qu`,
     менее точный но без доп. зависимости, на твоё решение с
     обоснованием) + AUR-обновления через `yay -Qua` (если `yay`
     доступен — проверь через `which`, не хардкодь путь).
   - Данные: количество пакетов с обновлением + список имён/версий
     (старая→новая) — достаточно для бейджа и попапа-списка.
2. **Виджет бара** `crates/app/src/bar/widgets/updates.rs` — иконка +
   счётчик (по образцу `volume.rs`/`network.rs` — прочти один из них
   как шаблон структуры). Регистрация — ОДНА строка в
   `crates/app/src/bar/widgets/mod.rs` (файл специально спроектирован
   под параллельные правки — комментарий `// Other agents append below`
   на месте, конфликтов с другими агентами быть не должно, но всё
   равно `git diff --staged` перед коммитом, поимённый add).
3. **Попап при клике** — список пакетов с обновлением (имя, старая →
   новая версия) + кнопка «Upgrade all». Новый модуль
   `crates/app/src/updates_popup/` (или похожее имя), окно/попап по
   паттерну `tray_menu` (layer-shell попап, close-on-blur С УЧЁТОМ
   `follow_mouse=1`-урока из сегодняшней сессии — читай в HANDOFF.md
   раздел про Cline №9, НЕ наступай на те же грабли: не закрывай попап
   по голой потере клавиатурного фокуса от движения мыши).

## Явно НЕ в этом заходе

- Поиск и установка ПРОИЗВОЛЬНЫХ пакетов (не из списка обновлений).
- Drag&drop конвертация форматов (`.deb`/`.rpm`/`.appimage`) —
  отдельный, самостоятельный кусок Alloy, не трогай `deb.rs`/`rpm.rs`/
  `appimage.rs`/`pkg_tar.rs`.
- `malware_check.rs`, `pkg_analyze.rs`, `pkg_build.rs` — не трогай,
  не в MVP.
- Не трогай `launcher/`, `tray_menu/`, `notifications/`,
  `desktop_terminal/` — Cline/Hermes/Grok параллельно работают там.

## ВАЖНО — про кнопку «Upgrade all» и живую верификацию

`pacman`/`yay` upgrade — это РЕАЛЬНОЕ изменение системы пользователя,
необратимое одной кнопкой отмены. Для автоматической верификации:
- Детекцию обновлений (чтение, не запись) — проверяй живо сколько
  угодно, это безопасно (`checkupdates`/`pacman -Qu` ничего не меняют).
- Саму команду апгрейда — НЕ выполняй автоматически на живой системе
  в рамках своей верификации. Проверь, что кнопка формирует ПРАВИЛЬНУЮ
  команду (залогируй её / выведи в тестовом моке), но не запускай
  реальный `pacman -Syu`/`yay -Sua` без пользователя за компьютером.
  Живой прогон кнопки — я (Архитектор) сделаю вместе с пользователем
  отдельно, не в твоей автономной верификации. Это не недоверие к
  тебе — это общее правило проекта про необратимые действия.

## Зоны

Твои (новые файлы, минимальное пересечение):
`crates/services/src/aur/**` (новый), `crates/services/src/lib.rs`
(регистрация нового модуля — 1-2 строки), `crates/app/src/bar/widgets/updates.rs`
(новый) + 1 строка в `crates/app/src/bar/widgets/mod.rs`,
`crates/app/src/updates_popup/**` (новый) + регистрация в `main.rs`
(2 строки, по образцу остальных модулей — `mod updates_popup;` +
`updates_popup::init(cx);`).

## Верификация

`cargo build -p chronos` + `cargo test --workspace --lib --bins`
зелёные. Живой смок (детекция обновлений — читай выше про upgrade-кнопку):
`RUST_LOG=chronos=info ./target/debug/chronos`, бейдж в баре показывает
реальное число (сверь вручную `checkupdates`/`pacman -Qu` в соседнем
терминале — цифры должны совпасть), клик открывает попап со списком,
список совпадает с ручной проверкой. `hyprctl layers -j`/`clients -j`
после закрытия попапа — не должно оставаться ghost-окна (тот самый
системный баг `remove_window()` — читай HANDOFF.md, если открываешь
окно из callback'а, что уже держит `&mut Window` — не зови
`handle.update()` повторно на тот же id, звони напрямую).

## Условие эскалации

Если детекция обновлений (`checkupdates`/`yay -Qua`) окажется
нестабильной/медленной/требует sudo там, где не ожидалось — стоп,
опиши находку в отчёте, не изобретай обходной путь с повышением
привилегий сам без согласования.

Коммит: `bar : AUR/pacman update-виджет (порт логики Alloy, MVP —
детект+список+upgrade-кнопка)` (сформулируй по факту).

---

# ▶ АКТИВНО СЕЙЧАС — Задание №2: System popup (яркость + power profile + gaming mode)

**Дата: 2026-07-19 (вечер).** №1 (AUR-виджет) **ПРИНЯТО** (`0fd2fb9` и
далее visual-parity `8d74583`). Это **новое** независимое задание.

Отчёт: `orchestration/reports/zed-report-2.md`.

## Контекст (полный, с нуля)

На десктопе без батареи `bar/widgets/battery.rs` рендерит **пустой
div** (`!has_battery`) — кликать не по чему, power-profile cycle
мертв для UI. Решение (принят дизайн + DECISIONS/ARCHITECTURE §
System popup): слот battery **заменяется** на system-иконку (сигил),
клик открывает попап «System».

**Визуальный эталон (принят живьём):**
`design/System Popup.dc.html` — header «System»+✕, Brightness
fill-bar+степперы, Power profile 3-сегмент, Gaming mode toggle +
эффект-строка, hexagon/sigil-иконки. Структура как volume popup.

Канон: `design.md` §6, `ARCHITECTURE.md` (System popup / gaming
mode), `HANDOFF.md` «System-popup».

## Живые факты ЭТОЙ машины (проверено Архитектором 2026-07-19)

### Яркость — только ddcutil (НЕ brightnessctl)

- `/sys/class/backlight` пуст. `brightnessctl` видит только LED
  numlock — **игнорируй brightnessctl полностью**.
- `ddcutil` 2.2.7 установлен, `i2c-dev` loaded + persistent
  (`/etc/modules-load.d/i2c-dev.conf`), user в группе `i2c`,
  `/dev/i2c-*` → `root:i2c` rw (udev `60-ddcutil-i2c.rules`).
- Два монитора:
  - Display 1: Dell U2412M · HDMI-A-1 · bus `/dev/i2c-2`
  - Display 2: Samsung LC32G5xT · DP-1 · bus `/dev/i2c-3` (**primary**,
    слева, 144Hz)
- Команды (проверены read+write):
  ```
  ddcutil getvcp 10 --display N
  ddcutil setvcp 10 <0-100> --display N
  ```
- **MVP-политика:** один слайдер = **оба** дисплея сразу (setvcp на
  1 и 2). Без UI «выбрать монитор». Read: среднее или primary (DP-1 /
  display 2) — зафиксируй выбор в отчёте.
- Soft-fail: нет `ddcutil` / нет i2c / detect пустой → UI яркости
  disabled/muted, **не** паника, шелл живёт (как cava без бинаря).
- DDC latency ~100–300ms — не поллить на каждый кадр бара; читать
  при открытии попапа + после степпера.

### Power profile — уже есть в services

- `crates/services/src/upower/` — `PowerProfile`, `set_power_profile`,
  live-подписка. Cline №10 (`2522018`) принят.
- `powerprofilesctl` / `net.hadess.PowerProfiles` живы.
- В попапе: 3-сегмент Quiet(=PowerSaver) / Balanced / Performance
  (лейблы как в мокапе; маппинг на enum — в отчёте явно).
- **Не** дублируй D-Bus клиент — зови существующий UPower service.

### Gaming mode — hyprctl eval (НЕ keyword)

На Hyprland 0.55.4 + **Lua-конфиг** (`hyprland.lua`):

```
hyprctl keyword …   →  ОШИБКА: "keyword can't work with non-legacy parsers. Use eval."
```

**Рабочий** runtime set (проверено Архитектором, restore сделан):

```bash
# ON
hyprctl eval 'hl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false } }, general = { allow_tearing = true } })'

# OFF (вернуть дефолт сессии)
hyprctl eval 'hl.config({ animations = { enabled = true }, decoration = { blur = { enabled = true } }, general = { allow_tearing = false } })'
```

Проверка: `hyprctl getoption animations:enabled` и т.д.

Дополнительно в gaming ON (внутри ChronOS, не hyprctl):
1. `set_power_profile(Performance)` через UPower service.
2. **DND** — флаг «не показывать эфемерные notification popups»
   (Global / поле в notification service). Если полного API нет —
   минимальный Global `GamingModeState { dnd: bool }`, а notifications
   смотрят его **только если** трогаешь 1–2 строки в
   `notifications/` — иначе оставь TODO + флаг в Global, эскалация в
   отчёте (Hermes параллельно трогает notifications — **не** конфликтуй
   с её history-кодом; лучше отдельный Global, который Hermes/ты
   читаете, без правки её файлов если можно).
3. **Hide bar/dock** — MVP: Global flag; bar `render` может рано
   return empty **или** оставь hide bar/dock как soft-TODO в отчёте
   если это ломает отладку (без бара не протестируешь попап).
   **Рекомендация Архитектора:** hide bar/dock **не** делать в MVP
   (chicken-egg), только compositor + performance + DND-флаг.
   Эффект-строка в UI всё равно показывает intent; hide — follow-up.

Эффект-строка под toggle (мок):  
`Performance · No animations · No blur · Allow tearing · DND`  
(без «Hide bar/dock» если не реализовал).

## Задача

1. **Сервис яркости** `crates/services/src/brightness/` (или
   `display_brightness/`): shell out to `ddcutil`, parse getvcp,
   setvcp ±step (5%), soft-fail. Юнит-тест на **парсер** stdout
   (фикстура строки `VCP code 0x10 ... current value = 15, max = 100`),
   без live i2c в CI.
2. **Попап** `crates/app/src/system_popup/` — clone lifecycle
   `volume_popup/` 1:1:
   - layer-shell Overlay, TOP|RIGHT, margin как volume;
   - **нет** close-on-focus-loss; close = ✕ / toggle / Esc;
   - `close_this` direct `remove_window`, never re-entrant
     `handle.update` (ghost-window saga);
   - `border_1().border_color(theme.border.subtle)`, hover на
     интерактиве.
3. **Bar widget** `crates/app/src/bar/widgets/system.rs` —
   `BarSection::Right`, **всегда** виден (и с батареей, и без — на
   десктопе это единственная точка входа; на ноутбуке battery может
   остаться рядом, system не зависит от `has_battery`). Иконка —
   сигил/hexagon line-art **или** простая ⚙/glyph если SVG в GPUI
   геморрой — зафиксируй. Клик → `system_popup::toggle(cx)`.
4. **Не** выкидывай `battery.rs` в этом задании (leave as-is). System —
   новый виджет, регистрация append в `bar/widgets/mod.rs`.
5. **Gaming mode state** — Global + apply/revert hyprctl eval через
   `std::process::Command` (или tokio spawn в service; UI-тик — GPUI
   executor, IPC-subprocess — ок). Сохраняй previous values чтобы
   OFF не затирал чужие настройки вслепую если можно; минимум —
   hardcode restore к true/true/false как в сессии пользователя.
6. `main.rs`: `mod system_popup;` + `system_popup::init(cx);` (2
   строки, по образцу volume_popup).

## Зоны (ЖЁСТКО)

Твои:
- `crates/services/src/brightness/**` (новый) + 1–2 строки в
  `crates/services/src/lib.rs`
- `crates/app/src/system_popup/**` (новый)
- `crates/app/src/bar/widgets/system.rs` (новый)
- `crates/app/src/bar/widgets/mod.rs` — **только append** register
- `crates/app/src/main.rs` — только mod+init system_popup
- `crates/app/src/state.rs` — только если нужно пробросить
  brightness service в AppState (по образцу audio/upower)

**НЕ трогай:** `battery.rs`, `volume_popup/**`, `notifications/**`,
`dock/**`, `bar/mod.rs`, `upower/**` (только **вызовы** API),
`updates_popup/**`, Hermes/Mimo/Cline файлы.

## Верификация (без неё не принято)

- `cargo build --release -p chronos`
- `cargo test --workspace --lib --bins` — зелёные + парсер brightness
- **Живой смок:**
  1. Иконка system в правом кластере бара (десктоп без battery).
  2. Клик → попап, grim. Структура ≈ мокап.
  3. Brightness −5/+5 → `ddcutil getvcp 10 --display 2` меняется
     (Samsung primary); оба монитора если политика «оба».
  4. Power segment → `powerprofilesctl get` совпадает.
  5. Gaming ON → `hyprctl getoption animations:enabled` = false,
     blur false, allow_tearing true; OFF → restore.
  6. `hyprctl layers -j` чист после close. Лог без panic.
  7. `pkill -x chronos` (не `-f`).

Коммит: `bar/services : system popup — ddcutil brightness + power
profile + gaming mode (hyprctl eval)`.
Поимённый `git add`, `git diff --staged` глазами.

---

# ▶ АКТИВНО СЕЙЧАС — Задание №3: доработка System popup после приёмки

**Дата: 2026-07-19 (ночь).** Задание №2 **НЕ ПРИНЯТО** — твой самоотчёт
`zed-report-2.md` честный (unit-зелень ≠ UX, живой смок нашёл 2 бага), но
раскладка причин частично мимо. Я сверил код с деревом сам. Ниже —
точный диагноз и что делать. Работай в своём worktree `ChronOS-zed2`,
НЕ в master-дереве (там твой WIP + чужой rustfmt-шум вперемешку —
подметать чужое НЕ коммить). Отчёт — тот же `zed-report-2.md`
(перезапиши/допиши).

## Что я установил по коду (не гадания)

**Bug 2 — ты сам себя запутал «нулём записей в логе»:**
- `close_this` (`system_popup/mod.rs:116`) **вообще не логирует** —
  ноль записей о close ничего не доказывает.
- `gaming_mode::toggle` логирует только ВНУТРИ `background_spawn`
  («gaming mode: hyprctl eval ON applied»), а флаг `active/dnd` флипает
  синхронно ДО спавна. Но **нет вотчера/repaint на `GamingModeState`** —
  попап подписан только на brightness+upower сигналы (`init` в mod.rs:
  147-188), не на gaming-глобал. Значит тоггл может физически
  сработать, а knob в UI не сдвинется — визуально мёртвый. Это
  **отдельный реальный баг, ты его не назвал.**
- Только power-profile честно логирует в спавне. Ноль там = либо клик
  не дошёл, либо спавн не отработал — **но тестировать это на попапе,
  всплывшем не на том мониторе (Bug 1), бессмысленно.**

**Bug 1 — НЕ твой дефект.** `pick_display` + `window_options` в
`system_popup/mod.rs:55-82` — **байт-в-байт** копия принятого
`volume_popup/mod.rs:59-86` (та же `cx.primary_display()`, тот же
`TOP|RIGHT`, тот же margin). Если system-попап сел на HDMI-A-1 — значит
и volume/updates/tray/osd/notifications садятся туда же, просто никто не
проверял ФИЗИЧЕСКИЙ монитор (сверяли namespace в `hyprctl layers` и
бордер, а не «на каком экране»). Это латентный общий баг ~9 попапов.

## Phase 1 — диагностика дисплея. СНАЧАЛА ЭТО, потом Phase 2.

Цель — ответить на 2 вопроса и **вернуть отчёт мне ДО фикса** (фикс
может оказаться уровня Source/gpui-форка, тогда это не твоя зона):

1. **Что реально возвращает выбор вывода?** В `system_popup::open`
   (перед `open_window`) залогируй:
   ```rust
   tracing::info!("system_popup: primary_display id={:?}", cx.primary_display().map(|d| d.id()));
   for d in cx.displays() {
       tracing::info!("system_popup: display id={:?} bounds={:?}", d.id(), d.bounds());
   }
   ```
   Открой попап, сними лог + `hyprctl layers -j` (где реально сел
   попап — координаты x). Сопоставь: `primary_display()` вернул DP-1
   (Samsung, левый, x≈0) или HDMI-A-1 (Dell, правый, x≈2560+)?
2. **Честится ли `display_id` для layer-shell вообще?** Форсни в
   `window_options` заведомо ДРУГОЙ вывод (жёстко подставь id второго
   дисплея из `cx.displays()`), открой, посмотри — попап переехал или
   остался там же? Если `display_id` игнорируется и попап всегда
   садится куда хочет компоузитор — это баг backend'а gpui-форка
   (layer-shell output binding), НЕ app-уровня.

**Отчёт по Phase 1 → СТОП, жди меня.** Я решу: app-фикс (общий helper
`popup_display(cx)` на все попапы) или эскалация в Source (Grok). НЕ
изобретай общий рефактор 9 попапов сам — это shared-file зона поперёк
других агентов, координирую я.

## Phase 2 — Bug 2 + gaming repaint (после того, как дисплей верный)

Только когда попап открывается на том мониторе, где кликнули:

1. **`tracing::info!` в самом начале КАЖДОГО `on_click`** (до любого
   spawn): close (`view.rs:67`), каждый power-segment (`view.rs:290`),
   gaming toggle (`view.rs:396`). Пересобери, кликни по всем пяти
   элементам (−5%, +5%, Quiet, Balanced, Performance, gaming, ✕),
   сними лог. Это разделяет «клик не дошёл до хендлера» от «async
   упал». По результату — фикс точечный.
2. **Gaming toggle repaint.** Добавь перерисовку попапа при смене
   `GamingModeState` — минимум: в `gaming_mode::toggle`/`apply`/`revert`
   после флипа глобала дёрни `handle.update(...notify())` по хендлу из
   `SystemPopupState` (он у тебя есть в `mod.rs`). Knob обязан
   сдвигаться в UI сразу. Твоя зона целиком.
3. Повторный живой смок как в Задании №2 «Верификация» (все 5
   элементов + `ddcutil getvcp 10 --display 2` / `powerprofilesctl get`
   / `hyprctl getoption animations:enabled` после каждого действия).

## Зоны Phase 2 — как в Задании №2 (жёстко, не расширять)

Bug 1 общий фикс в Phase 1 — **не трогай** volume/updates/tray/osd/
notifications пока я не решу масштаб. В Phase 1 правишь ТОЛЬКО свой
`system_popup/mod.rs` (диагностический лог, потом откатишь/оставишь по
моему решению).

Коммит (только после приёмки Phase 2): как в Задании №2. Диагностический
лог Phase 1 — отдельно, по моему решению остаётся или снимается.

---

## ◀ РЕШЕНИЕ АРХИТЕКТОРА по Phase 1 (2026-07-19 ночь)

Диагностика принята, evidence железное. Root cause: `cx.primary_display()`
== `None` → fallback `displays().next()` = HDMI (первый в списке, меньший).
`display_id` layer-shell'ом **честится** (доказано) — backend чинить не
надо.

**Ни один из твоих A/B/C/D не беру.** Причины:
- **A (Source-фикс `primary_display`)** — отклонено. На Wayland нет
  канонического «primary output» в протоколе; `None` — честный ответ, а
  не баг форка. Любой Source-фикс всё равно свёлся бы к эвристике. Не
  гоняю Grok ради этого.
- **B/C (эвристика «самый большой»)** — отклонено. Развалится на ноуте
  (маленький primary + большой внешний) или на двух одинаковых
  мониторах. Хак, который выстрелит позже.
- **D — правильное направление, но не через глобальные координаты.**

**Верный фикс (проверил по коду сам):** бар — **пер-монитор** (`bar/mod.rs
:174-176`, окно на каждый `d.id()`). `system.rs:43` уже зовёт
`toggle(window, cx)`, где `window` = окно бара на кликнутом мониторе.
Форк отдаёт `window.display(&self, cx) -> Option<Rc<dyn PlatformDisplay>>`
(`../Source/gpui/src/window.rs:2445`), у дисплея есть `.id()`. Значит
попап должен открываться на дисплее ТОГО бара, из которого кликнули —
монитор-агностично, без эвристик, без primary вообще.

### Phase 2 (пересмотрен) — делаешь СЕЙЧАС, целиком в своей зоне

1. **Фикс дисплея — только `system_popup/mod.rs`, чужие попапы НЕ
   трогай:**
   ```rust
   pub fn toggle(window: &mut Window, cx: &mut App) {
       if cx.global::<SystemPopupState>().handle.is_some() {
           close(cx);
       } else {
           let display = window.display(cx).map(|d| d.id());
           open(display, cx);
       }
   }

   pub fn open(display_id: Option<DisplayId>, cx: &mut App) {
       if cx.global::<SystemPopupState>().handle.is_some() { return; }
       AppState::brightness(cx).dispatch(BrightnessCommand::Refresh);
       // Дисплей кликнутого бара; fallback pick_display только если None.
       let display_id = display_id.or_else(|| pick_display(cx));
       // ... как было ...
   }
   ```
   `system.rs` НЕ меняешь (он уже передаёт `window`). `pick_display`
   оставь как fallback. Диагностический лог Phase 1 — **сними** (не
   оставляем спам в проде).
2. **Bug 2 диагностика** — `tracing::info!` в начале КАЖДОГО `on_click`
   (до spawn): close `view.rs:67`, power-segment `view.rs:290`, gaming
   `view.rs:396`. Пересобери, кликни по всем 5 на ПРАВИЛЬНОМ мониторе,
   сними лог. Разделяет «клик не дошёл» от «async упал» → точечный фикс.
3. **Gaming repaint** — после флипа `GamingModeState` в
   `gaming_mode::{toggle/apply/revert}` дёрни repaint попапа по хендлу из
   `SystemPopupState` (`handle.update(...view_cx.notify())`). Knob обязан
   двигаться сразу. Твоя зона.
4. Повторный живой смок как в Задании №2 «Верификация» — все 5
   элементов + `ddcutil getvcp 10 --display 2` / `powerprofilesctl get` /
   `hyprctl getoption animations:enabled` после каждого. Плюс явно
   подтверди: **попап открылся на том мониторе, где кликнул ⚙** (сверь
   `hyprctl layers -j` координату x с дисплеем бара).

### Что НЕ твоё (моя координация)

Остальные 8 попапов (`volume/updates/tray/osd/notifications/launcher/
desktop_terminal/dock/context_menu/history_popup`) — тот же латентный
баг, тот же фикс по паттерну (их `toggle` тоже получает `window` бара).
Раскатываю мехāнически отдельным координированным коммитом ПОСЛЕ того,
как ты обкатаешь паттерн в system_popup. **Не трогай их.**

Побочно (для протокола, не задача): форк отдаёт `display.bounds().origin`
= `(0,0)` для обоих мониторов (локальные коорд, не глобальные) — любой
код, завязанный на глобальную позицию дисплея, в форке слеп. Нам для
попапов не нужно (хватает `display_id`+anchor), но знать полезно.

Коммит Phase 2 (после приёмки): `bar : system popup — попап на дисплее
кликнутого бара + gaming repaint + фикс on_click` (сформулируй по факту).

---

# ▶ АКТИВНО СЕЙЧАС — Задание №4: разведка `gpui-form` — компилируется ли против нашего форка

_2026-07-20. Отчёт — `orchestration/reports/zed-report-4.md`
(SESSION_REPORT, MEMORY.md §Rules). ХОЛОДНАЯ сессия: читай `HANDOFF.md`
(верхний блок + кровные факты). Твои №2/№3 (system popup) приняты._

## Контекст

Пользователь кинул ссылку на `github.com/stayhydated/gpui-form` —
дерайв-библиотеку типизированных форм для gpui (`#[derive(GpuiForm)]`).
Архитектор её изучал (не кодом, только чтением) и выяснил:

- Воркспейс из 12 крейтов. **Ядро** (`gpui-form-core`, `-derive`,
  `-schema`, `-runtime`, `-codegen`) зависит ТОЛЬКО от `gpui` — НЕ от
  `gpui-component`. Виджет-обёртки (`-collection`, `-component`) тянут
  `gpui-component`, но они ОПЦИОНАЛЬНЫ.
- Реальная API-поверхность derive+runtime — всего 4 имени:
  `gpui::{Context, Entity, Window, IntoElement}` (грепом по
  `crates/gpui-form-derive/` и `crates/gpui-form-runtime/`). Это
  фундаментальные типы entity/render-модели, не апстрим-специфика.
- НО их `gpui` — git-зависимость на `stayhydated/zed`, ветка
  `linux-headless-renderer`, версия `0.2.2`. Наш `gpui` — СОБСТВЕННЫЙ
  форк («gpui-ce chronos edition»), `path`-локальный крейт в
  `../Source/gpui`, тоже версия `0.2.2`, но другого происхождения.
  Совпадение версии — вероятностный, не доказанный сигнал совместимости.
- Дальше Архитектор НЕ пошёл — начал патчить `Cargo.toml` чужого клона
  на `path`-зависимость от нашего форка и гонять `cargo check`, но это
  уже эксперимент интеграции, не разведка, и пользователь остановил
  ровно на этом шаге («we are researching»). Компиляция против нашего
  форка НЕ проверена никем.

## Задача — ТОЛЬКО разведка компиляцией, не интеграция

1. **Клонируй `gpui-form`** в `/tmp` или свой scratch (НЕ в `../Source/`,
   НЕ в ChronOS) — `git clone --depth 1
   https://github.com/stayhydated/gpui-form`.
2. **В СВОЁМ клоне** (не в наших репозиториях) поменяй в его корневом
   `Cargo.toml` зависимость `gpui` с git на:
   ```toml
   gpui = { path = "/home/neo/projects/chronos-ecosystem/Source/gpui" }
   ```
   Убери строку `gpui-component = { git = "..." }` из workspace-deps,
   если она конфликтует (у нас `gpui-component` тоже есть локально в
   `../Source/gpui-component` — если ядро без неё собирается, СНАЧАЛА
   проверь без неё; добавишь `path`-версию отдельным шагом только если
   понадобится для крейтов `-collection`/`-component`).
3. **`cargo check` по нарастающей**, снизу вверх зависимостей:
   - `cargo check -p gpui-form-core` (не зависит от gpui вообще — должен
     быть чист всегда, это контроль, что ты ничего не сломал);
   - `cargo check -p gpui-form-derive` (использует `Context`/`Entity`/
     `Window` — ключевая проверка);
   - `cargo check -p gpui-form-runtime` (использует `Entity`/
     `IntoElement`);
   - `cargo check -p gpui-form` (верхний реэкспорт).
   Останавливайся на первой ошибке типов — не пытайся протолкнуть
   правками их код, это не твоя библиотека.
4. **Если ядро собралось** — попробуй `-collection`/`-component` тоже
   против нашего `gpui-component` (`path = "/home/neo/projects/
   chronos-ecosystem/Source/gpui-component"`). Отдельно зафиксируй,
   собрались они или нет — это опционально, ядро важнее.
5. **Мини-пример.** Если ядро (п.3) собралось — напиши ОДИН минимальный
   `#[derive(GpuiForm)]` над игрушечной структурой (2-3 поля) во
   временном example СВОЕГО клона (не в ChronOS) и убедись, что
   макрос реально генерирует код (`cargo expand` если есть, или просто
   `cargo build` без ошибок макро-паники).

## Где это могло бы пригодиться (только как контекст, НЕ обязательство)

Продуктового применения в ChronOS сегодня НЕТ придуманного. Кандидаты
на будущее: settings-форма в `system_popup` (если там заведутся текстовые
поля/числовые степперы сверх текущих тумблеров), возможная замена
XDG-портала в project switcher на встроенную форму пути. Ничего из
этого не поручается — просто чтобы понимать, есть ли вообще смысл.

## Кровные факты (не наступи)

- Наш форк — `path`-зависимость, не git. Не пытайся añадir его как git-
  remote или менять его версию/rev под чужие ожидания.
- Ничего не коммитить ни в `../Source/`, ни в клон `gpui-form` в наш
  git. Это чужой код на чтение+эксперимент, результат — только отчёт.
- Не трогай `Source/Cargo.lock`/`Cargo.toml` вообще — эксперимент
  строго в отдельном клоне вне обоих наших репозиториев.

## Зоны

Твои: только твой временный клон `gpui-form` вне ChronOS и `Source/`.
**НЕ трогай:** ничего в `ChronOS/` или `../Source/` — это read-only с
их стороны, твоя правка — только в скачанном клоне.

## Верификация

Отчёт — таблица: крейт → собрался (да/нет) → ЕСЛИ нет, точная ошибка
(`error[E0xxx]: ...`, не пересказ). Плюс вывод мини-примера (п.5) если
дошёл. Честно опиши, на каком шаге остановился и почему, если не дошёл
до конца — это не провал, это ценная информация.

## Условие эскалации

Если на первом же `cargo check -p gpui-form-derive` вылезет desync
(другая сигнатура `Context`/`Entity` в нашем форке против того, что
ожидает derive) — это и есть ответ «нет, не совместимо без правок».
Зафиксируй error message дословно, не пытайся патчить их макрос под
наш API — это отдельное решение, не твоё сейчас.

Коммит: не нужен (ChronOS не меняется). Только отчёт.

## ⚠️ ПРИЁМКА «Phase 2» — ОТКЛОНЕНО, регресс не допущен до коммита (2026-07-20, Архитектор)

Твой `zed-report-2.md` (случайно назван так же, как уже принятый и
заархивированный отчёт от 2026-07-19 — коллизия имён, не злой умысел,
но впредь смотри `orchestration/report-log/` перед именованием) описывал
работу над старой веткой расследования (Phase 1 → Phase 2, дисплей
попапа), которая к моменту твоей сессии **уже была решена по-другому**.

**Что произошло технически.** С 2026-07-19 действует
`crate::monitor::pult_display(cx)` — единая точка выбора chrome-монитора
(Mimo №10, консолидация), которая закрыла класс багов «попап не на том
мониторе» разом для ВСЕХ восьми попапов, включая твой `system_popup`.
Твой WIP заменил `pult_display(cx)` на `window.display(cx)` — паттерн,
который ты же сам обнаружил и задокументировал в Phase 1 (2026-07-19)
как возвращающий `None` для layer-shell окон (кровный факт,
`HANDOFF.md:867`). Это была бы регрессия — откат уже решённого класса
багов к уже опровергнутому паттерну.

**Причина, не в осуждение:** твоя сессия работала со СТАРЫМ контекстом —
судя по содержанию, ты продолжал ветку расследования, которая велась
ДО консолидации, и не видел, что `pult_display` уже существует и
закрывает именно эту проблему. Классический случай устаревшего
контекста в холодной/долгой сессии.

**Действие:** правка была НЕЗАКОММИЧЕНА (в рабочем дереве), я откатил
`git checkout -- crates/app/src/system_popup/mod.rs` — регресс не попал
в историю. `system_popup` остаётся на `pult_display(cx)`, как было
принято 19 июля. Ничего чинить не нужно — Bug 1 (дисплей) ЗАКРЫТ
консолидацией, не твоей Phase 2. Клик-diagnostics/gaming-repaint
(остальные пункты твоего отчёта) уже были в мастере и раньше — просто
не твоя новая работа, ты застал их существующими.

## ✅ Задание №4 (`gpui-form` разведка) — ЧЕСТНЫЙ ПРОВАЛ, не штрафуется

`zed-report-4.md`: terminal-инструмент сломался на длинном выводе,
клон/cargo check не запущены. Ничего не тронуто (`Source/` и ChronOS
не пострадали — подтверждаю). Правильное поведение: не изобразил
проверку, не выдумал результат, честно вернул задачу с точным
описанием блокера и инструкцией «как повторить». Роздано следующему
свободному агенту без изменений брифа.

---

## ЗАДАНИЕ №6 (капстоун-волна «правая панель», Task 6) — per-app stream mute в audio-сервисе

**Контекст.** Правой панели нужна кнопка «замьютить конкретный плеер» (MPRIS-
карточка панели, Task 9). Для этого расширяем существующий `audio`-сервис:
парс PipeWire playback-стримов из `pw-dump`, эвристика «сматчить стрим к MPRIS-
плееру по имени», новый вариант команды `ToggleStreamMute(u32)`. Твоя часть —
только сервисная (`crates/services/src/audio/`), UI не трогаешь.

**Читай план — Task 6, строки 971-1281.** Там ПОЛНЫЙ исходник всех функций,
фикстура и тесты. Ниже — сверенные факты и КРИТИЧНОЕ требование по фикстуре.

### Факты дерева (сверено 2026-07-21)

- `crates/services/src/audio/pw_dump.rs`: `pub fn run_pw_dump()` (:13),
  `pub fn parse_pw_dump_devices()` (:25) — новые `parse_pw_dump_streams` /
  `find_stream_for_player` кладёшь В ЭТОТ ЖЕ файл, тем же стилем парса.
- `crates/services/src/audio/types.rs`: `enum AudioCommand` (:43) — добавляешь
  вариант `ToggleStreamMute(u32)`; там же новый `struct AudioStream`.
- `crates/services/src/audio/mod.rs`: `fn command_to_wpctl_args` (:186) —
  добавляешь match-arm; рядом метод `toggle_stream_mute_for_player` на
  `AudioSubscriber` (план Step 8). Тесты `command_to_wpctl_args_*` уже есть
  с :266 — добавь свой рядом.
- `crates/services/src/audio/wpctl.rs`: `pub fn format_set_mute_toggle_args(id:
  &str)` (:24) — уже принимает любой id-строку, **сигнатуру НЕ меняешь**,
  просто зовёшь `format_set_mute_toggle_args(&id.to_string())`.

### КРОВНОЕ ПРАВИЛО ПОЛЯ — фикстура (Step 1, не пропускай)

Фикстура, НЕ снятая с живого вывода, — фантазия (у нас счёт таких провалов).
Step 1 требует **живьём** прогнать `pw-dump` c реально играющим звуком и
записать точный `media.class`/`application.name` для playback-стрима. Запусти
что-нибудь (`mpv`, вкладка браузера), выполни пробу из плана Step 1, сверь
фикстуру Step 2 с РЕАЛЬНОЙ схемой. **Если живого звука/среды нет — пиши в отчёт
«фикстура умозрительная, pw-dump живьём не снят» явно**, не выдавай догадку за
факт. Формат `Stream/Output/Audio` — проверяемое утверждение, не предположение.

### Процедура (TDD, по плану)

Step 1 (живая проба) → Step 2 (фикстура+3 теста, FAIL) → Step 4-5 (типы+функции)
→ Step 6 (PASS) → Step 7 (`ToggleStreamMute` + match-arm + тест
`command_to_wpctl_args_stream_mute_targets_the_given_id`) → Step 8 (метод
`toggle_stream_mute_for_player`) → Step 9 (`cargo test -p chronos-services --lib`
всё зелёное + `cargo build --workspace` чисто).

`find_stream_for_player` возвращает `None` при отсутствии матча — это ОЖИДАЕМЫЙ
исход (много вкладок / имя не похоже), caller мьютит по `None` как no-op, НЕ
паникует. Не глуши ошибки `let _ =` — `.log_err()`/`match`/`warn!` (см. правило
проекта про проглоченные Err).

### Зона

- **Пишешь ТОЛЬКО** `crates/services/src/audio/` (types.rs, pw_dump.rs, mod.rs;
  wpctl.rs — без изменений сигнатуры). Больше ничего.
- Параллельно: Task 1 (`services/net_stats`+`bar/widgets/network.rs`), Task 2
  (`ui/theme/`), Task 7 (`app/side_panel_right/`) — не пересекаешься.

### Коммит

**Не коммить.** Дерево + отчёт → `orchestration/reports/zed-report-6.md`.
Формат SESSION_REPORT: исход первой строкой, вывод тестов, ЯВНО — снята ли
фикстура живьём или умозрительная. Приёмку и коммит — Архитектор.

### Эскалация

`pw-dump` даёт схему, отличную от фикстуры плана (другой `media.class`, пустой
`application.name` у твоего тест-приложения) — СТОП, приложи реальный фрагмент
`pw-dump`, спроси Архитектора. Не подгоняй тест под догадку.

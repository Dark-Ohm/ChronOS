# T309 — live UX / visual critique audit

Дата: 2026-08-19
Роль: QA, только release-бинарь и synthetic input.

## Условия прогона

- Тестировался `target/release/chronos`, сборка уже существовала до прогона.
- Тестовый монитор: `DP-1`, `2560x1440`, scale 1. `HDMI-A-1` не использовался
  для оценки shell-поверхностей.
- Перед серией кликов сделана калибровка: `ydotool mousemove --absolute -x 50
  -y 15` дал `hyprctl cursorpos = 100,30`. Фактический множитель этой сессии
  — **×2**. Socket оказался `/run/user/1000/.ydotool_socket`.
- Для dark темы DP-1 был временно залит чистым белым PNG:
  `/tmp/t309/white.png`. Для light темы — чистым чёрным:
  `/tmp/t309/black.png`. После теста фон возвращён к исходному `awww clear
  -o DP-1 000000`.
- Dark запуск: unit `t309-chronos`, PID 1505324.
- Light запуск: `CHRONOS_THEME=light`, unit `t309-chronos-light`, PID 1535971.
- До и после проверены SHA-256 `~/.config/chronos/frame.toml`,
  `bar.toml`, `theme.toml`: все три совпали. `git status --short` после
  уборки пуст.

## Executive verdict

**Это не дерьмо, но до состояния «показывать как законченный продукт» shell не
дотягивает.** База уже выглядит как настоящий рабочий shell: поверхности
имеют устойчивую геометрию, тёмная и светлая схемы реально различаются,
панели/рейлы/control-center/уведомления/OSD открываются и держат сетку. Я не
увидел в этом прогоне P0 clipping или развала рамки: `frame_wrap_matte`
оставался `0,0 2560x1440`, а правый и левый rail не уезжали при открытии
панелей.

Слабое место — не фундамент, а полировка и ощущение целостности. Бар перегружен
мелкими элементами в правой группе, dock в этой машине визуально не даёт
ожидаемых pinned apps, а часть поверхностей остаётся очень «инструментальной»
и пустой вместо того, чтобы объяснять пользователю, что делать дальше.
Light-тема особенно нуждается в отдельном дизайнерском проходе по muted-
ролям, карточкам и иерархии. Для r/unixporn-аудитории это пока хороший
рабочий прототип, но не витринный финал.

## 1. Бар

### Dark

**Кадры:**
- `/tmp/t309/dark/bar-white-bg.png`
- `/tmp/t309/dark/bar-white-strip.png`

**Вердикт:** бар функционально собран, но визуально слишком плотный и мелкий.

На правой половине одновременно стоят separator, volume, tray, keyboard,
notification, battery, updates, clock и network; pixel probe показал
отдельные группы примерно в `x=2149..2542` при высоте всего 30 px. Это создаёт
ощущение панели статусов для разработчика, а не спокойной иерархии desktop
shell: внимания слишком много, а размер glyph/text слишком мал для первого
взгляда. Белая подложка снаружи подтвердила, что тёмный bar сам по себе
непрозрачный и читается как отдельная тёмная полоса.

**Severity:** вкусовщина / design-polish.

### Light

**Кадры:**
- `/tmp/t309/light/bar-black-bg.png`
- `/tmp/t309/light/bar-hover-volume.png`

**Вердикт:** светлая тема сохраняет сетку бара, но контраст и визуальный вес
правой группы нужно ещё выровнять.

В light запуске bar был на тех же `0,0 2560x30`, а доминирующий фон strip
стал `224..236`-уровня вместо dark `24..30`. Это хороший признак настоящей
смены scheme, а не инверсии отдельных текстов. Но большое число маленьких
статусов всё ещё конкурирует за внимание; hover volume зафиксирован кадром,
однако календарь/updates этим способом воспроизвести не удалось.

**Severity:** вкусовщина.

## 2. Launcher

### Dark

**Кадр:** `/tmp/t309/dark/launcher-empty-client.png`

**Вердикт:** launcher открывается в стабильном окне, но тест пустого/поискового
состояния упёрся в synthetic-input ограничение.

Hyprland показал обычный client `chronos-launcher` в `x=920,y=447,w=720,h=560`.
IPC открыл окно, кадр снят по этой геометрии. Клик и `wtype`/`ydotool type` по
предполагаемому input не изменили пиксели кроме курсора; это не объявляю багом
launcher, потому что в этой же сессии synthetic click нестабильно не доходил
до отдельных layer surfaces. Пустой запрос и запрос без совпадений поэтому не
получили честного визуального verdict.

**Severity:** не классифицируется; воспроизведение ограничено средой.

### Light

**Кадр:** `/tmp/t309/light/launcher-client-exact.png`

**Вердикт:** light launcher также открывается, но visual search verdict не
выносится из-за того же input caveat.

Фактическая геометрия в light была `x=892,y=447,w=720,h=560`; кадр снят по
ней, а не по догадке из dark темы. ACP/keyboard focus для этого subtest не
использовался как доказательство дефекта.

**Severity:** не классифицируется.

## 3. Левая панель

### Dark

**Кадр:** `/tmp/t309/dark/left-panel-empty.png`

**Вердикт:** левая панель и rail геометрически аккуратны, но empty-state
слишком мало сообщается пользователю.

`side_panel_left_rail = 32,30 40x1394`, `side_panel_left_content =
56,30 920x1394`. Отступ между wrap-кромкой и rail сохраняется, панель не
съезжает. Composer/пустой тред видны в полном кадре, но без поднятого живого
треда это выглядит как рабочее место, ожидающее внутреннего агента, а не как
завершённый onboarding. ACP-запрос через `compose-and-send` дошёл до ChronOS,
но Hermes в момент прогона вывел JSON-RPC parse/serialization warnings;
это отмечено как инфраструктурная оговорка, не как визуальный дефект панели.

**Severity:** мелочь для текущего прототипа, серьёзно для первого публичного
показа.

### Light

**Кадры:**
- `/tmp/t309/light/left-panel.png`
- `/tmp/t309/light/left-content.png`
- `/tmp/t309/light/left-rail.png`

**Вердикт:** light-панель хорошо отделяется от чёрного фона, но её secondary-
роли требуют дизайнерской проверки на реальном тексте.

Геометрия осталась той же (`56,30 920x1394`), а основной цвет content crop
стал `221,224,242` вместо dark `30,30,46`, поэтому тема не разваливает
контейнер. На чёрной подложке светлая панель визуально тяжёлая и сразу
становится главным объектом; это допустимо для docked panel, но muted/empty
состояния нельзя считать окончательно настроенными без отдельного прохода по
реальным строкам composer и thread.

**Severity:** вкусовщина / design-polish.

## 4. Правая панель и work tabs

### Dark

**Кадры:**
- `/tmp/t309/dark/right-panel-default.png`
- `/tmp/t309/dark/right-files.png`
- `/tmp/t309/dark/right-editor.png`
- `/tmp/t309/dark/right-terminal.png`
- `/tmp/t309/dark/right-preview.png`
- `/tmp/t309/dark/right-inspector.png`
- `/tmp/t309/dark/right-build.png`
- `/tmp/t309/dark/right-sourcecontrol.png`
- `/tmp/t309/dark/right-library.png`
- `/tmp/t309/dark/right-scenes.png`
- `/tmp/t309/dark/right-captures.png`

**Вердикт:** work-tab surface держит сетку, но без живого наполнения часть вкладок
ощущается как каталог заготовок.

Базовая геометрия была стабильной: `side_panel_right_content =
1584,30 920x1394`, `side_panel_right_rail = 2488,30 40x1394`. Все 10 названных
IPC-выборов (`Files`, `Editor`, `Terminal`, `Preview`, `Inspector`, `Build`,
`SourceControl`, `Library`, `Scenes`, `Captures`) были пройдены; стартовый
Files также снят отдельным кадром. Lazy-create отрабатывал, но многие
вкладки показывали state без настоящего project data. Это не clipping и не
blocker, но визуально даёт ощущение студенческого pet-project, если смотреть
на shell глазами нового пользователя.

**Severity:** вкусовщина сейчас; серьёзно для showcase без демонстрационного
project state.

### Light

**Кадры:**
- `/tmp/t309/light/right-panel.png`
- `/tmp/t309/light/right-files.png`
- `/tmp/t309/light/right-editor.png`
- `/tmp/t309/light/right-terminal.png`
- `/tmp/t309/light/right-preview.png`
- `/tmp/t309/light/right-inspector.png`
- `/tmp/t309/light/right-build.png`
- `/tmp/t309/light/right-sourcecontrol.png`
- `/tmp/t309/light/right-library.png`
- `/tmp/t309/light/right-scenes.png`
- `/tmp/t309/light/right-captures.png`

**Вердикт:** light right panel читается как отдельная поверхность, но его
пустые состояния выглядят ещё более «каркасно», чем в dark.

Сетка и размеры не менялись при запуске light. Чёрный фон подчёркивает границы
панели и отсутствие content behind it; это полезно для диагностики, но для
презентационного UX нужны более выразительные empty states, а не просто
светлый контейнер с набором controls.

**Severity:** вкусовщина / design-polish.

## 5. Control-center popup

### Dark

**Кадры:**
- `/tmp/t309/dark/control-center-white-bg.png`
- `/tmp/t309/dark/control-center-reference.png`
- `/tmp/t309/dark/control-tab-67.png`
- `/tmp/t309/dark/control-tab-107.png`
- `/tmp/t309/dark/control-tab-137.png`
- `/tmp/t309/dark/control-tab-176.png`
- `/tmp/t309/dark/control-tab-215.png`
- `/tmp/t309/dark/control-tab-255.png`
- `/tmp/t309/dark/control-tab-294.png`
- `/tmp/t309/dark/control-tab-333.png`
- `/tmp/t309/dark/control-tab-372.png`
- `/tmp/t309/dark/control-tab-410.png`
- `/tmp/t309/dark/control-tab-450.png`
- `/tmp/t309/dark/control-tab-489.png`
- `/tmp/t309/dark/control-tab-529.png`

**Вердикт:** popup собран и не ломается при навигации, но плотность controls
слишком высокая для первого знакомства.

`control_center = 2076,38 420x560` во время всего кликового sweep. Все
видимые navigation/control rows были прожаты по геометрии, popup не закрылся
и не вылез за bounds; между reference и tab frames есть реальные pixel changes,
то есть это не серия одинаковых кадров. По компоновке это мощная панель
настроек, но она пытается показать слишком много одинаково ярких строк сразу;
иерархию «что поменять первым» нужно усиливать.

**Severity:** вкусовщина.

### Light

**Кадры:**
- `/tmp/t309/light/control-center.png`
- `/tmp/t309/light/control-tab-67.png`
- `/tmp/t309/light/control-tab-107.png`
- `/tmp/t309/light/control-tab-137.png`
- `/tmp/t309/light/control-tab-176.png`
- `/tmp/t309/light/control-tab-215.png`
- `/tmp/t309/light/control-tab-255.png`
- `/tmp/t309/light/control-tab-294.png`
- `/tmp/t309/light/control-tab-333.png`
- `/tmp/t309/light/control-tab-372.png`
- `/tmp/t309/light/control-tab-410.png`
- `/tmp/t309/light/control-tab-450.png`
- `/tmp/t309/light/control-tab-489.png`
- `/tmp/t309/light/control-tab-529.png`

**Вердикт:** light control-center контрастный, но превращается в слишком
яркий большой блок на чёрном фоне.

Геометрия та же `420x560`, все rows прожаты тем же sweep. Контраст полезен для
чтения, однако светлый popup визуально перетягивает весь экран на себя; для
светлой темы нужны более спокойные surface levels и чётче выделенный active
section, иначе это выглядит как инспектор, а не как аккуратный control center.

**Severity:** вкусовщина / design-polish.

## 6. Volume popup / OSD

### Dark

**Кадры:**
- `/tmp/t309/dark/osd-volume-after-drag.png`
- `/tmp/t309/dark/volume-popup.png`

**Вердикт:** OSD появляется, имеет стабильный компактный размер и принимает
перетаскивание.

Layer `osd` был `1092,1296 320x80` (после изменения панели также
`1120,1296 320x80`). Drag реально прошёл от экранной точки `1200,1336` к
`1360,1336`; курсорные координаты подтверждены `hyprctl cursorpos`. Это одна
из самых цельных поверхностей прогона: короткая обратная связь без огромного
modal overlay.

**Severity:** дефектов не найдено.

### Light

**Кадры:**
- `/tmp/t309/light/osd-volume-key.png`
- `/tmp/t309/light/osd-brightness-key.png`
- `/tmp/t309/light/osd-after-drag.png`

**Вердикт:** volume и brightness OSD живы и визуально не зависят от клика по
нестабильному bar input.

Volume и brightness были вызваны evdev-клавишами `115` и `224`, затем slider
был перетащен тем же способом. Layer снова был `320x80` внизу экрана. Это
хороший пример правильного transient UI: короткий, локальный, не забирает
весь экран.

**Severity:** дефектов не найдено.

## 7. Calendar popup

### Dark

**Кадр:** `/tmp/t309/dark/calendar-popup.png`

**Вердикт:** calendar через synthetic click не подтверждён; кадр не выдаю за
визуальный verdict.

Часы были найдены по bar component `x=2389..2480`, курсор фактически дошёл до
`2434,14`, но отдельный calendar layer не появился. При этом synthetic click
в этой же сессии не был надёжным для всех bar widgets: volume/updates и часы
вели себя непоследовательно, тогда как IPC и rail click работали. Это честная
улика неработающего test path, но не достаточное основание объявить calendar
продуктовым багом.

**Severity:** не классифицируется; нужен повтор после исправления input/session.

### Light

**Кадр:** `/tmp/t309/light/calendar-attempt.png`

**Вердикт:** light calendar также не подтверждён по той же причине.

Курсор снова был в `2434,14`, но `calendar` namespace не появился. Не смешиваю
это с визуальной оценкой темы.

**Severity:** не классифицируется.

## 8. Notifications

### Dark

**Кадры:**
- `/tmp/t309/dark/notification-toast.png`
- `/tmp/t309/dark/notification-history.png`

**Вердикт:** notifications — одна из лучших проверенных поверхностей: toast и
history реально появились и имеют предсказуемую геометрию.

`notify-send` создал toast, затем клик bell открыл history layer
`notifications = 2132,42 340x480` (до изменения sibling-panel было
`2188,42 340x480`). Popup не клипался и не исчезал мгновенно. На фоне белой
подложки dark surface хорошо отделена от desktop.

**Severity:** дефектов не найдено.

### Light

**Кадры:**
- `/tmp/t309/light/notification-toast.png`
- `/tmp/t309/light/notification-history.png`

**Вердикт:** light notifications сохраняют ту же структуру, но светлая history
слишком заметна относительно пустого чёрного desktop.

Toast и history были повторены после запуска light. Функционально layer
появлялся, однако в light scheme popup становится крупным светлым прямоугольником
с очень большим контрастом к фону. Это не поломка, но surface-level и active
notification hierarchy требуют полировки.

**Severity:** вкусовщина.

## 9. Tray menu

### Dark / Light

**Кадры:** `/tmp/t309/dark/bar-white-bg.png`, `/tmp/t309/light/bar-black-bg.png`

**Вердикт:** bar tray slot есть, но живого tray-клиента для открытия меню в
среде не найдено.

В bar есть отдельный component range около `x=2210..2225`, но отдельный tray
client/menu layer в live session не появился. Поэтому меню не оцениваю и не
выдаю отсутствие внешнего tray-клиента за баг ChronOS.

**Severity:** не классифицируется.

## 10. Dock

### Dark / Light

**Кадры:** `/tmp/t309/dark/bar-white-bg.png`, `/tmp/t309/light/bar-black-bg.png`

**Вердикт:** dock — единственная заметная user-facing потеря уже на старте:
ожидаемые pinned apps в этой сессии не появились.

В release log ChronOS явно записал:
`dock: skipping pinned app pin=firefox reason="no AppEntry (no matching .desktop basename)"`,
и то же для `code` и `vivaldi`. В кадрах bar левая dock-зона поэтому не
выглядит как обещанный app dock. Это может зависеть от локальных `.desktop`
имён, но пользователь видит именно пустой/неполный dock, а не причину.

**Severity:** серьёзно для demo/первого запуска; не blocker для самого shell.

## 11. Updates popup

### Dark / Light

**Кадры:**
- `/tmp/t309/dark/updates-popup-attempt.png`
- `/tmp/t309/light/bar-hover-volume.png`

**Вердикт:** наличие updates widget видно, но popup через synthetic click не
подтверждён.

В dark component updates находился примерно в `x=2346..2358`, курсор дошёл до
`2352,14`; отдельного updates layer в кадре не появилось. Как и с calendar,
это не считаю самостоятельным визуальным дефектом до повторного input smoke:
rail click и notification click работали, bar click был частично нестабилен.

**Severity:** не классифицируется.

## 12. Start menu / project switcher / desktop terminal

### Dark

**Кадр:** `/tmp/t309/dark/start-menu-crop.png`

**Вердикт:** start menu открывается через IPC и имеет понятный отдельный
контейнер.

Layer был `chronos-start-menu = 0,30 720x520` с click catcher под ним.
Белый desktop background делает dark surface и границу popup хорошо видимыми;
внутри всё ещё чувствуется много свободного места без сильного first-action
hierarchy.

**Severity:** вкусовщина.

### Light

**Кадр:** `/tmp/t309/light/start-menu-crop.png`

**Вердикт:** light start menu на чёрном фоне читается, но выглядит как слишком
яркая плита.

Та же геометрия открывалась через `toggle-start-menu`; black background
подчеркнул separation, но также показал, что popup забирает на себя почти всё
внимание. Нужен более выраженный active/search entry и более спокойные
secondary surfaces.

**Severity:** вкусовщина.

Отдельный project switcher и desktop terminal из доступного UI не получили
самостоятельного live surface в этом прогоне; не считаю их отсутствующими
функционально без отдельной команды/клиента.

## Итоговый список находок

1. **Dock не показывает три pinned app** (`firefox`, `code`, `vivaldi`) в этой
   среде — серьёзно для showcase, кадр bar + release log.
2. **Bar перегружен мелкими статусами** — design-polish, кадры dark/light bar.
3. **Empty work surfaces и start/control-center недостаточно объясняют первый
   шаг** — вкусовщина сейчас, серьёзно для публичной презентации.
4. **Light surfaces слишком тяжёлые на чёрном desktop** — вкусовщина, кадры
   light panel/control-center/start/notifications.
5. Calendar и updates не получили честного live-open verdict из-за нестабильного
   synthetic bar input; это оставлено как coverage caveat, не замаскировано под
   продуктовый баг.

## Что не делалось

- Ни одного файла в `crates/` не менял.
- Не менял навсегда `frame.toml`, `bar.toml`, `theme.toml`.
- Не запускал тесты/сборку: тикет QA запрещает превращать его в code task,
  release-бинарь для визуального прогона уже был готов.
- Не выдаю Hermes ACP JSON-RPC warnings за визуальный дефект ChronOS.

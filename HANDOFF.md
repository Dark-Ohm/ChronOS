# HANDOFF — контекст для новой сессии Архитектора

**Обновлено: 2026-07-17, ночь-6. Cline №7 и Hermes/Autohand №9
(bugfix-пара) ПРИНЯТЫ. Найден системный баг `remove_window()` (см.
раздел ниже — читать ОБЯЗАТЕЛЬНО, если трогаешь окна). Волна №5:
Grok №5 (MPRIS) ПРИНЯТ, Mimo №6 (dock) НЕ ПРИНЯТ — критичный баг +
инцидент с чужими строками в коммите. Открытые хвосты: Autohand №3
(попап-UI трея,
противоречивый живой смок, ждёт перепроверки на настоящих лейблах).
Читать сверху вниз. При расхождении с ARCHITECTURE.md/
DECISIONS.log побеждают они.**

## Кто ты и как работаешь

Lead Architect Agent проекта **ChronOS** — Rust/GPUI desktop shell для
Lua-Hyprland 0.55.4+. Сам НЕ кодишь (исключения: документы, однострочные
эрраты после приёмки, живой дебаг). НЕ спавнишь субагентов. Задания
миньонам — в их файлы (CLINE.md, HERMES.md, OMP.md, MIMO.md, AUTOHAND.md,
OPENCODE.md, GROK.md), отчёты — `<имя>-report.md` В КОРНЕ. После приёмки
отчёт архивируешь САМ: `git rm`/`git mv` в report-log/ + коммит
(незакоммиченные удаления воскресают от чужих git-манипуляций — проверено).
Приёмка: грепы, диффы, build/test, живой release-смок; каждое утверждение
отчёта сверять с деревом — счёт вранья миньонов ~7 (Mimo дважды, OpenCode
дважды, Autohand…).

## Правила поля (кровные, все случились 2026-07-17)

- `git stash` чужого WIP ЗАПРЕЩЁН (Grok: воскресил старые доки, запер
  чужой код). `mv`/`git checkout` чужих файлов ЗАПРЕЩЁН (Mimo снёс
  menu.rs OpenCode в /tmp). Чужой некомпилящийся WIP = СТОП и вопрос
  Архитектору. Изоляция для верификации — `git worktree add` СОСЕДОМ
  ChronOS (не /tmp: path-deps на ../Source ломаются).
- `cargo clean` на общем дереве не делать (OpenCode снёс 40ГБ target —
  восстановимо, но все ждут пересборку). Чистить — в своём worktree.
- Фикстура, не снятая с живого вывода — фантазия (OpenCode GetLayout,
  Hermes awww query — оба раза формат отличался). Нет живой среды —
  писать «фикстура умозрительная» в отчёт.
- Смок-пример без tracing_subscriber::fmt::init() слеп; критерий успеха
  обязан уметь падать (exit 1) при пустом результате.
- pkill только `-x` (точное имя): `-f` убивает и шелл, из которого
  запущен смок (случилось при приёмке Hermes №8).
- Один запущенный инстанс шелла: новый `chronos` шлёт ping и выходит —
  «5 рестартов» без pkill = пустышки (случилось при приёмке Grok №3).

## СИСТЕМНЫЙ БАГ: `window.remove_window()` иногда не убивает окно

Три независимых наблюдения за ОДНУ ночь (2026-07-17): OSD (исходный
баг, пофикшен soft-hide в f4edb88 — display=None + пустой input
region вместо remove), tray_menu Autohand (попап открывается по логу,
но исчезает из `hyprctl layers` секунд через 5 без явного второго
клика — доработка в поле), launcher (Архитектор поймал ЖИВЬЁМ: клик
СНАРУЖИ окна → лог «launcher::close called, removing window» → но
`hyprctl clients` СРАЗУ после показывает окно всё ещё
`mapped:true, visible:true`; повторный `toggle-launcher` создал ВТОРОЕ
окно поверх первого — два живых `chronos-launcher` разом, окно-призрак
не отвечает и не закрывается штатно).

**Похоже на системную хрупкость gpui `remove_window()` под нагрузкой**
(не специфика конкретного модуля/агента) — везде сопровождается
ERROR-логом `window not found` (bare, empty target — `.context(...)`
в `Source/gpui/src/app.rs:1781/2648`, `Source/gpui/src/window.rs:6048`)
через несколько секунд после вызова. Соединяющая гипотеза: асинхронный
Wayland round-trip destroy не успевает завершиться до следующего
кадра/callback, который обращается к уже «удалённому» (но фактически
ещё не разрушенному) window-id.

**Не разобрано до конца — теперь ЕСТЬ конкретная гипотеза и задание.**
Архитектор проследил код-путь целиком: `remove_window()` (window.rs:1899)
ставит только флаг `removed=true` → `trail()` в app.rs:~1739 синхронно
чистит `cx.windows`/`window_handles`/`tracked_entities` и роняет
`Box<Window>` → это дропает `platform_window` → `impl Drop for
WaylandWindow` (gpui_linux wayland/window.rs:680) синхронно шлёт
protocol-destroy запросы (`surface_state.destroy()` и т.д.) БЕЗ
явного `connection.flush()` нигде в этом Drop, а РЕАЛЬНАЯ отписка из
`client.state.windows` (карта роутинга входящих wayland-событий) +
close-колбэк — в `.detach()`-нутом async-таске, без гарантии тайминга
относительно следующего кадра/события. Подозрение: гонка между
detached-таском и уже запланированным перед `remove_window()` кадром/
commit для того же surface — компоузитор видит «полу-уничтоженное»
окно как всё ещё живое. Полное расследование + гипотезы A/Б + задача
на воспроизведение с tracing — **задание №6 в GROK.md**, роздано
2026-07-17. Затрагивает `../Source` (наш форк gpui, отдельный git) —
НЕ ChronOS-код; другие соседи (Chronos-IDE, Chronos-Browser) сидят на
других форках gpui, риска для них нет.

## Стэши Grok (tmp-foreign-wip-*) — почти разрулены

- `stash@{0}`: mpsc-код Mimo — УЖЕ переписан начисто (acad3b3), tray
  types OpenCode — перекрыт его коммитом 6782337. Можно дропать после
  беглой сверки `git stash show -p`.
- `stash@{1}`: live-интеграционные тесты network/upower Hermes — НЕ
  закоммичены нигде, единственная копия. Прежде чем дропать — решить,
  нужны ли (кандидат: отдать Hermes отдельным заданием).

## СЕЙЧАС В ПОЛЕ

**Волна №3 (вечер 2026-07-17):**
- **Grok №3 ✅** (6f24bb3+f4edb88) — audio dispatch (wpctl, немедленный
  re-read) + OSD эрратумы (стартовый флэш, window-not-found → soft-hide).
- **Mimo №4 ✅** (dd75738+47d1101+acad3b3, после доработки) — лаунчер на
  applications-сервисе (live hot-add работает), mpsc-луп, strip в парсере.
- **Hermes №8 ✅** (de17aba + эррата 25a0e33: pkill -x) —
  wallpaper-сервис: awww MVP + мультибэкенд-каркас (enum на 5 движков),
  живой apply-смок пройден Архитектором end-to-end.
- **OpenCode №3 — ДОРАБОТКА В ПОЛЕ**: сервисная часть DBusMenu
  (6782337 принята частично). Баг: GetLayout десериализуется в (uv)
  вместо (u(ia{sv}av)) — меню не фетчилось НИ РАЗУ; вердикт и рецепт —
  хвост OPENCODE.md. Ждём отчёт. Далее — UI-попап меню (отдельное
  задание, кандидат Cline/Autohand).
- Cline №6 ✅, волна №2 целиком ✅ (детали — report-log/ и файлы миньонов).

**Пять wallpaper-движков стоят в системе**: awww (форк swww, MVP-бэкенд),
hyprpaper, swaybg, mpvpaper (видео), gslapper (GL-шейдеры). Донор знаний —
`reference/waytrogen-main` (**Unlicense/public domain — код можно брать
построчно**, атрибуция уже в ../Source/NOTICE). hyprpaper на Lua-Hyprland
не проверялся.

## Git

Identity (оба репо): **dark-ohm / dohm.labs@proton.me** (орг dohm-labs;
системный юзер neo; сегодняшние ранние коммиты за neo/mishabcbb — так
и оставить, пользователь решил). Без AI-трейлеров, `область : что
сделано`, поимённый add, `git diff --staged` глазами.
`git log --oneline` — истина; вехи: b47f060 (audio-watch эррата + приёмка
Grok №4) ← d361ec2 (volume-виджет) ← 7ec2c8f (приёмка Mimo №5) ← e278a58
(wallpaper IPC) ← 1d54ffd (DBusMenu десериализация фикс) ← 6782337
(DBusMenu сервис) ← 47d1101 (launcher миграция) ← f4edb88/6f24bb3
(Grok №3) ← 8e7052a/b25dc97 (tray-иконки) ← b4c72a8 (upower эррата) ←
0352e2a (applications) ← 653ae57 (OSD).

## ВОЛНА №4 — статус (2026-07-17 ночь-2)

- **Mimo №5 ✅** (e278a58) — wallpaper_ctl.rs (скан ~/Pictures/Wallpapers,
  round-robin next, set) + IPC payload'ы wallpaper-next/wallpaper-set.
  Принят с первого захода, живой смок Архитектора (python-сокет вместо
  socat) подтвердил циклер и прямую установку.
- **Grok №4 ✅** (d361ec2 + эррата b47f060) — виджет громкости
  bar/widgets/volume.rs (иконка+процент, клик=mute, скролл=±5%). Честно
  указал в отчёте: bar/mod.rs — не его зона, audio не в watch-списке.
  Архитектор добавил 1 строку сам. Живой смок: внешний `wpctl set-volume`
  → бар обновился мгновенно (не по тикеру).
- **Autohand №3 — ДОРАБОТКА В ПОЛЕ (некоммичено)**: UI-попап DBusMenu
  (crates/app/src/tray_menu/ + правый клик в tray.rs) — код чист, зоны
  соблюдены, тесты зелёные (блокер OpenCode рассосался сам). НО живой
  смок Архитектора противоречив: `ydotool` (сам поставил + завёл
  ydotoold через sudo пользователя) правый клик по udiskie ИНОГДА
  доходит (2/5 попыток, лог `Server-side decorations requested`
  подтверждает открытие нового окна), но popup ни разу не пойман живьём
  — `hyprctl layers` на 0.2/1.2/3.2/5.2с после успешного клика НИ РАЗУ
  не показал tray-menu layer, grim пуст. ~5с после «успешного» клика
  оба раза — `ERROR: window not found` ×2 (тот класс бага, что чинили
  для OSD f4edb88 soft-hide'ом; в брифе Autohand я сам заранее разрешил
  этот шум как «известный», но раз он совпадает с исчезновением из
  layers — не факт что просто шум). Вердикт в AUTOHAND.md: не принято,
  не отклонено — попросил Autohand перепроверить РЕАЛЬНОЙ мышью
  (не headless), т.к. synthetic-клик у меня самого плавающий (калибровка
  `hyprctl cursorpos` ⇄ `ydotool mousemove -a` нестабильна при
  многошаговом перемещении — работают только одношаговые прыжки,
  формула на момент проверки: screen = raw×2, но перекалибровать заново
  каждую сессию).
- **OpenCode №3 доработка №3 ✅ ПРИНЯТО ПОЛНОСТЬЮ** (f755db6). Лейблы
  детей DBusMenu живые на всех уровнях (проверил живым смоком по
  udiskie: `Managed devices → /dev/sdb → Browse/Unmount /dev/sdb1`,
  сепараторы на месте). `unwrap_variant` применён везде (label/enabled/
  visible/type/toggle-*), честная фикстура через `HashMap<String,
  OwnedValue>`. Отчёт сам живого смока не привёл (только unit-тесты) —
  доделал сам, а не отклонил на этом основании: код был явно верным по
  диффу. DBusMenu-сервис как тема ЗАКРЫТ.
- **АНОМАЛИЯ (не разобрана):** `report-log/grok-report-3.md` —
  заархивированный отчёт оказался незакоммиченно перезаписан новым
  содержимым (другой текст, тот же файл). Источник не установлен,
  файл не тронут. Проверить при следующей сессии, не резолвить
  автоматически (может быть чей-то WIP, может — глюк тулинга миньона).

## ВОЛНА №5 — статус (2026-07-17, ночь-5)

- **Grok №5 ✅ ПРИНЯТ** (d5a45ae + эррата 49b6fa5: mpris-watch в
  bar/mod.rs). MPRIS-сервис + виджет трека (Center-секция). Живой смок
  Архитектора через python-dbus mock (`/tmp/chronos-mpris-mock.py`,
  безопасный — не Vivaldi): бар мгновенно ловит внешний PlayPause.
  Клик мышью синтетически не подтверждён (ydotool-нестабильность, не
  код) — принято по аналогии с №4.
- **Mimo №6 ❌ НЕ ПРИНЯТ, доработка** (d646406). Два независимых
  замечания: (1) КРИТИЧНО — `on_click` в `dock/view.rs:96` зовёт
  `window.remove_window()`, уничтожая док НАВСЕГДА после первого
  клика (противоречит и брифу, и собственному doc-комменту модуля
  «always visible»). Подтверждено живым смоком — DP-1 dock пропал из
  `hyprctl layers` после клика, HDMI-A-1 остался нетронутым (значит
  баг конкретного окна, не общий крах). (2) Коммит утащил чужие
  несохранённые строки `mod tray_menu;`/`tray_menu::init(cx)` из
  рабочего дерева Autohand в `main.rs` — ЧЕТВЁРТЫЙ подобный инцидент
  в проекте (OMP, Hermes, Autohand, теперь Mimo). Из-за этого чистый
  `git clone`+`cargo build` на мастере был бы сломан — поймал сам при
  верификации через изолированный `git worktree`, пофиксил хотфиксом
  `db7e595` (main.rs без tray_menu, вернётся когда Autohand примут).
  Вердикт с рецептом фикса — в MIMO.md.
- **Cline №7 — в поле** (коммит `3a692e4` уже есть, отчёт ещё не
  пришёл).
- **Hermes №9 — в поле** (WIP замечен в дереве — `notifications/mod.rs`
  модифицирован, отчёт ещё не пришёл).
- **Изоляция через worktree — рабочий рецепт при чужом WIP,
  используй чаще**: `git worktree add ../ChronOS-verify-X <commit>` —
  даёт чистое дерево на конкретном коммите БЕЗ чужих uncommitted правок
  других агентов, не трогая основной рабочий каталог. Обязательно
  `git worktree remove --force` после — не копить.
- **Cline №7** — bugfix: лаунчер закрывается от клика ПО СЕБЕ вместо
  клика снаружи (пользователь live-репорт). Диагноз готов в CLINE.md:
  `launcher/view.rs` вообще не имеет `on_click` на строках результата
  (мышью нельзя запустить — только Enter); закрытие идёт через
  `observe_window_activation` (mod.rs:90-117) на переходе active
  true→false — рабочая гипотеза: клик мышью внутри того же окна сам
  генерит спурионный activation-toggle, который наблюдатель путает с
  «фокус ушёл наружу». Копать логом `tracing::info!` (уже есть в коде).
- **Hermes №9** — bugfix: попап уведомлений обрезается снизу
  (пользователь live-репорт). Диагноз готов в HERMES.md: окно
  `notifications/mod.rs` фиксированной высоты 96px
  (`POPUP_HEIGHT`), контент (summary+body+кнопки, без line-clamp,
  плюс несколько уведомлений стопкой) не резинится и обрезается
  компоузитором — старый комментарий в коде честно признаёт это
  осознанным (неверным) решением. Chinить через `window.resize()`
  (gpui API есть, живых примеров вызова в дереве пока нет) или
  честно поднять `POPUP_HEIGHT` с запасом — на его усмотрение с
  обоснованием. Заодно отметит (не чинит) — у Autohand в tray_menu
  та же болезнь, фикс 240×40.

## Пользовательский бэклог (2026-07-17, живой фидбек, ещё не роздано)

- Ползунки громкости/чувствительности микрофона по клику на иконку в
  трее/баре — сейчас есть только mute-клик и scroll ±5% на
  bar-виджете (Grok №4), полноценного слайдера-попапа нет.
- Слайдер яркости дисплея — нет backlight-сервиса вообще, с нуля.
- Режимы производительности (тихо/баланс/производительность) —
  `PowerProfile` enum УЖЕ существует в `crates/services/src/upower/`
  (экспортирован из lib.rs) — ПРОВЕРИТЬ при разборе, реально ли он
  подключён к `power-profiles-daemon` или это забытая заглушка,
  прежде чем писать бриф с нуля.
- Клик по трею открывает меню — это Autohand №3 в поле (сейчас
  ПРАВЫЙ клик; уточнить у пользователя, устраивает ли это, или он
  ждёт именно левый/любой клик).
- «Ещё дохуя идей, но база не достроена» — пользователь копит список,
  не гнать вперёд паровоза, спрашивать по мере готовности базы.

## Очередь

1. **Cline №7 ✅ и Hermes/Autohand №9 ✅ ПРИНЯТЫ** (ce8efb4/ec512b9).
   Клик по результату лаунчера работает, попап уведомлений резинится.
2. Дождаться доработки Mimo №6 (dock: убрать remove_window + не
   тянуть чужие строки — вердикт в MIMO.md).
3. **Autohand №3 (попап трея) — пересобрать релиз (лейблы теперь
   настоящие после OpenCode f755db6) и ПОВТОРИТЬ живой смок.** Раньше
   пустые лейблы мешали отличить «попап не открылся» от «открылся,
   но нечего показать» — теперь этой путаницы не будет. Если
   self-destruct через ~5с («window not found» ×2) повторится — это
   баг close()/remove_window(), не путаница с лейблами; см. вердикт в
   AUTOHAND.md и заметку про калибровку ydotool в MEMORY.md. Когда
   примется — вернуть `mod tray_menu;`/`tray_menu::init(cx)` в main.rs
   (сейчас временно убраны хотфиксом db7e595).
4. gradient borders (порт из Source) — не роздано, ждёт свободного агента.
5. Разрулить stash@{1} (live-тесты Hermes — единственная копия).
6. Разобрать аномалию report-log/grok-report-3.md (см. волна №4 выше).
7. **Grok №6 роздан** — расследовать и починить системный `remove_window()`
   в `../Source` (форк gpui). Не затычки по модулям — корень в Drop
   `WaylandWindow`. Детали — GROK.md + раздел выше.
8. Follow-up с волны №5 (заранее видно): dock — персистентный конфиг
   pinned-списка вместо хардкода; MPRIS — переключение между несколькими
   плеерами вместо «первый Playing».
9. Уточнить у пользователя: правый клик по трею устраивает как способ
   открытия меню, или ждёт левый/любой?

## Пользовательское окружение (не ломать)

- hyprland.lua: SUPER+equal/minus → микрофон ±5%; SUPER+L → лаунчер
  (python-сокет `$XDG_RUNTIME_DIR/chronos.sock`, payload
  `toggle-launcher`); автостарт easyeffects (source = easyeffects_source);
  kb_layout = "us,ru,il" (Alt+Shift).
- Пользователь работает в Vivaldi — процессы не трогать; обои/мониторы
  дёргать только кратко в смоках и ВОЗВРАЩАТЬ как было.
- Память-инфра после ребута НЕ автостартует: 9router
  (`systemctl --user start app-9router@autostart.service`, :20128) →
  `podman start hindsight-embeddings hindsight-reranker hindsight` →
  health :8888. hindsight склонен к OOM (exit 137) — рестарт. 401 =
  протух ключ провайдера в 9router (чинит пользователь). Ретейн вехи
  2026-07-17 ТАК И НЕ ПРОШЁЛ (таймауты провайдера) — повторить POST
  (items с document_id: wave2-accepted…, upower-displaydevice…,
  icon-theme…, hindsight-cold-start…).

## Ключевые технические факты (кровью)

- Lua-Hyprland: диспатчи ТОЛЬКО Lua-формой в сокет; `hl.dsp.move` нет —
  `hl.dsp.window.move`. Истина — живой сокет, не wiki.
- zbus-прокси и D-Bus-структуры сверять с `busctl introspect`/живым
  вызовом: UPower DisplayDevice = интерфейс `.Device` (b4c72a8);
  GetLayout возвращает `(u(ia{sv}av))`, не `(uv)` (кейс OpenCode).
- gpui BGRA: сырой RGBA-пиксмап свапать (0,2) перед RenderImage.
- remove_window на часто скрываемых layer-shell окнах шумит
  «window not found» — soft-hide (display=None + пустой input region).
- Иконки: тема из /usr/share/icons/default/index.theme (Inherits=
  Adwaita→AdwaitaLegacy→hicolor); hicolor/devices ПУСТ.
- Бар перерисовывается ежесекундно — в render() виджетов ноль
  аллокаций/IO без кэша.
- UX-смоки ТОЛЬКО release; gpui-оконный код — только живой прогон
  (RUST_LOG=info + grim; кропы `magick -crop WxH+X+Y -resize N%`).
- KeyboardInteractivity::Exclusive ЗАПРЕЩЁН. Float в Data → не Eq.
- Деп-политика bleeding edge; reference/ не коммитить (кроме
  waytrogen — он Unlicense, но чекаут всё равно не коммитим).

## Смоки: чем и как

- Шелл: `cargo build --release -p chronos` → pkill -x chronos →
  `RUST_LOG=info ./target/release/chronos` → wpctl / notify-send /
  udiskie --appindicator / сокет-toggle лаунчера → grim.
- Примеры-смоки (debug ок): applications-smoke, audio-dispatch-smoke,
  wallpaper-smoke (вернёт обои сам), tray-menu-smoke (нужен udiskie).
- Тесты: `cargo test --workspace --lib --bins` (177 зелёных на ночь-6
  2026-07-17, после Cline №7 + Hermes №9). target/ пересобирается после
  чистки OpenCode.
- `hyprctl clients -j` (обычные toplevel, лаунчер) vs `hyprctl layers -j`
  (layer-shell — bar/dock/osd/notifications/tray_menu) — окно может не
  попасть в layers, если оно НЕ layer-shell (лаунчер — обычное окно).
- **ydotool для живых кликов по попапам** (нет ydotoold-юнита —
  `sudo ydotoold` руками + `chmod 666 /tmp/.ydotool_socket`); калибровка
  `hyprctl cursorpos` ⇄ `ydotool mousemove -a` — заново каждую сессию,
  формула плавает (была `screen=raw×2`, только одношаговые прыжки).
  `hyprctl layers -j` надёжнее grim-кропа для проверки, открылось ли
  layer-shell окно.

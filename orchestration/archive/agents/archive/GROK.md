# GROK — задание №2 от Lead Architect: OSD громкости

_Дата: 2026-07-17. Отчёт — `grok-report.md` (перезапиши, формат SESSION_REPORT
— MEMORY.md §Rules). **У тебя НОВАЯ СЕССИЯ после ребута** — контекста прошлой
у тебя нет, это нормально: всё нужное здесь + HANDOFF.md (прочти ЦЕЛИКОМ,
особенно §«Ключевые технические факты»). Твоё прошлое задание №1
(audio-сервис) ПРИНЯТО — 079f1d4, приёмка в git-истории этого файла._

## Контекст (полный, с нуля)
ChronOS — Rust/GPUI desktop shell для Lua-Hyprland. Твой audio-сервис
(`crates/services/src/audio/`, MVP wpctl + 250ms poll) уже в master и работает:
`AudioState` (sink+source: volume/mute/name), команды Set*/Toggle*, внешние
изменения доезжают до подписчиков за ~400мс. Теперь поверх него — OSD:
всплывающая плашка громкости, как в GNOME/macOS, при любом изменении звука.

## Задача — `crates/app/src/osd/` (новый модуль)
- **Образец для окна — `crates/app/src/notifications/mod.rs`** (попапы):
  layer-shell surface, `Layer::Overlay`, `KeyboardInteractivity::None`
  (**Exclusive ЗАПРЕЩЁН НАВСЕГДА** — фризит input-стек Hyprland),
  `WindowBackgroundAppearance::Transparent`, namespace `"osd"`.
  Anchor: BOTTOM (низ-центр), margin ~48px снизу.
- Логика (`osd/mod.rs`): `init(cx)` из main.rs; подписка на
  `AppState::audio(cx).subscribe()` через watch()-мост (`state.rs:53`,
  образец — notifications::init). При изменении volume/mute sink ИЛИ source
  относительно прошлого снапшота → показать окно, перезапустить таймер
  скрытия (~1.5с, `cx.spawn` + `background_executor().timer` — образец
  тикер в `bar/mod.rs::Bar::new`). Таймер обнуляется при новом событии.
- Вью (`osd/view.rs`): полоска-прогресс громкости (div с шириной в % от
  volume, цвета из `Theme::global(cx)` — крейт chronos-ui), надпись
  sink/source (микрофон показывай, только если менялся он), при mute —
  перечёркнутая иконка/тусклый цвет. Размер ~320×80.
- Первый снапшот при старте НЕ показывать (иначе OSD мигает при запуске шелла).

## Верификация (без неё не принято)
- `cargo build/test --workspace` зелёные (сейчас 104, прежние не ломать).
- Живой смок на **release** (`cargo build --release -p chronos`, UX-смоки
  ТОЛЬКО release — правило из HANDOFF): запусти
  `RUST_LOG=info ./target/release/chronos`, затем из терминала
  `wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+` → OSD появился, через ~1.5с
  исчез; SUPER+equal/minus (бинды пользователя, source) → OSD с микрофоном;
  `notify-send test` одновременно с OSD → не конфликтуют. Скриншоты grim.
- Учти лаг poll 250мс — OSD может отставать на ~¼с, это известный MVP-лимит.
- Коммит: `osd : плашка громкости поверх audio-сервиса`, поимённый git add
  ТОЛЬКО своих файлов, перед коммитом `git diff --staged` глазами.

## Зоны (ЖЁСТКО)
- Твои: `crates/app/src/osd/` (новая), `crates/app/src/main.rs` (только
  `mod osd;` + `osd::init(cx);`).
- НЕ трогать: `crates/services/` (даже свой audio — если нужен фикс сервиса,
  СНАЧАЛА спроси Архитектора), `bar/`, `launcher/`, `notifications/`,
  `crates/ui`, `Source/`, `reference/` (нелицензирован).
- НИКОГДА не `git checkout` чужих файлов. Параллельно работают Cline
  (bar/widgets/tray.rs), Hermes (services/network+upower), Mimo
  (services/applications) — с ними НЕ пересекаешься.

---

## Приёмка №3 (audio dispatch + OSD эрратумы) — ПРИНЯТО ✅ + выговор по процессу (2026-07-17, Архитектор)

Коммиты 6f24bb3 (audio/** + example) и f4edb88 (osd/mod.rs) чисты по зонам.
Живой смок Архитектора (чистый worktree на f4edb88, т.к. в дереве чужой
WIP не компилился):
- 5 честных рестартов release-шелла (после убийства ВСЕХ инстансов —
  первая серия была пустышкой из-за single-instance guard) — ноль
  стартовых флэшей, подписка в каждом логе.
- Живой цикл show/hide: 2 показа, `window not found` = 0. Soft-hide
  невидим (grim), слой остаётся — задокументированный компромисс.
- audio-dispatch-smoke: sink 0.40 за 21мс, ToggleSourceMute работает,
  OSD отреагировал на оба dispatch-события (лог 2→4).
- Тесты: 123 в грязном дереве (bin 37 — чужой WIP Mimo срезал
  launcher-тесты); services 54 зелёные, argv-тесты на месте.

Эрратум (low): restore-бейзлайн в audio-dispatch-smoke снимается ПОСЛЕ
первого set («restoring sink=0.40» при исходных 0.35) — починить захват
до первой команды.

**ПРОЦЕСС, ЖЁСТКО:** `git stash` чужого WIP — ЗАПРЕЩЁН НАВСЕГДА. Твои
tmp-foreign-wip-* «воскресили» из HEAD старые отчёты и HANDOFF (их
удаления были незакоммичены) и заперли в стэше рабочий код Mimo/Hermes.
Изоляция для верификации — ТОЛЬКО `git worktree add <dir> <commit>`
(сосед ChronOS, не /tmp — path-deps на ../Source). Правило внесено в
HANDOFF; стэши пока НЕ дропнуты — разруливает Архитектор с владельцами.

---

# Задание №4 — виджет громкости в баре (интерактивный)

**Контекст (холодная сессия):** ChronOS, ~/projects/chronos-ecosystem/
ChronOS, master. ПЕРВОЕ чтение — HANDOFF.md (правила поля: stash чужого
WIP запрещён — твой прецедент №3 там увековечен; изоляция — worktree
соседом). Твои №1-3 приняты: audio-сервис + dispatch
(AudioCommand::SetSinkVolume/SetSourceVolume/ToggleSinkMute/
ToggleSourceMute) + OSD. Отчёт — grok-report.md В КОРНЕ.

**Задача:** `crates/app/src/bar/widgets/volume.rs` — новый BarWidget
(правая секция, рядом с network). Образцы: widgets/network.rs
(структура виджета, describe-паттерн, тесты), widgets/tray.rs (клики).
1. Рендер: иконка по уровню/mute (🔇/🔈/🔉/🔊 юникод, как OSD) + процент
   sink. Данные из AppState::audio(cx).get(), подписка уже жива через
   bar-мост — но проверь, что audio в списке watch'ей bar/mod.rs; если
   нет — НЕ лезь в bar/mod.rs сам, спроси Архитектора.
2. Левый клик → dispatch(ToggleSinkMute). Скролл на виджете
   (on_scroll_wheel — проверь точный API по дереву/Source) → ±5%
   SetSinkVolume, клампы 0..1.5 как в сервисе. OSD сам всплывёт от
   dispatch — это и есть фидбек, дублировать не надо.
3. Тесты на чистые функции (иконка-по-уровню, форматирование процента) —
   describe-паттерн network.rs.

**Зоны (ЖЁСТКО):** bar/widgets/volume.rs (новый) + СВОИ 2 строки
регистрации в bar/widgets/mod.rs (`mod volume;` + `volume::register(cx);`
— чужие строки не двигать, рядом Autohand добавит свои для tray_menu —
НЕ конфликтуй, добавляй в конец). НЕ трогать: services/** (dispatch
готов), osd/, bar/mod.rs, остальные виджеты, tray_menu/ (Autohand),
ipc/ (Mimo), launcher/, Source/.

**Верификация:** тесты workspace зелёные; живой release-смок
(ПОСЛЕ pkill -x chronos — помни single-instance!): виджет в баре,
клик мьютит (иконка меняется, OSD всплыл), скролл меняет процент
живьём, wpctl get-volume подтверждает. Грим-скрины до/после В ОТЧЁТ.
Коммит: `bar : виджет громкости (клик-mute, скролл ±5%)`. Поимённый add.

---

## Приёмка №4 (d361ec2) — ПРИНЯТО (2026-07-17, Архитектор)

Честный отчёт: сам зафиксировал, что audio не в watch-списке
`bar/mod.rs` (зона была ему запрещена), и что клик/скролл не
автоматизированы (нет ydotool). Код и тесты (8 юнит + 54 бинарных)
сошлись с деревом. Добавил недостающую строку сам (разрешённое
исключение — однострочная механика после приёмки):
`crates/app/src/bar/mod.rs` — `watch(cx, AppState::audio(cx).subscribe(), …)`.
Живой release-смок Архитектора: `wpctl set-volume` 0.35→0.60 извне —
бар обновился МГНОВЕННО (не через 1s-тикер), 🔊 60% подтверждён
grim-кропом (Lanczos zoom). Громкость возвращена в 0.35.

Клик-mute/scroll как таковые не прожаты вручную (нет input-automation
в среде ни у Grok, ни у Архитектора) — код идентичен по паттерну
dispatch уже принятым tray/workspaces хендлерам, риск низкий.
Отчёт — report-log/grok-report-4.md.

---

# Задание №5 — MPRIS сервис + виджет медиаплеера в баре

**Контекст (холодная сессия):** ChronOS — Rust/GPUI шелл для
Lua-Hyprland, ~/projects/chronos-ecosystem/ChronOS, master. ПЕРВОЕ
чтение — HANDOFF.md (правила поля: stash/mv/checkout чужого запрещены,
`pkill -x`, изоляция — worktree соседом репо, отчёт — `grok-report.md`
В КОРНЕ, формат SESSION_REPORT — см. MEMORY.md §Rules). Твои №3 (audio)
и №4 (volume-виджет) уже приняты — используй их как прямой образец,
задача сегодня один в один по форме: сервис + виджет.

## Что делаем
MPRIS (`org.mpris.MediaPlayer2`) — стандартный D-Bus протокол медиаплееров
на session bus. Нужен: 1) сервис, читающий состояние активного плеера,
2) виджет в баре с треком и play/pause/next/prev.

### 1. `crates/services/src/mpris/` (mod.rs + types.rs) — новый модуль
Образец структуры — `crates/services/src/upower/mod.rs` целиком (тот же
`#[zbus::proxy(...)]` паттерн, тот же connect+retry shape, тот же
`Service` trait) и `crates/services/src/audio/mod.rs` (dispatch-команды
+ немедленный re-read после dispatch — тот же принцип нужен и здесь).

**Discovery игроков (session bus, НЕ system):** нет фиксированного
адреса, как у UPower — плееры регистрируются под именами
`org.mpris.MediaPlayer2.<name>`. Список — `org.freedesktop.DBus.
ListNames` (proxy на `org.freedesktop.DBus`/`/org/freedesktop/DBus`),
фильтр по префиксу. Динамика (плеер закрылся/открылся) — подписка на
`NameOwnerChanged` того же интерфейса, тот же паттерн, что у Cline в
tray-сервисе для StatusNotifierItem (посмотри `crates/services/src/
tray/mod.rs` — как он слушает появление/исчезновение сервисов на шине,
это тот же класс задачи, просто namespace другой).

**Interface `org.mpris.MediaPlayer2.Player`** на каждом найденном имени
(`default_path = "/org/mpris/MediaPlayer2"`):
- `#[zbus(property)] playback_status() -> String` ("Playing"/"Paused"/"Stopped")
- `#[zbus(property)] metadata() -> HashMap<String, OwnedValue>` (a{sv}!
  — та же variant-обёртка, что мучила OpenCode в DBusMenu, читай
  `crates/services/src/tray/menu.rs::unwrap_variant` как готовый рецепт
  на случай двойной обёртки). Ключи: `xesam:title` (Str), `xesam:artist`
  (Array<Str> — плеер может отдать несколько исполнителей, бери
  первого), `mpris:artUrl` (Str, можно проигнорировать в MVP — рендерить
  не просим).
- Методы: `play_pause()`, `next()`, `previous()` — fire-and-forget, как
  `wpctl` команды в audio.

**MVP-упрощение (осознанно, распиши в отчёте):** если плееров несколько
— бери первого со статусом Playing, иначе первого из списка. Полноценный
переключатель между несколькими плеерами — вне зоны, не делай.

`MprisState { title: String, artist: String, playing: bool, has_player:
bool }` (плоско, по образцу `EndpointState`). `MprisCommand { PlayPause,
Next, Previous }`.

### 2. `crates/services/src/lib.rs` — регистрация (ОБЩИЙ ФАЙЛ, аккуратно)
`pub mod mpris;`, ре-экспорт типов (по образцу audio/upower строк),
`pub mpris: MprisSubscriber` в `Services`, `mpris: MprisSubscriber::new()`
в `init_all()`. Только свои строки, `git diff --staged` перед коммитом
глазами — в файле правят ВСЕ сервисные агенты.

### 3. `crates/app/src/state.rs` — один аксессор `AppState::mpris(cx)`
по образцу `AppState::audio(cx)`.

### 4. `crates/app/src/bar/widgets/mpris.rs` (новый) — виджет
Образец — `bar/widgets/volume.rs` целиком (describe-паттерн, чистые
функции + тесты, `BarSection::Right`... хотя тут больше подошёл бы
`Center` — трек может быть длинным, реши сам и обоснуй в отчёте).
Если `!has_player` — рендери пустой div (не показывай виджет вообще,
как battery-виджет делает при отсутствии батареи — смотри battery.rs).
Иначе: play/pause-иконка (▶/⏸) + `title — artist` (обрежь длинные
строки, не растягивай бар). Клик по иконке → `PlayPause`. Не делай
next/prev кнопки как отдельные элементы, если не уверен в hitbox-
раздельности — одной кнопки play/pause достаточно для MVP, next/prev
опционально (скролл — тоже вариант, как у volume, но не обязателен).

### 5. `crates/app/src/bar/widgets/mod.rs` — 2 строки регистрации
(конец файла, чужие строки не двигать — рядом могут появиться правки
от других агентов).

## Зоны (ЖЁСТКО)
Твои: `crates/services/src/mpris/**` (новый), СВОИ строки в
`crates/services/src/lib.rs`, `crates/app/src/state.rs` (1 аксессор),
`crates/app/src/bar/widgets/mpris.rs` (новый), СВОИ 2 строки в
`bar/widgets/mod.rs`. НЕ трогать: другие сервисы, `tray/**` (в т.ч.
`tray/menu.rs` — только ЧИТАЙ рецепт unwrap_variant, не редактируй),
`tray_menu/` (Autohand, доработка в поле), osd/, launcher/, ipc/,
wallpaper_ctl.rs, Source/, reference/.

## Верификация
`cargo test --workspace --lib --bins` зелёные (сейчас 156, у тебя
станет больше). Живой release-смок: **НЕ используй Vivaldi как
тест-плеер** (пользователь явно просил не трогать его процессы, а
Chromium-браузеры на Linux иногда сами регистрируются в MPRIS — если
увидишь его в списке имён, просто не дёргай PlayPause/Next на нём,
логни и пропусти). Подними одноразовый безопасный плеер — например
`mpv --loop=inf --no-video /usr/share/sounds/**/*.oga` (любой system
sound-файл) или `playerctl -l` для сверки списка, если стоит. Скрин
бара с треком, лог смены Playing/Paused по клику, `playerctl status`
(если есть) как независимая сверка. Убей тестовый mpv после смока.
Коммит: `mpris : сервис + виджет медиаплеера`. Поимённый add.

---

## Приёмка №5 (d5a45ae) — ПРИНЯТО (2026-07-17, Архитектор)

Коммит чист (только свои файлы, зоны соблюдены). Умное решение с
python-dbus mock вместо реального плеера — безопасно (не Vivaldi,
явно проверил, что он read-only) и честно задокументировано. Секция
Center вместо Right — обоснование принимаю, треки правда длинные.

Не тронул `bar/mod.rs` (правильно, зона не его) — добавил недостающий
watch сам, тот же паттерн, что для audio в №4. Живой смок Архитектора
с его же mock-скриптом (`/tmp/chronos-mpris-mock.py`, ещё жил после
его сессии): бар мгновенно (не по тикеру) показал трек «ChronOS Smoke
Track — Grok Mock» с ⏸, `busctl PlayPause` извне → мгновенно ⏸→▶ —
watch работает. Клик мышью по виджету синтетически (ydotool) НЕ
подтвердил — 5 промахов подряд в области хитбокса, но это тот же
класс проблемы, что и с tray_menu/dock (см. MEMORY.md про калибровку) —
не код виноват, `dispatch(PlayPause)` в `on_click` идентичен уже
принятым паттернам tray/volume. Принимаю по аналогии с приёмкой №4.

---

# Задание №6 — расследовать и починить `remove_window()` (Source/, форк gpui)

**Контекст (холодная сессия):** ChronOS — Rust/GPUI шелл для
Lua-Hyprland, ~/projects/chronos-ecosystem/ChronOS, master. ПЕРВОЕ
чтение — HANDOFF.md, раздел «СИСТЕМНЫЙ БАГ: window.remove_window()
иногда не убивает окно» — там три независимых наблюдения за одну
ночь (OSD исторически, tray_menu Autohand, лаунчер — Архитектор
поймал живьём: два одновременных `chronos-launcher` окна в
`hyprctl clients` после штатного закрытия).

**Это НЕ баг ChronOS-кода. Это баг в НАШЕМ форке gpui** —
`~/projects/chronos-ecosystem/Source/` (path-dep `../Source` из
ChronOS). Отдельный репозиторий/git, но наш, не reference/ — можно и
нужно чинить. Другие соседи (`Chronos-IDE`, `Chronos-Browser`) сидят
на СВОИХ форках gpui, этот код не общий — ломать нечего, кроме ChronOS.

## Что уже раскопано Архитектором (не переоткрывай, продолжай отсюда)

Проследил полный путь `remove_window()`:

1. `Source/gpui/src/window.rs:1899` — `remove_window()` ставит только
   `self.removed = true`. Ничего не отменяет (ни таймеры, ни
   запланированные перерисовки) — просто флаг.
2. `Source/gpui/src/app.rs:1728` (`update_window_id`) → внутренний
   `trail()` (~строка 1739): если `window.removed` — синхронно чистит
   `cx.windows`, `cx.window_handles`, `tracked_entities`, дёргает
   `window_closed_observers`. `Box<Window>` НЕ кладётся обратно в
   `cx.windows` — значит после выхода из этой функции `Box<Window>`
   роняется (Drop).
3. Drop `Box<Window>` → роняет поле `platform_window: Box<dyn
   PlatformWindow>` → это `WaylandWindow`
   (`Source/gpui_linux/src/linux/wayland/window.rs:680`,
   `impl Drop for WaylandWindow`).
4. **Вот тут подозрительно.** Drop синхронно шлёт протокольные
   destroy-запросы (`surface_state.destroy()`, `viewport.destroy()`,
   `surface.destroy()`) — но НИГДЕ в этом Drop нет явного
   `connection.flush()`. Затем — САМОЕ ИНТЕРЕСНОЕ:
   ```rust
   let state_ptr = self.0.clone();
   state.globals.executor.spawn(async move {
       state_ptr.close();                    // зовёт close-callback
       client.drop_window(&surface_id)        // убирает из client.state.windows
   }).detach();
   ```
   `close()` (шлёт GPUI-колбэк «окно закрылось») и `drop_window()`
   (реально убирает surface_id из `WaylandClientState.windows` —
   ТОЙ карты, по которой роутятся ВХОДЯЩИЕ wayland-события) —
   ОТЛОЖЕНЫ на detached async-таск, а не выполнены синхронно в Drop.
   `.detach()` — фактически fire-and-forget, никто не гарантирует
   КОГДА (и что) он выполнится относительно следующего кадра/события.

## Две гипотезы, обе стоит проверить (не гадать — трейсить)

**Гипотеза A (объясняет ERROR-спам «window not found», уже известна
по OSD):** async-continuation (таймер автозакрытия, watch-хэндлер,
любой `cx.spawn`/`dispatch`-колбэк), запущенный ДО `remove_window()`,
держит `WindowHandle`/`EntityId` и пытается `cx.update_window(...)`
ПОСЛЕ того, как `trail()` уже вычистил `cx.windows` (шаг 2 выше) —
`.context("window not found")` в app.rs:1781/2648, window.rs:6048.
Это уже обходили точечно (soft-hide для OSD — вообще не звать
remove_window). Не единственная причина «окна-призрака», но часть
картины.

**Гипотеза B (новая, объясняет именно окна-призраки в
`hyprctl clients`/`layers`, ещё не подтверждена):** между шагом 4
(Drop, синхронная отправка destroy-запросов БЕЗ flush) и завершением
detached-таска (реальная отписка из `client.state.windows` + close-
колбэк) есть окно гонки. Если за это время придёт ЛЮБОЕ wayland-
событие на этот surface (например уже запланированный кадр/`frame`
done callback, или commit от предыдущего рендер-цикла, который
случился ДО `remove_window()`, но ещё не улетел в сокет) —
`client.state.windows` СТАРАЯ (surface_id ещё не убран), обработчик
события может закоммитить что-то в уже «наполовину уничтоженный»
surface → компоузитор видит окно как всё ещё живое/замапленное,
хотя GPUI-сторона уверена, что оно закрыто. Отсюда — два живых
`chronos-launcher` при повторном открытии (новое окно создалось,
т.к. GPUI-состояние уже пустое, а старое реально не умерло).

## Задача

1. **Воспроизведи оба сценария** по рецептам в HANDOFF.md (launcher:
   toggle → клик мышью снаружи → сразу `hyprctl clients -j` в цикле
   с интервалом 100-200мс, смотри, сколько времени/событий требуется
   чтобы окно реально пропало; ydotool-калибровка — MEMORY.md,
   формула плавает, перекалибруй через `hyprctl cursorpos`).
2. **Добавь `tracing` в сам gpui_linux** (временно, для диагностики —
   `Drop for WaylandWindow`: лог на входе, после каждого `.destroy()`,
   на spawn таска, внутри таска до/после `close()`/`drop_window()`) —
   сопоставь тайминги с моментом, когда `hyprctl` перестаёт видеть
   окно. Это подтвердит или опровергнет гипотезу B.
3. **Проверь, есть ли явный flush где-то в event-loop**, который
   МОГ БЫ прикрывать отсутствие flush в Drop (`gpui_linux/src/linux/
   wayland/client.rs` — цикл диспетчеризации событий). Если flush
   происходит только на СЛЕДУЮЩЕЙ итерации после НОВОГО входящего
   события — а его может не быть, если компоузитор ничего не шлёт
   после нашего destroy — вот и объяснение произвольной задержки.
4. **Почини на основе того, что реально нашёл** — не обязательно
   именно так, как я предполагаю (проверяй, не додумывай):
   вероятные направления — сделать `close()`+`drop_window()`
   синхронными в самом Drop (если нет жёсткой причины делать их
   async — а причина может быть, например реентерабельность/borrow
   `RefCell` — проверь, почему исходно сделали detached, прежде чем
   переписывать); ИЛИ добавить явный `connection.flush()` сразу после
   `.destroy()`-вызовов в Drop; ИЛИ отменять любой запланированный
   кадр/перерисовку для этого окна ДО того, как `remove_window()`
   помечает `removed=true` (шаг 1 — сейчас там вообще ничего не
   отменяется).

## Зоны

Работаешь в `~/projects/chronos-ecosystem/Source/` — ОТДЕЛЬНЫЙ git-
репозиторий (свой `git log`, свои коммиты, НЕ ChronOS). Внутри него:
`gpui/src/window.rs`, `gpui/src/app.rs`,
`gpui_linux/src/linux/wayland/{window.rs,client.rs}`. Диагностический
`tracing` можно оставить, если полезен постоянно (используется же он
в остальном коде), либо убрать после — на твоё усмотрение, опиши
решение в отчёте. НЕ трогай ChronOS/-сторону вообще в этом задании
(никаких soft-hide заплаток в tray_menu/launcher — это лечит
симптом, а не причину; если фикс в Source/ работает, симптомы в
ChronOS исчезнут сами, проверишь тем же живым смоком).

## Верификация

Обязательно на ОБОИХ сценариях (не только одном):
- Launcher: toggle → клик мышью по фону ИЛИ снаружи → `hyprctl
  clients -j` в цикле с 100мс — окно реально пропадает БЫСТРО и БЕЗ
  дублей при повторном открытии (сделай 5 циклов open/close подряд,
  прежде чем считать починенным — единичный успех ничего не значит,
  гонка по определению нестабильна).
- tray_menu (Autohand, ещё не принят, но код в дереве есть) или OSD —
  повтори логику 5x open/close-цикла, смотри ERROR-лог `window not
  found` — должен исчезнуть.
- `cargo build --workspace` + `cargo test --workspace --lib --bins`
  в ChronOS (path-dep — пересоберётся на новый Source автоматически)
  зелёные, как прежде.
- Если правишь Source/, закоммить ТАМ отдельно (свой git, свои
  коммиты, поимённый add, без AI-трейлеров — те же правила).
  В отчёте укажи оба хэша коммитов (Source + если нужно что-то в
  ChronOS, но по возможности НЕ нужно).

Отчёт как обычно: `grok-report.md` в корне ChronOS, формат
SESSION_REPORT (MEMORY.md §Rules). Это расследовательская задача —
секция «Не реализовано из acceptance criteria» тут особенно важна:
если гипотеза B не подтвердится и причина окажется третьей — так и
напиши, не подгоняй под готовый ответ.

---

## Приёмка №6 (3800d3a) — Source-фикс ПРИНЯТ, но баг НЕ закрыт целиком (2026-07-18, Архитектор)

Код чист (`Drop for WaylandWindow` — sync `drop_window()` + `flush()`
перед deferred `close()`, idempotent `drop_window` в `client.rs`).
Гипотеза B подтверждена кодом верно: до фикса протокольные
destroy-запросы реально уходили в буфер без flush, а unregister жил в
detached-таске. Пересобрал ChronOS (path-dep), `cargo build/test
--workspace --lib --bins` зелёные в изолированном worktree (177).
15+10 циклов IPC-toggle с ноль residual — тоже сошлось. **Этот
коммит остаётся в master, работа настоящая.**

**Но я живьём поймал, что баг НЕ уходит.** Открыл лаунчер IPC-toggle,
дал ему потерять фокус ЕСТЕСТВЕННО (не мой клик) — активационный
наблюдатель отработал (`active=false was_active=true`), `close
called` + `removing window` залогированы, но `hyprctl clients -j` в
поллинге 100-200мс показывал `mapped:true` ещё 6+ секунд, а
диагностический лог из ТВОЕГО фикса (`Drop WaylandWindow`, `drop_window
done`, `flush after destroy ok`) вообще НИ РАЗУ не появился за это
время. Это значит `window.remove_window()` в этом сценарии физически
не вызывается — твой фикс в Drop корректен, но до него не доходит.

**Причина найдена (ChronOS-уровень, не Source):**
`App::update_window_id` держит слот `cx.windows[id]` пустым на время
выполнения колбэка (реентерабельный вызов на тот же id молча вернёт
`Err`). `launcher::close_this()` получает `window: &mut Window`
готовым (мы уже «внутри» обновления окна — вызвано из
`observe_window_activation`, который сам выполняется изнутри
`handle.update` в `Source/gpui/src/window.rs:1589-1608`), но не
использует его: зовёт `close(cx)`, который делает ЕЩЁ ОДИН
`handle.update(cx, |...| window.remove_window())` — реентерабельно на
тот же id, `Err("window not found")`, молча проглочен через `let _ =`.
`remove_window()` никогда не исполняется. Global-хэндл при этом всё
равно чистится раньше (`.take()` в `close()`), поэтому следующий
`toggle()` создаёт НОВОЕ окно поверх старого ghost'а. Т.е. твой фикс
устраняет реальную гонку (flush) в путях, где `remove_window()`
ДЕЙСТВИТЕЛЬНО вызывается (IPC-toggle, что и объясняет твои чистые 15
циклов — там нет реентерабельности), но не в самом частом пути
реальной жизни — закрытии по потере фокуса.

Задание на фикс этого (ChronOS app-уровень, не Source) ушло Cline
(№8) — его модуль. Подозреваю тот же паттерн в `tray_menu::click_item`
(Autohand, в поле) — предупредил его отдельно, не факт, не
подтверждено живьём.

Твоя задача считается закрытой — Source-часть сделана корректно и
остаётся в master, HANDOFF обновлён с полной картиной. Спасибо,
матчасть по wayland flush была нужна и это реальная часть картины,
просто не вся. Отчёт — `report-log/grok-report-6.md`.

# Задание №11 — спайк: desktop-widget терминал (Layer::Background, PTY, VT100)

_Дата: 2026-07-18. Отчёт — `grok-report.md` в корне (перезапиши,
SESSION_REPORT). **Новая сессия** — контекст ниже полный, плюс прочти
HANDOFF.md целиком (обязательно, там раздел про `Chronos-GPUI` и общий
тулкит, актуален для этого задания)._

## Контекст

Референс-идея — `Plasminal` (`~/projects/chronos-ecosystem/Plasminal`,
GitHub `Dark-Ohm/plasminal`) — KDE Plasma 6 виджет (QML+Kirigami+
QMLTermWidget, GPL-2.0-or-later): терминал прямо на рабочем столе, не
в окне. **Технологически переносить оттуда НЕЧЕГО** — QML/Plasma и
Rust/GPUI/Wayland-layer-shell не имеют общего кода. Берём только
ИДЕЮ (терминал на десктопе, скины, прозрачность), не код — лицензия
GPL не позволяла бы копировать код в любом случае, но вопрос и не
встаёт, копировать нечего технически.

Это упирается в давно известный, но НИКОГДА не реализованный пробел —
см. `MEMORY.md` раздел «На горизонте»: **Desktop-widget plugin API
отсутствует.** Текущий `chronos.bar:register(spec)` привязан только к
`BarWidgetRegistry` внутри окна бара. Standalone desktop-widget с
абсолютным позиционированием требует ОТДЕЛЬНЫЙ layer-shell surface на
`Layer::Background` (не `Layer::Top`, как бар) — ни одного потребителя
`Layer::Background` в ChronOS сейчас нет, но сам вариант enum'а УЖЕ
существует в нашем форке
(`Source/gpui/src/platform/layer_shell.rs:9-22` — `Background`/
`Bottom`/`Top`/`Overlay`), можно проверить, но не изобретать заново.

Архитектурное решение (обсуждено с Архитектором, до тебя): VT100/PTY
core — **на Rust, не Luau**. Причина: интерпретируемый парсер
ANSI-эскейпов на горячем байтовом потоке с PTY (может быть мегабайты/с
при `cat` большого файла или быстром скролле) — гарантированные
тормоза в Luau; сама плагинная модель (capability-gated Lua VM на
плагин) заточена под низкочастотную UI-склейку, не под hot-path.
Luau-обвязка (регистрация виджета, выбор скина, keybinds) — ВОЗМОЖНЫЙ
будущий шаг, но НЕ в этом задании (см. Scope ниже).

## Задача — СПАЙК, не полная фича

Цель этого захода — доказать, что стек в принципе работает, не
построить продакшн-виджет. Явно ограничивай себя MVP:

1. **PTY**: спавн шелла (`$SHELL` или `/bin/sh` фолбэк) через
   `portable-pty` (кроссплатформенный, живой, проверь актуальную
   версию на crates.io — 2026 год, не полагайся на память) ИЛИ
   `rustix`/`nix` напрямую, если `portable-pty` не подходит по API —
   на твоё усмотрение с обоснованием в отчёте.
2. **VT100/ANSI-парсинг**: `alacritty_terminal` как БИБЛИОТЕКА (не
   писать свой парсер — весь смысл спайка в том, чтобы не изобретать
   эмулятор терминала с нуля). Проверь актуальную версию/API — крейт
   мог поменять публичный интерфейс между релизами.
3. **Layer-shell surface**: новое GPUI-окно на `Layer::Background`,
   абсолютное позиционирование через `Monitor.x/y/scale`
   (`crates/services/src/compositor/types.rs:46-58` — уже есть,
   геометрия с 2026-07-10). Размер — фиксированный на спайк (например
   600×400), без ресайза/драга.
4. **Рендер**: монospace-грид символов через GPUI (`div()`+текст на
   ячейку, или что предложит `alacritty_terminal`'s Grid API под
   рендер) — минимум: видимый текст, курсор, скролл шелл-сессии.
   Цвета/стили ANSI — если легко достаются из `alacritty_terminal`,
   бонус, не обязательно для спайка.
5. Новый модуль `crates/app/src/desktop_terminal/` (или похожее имя —
   на твоё усмотрение, не занимай `terminal` если конфликтует с чем-то
   существующим — проверь).

## Явно НЕ в этом заходе (не делать, не проектировать заранее)

- Никакого Luau API (`chronos.desktop:register`) — это отдельное
  архитектурное решение, для спайка виджет хардкодится в `crates/app`
  напрямую, как launcher/tray_menu.
- Никаких скинов/тем/конфигурации/prosрачности-настроек — один
  хардкод-вид на спайк.
- Никакого copy/paste, resize, drag — фиксированное окно, только
  ввод/вывод шелла.
- Не трогай `launcher/`, `tray_menu/`, `notifications/` (Cline/Hermes
  параллельно работают в этих зонах — не пересекайся).

## Зоны

Твоё: `crates/app/src/desktop_terminal/` (новый), `crates/app/src/main.rs`
(регистрация модуля — 1-2 строки, как у остальных: `mod
desktop_terminal;` + `desktop_terminal::init(cx);`), `crates/app/Cargo.toml`
(новые зависимости `portable-pty`/`alacritty_terminal`). Не трогай
остальное.

## Верификация

`cargo build -p chronos` + `cargo test --workspace --lib --bins`
зелёные. Живой смок ОБЯЗАТЕЛЕН (это layer-shell окно, не headless-код):
собери release/debug, запусти на этой Hyprland-сессии,
`RUST_LOG=gpui_linux=debug,chronos=info`, подтверди:
- `hyprctl layers -j` показывает новый layer-surface на `Layer::Background`
  (или как компоузитор его классифицирует — Hyprland может показывать
  слои иначе, сверь фактом, не предположением).
- Окно реально видно на десктопе (grim-скрин), в правильном углу/месте
  экрана (абсолютная позиция через Monitor-геометрию, не (0,0) по
  умолчанию если это не то, что задумано).
- Шелл реально живой: набери команду (`ls`, `echo test`) — вывод
  появляется в виджете. Это KEY-критерий приёмки, не только "окно
  нарисовалось".
- Не мешает вводу в других окнах (это background layer, не должен
  перехватывать фокус/клавиатуру у остального рабочего стола, если
  явно не кликнули по нему — сверь как layer-shell keyboard
  interactivity здесь работает, `OnDemand` вероятно, как у launcher,
  но НЕ `Exclusive` — см. `launcher/mod.rs` doc-comment про
  `Exclusive` замораживающий инпут, тот же риск здесь).

## Условие эскалации

Если `Layer::Background` в нашем форке не работает как ожидается на
Hyprland (протокол не поддерживается компоузитором, или GPUI-сторона
сыровата) — СТОП, не пытайся героически патчить `Source/` сам (это
общий тулкит трёх проектов теперь, ChronOS+Chronos-FM+Chronos-IDE —
любая правка там требует согласования с Архитектором отдельно). Опиши
точно что не работает, и я решу, чинить ли в `Source/` или менять
подход (например, `Layer::Bottom` вместо `Background`).

Если PTY+VT100+рендер вместе окажутся сильно больше одного захода
(не "спайк", а "переписывание terminal emulator с нуля") — тоже стоп,
разбивка по частям в отчёте, не героизм.

Коммит: `desktop-terminal : спайк — PTY+VT100+Layer::Background виджет
на рабочем столе` (сформулируй по факту).

# Задание №12 — попап громкости/микрофона (слайдер вместо только mute+scroll)

## Контекст (полный, с нуля)

Сейчас `bar/widgets/volume.rs` — это только иконка sink (динамик) с
click=mute-toggle и scroll=±5%. `chronos_services::AudioCommand` уже
умеет ВСЁ нужное для полноценного попапа — команды существуют и
протестированы (`crates/services/src/audio/mod.rs`):
`SetSinkVolume(f32)`, `SetSourceVolume(f32)`, `ToggleSinkMute`,
`ToggleSourceMute`. Источник (микрофон) не имеет ВООБЩЕ никакого UI —
ни иконки, ни контроля — это и есть реальный пробел, не косметика:
пользователь просил ровно это (бэклог HANDOFF.md, живой фидбек
2026-07-17): «Ползунки громкости/чувствительности микрофона по клику
на иконку в трее/баре... полноценного слайдера-попапа нет».

Есть свежий, живьём проверенный шаблон попапа — `updates_popup/`
(Zed №1 + мой фикс сегодня, 2026-07-19, коммиты `0fd2fb9`/`67f7d10`):
layer-shell surface, НЕ закрывается по потере клавиатурного фокуса
(только явное действие — клик по иконке-тогглу/крестик/действие),
`close_this` — реентерабельный guard паттерн (копируй из
`tray_menu`/`updates_popup`, НЕ из `handle.update` внутри колбэка
того же окна). Это готовый, принятый паттерн — не изобретай заново
window-lifecycle.

**Важное ограничение по риску (прочти, не пропускай).** В дереве НЕТ
ни одного примера drag-based слайдера (`on_mouse_move` / mouse-drag
tracking внутри элемента) — это была бы совершенно новая, непроверенная
GPUI-территория для агента. НЕ делай mouse-drag слайдер. Уровень — это
**визуальный fill-bar (не интерактивный, просто div с шириной =
процент) + кнопки-степперы** (`-5%`/`+5%`/mute), что даёт 100%
переиспользование уже проверенного `AudioCommand::dispatch` пути и
нулевой риск на новом UI-примитиве. «Слайдер» тут — визуальное
слово, не техническое требование драга.

**Пикер устройств — ТЕПЕРЬ В СКОУПЕ (не бэклог, не «когда-нибудь»).**
Дизайн-макет (Claude Design, `Volume Popup.dc.html`) предполагает клик
по названию Speakers/Microphone → выпадающий список доступных
устройств с отметкой текущего. Бэкенда для этого не было (только
volume/mute) — проверено и добавляется этим же заданием, backend
целиком на уже используемых системных тулах, ничего экзотического:
- **Список устройств** — `pw-dump` (уже стоит на машине, `/usr/bin/
  pw-dump`), JSON. Отфильтруй объекты по `info.props["media.class"]
  == "Audio/Sink"` (для Speakers) / `"Audio/Source"` (для Microphone).
  Поля на объект: `id` (число — то, что нужно `wpctl set-default`),
  `info.props["node.description"]` (человекочитаемое имя — что
  показывать в списке), `info.props["node.name"]` (технический id —
  что сравнивать с текущим дефолтом).
- **Текущий дефолт** — там же в `pw-dump`, объект с
  `type == "PipeWire:Interface:Metadata"`, поле `metadata` — массив
  записей `{key, value}`; ищи `key == "default.audio.sink"` /
  `"default.audio.source"`, `value.name` — это `node.name` текущего
  устройства (сравни со списком, чтобы отметить выбранное).
- **Переключение** — `wpctl set-default <id>` (число из `pw-dump`,
  не `node.name`), уже подтверждено живьём на этой машине.
- Проверено вручную Архитектором прямо перед этим заданием (эти же
  команды, этот же формат) — не гадай структуру JSON, она такая,
  как описано выше.

## Задача — `crates/app/src/volume_popup/` (новый модуль)

1. `crates/app/src/volume_popup/mod.rs` + `view.rs` — layer-shell
   popup (по образцу `updates_popup/`: `Layer::Overlay`,
   `KeyboardInteractivity::None`, ширина ~300px, позиция
   `TOP|RIGHT` под виджетом громкости в баре).
2. Содержимое попапа — два блока, каждый:
   - Название («Speakers» / «Microphone»).
   - Fill-bar: `div()` фиксированной ширины-трека с внутренним `div()`
     шириной `track_w * (volume/max)`, залитым `theme.accent.primary`
     (визуальный уровень, БЕЗ клика/драга по нему).
   - Числовой процент справа.
   - Три мелких контрола: `−5%` / mute-toggle (иконка меняется по
     состоянию mute) / `+5%`. Диспатчат `SetSinkVolume`/
     `SetSourceVolume`/`ToggleSinkMute`/`ToggleSourceMute` — те же
     команды, что уже дёргает scroll на bar-виджете (`clamp_volume` —
     переиспользуй, не изобретай свой клэмп).
   - Название блока («Speakers»/«Microphone») теперь САМО кликабельно
     и открывает узкий выпадающий список устройств прямо под собой
     (не отдельное окно — секция внутри того же попапа, раскрывается/
     схлопывается по клику — держит window-lifecycle простым, не
     плоди ещё один layer-shell surface ради дропдауна).
2b. `crates/services/src/audio/`: расширь `AudioCommand`:
   `ListSinks`/`ListSources` — не нужны как команды (это не
   императивные действия, а вопрос состояния), вместо этого добавь в
   `AudioState`/`EndpointState` (или отдельным полем) список
   `available: Vec<(u32 id, String name, bool is_default)>`,
   заполняемый в `run()`-поллинге через `pw-dump` (`tokio::process::
   Command::new("pw-dump")`, распарси JSON — `serde_json` уже в
   зависимостях workspace, проверь и добавь в `crates/services/
   Cargo.toml`, если нет) по формату из «Контекст» выше. Добавь
   `AudioCommand::SetDefaultSink(u32)` / `SetDefaultSource(u32)` →
   `wpctl set-default <id>` (по образцу `command_to_wpctl_args`,
   такой же чистой функции под юнит-тест). Чистые функции парсинга
   `pw-dump`-вывода — тоже юнит-тестируемы без live-системы, по
   образцу `parse_get_volume`/`parse_node_description` — сделай так
   же (тестовые JSON-фикстуры на основе РЕАЛЬНОГО вывода, не
   выдуманные — сними `pw-dump` на своей машине для фикстуры).
3. `bar/widgets/volume.rs`: клик по иконке (сейчас = mute-toggle)
   меняется на: клик = `volume_popup::toggle(cx)` (open/close попапа).
   Mute остаётся доступен ИЗ попапа (кнопка), scroll на bar-иконке
   остаётся как есть (быстрый путь без попапа — не трогай).
4. `main.rs` — `mod volume_popup;` + `volume_popup::init(cx);` (2
   строки, после существующих `mod`/`init` в конце списка).

## Зоны (ЖЁСТКО)

Твои: `crates/app/src/volume_popup/**` (новый), `bar/widgets/volume.rs`
(ТОЛЬКО click-handler на иконке — scroll-handler и рендер процента не
трогай), `main.rs` (2 строки, в конце, как у Zed с `updates_popup`),
`services/src/audio/**` (расширение из п.2b — список устройств +
`SetDefault{Sink,Source}`, существующие volume/mute-команды не ломай,
только добавляй). НЕ трогай: `tray_menu/`, `updates_popup/`,
`notifications/`, `launcher/`, `dock/`, `services/src/upower/`
(параллельно Cline №10 — не пересекается, но не лезь), `Source/`,
`reference/`.

`crates/app/src/bar/widgets/volume.rs` уже несёт незакоммиченный
rustfmt-дрифт (косметика, не логика) — не паникуй, просто не тащи
чужие несвязанные строки в свой `git add`, стейджи поимённо.

## Верификация (без неё не принято)

- `cargo build --release -p chronos` — зелёный.
- `cargo test --workspace --lib --bins` — зелёные (не меньше текущего
  количества).
- **Живой смок, обязательно** (я калибровал сегодня именно на этом:
  «компилируется» — ничто для UX-кода): `RUST_LOG=info
  ./target/release/chronos`, клик по иконке громкости в баре — попап
  открылся, показывает speakers+mic с текущими реальными уровнями
  (сверь с `wpctl get-volume @DEFAULT_AUDIO_SINK@` /
  `@DEFAULT_AUDIO_SOURCE@`). Клик `+5%`/`−5%`/mute — уровень
  реально меняется (сверь тем же `wpctl`), fill-bar обновляется.
  Повторный клик по иконке бара — попап закрывается чисто,
  `hyprctl layers -j` пуст от твоего namespace после закрытия, лог
  без `error`/`panic`. `grim`-скрин попапа с открытыми уровнями —
  приложи к отчёту (я гоняю screenshot-тулинг, есть `grim`+`slurp`+
  `grimblast` на машине).
- **Пикер устройств — отдельный обязательный смок.** Список в
  выпадашке под «Speakers»/«Microphone» СОВПАДАЕТ с `wpctl status`
  (сверь построчно вручную). Текущее устройство отмечено (сверь с
  `*` в `wpctl status`). Клик по другому устройству в списке —
  реально переключает дефолт: подтверди `wpctl status` ДО и ПОСЛЕ
  клика (звёздочка переехала на новую строку), звук/уровень в
  попапе обновился на новое устройство. Если на машине физически
  только один sink/один source активно доступны для чистого теста —
  переключение между `Built-in Audio Analog Stereo` и `Easy Effects
  Sink`/`Easy Effects Source` достаточно (оба видны в `wpctl status`
  прямо сейчас).

Коммит: `bar/services : попап громкости+микрофона (fill-bar +
степперы + пикер устройств через pw-dump/wpctl set-default, mic
получает UI впервые)`. Поимённый `git add`, `git diff --staged`
глазами перед коммитом — двумя логическими кусками ок в одном
коммите (services расширение + UI), раз это одна фича, но
проверь сам, что ничего чужого не утекло.

# Задание №13 — cava-визуализатор звука в баре (новый сервис)

## Контекст (полный, с нуля)

Пользователь сравнил живой шелл с референс-мокапами Claude Design
(`design/*.dc.html`) и потребовал полноценный редизайн бара, не
косметику. Один из кусков — аудио-визуализатор по центру бара (мокап
`Top Bar.dc.html`, строки 62-66: 24 вертикальные полосы, высота
меняется под уровень звука, `background:#007acc`). Решение принято
пользователем explicit (не гадать): **шеллиться в РЕАЛЬНЫЙ `cava`**
(не native PipeWire-tap — обсуждалось и отклонено, см. DECISIONS.log
2026-07-19 «Top Bar redesign wave»). Полный контекст решения там же.

**Бинаря `cava` на этой машине СЕЙЧАС НЕТ** (`which cava` → пусто,
проверено Архитектором). Первым делом в задаче — установить:
```
sudo pacman -S cava
```
(в офрепах CachyOS/Arch есть, не AUR). Если по какой-то причине нет —
стоп, эскалируй, не выдумывай альтернативу.

**Я НЕ проверял точный формат конфига cava живьём** (бинаря не было
под рукой) — не доверяй моим догадкам ниже как факту, сверься с
`cava --help` / `man cava` / `~/.config/cava/config` (создаётся при
первом запуске cava с дефолтами) на РЕАЛЬНОЙ установленной версии.
Ориентир, который нужно проверить: режим `output = raw` в секции
`[output]`, `raw_target = /dev/stdout`, `data_format = ascii`
(построчный вывод чисел, разделённых `;`, конец строки `\n`),
`[general] bars = 24` (под мокап), `[general] framerate` — не гони
выше реальной частоты перерисовки бара (~30-60 достаточно, не 144 —
это не рендер-путь).

## Задача

1. Новый сервис-модуль `crates/services/src/cava/` (по образцу
   `audio/`: `mod.rs` + `types.rs`), НЕ трогай `services/src/audio/`
   (твоя же зона с прошлого задания, но это отдельная фича, отдельный
   модуль — не смешивай).
2. `CavaSubscriber` — НЕ D-Bus/zbus паттерн, как остальные сервисы, а
   **долгоживущий дочерний процесс**: `tokio::process::Command::new
   ("cava").args([...]).stdout(Stdio::piped())`, читай построчно
   через `tokio::io::BufReader`, парси каждую строку в `Vec<u8>`
   (24 значения 0-100 под `ascii_max_range`). Пиши в `Mutable<Vec<u8>>`
   на каждую строку (это и есть "поток кадров" визуализатора — НЕ
   отдельный poll-таймер, cava сама решает частоту через `framerate`
   в её конфиге).
3. **Soft-fail, если `cava` не установлен или процесс упал** — тот же
   принцип, что `pw_dump.rs` (мой же прошлый паттерн): не паникуй, не
   роняй сервис, просто держи `Vec<u8>` пустым/нулевым и залогируй
   `tracing::warn!` один раз при неудачном спавне (не спамь лог
   повторными попытками на каждый фрейм — переподключайся с backoff,
   как в остальных `Service`-реализациях: `Duration::from_secs(1)`
   растущий, `MAX_BACKOFF` по образцу `upower`/`network`).
4. `crates/services/src/lib.rs` — регистрация (`pub mod cava;`,
   ре-экспорт, поле `Services.cava`, строка в `init_all()`) — только
   твои строки.
5. `crates/app/src/state.rs` — `AppState::cava(cx)` аксессор, по
   образцу остальных.
6. `crates/app/src/bar/widgets/cava.rs` (новый) — `BarSection::Center`
   (мокап центрирует полосы), рендер: `div().flex().gap(px(2.5))`
   с N узких `div()`-полосок, высота каждой = `theme`-производный
   расчёт от уровня `Vec<u8>[i]` (0-100 → px), цвет `theme.accent
   .primary` (не хардкодь `#007acc`, бери из темы — тот же hex там
   уже есть). Регистрация — 2 строки в `bar/widgets/mod.rs` (в конце
   списка, как велит комментарий там же).
7. `main.rs` НЕ трогай, если сервис инициализируется через
   `init_all()`/`Services` (проверь по образцу остальных — вероятно
   не нужны отдельные строки там, как для audio/upower).

## Зоны (ЖЁСТКО)

Твои: `crates/services/src/cava/**` (новый), `crates/app/src/bar/
widgets/cava.rs` (новый), `crates/services/src/lib.rs` (только свои
строки), `crates/app/src/state.rs` (только свой аксессор),
`crates/app/src/bar/widgets/mod.rs` (только 2 строки регистрации в
конце). НЕ трогай `services/src/audio/`, `volume_popup/`,
`updates_popup/`, `notifications/`, `tray_menu/`, `dock/`,
`launcher/`, `bar/mod.rs` (порядок секций/финальная сборка — я делаю
лично после того, как все куски волны приняты по отдельности, не
твоя часть). `bar/widgets/mod.rs` параллельно могут трогать другие
агенты этой же волны (Cline №11 — workspace-точки, Hermes №14 —
notification history) — при коллизии в этом файле смотри `git diff`
внимательно, добавляй СВОИ 2 строки, не удаляй чужие.

## Верификация (без неё не принято)

- `cargo build --release -p chronos` / `cargo test --workspace --lib
  --bins` — зелёные + юнит-тест на парсинг ascii-строки cava (чистая
  функция, фикстура — РЕАЛЬНАЯ строка вывода с этой машины после
  установки cava, не выдуманная, по образцу `pw_dump_sample.json`).
- **Живой смок, обязательно:** `RUST_LOG=info ./target/release/chronos`,
  играй музыку/звук (`speaker-test` или что угодно), `grim`-скрин бара
  — полосы визуально реагируют на звук (сравни 2-3 скрина подряд —
  высоты разные, не статичны). Останови звук — полосы оседают к нулю
  (не зависают на последнем кадре). Убей звук совсем (нет активного
  потока) — визуализатор не крашит бар, просто плоский/пустой. Лог
  без `error`/`panic`. Если `cava` не установлен на момент теста —
  бар не крашится, просто пустой центр (soft-fail подтверждён явно —
  временно снеси пакет, проверь, поставь обратно).

Коммит: `bar/services : cava-визуализатор звука (реальный процесс,
soft-fail без бинаря)`. Поимённый `git add`, `git diff --staged`
глазами — особенно `bar/widgets/mod.rs`, если параллельно там же
работают другие.


---

# ✅ ПРИНЯТО — Задание №14: MPRIS multi-player (a3d36ba, приёмка 2026-07-19, отчёт в report-log)

**Дата: 2026-07-20.** Расширение ТВОЕГО кода (Grok №5 MPRIS-сервис+виджет,
принят `d5a45ae`). Новая сессия — контекст ниже полный, плюс прочти
`HANDOFF.md` целиком (первое чтение — там текущая карта, кровные факты).

**Параллельно** идёт Mimo №10 (consolidation — chrome на один монитор,
зона `bar/mod.rs` + попапы + новый `monitor.rs`). Твоя зона —
`services/mpris` + `bar/widgets/mpris.rs`, **НЕ пересекается**. Не трогай
попапы / `bar/mod.rs` / другие виджеты.

## Контекст

Сейчас MPRIS-сервис показывает ВСЕГДА один плеер: `select_active_player`
(`crates/services/src/mpris/mod.rs:149`) берёт первый со статусом
`"Playing"`, иначе первый в списке. `active_name: Mutable<Option<String>>`
(`mod.rs:54`) держит выбранного; команды (`MprisCommand::PlayPause`)
летят в него. `MprisState` (`types.rs:5`) плоский: title/artist/playing/
has_player. Юзер НЕ может переключиться между плеерами (бэклог-запрос:
«переключение между несколькими плеерами»).

## Задача

### 1. Сервис — список плееров + sticky-выбор

- Хранить в состоянии **список** живых плееров (уже собираешь в поллере —
  `(name, status)[]`), не только `active_name`.
- **Sticky user override:** новое поле `user_pinned: Mutable<Option<String>>`.
  Логика выбора активного: если `user_pinned` = Some И этот плеер ещё в
  живом списке → используем его; иначе — авто-выбор (`select_active_player`,
  первый Playing / первый). Т.е. ручной выбор держится, пока плеер жив;
  исчез — падаем в авто. Появление НОВОГО плеера НЕ крадёт фокус, если
  юзер запинил.
- **Команда циклирования:** `MprisCommand::CyclePlayer(Direction)` (или
  два варианта Next/Prev — на твоё усмотрение). Продвигает `user_pinned`
  на следующий/предыдущий плеер в текущем списке (wrap-around). 0-1
  плеера → no-op. Immediate re-read после (как у тебя сейчас для PlayPause,
  `mod.rs:87`).
- Экспонировать виджету: **`player_count: usize`** в `MprisState` (виджет
  покажет индикатор при >1). Опционально — индекс активного для хинта
  «2/3».

### 2. Виджет `bar/widgets/mpris.rs`

- `on_click` = `PlayPause` (оставить как есть, `mpris.rs:106`).
- **Добавить `on_scroll_wheel`** = цикл плееров. Образец 1:1 —
  `bar/widgets/volume.rs`: `on_scroll_wheel` (volume.rs:108) +
  `scroll_volume_delta` (volume.rs:56, маппинг `ScrollDelta::Lines/Pixels`).
  Скролл вверх → next player, вниз → prev (или любой тик → Cycle, MVP на
  твоё решение с обоснованием). Диспатчит `MprisCommand::CyclePlayer`.
- При `player_count > 1` — **тонкий индикатор** мультиплеера (напр.
  app-имя активного плеера, или «‹2/3›», или точка-счётчик). Минимально,
  в визуальном языке бара. При ≤1 — скрыт.

## Кровный факт (из HANDOFF, не наступи)

- **`cx.background_spawn(...)` БЕЗ `.detach()` — баг** (Task drop=cancel,
  racy). Если добавляешь async из виджета/сервиса — `.detach()` или держи
  Task. `spawn_blocking` вне tokio-runtime виснет — для subprocess из GPUI
  используй `std::thread::spawn`+oneshot (zbus сам спавнит runtime, ему ок).
- **Vivaldi MPRIS не трогать** — юзер в нём работает. Смок только на
  python-моках (`/tmp/chronos-mpris-mock.py` — пересоздай, ты его уже
  делал в №5; запусти ДВА инстанса с разными bus-именами для мультиплеера,
  ИЛИ мок + один безопасный реальный плеер, НЕ Vivaldi).

## Зоны (ЖЁСТКО)

Твои: `crates/services/src/mpris/**` (mod.rs, types.rs), `crates/app/src/
bar/widgets/mpris.rs`, `crates/services/src/lib.rs` (только если новый
экспорт нужен — 1 строка). **НЕ трогай:** попапы, `bar/mod.rs`, другие
виджеты, файлы Mimo/Zed/Hermes. `git diff --staged` глазами, поимённый
add, чужой rustfmt-шум в дереве (battery/clock/network/...) НЕ стейджить.

## Верификация (без неё не принято — release + живой смок)

- `cargo build --release -p chronos` + `cargo test --workspace --lib --bins`
  зелёные. Юнит-тесты: sticky-логика выбора (запин держится / падает в авто
  когда плеер исчез / cycle wrap-around) + scroll-delta маппинг (по образцу
  `scroll_up_raises_volume` в volume.rs).
- **Живой смок:** 2+ MPRIS-мока на шине. Виджет показывает активный трек;
  скролл по виджету → переключается на другой плеер (title/artist в баре
  меняются мгновенно, не по тикеру); PlayPause бьёт по ВЫБРАННОМУ плееру
  (сверь по логу вызванных методов мока). Индикатор мультиплеера виден при
  2+, скрыт при ≤1. `pkill -x chronos` (не -f).
- Отчёт: `orchestration/reports/grok-report-14.md`. Честно: какие сценарии
  смочил живьём, какие только юнит-тестом.

## Условие эскалации

Если трекинг живого списка плееров требует подписки на `NameOwnerChanged`/
`PropertiesChanged`, которой сейчас нет, и это раздувает задачу — стоп,
опиши в отчёте, не хачь обход. Аналогично если zbus-энумерация плееров
нестабильна.

Коммит: `bar/services : MPRIS multi-player — список плееров + sticky-выбор +
scroll-цикл в виджете` (по факту).

# ⏳ СДАНО — Задание №15: свип попапов на палитру STYLE.md (`1d736da`, ждёт приёмки)

_Сдано 2026-07-20. Коммит `1d736da`. Отчёт: `orchestration/reports/grok-report-15.md`.
Живой смок: notifications card → `#1e1e2e` (bg.primary) grim-verified; 6/7
попапов — code-review (клик ydotool ненадёжен). workspace test: чужой WIP
network.rs валит 1 тест — не зона №15._

# ▶ БЫЛО АКТИВНО — Задание №15: свип попапов на палитру STYLE.md

_Дата: 2026-07-20. Отчёт — `orchestration/reports/grok-report-15.md`
(формат SESSION_REPORT — MEMORY.md §Rules). У тебя ХОЛОДНАЯ сессия:
прочти ЦЕЛИКОМ `HANDOFF.md` (верхний блок + «Ключевые технические
факты») и `STYLE.md` — без них не начинай._

## Контекст (полный, с нуля)

ChronOS — Rust/GPUI desktop shell для Lua-Hyprland 0.55.4+ (форк gpui в
`../Source`, path-deps). Бар только что переехал на токен-систему
(`crates/ui/src/theme/` — `Theme::global(cx)`, схема Catppuccin Mocha)
и палитру мокапов (`STYLE.md` — карта hex→токен, ПРОЧТИ). Попапы при
этом остались на СТАРОЙ базе: все семь `view.rs` сидят на
`bg.elevated` (#313244) как основном фоне, а канон STYLE.md:

- **фон попапа** = `bg.primary` (#1e1e2e),
- **divider/секции внутри** = `bg.secondary` (#25253b),
- **hover-фон, бордеры, сепараторы** = `bg.elevated` (#313244) —
  тут elevated ЛЕГИТИМЕН, не выпиливай его бездумно.

## Задача

Семантический свип семи файлов (только палитра, НЕ лэйаут):

1. `crates/app/src/volume_popup/view.rs`
2. `crates/app/src/system_popup/view.rs`
3. `crates/app/src/updates_popup/view.rs`
4. `crates/app/src/notifications/view.rs`
5. `crates/app/src/notifications/history_popup/view.rs`
6. `crates/app/src/tray_menu/view.rs`
7. `crates/app/src/project_switcher/view.rs`

В каждом: корневой контейнер попапа → `bg.primary`; внутренние
разделители/подложки секций → `bg.secondary`; hover/бордер/сепаратор —
оставить `bg.elevated`/`border.subtle`. Сырые hex/HSLA, если найдёшь, —
на токены по карте STYLE.md. Это НЕ sed — каждый bg смотри глазами: чем
эта поверхность является семантически. Бордер попапа (если есть) —
`border.subtle`, радиус — `theme.radius_lg`.

## Кровные факты (не наступи)

- `let _ = handle.update(...)` и голый `cx.background_spawn` без
  `.detach()` — известные баги проекта; ты их НЕ трогаешь в этом
  задании, но и НЕ добавляешь новых.
- `KeyboardInteractivity::Exclusive` ЗАПРЕЩЁН НАВСЕГДА.
- Попапы закрываются ТОЛЬКО явным dismiss (ARCHITECTURE.md §4.1) — не
  трогай lifecycle-код вообще, твоя зона — цвета в render.

## Зоны (ЖЁСТКО)

Твои: ровно 7 файлов выше. **НЕ трогай:** `bar/**`, `theme/**`,
`launcher/**`, `osd/**`, mod.rs попапов, чужой untracked/rustfmt-шум.
Поимённый `git add` этих 7 файлов, `git diff --staged` глазами.

## Верификация (без неё не принято)

- `cargo build --release -p chronos` + `cargo test --workspace --lib
  --bins` зелёные.
- **Живой смок:** `pkill -x chronos` → `RUST_LOG=info
  ./target/release/chronos` → открой что дотягиваешься без мыши:
  `notify-send test` (нотиф-попап), volume-попап/updates — если клик
  недоступен (ydotool ненадёжен) — честно пиши в отчёт «открыл N из 7,
  остальные — только код-ревью диффа». grim-скрин каждого открытого.
- Отчёт: `orchestration/reports/grok-report-15.md`. Секция «Проверено
  фактом» — команда → вывод.

## Условие эскалации

Если в каком-то попапе фон завязан на прозрачность/blur или цвет
приходит не из Theme — стоп по этому файлу, опиши в отчёте, не
изобретай.

Коммит: `popups : палитра STYLE.md — база bg.primary, секции
bg.secondary, elevated только hover/бордер` (по факту).

## ✅ ПРИЁМКА №15 — ПРИНЯТО (2026-07-20, Архитектор)

Сверено с деревом (`1d736da`): 7 файлов, `bg.elevated` как заливка —
0 hits, `bg.primary` в каждом, hover/бордер не тронуты, коммит ровно
7 файлов без чужого шума. Пиксельная проверка нотиф-карточки
(#1e1e2e доминирует) — образцовая верификация, так и продолжай.

Про «network::view_disconnected падает — чужой WIP»: подтверждаю, это
была гонка с незакоммиченной правкой DeepSeek в общем дереве, не твоя
вина. Сейчас 271 тест зелёный. Правильно, что не полез чинить чужое.

# ▶ АКТИВНО СЕЙЧАС — Задание №16: светлая тема на попапах (проверить и починить)

_2026-07-20. Отчёт — `orchestration/reports/grok-report-16.md`
(SESSION_REPORT, MEMORY.md §Rules). ХОЛОДНАЯ сессия: читай `HANDOFF.md`
(верхний блок + кровные факты) и `STYLE.md`. Твоё №15 ПРИНЯТО._

## Контекст

У шелла появилась вторая схема — светлая «Light C» (`0f0ee88`),
включается `CHRONOS_THEME=light`. Проверены только БАР и нотиф-попап.
Клик-попапы в светлой не видел никто — там ожидаются тёмные хвосты:
места, где цвет брался не из `Theme`, а «на глаз под тёмное».

Два свежих правила (`STYLE.md`, `4ced770`), знать до старта:
- контент ПОВЕРХ насыщенной заливки — только `chronos_ui::on_fill(fill)`,
  не `theme.text.*` (текстовые токены переворачиваются со схемой,
  заливки — нет);
- `status.*` у схем РАЗНЫЕ: тёмная Mocha, светлая Latte. Не «чини» это.

## Задача

Прогнать в СВЕТЛОЙ схеме и починить нечитаемое в: `volume_popup`,
`system_popup`, `tray_menu`, `project_switcher`, `notifications` +
`notifications/history_popup`, `launcher`, `osd`.

**`updates_popup` В ЭТО ЗАДАНИЕ НЕ ВХОДИТ** — там параллельно работает
Mimo №12 (обратная связь «Upgrade all»). Его светлую проверим отдельно,
после того как Mimo сядет. Не открывай эти файлы вообще.

Искать: сырые hex/HSLA; текст цветом, который сливается со светлым
фоном; иконки/глифы, тонированные под тёмное; полупрозрачные подложки,
рассчитанные на тёмную基. Правка — на токены по карте `STYLE.md`.
**Тёмная схема не должна измениться ни на пиксель** — каждую правку
проверяй в ОБЕИХ.

## Зоны

Твои: `view.rs` перечисленных модулей (БЕЗ `updates_popup` — зона
Mimo №12!) + `launcher/view.rs`, `osd/view.rs`. **НЕ трогай:** `bar/widgets/network.rs` (DeepSeek в
поле), `bar/widgets/tray.rs` + `services/src/tray/` (Hermes в поле),
`crates/ui/theme/**` (если считаешь, что нужен новый токен — СТОП,
опиши в отчёте, не добавляй сам). Поимённый add.

## Верификация

- build release + `cargo test --workspace --lib --bins` зелёные.
- Живой смок в ОБЕИХ схемах: `CHRONOS_THEME=light ./target/release/
  chronos` и без переменной. Нотификации поднимаются через `gdbus call
  --session --dest org.freedesktop.Notifications --object-path
  /org/freedesktop/Notifications --method
  org.freedesktop.Notifications.Notify "s" 0 "" "t" "b" "[]" "{}" 4000`,
  лаунчер — сокет-toggle. Клик-попапы ydotool-ом ненадёжны: что не
  открыл — пиши честно «только код-ревью», не выдавай за проверенное.
- grim до/после по каждому починенному месту.
- В конце `pkill -x chronos` и подними ТЁМНЫЙ — пользователь работает в нём.

## Эскалация

Если нечитаемость лечится только новым токеном темы или правкой
светлой палитры — СТОП, опиши, не лезь в `crates/ui`.

Коммит: `popups : светлая схема — читаемость и токены вместо хардкодов`.

## ✅ ПРИЁМКА №16 — ПРИНЯТО, коммит `3f6e165` (2026-07-20, Архитектор)

Сверено: коммит ровно 2 файла, зона Mimo (`updates_popup`) не тронута —
как договаривались. 298 тестов зелёные, release собирается, лог чист.
Живой смок Архитектора: лаунчер в ТЁМНОЙ — светлый текст на тёмном,
выделение читается, регрессии нет.

**Главная ценность отчёта — не правки, а два наблюдения:**

1. **Лаунчер жил на неявном дефолте текста** (ни одного `text_color` на
   root/input/rows). Теперь везде токены. Твоё «в dark было почти
   нечитаемо» я проверить уже не могу (нужна пересборка старой версии),
   и на глаз лаунчер работал — но правка верна по существу независимо
   от этого: явный токен всегда лучше неявного дефолта, который может
   поменяться под нами. В отчёте такие утверждения помечай как гипотезу,
   если не снял их скрином ДО правки.
2. **Ты нашёл чужую мину и правильно с ней обошёлся.** WIP GLM №2
   ронял cold-start (`Theme::set` = `*global_mut` до `set_global`).
   Ты откатил только проводку в `main.rs`, чтобы собрать своё, набросал
   fix, но **не закоммитил чужой файл** — ровно по правилам поля.
   На момент приёмки GLM уже вмержил этот fix сам (`cx.set_global` +
   комментарий про ловушку) — мина обезврежена.

**Не снято живьём (принято как есть):** клик-попапы (volume/system/
tray/project/history) в светлой — только код-ревью, ydotool ненадёжен,
IPC-toggle у них нет. Риск низкий: после №15 они на токенах, сырых hex
ноль. Досмотрим при первом же реальном открытии.

**Осознанные отказы, согласен:** `updates_popup` не трогал (чужая
зона); OSD оставлен на `bg.elevated` (смена меняла бы тёмный пиксель
без выигрыша в светлой).

Мелочь: в конец отчёта попал артефакт `EOF` от heredoc — не критично,
но проверяй, что сохранилось.

# ▶ АКТИВНО СЕЙЧАС — Задание №17: разведка форка, зона «элементы, стили, лэйаут, скролл»

_2026-07-20. Отчёт — `orchestration/reports/grok-report-17.md`. ХОЛОДНАЯ
сессия. Read-only разведка, часть волны из 4 агентов._

## Общее для всей волны «разведка форка» (одинаково у всех четверых)

**Режим — READ-ONLY по `../Source/`.** Ты НИЧЕГО не меняешь в форке:
ни правок, ни форматирования, ни `cargo fix`, ни удаления `target/`.
Только чтение и `cargo check`/`cargo run --example` на чтение. Любая
правка форка = провал задания.

**Пишешь ТОЛЬКО в `skills/chronos-gpui/`** и только в СВОИ файлы (список
в твоём разделе). `SKILL.md` — файл Архитектора, НЕ трогай: параллельно
работают ещё трое, общий файл = гарантированная коллизия.

**Зачем всё это.** 2026-07-20 выяснилось, что «кровный факт» проекта —
«`overflow_y_scroll` не резолвится в форке, скролла нет» — ЛОЖЬ. Метод
есть (`Source/gpui/src/elements/div.rs:1429`), просто живёт в трейте
`StatefulInteractiveElement`, реализованном только для `Stateful<E>`
(`:3752`) — то есть нужен `.id(...)`. Ошибку компиляции «нет метода»
приняли за отсутствие фичи, и это ограничение расползлось по 6
документам и 2 брифам. Рабочий пример при этом лежал в самом форке:
`Source/gpui/examples/scrollable.rs`. **Твоя задача — чтобы такого
больше не случалось: собрать проверяемую правду о форке.**

**Стандарт доказательности (жёсткий).** Каждое утверждение несёт ЛИБО
`путь:строку` из `Source/`, ЛИБО имя примера, который это демонстрирует.
Формулировки «вроде бы», «判 кажется», «должно работать» — запрещены. Если
что-то не проверил — пиши явно «не проверено». Особая ценность —
находки класса «мы думали X, на самом деле Y»: выноси их в отдельный
раздел `## Ловушки и опровержения` своего файла.

**Батчи по 3 (требование пользователя).** Разбей свою зону минимум на 3
партии и иди партиями, а не пытайся проглотить всё разом: у тебя
холодная сессия и ограниченный контекст. Если у тебя есть возможность
делегировать под-агентам — делегируй партиями по 3 параллельно (каждому
свой непересекающийся список файлов), потом сводишь сам. В отчёте
перечисли, как разбил.

**Что писать про примеры.** Для каждого примера своей зоны: что он
демонстрирует, какой API доказывает, компилируется ли
(`cargo check --example <имя> -p 'path+file:///home/neo/projects/
chronos-ecosystem/Source/gpui#0.2.2'` — спецификатор пакета
ОБЯЗАТЕЛЬНО такой, иначе cargo ругается на неоднозначность `gpui`),
и переносимо ли это на ChronOS (мы — layer-shell шелл, не обычное окно).

**Формат файлов — продакшн-скилл.** Твой `references/*.md`: заголовок,
краткое «когда грузить», затем разделы с таблицами API и примерами кода
С УКАЗАНИЕМ ИСТОЧНИКА. Твой `evals/*.eval.md`: 5-8 вопросов с
ПРОВЕРЯЕМЫМИ ответами (вопрос → ответ → чем доказан: файл:строка или
пример). Эти evals — способ проверить, что скилл реально учит, а не
пересказывает.

**Верификация задания.** `cargo check` тех примеров, что упоминаешь как
рабочие (не на слово). Отчёт — `orchestration/reports/<имя>-report-N.md`,
формат SESSION_REPORT (MEMORY.md §Rules). В отчёте обязательно: сколько
файлов/примеров реально прочитал, что НЕ успел, и список находок
«думали X — оказалось Y».

**Не изобретай архитектуру.** Ты не предлагаешь, как переписать ChronOS.
Ты документируешь, что форк УМЕЕТ. Выводы «нам стоит применить это
там-то» — максимум одной строкой в конце раздела, как заметка.

## Твоя зона (Grok)

**Читаешь:** `Source/gpui/src/elements/**` (div.rs — он огромный, это
ядро), `styled.rs`, `style.rs`, `taffy.rs`/интеграция лэйаута,
`Source/gpui_macros/**` (там ГЕНЕРИРУЮТСЯ style-методы — именно поэтому
их не находит грep по `pub fn`), `geometry.rs` в части единиц (px/rems/
fraction). Примеры своей зоны: `scrollable.rs`, `list_example.rs`,
`data_table.rs`, `grid_layout.rs`, `gradient.rs`, `opacity.rs`,
`pattern.rs`, `painting.rs`, `text_*`, `uniform_list`-подобные,
`anchor.rs`, `popover.rs` — всё про содержимое и раскладку.

**Пишешь:** `skills/chronos-gpui/references/elements-styling-layout.md`
и `skills/chronos-gpui/evals/elements-styling-layout.eval.md`. Больше
ничего.

## Что обязательно раскрыть

1. **Трейтовая иерархия элементов** — `InteractiveElement` vs
   `StatefulInteractiveElement` vs `Styled`: какой метод где живёт и
   ПОЧЕМУ иногда «метода нет». Это корень истории со скроллом — разбери
   его как учебный случай, с `.id()` → `Stateful<E>`.
2. **Скролл целиком**: `overflow_scroll`/`_x_`/`_y_`, `track_scroll`,
   `ScrollHandle` (что умеет: программная прокрутка? scroll_to?),
   `uniform_list`/`list` если есть — виртуализация. Как сделать
   автоскролл к низу (нам нужно для лога терминала).
3. **Что из Style реально ЕСТЬ**, а чего нет: `max_h`/`max_w`
   (проверь — в `gpui-layer-shell` скилле написано, что в `Style` нет
   `max_height`; сверь с исходником и вынеси вердикт),
   overflow-варианты, flex/grid, absolute/anchor.
4. **Списки**: как правильно рисовать длинный список — наивные дети,
   `uniform_list`, виртуализация. У нас список обновлений сейчас
   клипается с костылём «+N more» — оцени, что форк предлагает взамен.
5. **Текст**: усечение/ellipsis, перенос, моноширинный, выделение
   (`text_selection` пример в gpui-component — не твоя зона, но
   отметь ссылкой).

Коммит: `skills : chronos-gpui — элементы, стили, лэйаут, скролл (разведка форка)`.

---

## ⚠️ ТЫ ЗАПУЩЕН ИЗ `Source/` — прочти до первой команды

Тебя стартуют из `/home/neo/projects/chronos-ecosystem/Source`, чтобы не
тащить в контекст шум основного репо. Отсюда три жёстких следствия.

**1. `Source/` — ЭТО ОТДЕЛЬНЫЙ GIT-РЕПОЗИТОРИЙ (наш форк gpui).**
Не наш шелл. В нём НЕЛЬЗЯ:
- `git add` / `git commit` / `git checkout` / `git stash` — НИЧЕГО.
  Твоя работа коммитится в репозиторий ChronOS, не сюда.
- создавать файлы, писать заметки, складывать черновики;
- `cargo fix`, `cargo fmt`, автоправки IDE, «заодно починил варнинг»;
- **`cargo clean`** — в проекте уже был прецедент на −40 ГБ target,
  после которого все ждали пересборку.

Единственное, что тебе тут можно: **читать** и запускать
`cargo check --example …` / `cargo run --example …`.
Если увидишь в `Source/` незакоммиченные чужие правки — не трогай,
просто отметь в отчёте.

**2. Все пути на запись — АБСОЛЮТНЫЕ, в репозиторий ChronOS.**
Корень шелла: `/home/neo/projects/chronos-ecosystem/ChronOS`

- твои файлы скилла:
  `/home/neo/projects/chronos-ecosystem/ChronOS/skills/chronos-gpui/…`
- твой отчёт:
  `/home/neo/projects/chronos-ecosystem/ChronOS/orchestration/reports/…`
- коммитить так (из ChronOS, не из Source):
  ```
  cd /home/neo/projects/chronos-ecosystem/ChronOS
  git add skills/chronos-gpui/<только свои файлы>
  git diff --staged   # глазами
  git commit -m "…"
  ```

**3. Контекст проекта тебе всё равно нужен — читай его по абсолютным
путям** (в `Source/` его нет):
- задание (этот файл целиком):
  `/home/neo/projects/chronos-ecosystem/ChronOS/orchestration/agents/<ТВОЁ_ИМЯ>.md`
- состояние поля и кровные факты:
  `/home/neo/projects/chronos-ecosystem/ChronOS/HANDOFF.md`
- формат отчёта (SESSION_REPORT):
  `/home/neo/projects/chronos-ecosystem/ChronOS/MEMORY.md` §Rules
- скелет и навигация скилла (НЕ редактировать):
  `/home/neo/projects/chronos-ecosystem/ChronOS/skills/chronos-gpui/SKILL.md`

**Проверка перед первым коммитом** — убедись, что ты не в форке:
```
git -C /home/neo/projects/chronos-ecosystem/Source status --short   # должно быть ЧИСТО
```
Если там что-то появилось из-за тебя — откати СВОИ изменения и скажи
Архитектору. Чужое не трогай.

## ✅ ПРИЁМКА №17 — ПРИНЯТО, коммит `f4d2ebc` (2026-07-20, Архитектор)

Независимо перепроверил ключевые находки по исходникам форка (не по
твоему отчёту): трейтовая граница `InteractiveElement`/
`StatefulInteractiveElement` — `div.rs:699/969/1213/1429/1475/1695/3752`
— совпадает 1:1. `FollowMode::Tail` (`list.rs:113`) и `set_follow_mode`
(`:617`) — тоже. Это ценнейшая находка волны: для нашего будущего
терминала (Mimo/desktop_terminal) есть готовый механизм автопрокрутки
к хвосту, не только one-shot `scroll_to_bottom` (`div.rs:4063`).

Коммит чистый (ровно 2 файла), Source не тронут. Твой пост-сдачный
проход через fable-judge + fable-method audit + Philip — правильное
поведение, само по себе доказательство добросовестности: Philip нашёл
у тебя же High-долг (`gpui-layer-shell` учит устаревшему «скролла нет»)
и ты его не скрыл, а вынес в отчёт как follow-up. Так и продолжай.

**Follow-up, роздан отдельно:** патч `gpui-layer-shell` под найденное
(в т.ч. поправка Hermes: `on_scroll_wheel` НЕ требует `.id()`, в отличие
от `on_click`/`overflow_y_scroll` — это на `InteractiveElement`, не на
`StatefulInteractiveElement`).

---

# ▶ АКТИВНО СЕЙЧАС — Задание №18: разведка `gpui-animation` — компилируется ли против нашего форка

_2026-07-20. Отчёт — `orchestration/reports/grok-report-animation.md`
(SESSION_REPORT, MEMORY.md §Rules). ХОЛОДНАЯ сессия: читай `HANDOFF.md`
(верхний блок + кровные факты, особенно про `EasingCurve` уже в форке —
находка DeepSeek №2). Твои №16/№17 приняты._

## Контекст

`https://crates.io/crates/gpui-animation` — декларативный слой
state-driven переходов для GPUI (`.with_transition("id")
.transition_on_hover(duration, curve, |state, style| ...)`). Архитектор
изучил код:

- Зависимости: `gpui = "0.2.2"` (голый, БЕЗ `gpui-component`),
  `dashmap`, `parking_lot`, `smol`. Версия точно совпадает с нашим
  форком.
- Даёт слой ПОВЕРХ уже вшитых в наш форк `EasingCurve`
  (`Source/gpui/src/easing.rs`, 658 строк, Kael-атрибуция — DeepSeek
  №2 нашёл, что порт из DECISIONS.log 2026-07-16 уже сделан). Библиотека
  не math-кривые, а механизм «на hover плавно перейди от стиля A к B».
- **Модель исполнения проверена Архитектором на исходниках:**
  `cx.spawn(Self::animation_tick).detach()` (`src/transition.rs:92`) —
  наш GPUI executor, НЕ третий рантайм. `smol::Timer::after` внутри —
  просто future-примитив в уже нашей таске, не отдельный поток событий.
  Совпадает с DECISIONS «Runtime split». `.detach()` уже стоит в
  библиотеке — она сама соблюдает правило, которое мы весь день ловили
  как баг у СВОИХ виджетов (сеть, DeepSeek №1).
- Рисует НЕ через собственный `Element`/GPU-текстуры, а через обычный
  builder API (`Styled`/`ParentElement`/`FluentBuilder`) — риск ниже,
  чем у чего-то с кастомным paint.
- README: «early development stage, API subject to change» — версия
  `0.2.60` при этом много минорных релизов, готовься к возможным
  breaking changes.
- Кандидат применения (НЕ задача сейчас, только контекст): hover шести
  виджетов правого кластера бара (Mimo №11, сейчас мгновенный щелчок
  цвета через `.hover(|s| s.bg(...))`) — с этой библиотекой стал бы
  плавным переходом.

## Задача — ТОЛЬКО разведка компиляцией, не интеграция

1. Клонируй `gpui-animation` в свой scratch (НЕ в ChronOS, НЕ в
   `../Source/`) — тот же протокол, что Cline применил к `gpui-form`
   (см. `orchestration/report-log/` его отчёт как образец методологии,
   если нужен ориентир по стилю доказательности).
2. Патч `gpui` на `path` к нашему форку. Проверь транзитивные
   зависимости (`dashmap`/`parking_lot`/`smol` — версии, нет ли у них
   своих gpui-зависимостей, маловероятно, но проверь).
3. `cargo check -p gpui-animation` (библиотека сама).
4. **Мини-пример.** Возьми пример из их README (`.with_transition(...)
   .transition_on_hover(Duration::from_millis(300), Linear, |hovered,
   style| ...)`) на простом `div()` с `.id(...)`, собери отдельным
   example в клоне. Это должно доказать: типы совпадают, `cx.spawn`
   реально планируется на нашем executor'е (можешь добавить
   `tracing::info!` внутрь колбэка перехода, если реально запустишь
   пример — но `cargo run` в headless-среде может не иметь Wayland-
   дисплея, тогда останавливайся на `cargo check`, честно пиши почему).
5. Если есть возможность собрать и ЗАПУСТИТЬ пример (у тебя есть
   Wayland-сессия) — живой смок: hover по элементу → плавный переход
   цвета/размера, а не мгновенный щелчок. Если нет — не выдумывай,
   честно `cargo check`-only.

## Кровные факты (не наступи)

- Наш форк — `path`-зависимость, не трогай.
- Ничего не коммитить в `../Source/` или ChronOS — только клон + отчёт.
- `../Source/` read-only.

## Зоны

Твои: только клон вне ChronOS и `../Source/`.

## Верификация

Таблица: крейт/пример → собрался (да/нет) → точная ошибка если нет.
Если дошёл до живого смока — grim/лог. Раздели «компилируется» от
«анимация реально плавная на экране» так же чётко, как это сделал Cline
в №1 (разница между typecheck-уровнем и runtime-поведением).

## Условие эскалации

Если `cx.spawn(Self::animation_tick)` не типизируется против нашего
`Context`/`App` (сигнатура разошлась с апстримом) — это и есть находка,
зафиксируй точную ошибку типов, не пытайся патчить их код.

Коммит: не нужен (ChronOS не меняется). Только отчёт.

## ✅ ПРИЁМКА — Задание №18 (`gpui-animation` разведка), ПРИНЯТО (2026-07-20, Архитектор)

Самая строгая разведка из трёх сегодняшних, и вердикт другой — не «да
без правок», а честное «нет из коробки, да после 3 хирургических
патчей их кода». Перепроверил каждую цифру независимо:

- Оба репо чисты, диф клона = ровно заявленное (`Cargo.toml/.lock`,
  `src/{transition,interpolate}.rs`, новый `mini_hover.rs`).
- **Все три API-дельты подтверждены в исходниках форка лично:**
  `AsyncApp::update` → `R` не `Result<R>` (`async_context.rs:163`),
  `BoxShadow.inset: bool` — пятое поле сверх стандартных четырёх
  (`style.rs:353`), `Style.text` помечен `#[refineable]`
  (`style.rs:291`). Не пересказ — прочитал сам.
- **Перепрогнал компиляцию в обе стороны.** Патчи на месте →
  `cargo check` чист. Временно откатил ТОЛЬКО правки исходников (не
  `Cargo.toml` — иначе тест был бы против настоящего crates.io, не
  нашего форка; сам сначала ошибся так же, поймал и переделал) →
  получил РОВНО 8 ошибок той же классификации (`E0063`×1, `E0308`×4,
  `E0599`×3), что в таблице отчёта.
- **Живой смок — не поверил описанию, посмотрел все три скриншота
  глазами.** idle: тёмный box. mid: явный промежуточный синий,
  отличный и от idle, и от hover — доказательство интерполяции, а не
  snap. hover: целевой светло-голубой. Это лучшее доказательство за
  весь сегодняшний день — переходный кадр либо есть, либо его нет, тут
  спрятаться некуда.

**Особо ценю честность вывода вопреки ожиданию из брифа.** Бриф
предполагал (с моих слов) параллель с Cline — «наверное, тоже
скомпилируется чисто». Ты не подогнал результат под ожидание: нашёл,
что не компилируется, разобрал причину до конкретных строк, и это
честнее и полезнее, чем красивое «да» было бы. Условие эскалации
брифа (несовместимость `cx.spawn`) не сработало — ты это тоже прямо
написал, не подгоняя находку под сценарий, который я предполагал.

**Побочная, но важная находка для канона:** «версии 0.2.x совпадают» —
НЕ значит совместимость ABI/API, когда одна сторона git/path-форк, а
другая — crates.io тех же цифр. Это общее правило для ЛЮБОЙ будущей
разведки внешних gpui-крейтов, не только этого случая.

**Решение по применению:** без форка библиотеки (или `[patch]` +
vendored tree с этими 3 правками) ChronOS не может зависеть от неё
as-is. Патчи малы и стабильны, но это меняет вес решения — не
«добавить зависимость», а «взять на сопровождение форк библиотеки».
Записываю в roadmap как «требует форка библиотеки», не как «готово к
использованию» — статус ощутимо ниже, чем у `gpui-form`/`gpui-rsx`.

Клон можно снести — не блокер.

---

## Задание — ВЕНДОР `gpui-animation` в `Source/` (2026-07-20, продолжение №18)

**Одной строкой:** разведка принята, решение — **берём**. Твоя задача:
превратить твой же патченный клон в постоянный vendored path-крейт
`Source/gpui-animation/`, который ChronOS сможет держать как локальную
зависимость. Это НЕ новая разведка — механический вендоринг того, что
ты уже доказал компиляцией.

### Контекст (полный — рассчитывай на холодную сессию)

`gpui-animation` (`github.com/chi11321/gpui-animation`, crates.io
`0.2.60`, upstream `ad77bea`, лицензия **MIT OR Apache-2.0**) —
декларативный слой переходов поверх gpui (`.transition_on_hover(...)`).
Против нашего форка gpui (`gpui-ce chronos edition`,
`/home/neo/projects/chronos-ecosystem/Source/gpui`) **не** компилируется
из коробки — нужны 3 хирургические правки ИХ исходников (ты их нашёл в
№18). Cargo не умеет патчить чужой source через манифест → единственный
путь взять библиотеку = вендорить её код к нам и держать эти 3 правки на
сопровождении. Отсюда эта задача.

Твой рабочий клон с уже применёнными патчами:
`/home/neo/scratch/gpui-animation-recon` (там же `target/`, патчи в
`src/{transition,interpolate}.rs`).

### Что произвести

Новый крейт-директория `Source/gpui-animation/` (СОСЕД `Source/gpui/`),
несущий их исходник с применёнными 3 дельтами:

1. Скопируй из клона в `Source/gpui-animation/`: весь `src/`, `Cargo.toml`,
   и **их файлы лицензии** (`LICENSE*`/`COPYING*` если есть; если в
   апстриме их нет физически — создай `LICENSE-MIT` и `LICENSE-APACHE`
   стандартными текстами, т.к. `license = "MIT OR Apache-2.0"` в их
   Cargo.toml это требует юридически). Атрибуция — обязательна, это
   Apache-2.0 периметр.
2. В `Source/gpui-animation/Cargo.toml`: зависимость `gpui` → `gpui = {
   path = "../gpui" }` (в клоне было `path` на абсолютный
   `.../Source/gpui`; здесь — относительный `../gpui`). Версию/фичи
   сохрани как в клоне. Добавь в начало `Cargo.toml` коммент-строку:
   `# vendored from github.com/chi11321/gpui-animation @ad77bea (MIT OR
   Apache-2.0); 3 fork-deltas applied — see PATCHES below`.
3. **Три дельты — применить дословно** (это твои же находки из
   `report-log/grok-report-animation.md`, привожу для холодной сессии):

   - **Дельта 1 — `AsyncApp::update` non-fallible.** В нашем форке
     (`Source/gpui/src/app/async_context.rs:163`) `update<R>(...) -> R`,
     не `Result<R>`. Их код зовёт `cx.update(...).ok();` в
     `src/transition.rs` (было `:207` и `:243`). Убрать `.ok()` в обоих
     местах (метод `.ok()` не существует на `()`).
   - **Дельта 2 — `BoxShadow.inset`.** Наш форк добавил пятое поле
     `pub inset: bool` (`Source/gpui/src/style.rs`, ~`:353`). Их
     `impl Interpolatable for BoxShadow` (`src/interpolate.rs`, ~`:384`)
     не заполняет `inset` → `E0063`. Добавить в конструируемый
     `BoxShadow`: `inset: if t < 0.5 { self.inset } else { other.inset }`.
   - **Дельта 3 — `Style.text` `#[refineable]`.** В нашем форке
     `Style.text` помечен `#[refineable]` (`Source/gpui/src/style.rs`,
     ~`:291`), поэтому в `StyleRefinement` поле `text` —
     `TextStyleRefinement` напрямую, не `Option<...>`. Их
     `fast_interpolate` (`src/interpolate.rs`, ~`:446-456`) матчит
     `Option` → `E0308`. Заменить на прямой вызов:
     `self.text.fast_interpolate(&other.text, t, &mut out.text)` (без
     `Option`-разбора, без `as_mut`).

   Строки могли сдвинуться при копировании — ищи по конструкции, не по
   номеру. В твоём клоне патчи УЖЕ применены — можешь просто перенести
   исходники как есть и сверить, что все три на месте.
4. **Задокументируй патчи внутри крейта:** создай
   `Source/gpui-animation/PATCHES.md` с этими тремя дельтами (что, где,
   почему — форк-vs-crates.io), чтобы будущий апдейт библиотеки знал, что
   переприменить. Это не бюрократия: без этого файла следующий, кто
   дёрнет `0.2.61`, потеряет правки молча.

### Зоны — СТРОГО

- **Пишешь ТОЛЬКО в `Source/gpui-animation/`** (новая директория).
- **НЕ трогаешь:** `Source/Cargo.toml` (workspace members),
  `Source/NOTICE`, любой файл в ChronOS. Проводку в workspace + NOTICE +
  зависимость `crates/app` делает Архитектор на приёмке. Если тронешь
  shared-файл — это #1 эпидемия проекта (несамодостаточные коммиты
  общих файлов), отчёт заверну.
- Чужие незакоммиченные правки в `Source/` (если увидишь) — не трогать,
  отметить в отчёте.

### Верификация

Новый крейт обязан компилироваться против форка:
```
cd /home/neo/projects/chronos-ecosystem/Source/gpui-animation
cargo check
```
Если корневой workspace `Source/` попытается «поглотить» крейт и cargo
ругнётся `current package believes it's in a workspace` — **временно**
добавь пустую секцию `[workspace]` в `Source/gpui-animation/Cargo.toml`,
прогони `cargo check`, потом **убери** её и явно напиши в отчёте, что так
делал (Архитектор решит членство в workspace на проводке). Не оставляй
`[workspace]` в финальном крейте.

Отчёт: таблица «файл → скопирован/пропатчен → cargo check да/нет»,
точный вывод `cargo check` (EXIT code), список всех трёх дельт с
подтверждением «на месте», `git -C ../.. status --short` в Source (должна
светиться ТОЛЬКО твоя новая директория + ничего в shared-файлах).

### Коммит

**Не коммить.** Отдаёшь рабочее дерево `Source/` с новой директорией +
отчёт. Проводку и коммит (в Source, с атрибуцией в NOTICE) делает
Архитектор. Отчёт → `orchestration/reports/grok-report-animation-vendor.md`.

### Условие эскалации

Если при переносе всплывёт ЧЕТВЁРТАЯ ошибка компиляции (не одна из трёх
известных дельт) — СТОП, зафиксируй точную ошибку типов, не изобретай
правку. Это значит, что копирование что-то потеряло или форк сдвинулся —
разберём вместе.

## ✅ ПРИЁМКА — вендор `gpui-animation`, ПРИНЯТ (2026-07-21, Архитектор)

Независимо воспроизвёл всё:
- `Source/gpui-animation/` на месте: `src/`, `Cargo.toml` (gpui=`../gpui`),
  `PATCHES.md`, `LICENSE-MIT/APACHE` (апстрим их не шипил — созданы честно),
  `README.md`. examples не тащил — правильно.
- **Три дельты сверил построчно:** `transition.rs:207/243` — `cx.update(...)`
  без `.ok()` + коммент `// fork:`; `interpolate.rs:390` — `inset: if t<0.5`;
  `:451` — прямой `self.text.fast_interpolate`. Остаточных `.ok()` на update
  ноль. Четвёртой ошибки нет.
- **Сам прогнал `cargo check` против форка** (временный `[workspace]`, убрал
  после) → `Finished 26.02s`, EXIT 0, ноль ошибок в крейте (warnings — это
  missing-docs самого форка). `[workspace]` в финале отсутствует, `Cargo.lock`
  снят.
- Source чист: `?? gpui-animation/`, shared-файлы (`Cargo.toml`/`NOTICE`) не
  тронуты — дисциплина зон соблюдена идеально.

Проводку (workspace.members + NOTICE-атрибуция + коммит в Source) сделал
Архитектор. Отчёт архивирован в `report-log/grok-report-animation-vendor.md`.
Образцовый вендоринг: PATCHES.md под будущий апдейт — ровно та
предусмотрительность, которой не хватает большинству.

---

## ЗАДАНИЕ (капстоун «правая панель», Tasks 3+4) — сервис `system_resources`: CPU/RAM/GPU

**Контекст.** Правой панели нужны живые метры CPU/RAM/GPU (спектр-бары, Task 10
навесит UI). Ты пишешь БЭКЕНД-сервис `system_resources` по нашему `Service`-
паттерну (Mutable + poll-луп), как `UPowerSubscriber`/`CavaSubscriber`. Две
задачи одним заходом: **Task 3** (CPU/RAM через `sysinfo`) + **Task 4** (GPU-
процент через `nvml-wrapper`, деградация до `None` без Nvidia). Task 4 расширяет
тот же файл — поэтому вместе, один агент.

**Читай план — Task 3 (строки 479-688) + Task 4 (692-795).** Там ПОЛНЫЙ исходник
`types.rs`, `mod.rs`, тесты и процедура TDD. Ниже — факты и КРИТИЧНОЕ про зоны.

### Что производишь

- `crates/services/src/system_resources/types.rs` — `SystemResourcesState
  { cpu_percent: f32, ram_percent: f32, gpu_percent: Option<f32> }`
  (`#[derive(Clone,Debug,Default,PartialEq)]` — Float → НЕ derive Eq).
- `crates/services/src/system_resources/mod.rs` — `sample_cpu_ram(&mut System)`
  (чистая, тестируемая без рантайма), `sample_gpu(Option<&Nvml>) -> Option<f32>`
  (Task 4), `SystemResourcesSubscriber` (impl `Service`), `run()` poll-луп 1с.
  NVML init ОДИН раз вне лупа, `None` при любой ошибке (нет Nvidia/драйвер) —
  строку GPU панель просто скроет, это НЕ ошибка лупа.
- Тесты: `sample_cpu_ram_reads_real_host_values_in_range` (percentages в [0,100]),
  `gpu_sample_none_when_nvml_unavailable_does_not_panic`. На ЭТОЙ машине
  (RTX 3070) NVML инициализируется → `Some(_)`, но тест держит инвариант для
  обеих веток. sysinfo первый sample может дать 0.0 (нужен второй вызов ~200мс
  спустя) — это документированное поведение sysinfo, не баг.

### Зоны — ВНИМАНИЕ, project-wide файлы (kровный факт эпидемии)

- **Полностью твои (создаёшь):** `crates/services/src/system_resources/{mod,types}.rs`.
- **Shared — правишь МИНИМАЛЬНО, и ЯВНО перечисляешь в отчёте построчно:**
  - `Cargo.toml` (корень): `[workspace.dependencies]` += `sysinfo = "0.39.3"`,
    `nvml-wrapper = "0.12.1"`.
  - `crates/services/Cargo.toml`: `[dependencies]` += `sysinfo.workspace = true`,
    `nvml-wrapper.workspace = true`.
  - `crates/services/src/lib.rs`: `pub mod system_resources;` + поле в `Services`
    (`pub system_resources: …Subscriber`) + строка в `init_all`.
  - `crates/app/src/state.rs`: accessor `AppState::system_resources(cx)`.
  Эти правки нужны, чтобы твой модуль компилировался и тестировался — делай, но
  держи в них ТОЛЬКО свои строки. **Параллельно GLM пишет Task 5 (power) и тоже
  правит `lib.rs`/`state.rs`.** Чтобы не затереть друг друга: работай в
  worktree-СОСЕДЕ ChronOS (`git worktree add ../ChronOS-grok`, path-депы на
  ../Source не ломаются) ИЛИ убедись что твоя сессия единственная на дереве.
  Финальную проводку shared-файлов сверяю и коммичу Я на приёмке — твой отчёт
  должен перечислить точные добавленные строки, чтобы я их чисто применил.

### Верификация (приложи вывод)

1. `cargo test -p chronos-services --lib system_resources` — тесты зелёные
   (включая gpu-тест; на RTX 3070 вернёт `Some`).
2. `cargo build --workspace` — чисто.
3. Живьём (Task 3/4 — данные железа, «зелёный билд» не всё): краткий смок, что
   `sample_gpu` на этой машине даёт разумный процент (можно `cargo test` с
   `--nocapture` + временный `dbg!`, убрать до отчёта). Полный UI-смок под
   нагрузкой — это Task 10, не твой.
4. `git status --short` — твой модуль + перечисленные shared-строки, ничего лишнего.
5. `let _ = fallible()` запрещён; NVML/sysinfo ошибки — `.ok()`/`match`, не глушить.

### Коммит

**Не коммить.** Дерево + отчёт → `orchestration/reports/grok-report-19.md`.
Формат SESSION_REPORT: исход первой строкой, вывод тестов/билда, ЯВНЫЙ список
добавленных строк в shared-файлы. Проводку и коммит — Архитектор.

### Эскалация

`nvml-wrapper 0.12.1` не собирается против системного драйвера / API изменилось,
или `sysinfo 0.39.3` даёт другой API (`global_cpu_usage`/`refresh_cpu_usage`
переименованы) — СТОП, зафиксируй реальную сигнатуру, спроси. Версии не понижай
без добра (bleeding-edge политика).

# HANDOFF — контекст для новой сессии Архитектора

**Обновлено: 2026-08-18 — T266 и T271 приняты, HEAD починен, T303
переписан, T304/T305 заведены, `docs/orchestration/` ЗАКРЫТ.** HEAD —
см. `git log -1`.

**`docs/orchestration/` закрыт и физически удалён.** Кухня целиком в
`.chronos-ops/`: тикеты `active/<role>/` (+ `hold/`), инбокс отчётов
`reports-fresh/`, принятое `reports-log/<role>/` + `done/<role>/`,
`rework/<role>/`, `reject/<role>/`, заметки и кадры-улики `dump/notes/`,
старые точки входа ролей `dump/legacy-agents/`, реестр T-ID —
`.chronos-ops/MIGRATION.md`. Навигационные ссылки переписаны в
`CLAUDE.md`, `AGENTS.md`, `README.md`, `CONTRIBUTING.md`,
`checkpoint/ARCHITECT.md`, `checkpoint/TBD.md`, `.chronos-ops/README.md`.
Таблица каталогов из `ARCHITECT.md` убрана — единый источник теперь
`.chronos-ops/RULES.md`, дублировать её негде. Внутри архивных тикетов
и отчётов старые пути НЕ правились — исторический слепок.

**В инбоксе `reports-fresh/` лежат три отчёта, приёмки не было:**
T284 (frame Hide/Wrap — код в `d01820e`, тикет открыт в `active/front/`),
T281 (PARK — не архивировать до `+` владельца), T299 (разметка ролей для
167 архивных тикетов, RECON).

**T271 (проглоченные `Result` в IPC) ПРИНЯТ.** Код в `e172327b`. Своя
приёмка: `rg let _ =` по `crates/app/src/ipc/` → три места, все внутри
`#[cfg(test)]`; 14 `warn!` (мёртвый ресивер) + 4 `debug!` (teardown) —
посчётно как в отчёте; `mod.rs` 16 снятых `let _ = cx.update` → 16
голых; `unwrap`/`expect` не добавлены; lib 597/597, bins 789/789;
`cargo check` без предупреждений по `ipc/`.

**Главное из T271 — бриф был неправ, исполнитель это доказал.**
`AsyncApp::update` в форке возвращает `R`, не `Result`:
`../Source/gpui/src/app/async_context.rs:163`
`pub fn update<R>(&self, f: impl FnOnce(&mut App) -> R) -> R`. Значит 16
`let _ = cx.update(...)` в `ipc/mod.rs` **не глотали ошибку** — там её
нет, это шум. Не переоткрывать этот вопрос в будущих тикетах про
`let _ =`: для `cx.update` правильная правка — снять `let _ =`, а не
заворачивать в `if let Err`.

**Живой смок T271 прогнан мной** (оговорка отчёта про старый бинарь
устарела — release пересобран 00:20, коммит 21:53): `chronos-ipc ping`
exit 0, `toggle-side-panel-right` дважды — слои
`side_panel_right_rail`/`_content` ушли и вернулись.

**T266 (прозрачность поверхностей + блюр) ПРИНЯТ.** Своя приёмка:
`chronos-ui` 22/22, `chronos --lib` 597/597, `chronos-services
compositor` 4/4, `cargo check --bins` чисто — все три числа отчёта
сошлись с прогоном. Дефолт непрозрачный (`alpha = 1.0`, blur off,
`surface.rs:37-39`) — критерий «свежая установка пиксельно не
меняется» держится структурно. Тумблер блюра гейтится честно
(`probed && capability == Available`, `bar_settings.rs:359`) — планка
T246 соблюдена.

**Мой промах, найден приёмкой:** коммит `d01820e` был
несамодостаточным — `HEAD:crates/ui/src/theme/mod.rs:14` делал
`pub mod surface;`, а сам `surface.rs` в коммит не попал. Чистый
checkout HEAD не собирался ~сутки. Починено `d0c565d6` (три файла
хвоста: `surface.rs`, калиброванные флоры `DEFAULT_MIN_ALPHA` /
`LIGHT_MIN_ALPHA` = 0.70, `packaging/hyprland/45-surface-effects-chronos.lua`).
Эррата `e8b1b273`: в `surface_effects.rs` блок `if/else` возвращал
`Result`, но стоял statement'ом — ошибка применения persisted blur
терялась молча (класс T271, компилятор ловил warning'ом).

**Долг T266 на владельце:** попапы (volume / OSD / notifications /
tray / dock) живьём НЕ сняты — покрытие аналитическое через
связывающую плату `bg.elevated`. Приёмочный смок: `surface_alpha`
0.7, попапы на светлых и на тёмных обоях.

**Живые находки Lua-Hyprland 0.56.2 (T266, Task 6):** глобальный
`decoration.blur.enabled` обязателен — без него корректное layer-rule
не рендерит ничего; `ignore_alpha` в layer rule МОЛЧА убивает блюр;
`hl.layer_rule` идемпотентен по имени (обновление файла требует
рестарта Hyprland, `hyprctl reload` сбрасывает eval-глобалы →
probe вернёт `ModuleMissing`).

**T303 переписан** под фактическое состояние дерева: геометрия рамки
уже уехала в `d01820e` (единое кольцо `border` + `rounded`, не 5 div
с corner-патчами), тикет протух до раздачи. Остаток — P2: развести
`wrap.thickness` и `bottom_strip.height`, убрать `T303DEBUG`-лог,
живой grim.

**T304/T305 заведены** (`c8728c8`): settings-табы правого рейла
(System[0], Updates[1], Notifications[2], AcpSettings[13],
EditorSettings[17] = `BarSettingsTab`, HyprlandBinds[18], Display[19],
LauncherSettings[20]) уезжают в один anchored slide-popup
(control-center, видео-референс владельца). Тяжёлые work-tools и
пустышки Mcp/Lsp/ApiProviders остаются на докнутом рейле. T304 —
предварительный (`TabContent::create` → `&mut App`), T305 не
стартует до его приёмки: общий `tab/mod.rs`. `power_row.rs` — на
удаление (power покрыт `start_menu::rail_power_actions`, network/battery
— виджетами бара).

**Очередь FRONTEND:** T302 → T304 → T305 → T303 → T301.

**T298 (composer Select popup) ПРИНЯТ частично.** Вертикальный
клиппинг исправлен по корню — `content_size()` в
`Source/gpui_linux/.../window.rs` теперь возвращает `window_bounds.size`
вместо стухшего `bounds.size` (форк, коммит `cf34cf6`, тащит
`Co-Authored-By: Codebuff` — не почищено, отдельная эррата). Прочитан
и подтверждён на уровне источника (не с чужих слов): `window_bounds`
реально обновляется в `handle_xdg_surface_event`, layer-shell configure
роутится туда же. `cargo test -p gpui_linux` 23/23, `cargo test
--workspace --lib --bins` (ChronOS) 575/576 (1 фейл — доказанный флейк
не по теме, изолированный прогон зелёный), `cargo build --release`
чисто. Живой смок архитектора не дошёл до самого попапа — **найдена
отдельная находка T302** (контентная зона левой панели рендерится
пустой). Текст в попапе всё ещё без эллипсиса — **T301**. Тикет
`done/`, отчёт `report-log/`.

**Новые тикеты:** T301 (FRONTEND, P3, хвост T298 — эллипсис текста),
T302 (FRONTEND, P1, живая находка — левая панель открывается, но
контент не рисуется, сквозь него видны обои; логи чистые, без
panic/error). Оба в `.chronos-ops/active/front/`, `FRONTEND.md`
заполнена.

**Кухня `.chronos-ops` окончательно ПОД git** (368+ файлов) —
контрибьюторы берут задания отсюда. Весь рабочий канон архитектора
физически переехал из `docs/` сюда же, в `.chronos-ops/checkpoint/`:
этот `HANDOFF.md`, `ARCHITECTURE.md`, `ARCHITECT.md`, `TBD.md`,
`SOUL.md`, `MEMORY.md`, `REJECTED.md` (бывший `.chronos-ops/checkpoint/REJECTED.md`,
переименован). `.chronos-ops/design/` → `.chronos-ops/design/`,
`.chronos-ops/superpowers/` → `.chronos-ops/superpowers/`, обе as-is. `docs/`
теперь только продукт+сайт: `product/`/`style/`/`guides/` подпапки,
плюс `index.html`/`.nojekyll`/`landing/` (GitHub Pages, не переезжает
никогда) и `hyprland/` (живой конфиг, не переезжает).

Кухонный архив: 334 тикета расклассифицированы по ролям (`git mv`), 167
непонятых — **T299** (RECON). **T300** (BACKEND) — `docs/hyprland/
chronos-launcher.lua` vs `packaging/hyprland/40-windowrules-chronos.lua`
разошлись, комментарий "canonical copy" врёт, не смержено намеренно.

Дерево воркетри полностью прибрано: 7 стухших/уже-смерженных воркетри
снесены (сверено побайтовым diff, не только `git merge-base`), 8
мёртвых веток удалены. В поле — только основная копия (T298 уже
закрыт, живой правки в дереве не осталось) и парковая ветка
`feat/t285-acp-load-session` (T285 STOP).

`CLAUDE.md` чекпоинт-протокол расширен до 5 слоёв — добавлен lean-ctx
`ctx_knowledge`.

**Дальше:** T302 (P1, живая находка) → T301 → T299/T300 разбор
(RECON/BACKEND) по готовности исполнителей.

---

**Чекпоинт #35 (2026-08-17) — T287-B принят, T298 отклонён.**
HEAD `d345c23`. Два Архитектора, каждый своя зона — T265-H не моя, была
уже принята на второй сессии (в `done/`).

**T287-B (Sessions tab на кит `List`/`VirtualList`) ПРИНЯТ.** Исполнитель
Гермес/LongCat-2.0 (Nous free-тир) сначала завис на час — три подряд
900с обрыва стрима (`RemoteProtocolError`) + `401 out of funds` у
провайдера. Первая правка была мусором (докстринг + битые импорты,
`git checkout --` откатил). Тот же процесс (не убитый) дописал реальную
реализацию после нуджа: `+1061/-84`, кит `List`/`Input`/`PopupMenu`,
pin/archive/rename/delete напрямую через `ThreadStore`. Проверено
вживую: `cargo check`/`test`/`build --release` чисто, владелец прогнал
grim — pin/delete/rename/archive все прошли. Код `8e84d3f`, докс
`05cd0dd`, в `done/`.

**T298 (composer Select popup обрезан, дочерний T287-A) ОТКЛОНЁН.**
Тот же исполнитель применил `.menu_width(px(280/200))` — реальный код,
формула `bounds.size.width + px(2.)` сверена в `select.rs:554`,
компилируется. Отчёт сам честно не гонял grim. Владелец прогнал живьём:
**оба симптома живы** — попап всё ещё уезжает вниз (скриншот показывает
рабочий стол сквозь дыру ПОД границей layer-shell окна панели, не
просто overflow_hidend внутри канваса — баг серьёзнее гипотезы брифа),
текст строк всё ещё обрезан справа. Отчёт → `rejected/`, тикет остался
в `active/`, некоммиченный `.menu_width` фикс оставлен в дереве (может
быть частью верного решения). Следующий заход — смотреть границы
layer-shell popup-окна (`chronos-gpui-popup`/`gpui-layer-shell`/
`anchored-popups` скиллы), не внутреннюю вёрстку кита; тот же класс
бага что T243 wobble/T226 mx-overflow.

Заодно `reference/gpui-shell-main` и `reference/kael-main` снесены
(43M, отработаны, code-study завершён); `reference/waytrogen-main`
остался — живой донор CLI-контрактов для wallpaper-сервиса, терять
нельзя, своей альтернативы ещё нет.

---

**Обновлено: 2026-08-16 (чекпоинт #34 — эта сессия).**
HEAD `5ddfff4` (+ чужой `05cd0dd` T287-B live accept). Эта сессия:
T265-H v1 LIVE PASS, эпик T265 в `done/`. Super tap = Пуск, Super+Space =
OSD. Кухня: репо `.chronos-ops/` в git, очередь ещё orchestration;
экосистемная отдельно. T271 обновлён, ещё нужен (гигиена ipc). С T298
параллель: T271 или T284 без левой; не T266. Доработка Пуска — новый
тикет, не воскрешать T265. `composer.rs` в дереве грязный — не наш.

---

**Обновлено: 2026-08-16 (чекпоинт #33 — скелет кухни в git).**
`ChronOS/.chronos-ops/` закоммичен. Очередь ещё в
`docs/orchestration/`. Тикеты переезжают по одному, не пачкой.
T287-B не трогать.

---

**Обновлено: 2026-08-16 (чекпоинт #32 — кухни).**
У каждого репо своя `.chronos-ops/`. Экосистемная
(`chronos-ecosystem/.chronos-ops/`) — отдельная, только кросс-репо.
ChronOS-кухня заменит `docs/orchestration/` после cutover. Скелет
`RULES.md` переписан под репо. Cutover не начинать, пока T287-B в поле.
Коммитить `.chronos-ops/` в git — всё ещё не решено.

---

**Обновлено: 2026-08-16 (чекпоинт #31 — эпик T265 закрыт).**
Зонтик `T265-launcher-full-functionality` уехал в `done/`. Активных
launcher-тикетов нет. Доработка лаунчера/Пуска — новый тикет, не
воскрешать эпик.

---

**Обновлено: 2026-08-16 (чекпоинт #30 — T265-H v1 LIVE PASS).**
Владелец принял меню «Пуск» живьём. Дорабатывать потом, не сейчас.
После `#29`: Super tap / Super+Space, flush к монитору, меню поверх
левой панели, мокап `(1)` влит, Lock/Sleep в `assets.rs`, Files →
вкладка. Не трогать, пока не попросят.

---

**Обновлено: 2026-08-16 (чекпоинт #29 — T265-H принята).**
HEAD после приёмки — `6774bc6` + эррата flush/panel (этот чекпоинт).

**T265-H ПРИНЯТА** `feat(launcher): start menu — second surface on Layer::Overlay`.
Один заход, ядро + полный рейл. Overlay, не AnchoredPopup. Общая модель
с OSD. `dock-start` → меню, SUPER+R → OSD. Мой прогон (worktree, без
WIP T287-B): `start_menu` **9/9**, `--lib` **573/573**, release чисто.

Живой кадр владельца: меню открылось (`hyprctl` `chronos-start-menu`
`16,30 720×520`). Эррата владельца в том же заходе: (1) 16px слева —
сцена мокапа, не геометрия; прижать к монитору (`margin left = 0`);
(2) не закрывать левую панель — меню поверх неё. `lock.svg`/`suspend.svg`
не в `assets.rs` — Lock/Sleep пустые, не блокер. Files empty-state без
`select_tab` — хвост. Мокап владелец ещё полирует.

---

**Обновлено: 2026-08-16 (чекпоинт #28 — T297 принята).**
HEAD `9da7b92`.

**T297 ПРИНЯТА** `fix(launcher): submenu render, live favorites, curated categories`.
A: Desktop Actions плоским списком, не `submenu()`. B: зеркало
`self.config` в `apply_config_derived` (сигнал уже шёл — тикет устарел
после G). C: эвристику не брали, Hide from list. D: 11 Main Categories.
Зона 3 файла. Мой прогон (worktree, без WIP T287-B): launcher **84/84**.
Live grim — долг.

T265-H **в поле, тикет толстый** — приёмка слоями: ядро (Overlay +
All Apps + поиск) обязательно; рейл и power-футер можно вынести в H2/H3
из отчёта, не отклонять H. Канон `.chronos-ops/design/chronos-start-menu.html`.

---

**Обновлено: 2026-08-16 (чекпоинт #27 — T265-G принята, live открыт).**
HEAD `35fbb035` (+ docs после неё).

**T265-G ПРИНЯТА** `feat(launcher): settings page in right panel`.
`PanelTab::LauncherSettings` (ALL 21, не в `for_mode`), страница 7 групп,
`launcher_config` watcher 300ms, `apply_config_derived`, Tune в футере →
`select_tab`. Слайдеры — `bar_settings::slider_control`, toggle — `Switch`.
Cargo.lock / Source / left-panel не тронуты. Сверил сам.

Мой прогон: launcher **83/83**, side_panel_right **198/198**, `--lib` **563/563**.

Не блокер: у Hidden apps нет поиска (спека просила); категории только
Show, скрыть новую из UI нельзя. Live grim: колонки сразу на OSD, Unhide,
Tune → вкладка.

T265-H тогда ещё ждала мокап (снято в чекпоинте #28).

---

**Обновлено: 2026-08-16 (чекпоинт #26 — параллельная сессия, docs-правки).**
HEAD `c5151fb9` на момент записи — **не мой коммит**, другая
Architect-сессия параллельно приняла T265-F и T293 (см. #24/#25 ниже,
чекпоинты которых я не писал) и следом закоммитила `T286` (composer →
gpui-component Input) без приёмки. Не трогал T286 — раз рядом кто-то уже
делает приёмку в реальном времени, второй параллельный обзор того же
тикета создаёт риск гонки (ровно то, что уже ловил на T294/T265-B этой
сессией). Следующей сессии: проверить, принят ли T286 к этому моменту,
не удваивать работу.

Моя часть этого чекпоинта (см. файловую память
`checkpoint-2026-08-16-23-launcher-wave-plus-docs.md` за подробностями):
T275/T265-A..D/T294/T295 приняты живьём, T285 закрыт STOP (апстрим
`agent-client-protocol` 2.0.0 — `attach_session` приватный), T286/T287
разблокированы файлово, T297 заведена (эррата лаунчера — submenu
рендерится за границей окна, favorites не live-обновляются, дубль
hermes.desktop/hermes-desktop.desktop не баг дедупа, категорий тьма без
фильтра на XDG Main/Additional). `mold`+`sccache` подключены и
подтверждены живым прогоном. **README/CONTRIBUTING/AGENTS.md переписаны
в рабочем дереве, НЕ закоммичены** — показаны владельцу, жду одобрения
по тону перед коммитом.

---

**Обновлено: 2026-08-16 (чекпоинт #25 — T293 принята).**
HEAD `2ffe9e5`.

**T293 ПРИНЯТА** `feat(right-panel): Notifications tab replaces history popup`.
Колокольчик → `select_tab(Notifications)`, попап снесён, общий `history_list`.
`for_mode` оба режима после Updates, width 420, `icons/bell.svg`, MarkAllRead
при открытии. Тосты не тронуты. Left-panel WIP не в коммите.

Эррата моя: `default_dev_top`/`default_gamer_top` не содержали вкладку
(спека требовала тот же слот); у вкладки не было скролла — колонка
`overflow_hidden`, длинная лента обрезалась. Дописал. Empty T269-hero
с `bell` не сделали — остался текст «No notifications»; не блокер.

Мой прогон: `--lib` **558/558**. Live grim: колокольчик → вкладка, попапа
в hyprctl нет, тост жив — не закрыт.

---

**Обновлено: 2026-08-16 (чекпоинт #24 — T265-F принята с эрратой, live открыт).**
HEAD `ba810d8` + однострочная эррата теста.

**T265-F ПРИНЯТА** `feat(launcher): system action header`. Зона: `power.rs` (вынос `PowerAction`+arm), `launcher/system_actions.rs`, шапка `view.rs`, `services/power` lock/suspend/hibernate, `power_row.rs` UI-only. Cargo.lock / Source не тронуты. Сверил сам:

- 6 действий, `[system_actions] order`, мусор → дефолт+warn;
- Lock/Sleep/Hibernate один клик; Logout/Restart/Shutdown arm 3s + `Confirm?`;
- кнопки `gpui-component::Button` (Ghost/Danger/disabled+tooltip);
- Lock = `loginctl lock-session`; sleep/hibernate = `systemctl`; `/sys/power/state`;
- аватар `~/.face` / AccountsService, иначе инициал.

Эррата: `folder_serializes_and_reloads` не заполнил новый `system_actions` — `cargo check` зелёный, `cargo test --lib` красный. Минион честно не гонял `--lib` из-за чужого T293; я прогнал в worktree. На этой машине `disk` есть в `/sys/power/state` — disabled Hibernate живьём не увидим.

Мой прогон (worktree `ChronOS-wt-t265f`, чистое дерево `ba810d8`+эррата): `--lib` **557/557**; `chronos-services power` **10/10**. `--release` — см. ниже / в коммите.

**Live grim F не закрыт:** шапка, Lock, Confirm? (reboot не жать). T265-G разблокирована, не выдана. T293 WIP на master не трогал.

---

**Обновлено: 2026-08-16 (чекпоинт #23 — T265-E принята, live открыт).**
HEAD `52866c6`.

**T265-E ПРИНЯТА** `feat(launcher): prefix providers shell, files, calc, help`.
Зона: `launcher/providers/{mod,calc,shell,files,sysinfo,help}.rs` + `mod.rs` + `view.rs`. Cargo.lock / Cargo.toml / Source не тронуты. Сверил сам:

- `parse_prefix` + один `results()`-диспетчер; `view.rs` не раздут match'ем;
- `>` Enter-only `setsid $SHELL -lc` cwd `$HOME`; `/`/`~` → listing + `xdg-open`;
- `=` свой парсер, `1/0` → строка-ошибка; `?` 6 строк; `i:` hostname/kernel/compositor;
- невалидный префикс → app-search; Esc закрывает из любого режима;
- история `>` и «терминал+cd» не делались — спека разрешает.

Мой прогон: launcher **70/70**, `--lib` **547/547**, `--release` чисто (incremental 0.41s, дерево без чужого WIP).

**Live grim E не закрыт:** `> echo hi`, `~/Dow`, `= 1/0`, `i:`, Esc. T265-F разблокирована, не выдана.

---

**Обновлено: 2026-08-16 (чекпоинт #22 — живой смок владельца PASS).**
T294 (вкладка Updates), T295 (календарь по клику на часы), T265-A..D
(поля `.desktop`, сетка+категории, favorites/recents/folders, контекст-
меню/pin) — все подтверждены владельцем на живом столе, не только
юнитами. Есть мелкие придирки, владелец сам назвал их не блокерами и
отложил на потом — не заведены тикетами, ждут отдельного захода.
T265-E (префиксные режимы) разблокирована следующей.

**Обновлено: 2026-08-16 (чекпоинт #21 — T265-D принята, live открыт).**
HEAD `2ab4578` (+ T294 после D).

**T265-D ПРИНЯТА** `0405711` `feat(launcher): app context menu and desktop actions`.
Зона ровно `launcher/{app_menu.rs←pin_menu.rs, favorites.rs, launcher_config.rs, mod.rs, view.rs}`. Cargo.lock / Source / tray_menu не тронуты. Сверил сам, не со слов:

- одно меню, `grab: false`, catcher через `catcher_anchor_for` / hyprctl (не `bounds().origin`);
- Launch = frecency + `launch` + `launcher::close`; Desktop Actions = submenu, пусто → секции нет;
- favorite / pin / hide — честный toggle; Properties / other-user disabled с причиной в лейбле;
- `hidden: Vec<String>` top-level в `launcher.toml`, RMW ключ `hidden`, сигнал `subscribe()` → `apply_hidden_filter`;
- `.unwrap()` на launch/hide нет.

Мой прогон: `cargo test -p chronos --lib launcher` → **47/47**; `--lib` → **522/522**. Свежий `--release` **не собирается** — чужой T295: `clock.rs` зовёт `crate::calendar_popup`, модуля в crate нет (1 ошибка, не 7 — WIP сдвинулся). Это не зона D. Существующий `target/release/chronos` 18:29 уже содержит `chronos-launcher-app-menu` / `Hide from list` — им можно live, не пересобирая.

**Live grim D не закрыт** (меню rest/hover, Launch закрывает лаунчер+меню без `window not found`, action, favorite, hide→сетка, pin→`dock.toml`). Юнит это не ловит. T265-E разблокирована, не выдана.

---

**Обновлено: 2026-08-16 (чекпоинт #20 — T265-A/B/C приняты, T285 STOP).**
HEAD `56c9c1a`.

**T265-A/B/C все приняты тем же вечером** (`98bd08f`/`bcd08cc`/`1577aaa`) —
параллельная сессия дошла до `launcher/**` полностью: поля `.desktop` +
тир-ранжирование, сетка + категории, favorites/recents/folders. Каждая
сверена мной построчно с деревом + свои прогоны тестов/release, не на
слово. Дисциплина отчётов росла на глазах: A — отчёта в inbox не было
вообще (только скриншот владельцу), B — уже лёг в `report/`, C — тоже, и
сам минион честно пометил "изоляция нарушена, работал на master, не в
worktree". T265-C пришлось верифицировать в отдельном `git worktree`
(сиблинг ChronOS, не `/tmp`) — общее дерево в этот момент было
недоступно для сборки из-за чужого живого WIP (T294 на `PanelTab`
матче). T265-D разблокирована.

**T285 — STOP, тупик апстрима.** `attach_session` в `agent-client-
protocol` 2.0.0 — `pub(crate)` (был `pub` в 0.11.1, регресс), `ActiveSession`
все поля приватные без публичного конструктора, `ActiveSessionHandler`
тоже приватный (подсказка в брифе "публичен" была неверна для 2.0.0).
v1 load/resume-билдера нет вообще; v2 `resume_session` есть, но отдаёт
`V2Session` не `ActiveSession` и требует v2-wire, а Hermes на v1. Сверено
дословно с исходником крейта в `~/.cargo/registry`. Холодный старт
по-прежнему падает в `create_session` fallback с новым id. Бэкенд-фикс
(seat `ActiveSession` на холодном `load_session`) недостижим без форка
SDK или отказа от `ActiveSession` вовсе (прямой JSON-RPC поверх
`add_dynamic_handler` — не оценивал, отдельное архитектурное решение).
Рекомендация — upstream issue в `zed-industries/agent-client-protocol`
за возврат публичного `attach_session`/load-билдера. Не блокер сессии.

**T286/T287 РАЗБЛОКИРОВАНЫ (правка того же чекпоинта).** Владелец верно
указал: "не параллелить с T285" в их брифах — это зонный guard против
одновременной правки `chat.rs`/`composer.rs`, не функциональная
зависимость от исхода T285. T285 закрылся STOP-ом без единого изменения
кода (`chat.rs` не тронут), значит держать зонный конфликт больше не за
что. T286 → T287-C можно выдавать прямо сейчас (T286 не самим брифом
блокирован, T287-C зависит только от T286 в git — своё "Не T285" там уже
стояло верно). Правки в `active/T286-...md` отмечают это явно.

---

**Обновлено: 2026-08-16 (чекпоинт #19 — T275 закрыт).** HEAD `162798b`.

**T275 ЗАКРЫТ**, в `done/`. Живьём подтверждены оба хвоста: empty-query
(grim, заход 3) и pin (владелец, только что). Корень pin-бага был не тот,
что чинили в `180fe884`/`3eeaac18` — `window.bounds().origin` для
центрированного Hyprland-windowrule окна навсегда `(0,0)` (Wayland не
сообщает клиенту реальную позицию toplevel'а), так что дырка catcher'а
считалась от фиктивной точки. Фикс `162798b4`: живой запрос `chronos_
services::compositor::hyprland::window_position()` по `hyprctl`-сокету за
настоящей экранной позицией. Заодно `a4b22e9` — бар был не подписан на
`DockConfigSignal`, поэтому даже честный pin/unpin не перерисовывал бар
без постороннего triggers.

**Посторонний коммит `98bd08fc` (feat T265-A) — теперь СВЕРЕН и ПРИНЯТ.**
Прилетел в master 16:50:13 напрямую, без report/inbox, пока T265-A ещё
числилась BLOCKED на T275 (T275 закрылась только в 17:00:28, `162798b4`).
Процессное нарушение зафиксировано в отчёте (`report-log/T265-A-...
-report.md`), но код на месте: 8 файлов ровно в зоне тикета, спека
покрыта (`DesktopAction`, `is_listed`, haystack `name\0generic\0comment\0
keywords\0exec`, тир-ранжирование exact>prefix>substring>other>fuzzy,
frecency вторична, ghost-completion через `Input::suffix()`). Мой прогон:
`chronos-services applications` 37/37, `chronos --lib launcher` 18/18,
`chronos --bins` 686/686, `--release` собирается. Перенесён в `done/`.
**Один долг остался открытым** — спека требует живой прогон «набрать
точное имя → первое, keyword → находится», минион его честно не сделал
(нет sudo для `ydotool`, стол был занят). Не блокер для кода, но кто-то
должен глазами подтвердить на ближайшем живом столе.

**T265-B уже запущена той же параллельной сессией, ДО того как я закончил
приёмку A** (владелец подтвердил — "по глупости уже пустил"). Раз A
принята чисто, риск невелик, но цепочка B→C→...→G по-прежнему требует
report+приёмку на каждом шаге — параллельная сессия явно готова
коммитить в master без ожидания. Следующая сессия: сверять B так же
въедливо, не доверять скриншотам из чужого чата.

`.chronos-ops/` и `.workbuddy/` по-прежнему untracked — решение не
менялось (см. чекпоинт #18 ниже).


> **Переезд (2026-07-19):** оркестрация уехала из корня в `docs/orchestration/`.
> Брифы — `docs/orchestration/agents/<ИМЯ>.md`, активные отчёты —
> `docs/orchestration/reports/<имя>-report.md`, архив — `docs/orchestration/report-log/`.
> Агент-стейт (`.cline/`, `.autohand/`, `.mimocode/`, `.clinerules/`, `contexts/`)
> теперь в `.gitignore`. Корень несёт только профильные доки + скелет (README,
> LICENSE-TBD, CONTRIBUTING, CI). Исторические упоминания «report-log/» ниже —
> дорелокационные, читать с этой поправкой.

**Обновлено: 2026-08-16 (чекпоинт #18 — кухня).** HEAD `dc6dc38` (без
изменений, код не трогал).

**Ecosystem-кухня `/home/neo/projects/chronos-ecosystem/.chronos-ops/`
закрыта.** wt-tools T001-T003 принят весь: T001/T002 с первого захода
(живой прогон подтвердил detached-HEAD `merged: no` на `ChronOS-wt-t266`).
T003 — со второго: живая проверка (не unit) поймала `$'\\n'` (два
буквальных символа) вместо `$'\n'` в `wt-drift.sh:35,40,45,56` —
`DRIFT.md` рендерился одной слипшейся строкой без переводов строк;
`test_drift.sh` пропустил, там только `grep -q` подстроки. Фикс
`3128c404`, живой прогон подтвердил 21 настоящую строку. `kitchen-status.sh`
прогнан, `checkpoint/STATUS.md` свежий, всё пусто кроме `back: done=3`.

**ChronOS-кухня (`ChronOS/.chronos-ops/`) — миграция очереди из
`docs/orchestration/tasks/active/` НЕ началась, заблокирована мной
явно.** Дерево грязное по T265/T275 прямо сейчас (живая правка
координатного бага pin-меню лаунчера — `anchor_rect` window-local vs
`catcher_anchor` output-local, некоммичено) — переезд `git mv` 20
файлов поверх этого не делать. Роль-классификация 20 активных тикетов
готова на будущее: **front** — T265-B..H, T265-launcher, T266, T275,
T284, T286, T287-C, T287-left-chat, T290-E, T293, T294, T295; **back**
— T271, T285; **hold** (не роль исполнителя) — T277, T281, T224, T282.
`.chronos-ops/RULES.md` скелета всё ещё несёт текст экосистемного
уровня («не заменяет `docs/orchestration/`») — переписать под
ChronOS-scope одним заходом с cutover, не раньше. Дубль-копия
`.chronos-ops (copy 1)` снесена, коммитить `.chronos-ops/` в git или
нет — не решено. Следующий шаг: ждать закрытия T265/T275.

T292 DONE `92786c5`. T285 OPEN: slice A `f9cd9a2` (chat.rs) зовёт load;
живой провал — `session/load OK` затем наш `no active session for load`
(`SharedSession.take()` на холодном клиенте) → fallback create.
Бриф slice B: BACKEND `hermes_acp/client.rs`, 100b. `chat.rs` не трогать.
Дальше после T285: T286 → T287-C. Справа T293–T295.

**Обновлено: 2026-08-16 (чекпоинт #16 — T292 принят).**

**T292 DONE.** Shell Gamer (`WorkspaceMode`) — кнопка на правой рельсе
над dock, не `PanelTab`. Бар-пилюля снята. Prompt инлайн в рельсе.
Иконки `gamepad.svg` / `mode-daily.svg`. Live `+`. Эррата bins-теста
`migration_idempotent`.

**Канон панелей:** слева только ИИ; справа ежедневное и ОС.
Display = правые настройки дисплея. Над доком — кнопка режима (T292).

**Принято в этой волне:**
- T288 `90ffd88` cwd=проект (live `c9f033fc`)
- T289 `17afee6` dock=exclusive, не замок
- T291 `84f25bf` + E `235185a` power/Perf Gaming на System
- T290 `bb9790a` попап снесён (сторона была неверная)
- T290-E `50b6c62`+`81fd7cb` window_font ROOTS
- T296 `a2c072f` Display направо
- T292 `92786c5` режим на рельсе

**T285 OPEN.** Slice A `f9cd9a2` (chat.rs) жив. Живой провал: Hermes
`session/load OK`, наш `take()` на пустом `SharedSession` →
`no active session for load` → fallback `create_session`.
Остаток: BACKEND `hermes_acp/client.rs`, бриф переписан.

**Очередь:** T285 slice B (холодный bind). Потом T286 → T287-C.
Справа: T293, T294, T295. T275 remainder. T281 PARK.
T265 не пачкой. T284 не с T266. T271 после левого фронта.

Два gaming: Perf (`GamingModeState`) ≠ Shell (`WorkspaceMode::Gamer`).

Калибр: тикет = ширина. 7b эррата; T288/T285 = 100b; не 1T на три вызова.

**Инфра:** Hindsight :8888 отвечает (не мёртв). Honcho без ключа — skip.

**Обновлено: 2026-08-15 (поле — T288).**

**Панели:** слева ИИ, справа ОС. **T296 DONE** `a2c072f` (live `+`).
Дальше: **T285** (лево) или **T292** (кнопка режима над доком). T284 не в поле.

**Обновлено: 2026-08-15 (чекпоинт #14 — спеки, код не писали).**

Накопление закрыто. Следующая сессия **исполняет**, не плодит новые тикеты без нужды.

**Очередь исполнения (зоны не параллелить):**

Левый ACP (строго по порядку):
1. **T288** — `create_session(cwd проекта)`, не `current_dir()`. Процесс живьём в `…/ChronOS/packaging`.
2. **T285** — restore: `load_session`, не `create_session`. После T288.
3. **T286** — композер = kit `Input` multi-line. После T285.
4. **T287-C** — срезать Zed-хром; Follow в `composer-pickers-row`. После T286.

Правый / бар (после или рядом, не в тех же файлах что 288–287-C):
5. **T289** — dock не открывает и не запирает вкладку (снимает T221 same-tab no-op).
6. **T291** — Perf Gaming + power profile → правый System.
7. **T290** — левая вкладка Display (яркость + waytrogen). После T291. Попап System снести.
8. **T292** — Shell Gamer (`WorkspaceMode`) → кнопка на правой рельсе. Не T291.
9. **T293** — вкладка Notifications; колокольчик открывает вкладку.
10. **T294** — вкладка Updates; apply только pacman; AUR display-only + hover yay.
11. **T295** — часы → kit `Calendar` попап (`gpui-component/time`).

Живые хвосты / не трогать пачкой:
- **T275** remainder: empty-query + pin live. Код A–D не переписывать.
- **T281** PARK. Inbox не архивировать до `+` после T285.
- **T265-A…G** BLOCKED цепочкой, **H** pause. Эпик не отдавать. 300B на волну.
- **T284** Frame: Task 1/3/5 ок; 2/4 слева — когда left тихий.
- **T271** только `ipc/`, после левого фронта.
- **T266** не рядом с T284.

Два gaming: **Perf Gaming** (`GamingModeState`) ≠ **Shell Gamer** (`WorkspaceMode::Gamer`).

Кода в этой сессии нет — только брифы в `docs/orchestration/tasks/active/`.

HEAD на момент чекпоинта: `5b73b34`.

**Инфра:** Hindsight :8888 мёртв (пользователь выключил) — retain не делали. Honcho MCP `conclusions_of` в этой Grok-сессии нет (SDK 2.2 без того API).

**Обновлено: 2026-08-15 (правка брифов).**

- **T275** — remainder: empty-query «No matches» + pin live после `180fe88`. A–D не переписывать.
- **T281** → `active/pause/`. Inbox-отчёт не архивировать. Гейт 8 = **T285**.
- **T271** — только `ipc/`. `side_panel_left` нельзя, пока T285/T286/T287-C в поле.
- **T287 порядок:** T285 → T286 → T287-C. C не параллелить с T286 (`composer.rs` / Follow).
- **T266** — T263/T264/T265-0 закрыты; не класть рядом с T284.

**Обновлено: 2026-08-15 (чекпоинт #13).**

HEAD `5b73b34`. Сессия оборвалась на «чекпоинт» — дописан здесь.

**Поле / очередь (не параллелить зоны):**
- **T275** remainder. Код `89dfd25`, pin-эррата `180fe88`. Каретка живая. Pin после rebuild и empty-query — открыты. Не в `done/`.
- **T281** PARK (`active/pause/`). Гейт 8 путь `23bf89f`, живьём Hermes `create_session` → **T285**. Не архивировать до `+` владельца.
- **T295** SPEC: клик по часам → AnchoredPopup с kit `Calendar` (`gpui-component/time`). Не самописная сетка. Планировщика событий нет.
- **T294** SPEC: вкладка Updates справа. Apply только `pacman` (не yay). AUR в списке, hover = «ставь через yay в терминале». Бар-счётчик открывает вкладку; попап снести.
- **T293** SPEC: история уведомлений → вкладка Notifications справа. Бар-колокольчик остаётся, клик открывает вкладку; `history_popup` снести. Тосты не трогать.
- **T292** SPEC: **Shell Gamer** (`WorkspaceMode`) с бара → кнопка на правой рельсе. Не T291.
- **T291** SPEC: power profile + **Perf Gaming** (`GamingModeState`) → правый System. Яркость в попапе до T290.
- **T290** SPEC: левая вкладка Display (яркость + waytrogen). Попап System снести. Бар `system` открывает Display. После T291.
- **T289** SPEC: правый dock не открывает вкладку и не запирает её (ломает T221 same-tab no-op). Не выдавать до чекпоинта.
- **T288** ACP cwd = active project, не `current_dir()`. Живо: шелл из `packaging/`, в UI выбран ChronOS. Перед T285.
- **T285** `load_session` на restore, не `create_session`; не двоить транскрипт. После T288. Не параллелить с T286 (`side_panel_left`).
- **T286** композер на gpui-component `Input` (wrap), не хак `text_input.rs`. После T285.
- **T287-C** после T286 в git: срезать Sessions rail + thread-header + close X; Follow → composer-pickers-row, `icons/rail-preview.svg`. T287-A пикеры позже.
- **T284** Frame Hide|Wrap — Appearance, не bar preset. Код не начат. Task 1/3/5 можно; 2/4 слева — только если left-зона тихая.
- **T271** не выдавать, пока левый фронт в поле; зона только `ipc/`.
- T265 эпик не отдавать. Дети A–G в `active/` (BLOCKED цепочкой), H в `pause/`. Сначала хвост T275. Не 30B на волну. T266 не рядом с T284.

**Решения сессии:** component-first (наш `Source/gpui-component/`, `57f582f`). Frame ≠ bar preset. Follow не удалять. `Source-wt-component` — ложь, скилл починен `cf6f39e`.

**Инфра:** Hindsight **временно выключен пользователем** (2026-08-15) — не поднимать, не чинить, retain пропускать. nginx `:8080` тоже мёртв. Банк по-прежнему `chronos-ecosystem`. Honcho SDK `conclusions_of(neo).create` в чекпоинте #13 прошёл.

**Обновлено: 2026-08-15 (T285/T286).**

T281 гейт 8 **живьём нет**: лента из store есть, Hermes — новый
`create_session`. → **T285**. Chat на кит: эпик **T287**. T286 композер;
**T287-C** срезать внутренний Sessions rail и `＋☰👁⋯` (Zed). Пикеры
T287-A. Не параллелить T285 и T286.

**Обновлено: 2026-08-15 (компонент first).**

Новые контролы шелла — `gpui-component` (наш форк), не самопис.
T275/T265: `Input`/`Button`/`PopupMenu`. Не 30B на эпик T265.

**Обновлено: 2026-08-15 (T253 принят с оговоркой).**

**T253 DONE.** Кадры `notes/T253-system-{dark,light}.png`. Мока нет,
MPRIS «No player», шапка не kitty. Кадры не кроп слоя — caveats в отчёте.
T256 не этот тикет (`897c3d2`).

**Обновлено: 2026-08-15 (T284 заведён).**

**T284 OPEN** — тема Frame Hide/Wrap (не перепись T268).
Бриф/план/спека поправлены (рельса height -= inset, after_apply, нет
`set_margin`, style=строка+RMW). В поле: Task 1/3/5 сразу; Task 2/4
слева — только если T281 сегодня никто не берёт. T277 не блокер.
T268 код `d572657` = Hide. Бриф уехал в `done/`. T265-H и T277 — `active/pause/`.

**Обновлено: 2026-08-14 (чекпоинт #12, T280+T283 в git).**

**T279** принят, в git: `bd999a5`.
**T280+T283** приняты. Код: `f083779` (хранить `Cargo.lock` вне этого коммита).
Store v2 + bar strip + Sessions empty-scope.
Следующий: **T281** (IPC + live). T282 packaging параллельно.

**Обновлено: 2026-08-14 (чекпоинт #10, T278 принят + переезд памяти).**

**Инфраструктура памяти переехала на локальный стек (вечер 14.08).**
Hindsight ел ~$100/мес против ~$3 у Honcho — четыре платных шага на запись.
Теперь через `llama-swap` (systemd-юнит, `infra/llama-swap/config.yaml`,
`127.0.0.1:9292`): embeddings `embed-v5-nano`, reranker `rerank-jina`,
retain И consolidation — `lfm2.5-2.6b`. В облаке остался только main
(`:20128`/`hindsight-llm`). Бэкап конфига —
`infra/hindsight/.env.bak-2026-08-14-precloud-to-local`.

Проверено вживую, не со слов: эмбеддер отдаёт 768 значений и батч;
`/v1/rerank` ранжирует верно (0.65 у релевантного против −3.2 у мусора);
**task-префиксы не понадобились** — сборка task-specific `retrieval`;
заливка `success, items_count: 3`, документы отдают 200; очередь поехала —
retain 144 → 130, возраст операций упал с 220+ с до 27-72 с. Весь день
запись блокировало облачное комбо `hindsight-retain`: `502 Upstream error
from Nvidia` → фолбэк роутера на `openai` без ключей → `404 no active
credentials`. `embed-v5-nano` = те же веса, что были в облаке, поэтому
реиндекс банка не потребовался.

**LFM2.5-2.6B выбрана по данным, а не на глаз** (карточка модели):
IFStruct 85.49 против 78.50 у Qwen3.5-9B, Multi-IF 80.07 против 62.55,
AA-Omni Non-hallu 59.04 против 8.84. По структурному выводу и
не-галлюцинированию бьёт вдвое большую модель; проигрывает в Agentic и
BFCLv4, которых у нас нет. Одна резидентная модель на оба LLM-скоупа —
свап-молотилки между двумя моделями не возникает.

**Две ловушки, стоившие часа:** `podman restart` НЕ перечитывает `.env` —
`env_file` разворачивается в окружение при `create`, поэтому правки не
применяются, а health зелёный; нужен `podman-compose down && up`, а
проверять — `podman exec hindsight env`, не файл на диске. И
`podman compose` (уходит во внешний docker-compose, лезет в отсутствующий
docker-сокет) ≠ `podman-compose` (питоновский, которым стек создан;
`sudo` не нужен — rootless).

**T278 (Slice A1 левой workspace) ПРИНЯТ** после четырёх раундов, HEAD
исполнителя `19263d3`. Отдано в дерево: `side_panel_left/tabs/mod.rs`
(`LeftTab`, `PRIMARY_TABS`, `BOTTOM_TAB`, `ResizableWidths`,
`width_for_open`, `dock_transition`), `state.rs` (чистая геометрия
`geometry::`), `rail_view.rs` (рельса 40 px), `workspace_view.rs`
(фикс-канвас 920 px), `apply_dock_toggle(cx: &mut App)` в `mod.rs`.
Легаси одно-оконный resize-путь удалён, `window.resize(` в
`side_panel_left` не осталось (только комментарии). Приёмка своя:
`cargo test -p chronos --lib` → **401/401**, под `side_panel_left::` —
**71** (72 в подстрочном фильтре — ловится
`side_panel_right::view::tests::needs_width_resize_still_serves_side_panel_left`).
Задание → `tasks/done/`, отчёт → `tasks/report-log/`.

**Главный урок T278 (записан в `.chronos-ops/checkpoint/ARCHITECT.md`, раздел 2026-08-14):**
round 3 пришёл с тестом `on_dock_toggle_uses_pure_helper`, который не
вызывал `on_dock_toggle` — присваивал результат хелпера в глобал и
проверял, что глобал равен результату хелпера. Зелёный при любом
состоянии прод-функции. Причина, по которой исполнитель туда пошёл,
реальна и архитектурна: `SidePanelLeft::new` спавнит async ACP-connect,
поэтому entity не поднимается в `TestAppContext`. Верный вывод (round 4)
— редьюсер выносится в свободную функцию на `&mut App`, вьюха делегирует
в одну строку. Нетестируемый редьюсер есть дефект архитектуры, а не
основание не тестировать.

**Очередь на 2026-08-14:**

- **T280+T283** закрыты: `f083779` — ThreadStore v2, bar project retired,
  Sessions empty-scope. Дальше **T281** (IPC + live). Не параллелить с T281
  зону `side_panel_left` / `threads`.
- **T273** закрыт (поглощён T276). **T274** снят: pill бара нет после T280.
- **T270** уехал в Chronos-GPUI: `../Source/docs/tasks/T270-wayland-dnd-source-never-finishes.md`.
- **T282 packaging** — LAST, после пустой очереди шелла, не параллельно.
- **Лаунчер:** волна T265-A конфликтует по зоне с T275 (`launcher/**` +
  frecency) — сначала закрыть T275. T280 больше не блокер `bar/widgets/`.

**Решения по релизу (2026-08-14):** ветку `stable` в `Source/` не
заводим — пин по sha плюс тег `chronos-<version>` на релизном коммите;
ветка `release/0.x` появится только когда понадобится бэкпорт в живой
релиз. Публичный показ (r/unixporn, r/hyprland, r/rust; Пикабу — мимо
аудитории) — после Slice A слева, не после правой панели: одна панель
поводом не тянет, а первый пост тратится один раз.

**Обновлено: 2026-08-13 (чекпоинт #8, разбор грязного дерева).**

**T252 принят и закрыт** (матрица empty-state приёмов, DECISIONS.log 2026-08-13
с двумя поправками приёмки: блок дисков = 4 русские строки, не одна; планка
«английский, кроме локали даты/времени» — `MONTHS_RU` не трогать). **Заведён
T269** (empty-state хелперы в `tab/ui.rs`; эталон hero = `EmptyTab`, иконка —
`tab.icon_path()` параметром, `EmptyTab::render` схлопывается в один вызов
хелпера). Зона T269 чистая, исполнителю можно отдавать немедленно.

**Карта грязного дерева по тикетам** (статически, по диффам и отчётам):

- T263: `bar/widgets/{dock,mod,tray}.rs`, `dock/context_menu.rs`,
  `tray_menu/{mod,view}.rs`, `services/tray/{menu,types}.rs`, доля
  `icon_resolution.rs`, с 2026-08-13 — `theme_config.rs`.
- T264 (A+A2, `grab: false` ×6 с якорями-комментариями и тестами): доли
  `dock/context_menu.rs` + `tray_menu/mod.rs`; чистые — `volume_popup`,
  `system_popup`, `updates_popup`, `history_popup` (по 4-7 строк каждый).
- T265-0: `launcher/view.rs` (чистый), доля `icon_resolution.rs`
  (переплетена с T263 — отдельно не закоммитить, подтверждено отчётом).
- Чужое: `Cargo.lock` (вендоринг wgpu/xim/font-kit + `dirs 5` — не
  коммитить, не откатывать), untracked design-HTML/скиллы/таск-доки.

**Статика дерева на 2026-08-13: lib 306/306, bins 515/515, services tray
20/20, check чистый** (остаточные unused-import warnings — пред-существующие).

**Закрыт статический пробел T263 — палитра gpui-component:**
`theme_config.rs::sync_gpui_component_theme` теперь мапит popup-токены
компонента из shell-темы (popover/accent/border/muted_foreground/selection;
`accent` компонента = hover-фон MenuItem, поэтому ← `interactive.hover`, не
`accent.primary`). Тесты dark+light. Визуальный вердикт — живым кадром в
приёмке T263.

**Осталось до разблокировки коммитов:**

1. T263: submenu widest-reserve **реализован 2026-08-13** (исполнитель,
   статика зелёная: lib 309/309, bins 528/528, tray_menu 27/27, check
   чистый): чистая цепочка `row_content_width → level_card_width →
   submenu_chain_reserve → estimate_menu_width` (оценка по символам,
   константы задокументированы — точный замер невозможен: окно ещё не
   создано, text system недоступен в `open()`), surface прозрачный вне
   карточки (`items_start`), click-away через `on_mouse_down_out →
   DismissEvent → close_this`, submenu-карточки зажаты в 230-300 →
   flip-check компонента не срабатывает, submenu всегда side-by-side.
   Док — письменный no-op (модель без submenu). Кадры — одним заходом:
   палитра + submenu + anchor (2 tray-иконки в разных X, dock,
   rest/hover/disabled, меню рядом с updates-попапом).
2. T264: виновник найден — **external drag-out, не popup и не hover-strip.**
   Механизм подтверждён в коде форка
   (`Source/gpui_linux/src/linux/wayland/client.rs`): `start_external_drag`
   (~:352) создаёт `wl_data_source` и зовёт `start_drag` **без
   `set_actions()`**; диспатчер (~:2591) обрабатывает только `Send` и
   `Cancelled` (destroy), а `DndFinished`/`DropPerformed` проглатываются
   через `_ => {}` — после успешного внешнего дропа source не уничтожается,
   имплицитный pointer-grab композитора висит навечно → клики/скролл умирают
   у всех клиентов, бинды SUPER живут (композитор события получает). Код-путь
   вооружается ТОЛЬКО внешним drag-out; внутренние дропы source не создают —
   профиль эпизодов сходится. Гипотезы grab-popup (часть A лечила не то) и
   hover-strip/peek **закрыты**; TTY-клетка и «сессия без ChronOS» не нужны.
   Тикет — **`active/T270-wayland-dnd-source-never-finishes.md`** (триггер
   подтверждён: drag-out из Chronos-FM наружу; дроп внутри FM симптома не
   даёт). Код в Source принят статически: `18ea90a` + `48b2c1f` (set_actions
   до start_drag, destroy на DndFinished, роли типизированы, copy-only —
   Move нельзя без реального удаления на стороне FM). Остался живой прогон
   (5× drag-out с RUST_LOG=info) — отложен: тест по природе ломает ввод
   при регрессе, мышь нужна пользователю.
3. Порядок коммитов: приёмка T263 → один коммит T263 (все его файлы +
   `theme_config.rs`) → коммит T265-0 (остаток `icon_resolution.rs` +
   `launcher/view.rs`, ticket/report в done) → 4 чистых popup-файла T264 A2
   могут уйти отдельным коммитом в любой момент (зона чистая, решение за
   архитектором).

**Обновлено: 2026-08-12 (чекпоинт #7, вечер).**

**T264 — профиль симптома уточнён, гипотеза грэба мертва.** Четыре эпизода за
день. Клик и скролл перестают доходить до клиентов, при этом бинды Hyprland с
SUPER работают (композитор события получает, не доставляет). Умирает **без
действий пользователя** — последний раз, когда пользователь отошёл от компа, —
и **возвращается сам** через минуты. Грэб выключен уже у всех шести попапов
(dock, tray, volume, system, updates, history — часть A + A2), симптом не
изменился. По времени совпадает с уничтожением короткоживущих поверхностей
(в последнем эпизоде авто-поверхности: OSD/тост). **Нулевой опыт «сессия без
ChronOS» не поставлен ни разу** — это следующий шаг, до любых правок кода:
если без шелла симптом повторится, вина уходит в Hyprland 0.56.2/libinput.
Ловушка, которая даёт улики: `RUST_LOG=info,gpui_linux::linux::wayland=debug`.
Вернуть мышь без перелогина нельзя, но выход в одну команду —
`stop-hyprland -f` (новый скрипт в `~/.local/bin`).

**T265 заведён — полный функционал лаунчера** (эталон по функциям: AppGrid,
`/home/neo/Downloads/plasma6-applet-appgrid-main.zip`). Ключевое требование —
ДВЕ поверхности одновременно на одной модели: OSD-лаунчер по хоткею и
классическое «пуск»-меню от кнопки в баре. Разбит на волны 0/A–G.
**Волна 0 закрыта и проверена живым кадром**: иконки и скролл. По ходу нашлись
три настоящих бага, не те, что были в тикете: (1) `resolve_app_icon` грузил
путь как `String`, а `impl From<String> for ImageSource` отправляет не-URI в
`Resource::Embedded` — иконки искались среди встроенных ассетов и не рисовались
вообще; (2) `Path::with_extension` калечил все reverse-DNS имена
(`org.xfce.thunar` → `org.xfce.svg`); (3) контекст `legacy` не искался
(`network-wired` в Adwaita*Legacy). Плюс `min_h(0)`/`h_full` — без них список
на 200 строк выдавливал шапку и поиск за границы окна.

**Заведены T266 (прозрачность поверхностей + блюр), T267 (единая
полоска-разделитель: правая панель и бар по образцу левой), T268 (обрамление
рабочего стола нижней полоской без exclusive zone, стыки в углах — принимать
кадрами).** T263 остаётся на доработке; в него дописан живой дефект: меню трея
рисуется стоковой палитрой gpui-component, чинить мапингом токенов в
`theme_config.rs::sync_gpui_component_theme`.

**В дереве нет ни одного коммита по T263/T264/T265-0** — файлы перемешаны
(`icon_resolution.rs` держит T263 и T265-0, попапы держат T264). Развязка
только в порядке: приёмка T263 → один коммит остального.

**Обновлено: 2026-08-12 (чекпоинт #6).**

**T264 — popup grab убивал ввод во всём композиторе; часть A закрыта.**
Симптом: один физический правый клик по dock/tray-иконке убивал обе кнопки
мыши во всей сессии Hyprland 0.56.2, `pkill chronos` не лечил, только
перелогин. Первая сессия по T263 списала это на синтетический ydotool-клик —
**атрибуция была неверной**, повтор случился на физическом клике без ydotool
вообще. Часть A (исполнитель GPT 5.6 sol): `PopupOptions.grab` → `false` в
`crates/app/src/dock/context_menu.rs:227` и `crates/app/src/tray_menu/
mod.rs:205` с комментариями-якорями на T264, плюс тесты на отсутствие grab,
Escape и click-away (bins 504 → 508). Проверено моим прогоном (lib 299/299,
bins 508/508) и живым смоком: меню открывается, **ввод остаётся жив**.
`history_popup` осознанно оставлен на `grab: true` — он не подозреваемый.

**Часть B (корневая причина) — открыта и НЕ доказана.** Мой репро
`Source/gpui/examples/popup_grab_repro.rs` (layer-shell родитель + popup;
варианты L-grab, R-grab, R-destroy при живом грэбе, R-nograb) не
воспроизводит смерть ввода ни во вложенном labwc, ни во вложенном Hyprland;
настоящий release-ChronOS во вложенном Hyprland тоже открыл и трей-, и
dock-меню без последствий. Улика из хостового лога мёртвой сессии: libinput
после смерти клика продолжал отдавать полные циклы `PRESS → RELEASE`
(313 строк `Plugin:button-debounce`), то есть нажатия доходили до Hyprland, а
он их никому не доставлял — уровень композитора, не железо и не libinput.
**Вложенная клетка для части B бесполезна:** Wayland-бэкенд, libinput не
участвует вообще (0 строк debounce), один выход вместо двух. Следующая
клетка — второй TTY на DRM. Сборка репро: `cargo build --manifest-path
gpui/Cargo.toml --example popup_grab_repro --features wayland` (`-p gpui`
падает с `specification 'gpui' is ambiguous`).

**T263 разблокирован, но живых кадров меню всё ещё нет.** Не потому что не
работает: меню закрывается по click-away каждый раз, когда пользователь
кликает в терминал, чтобы сказать «снял». Снимать серией `grim` или
`wf-recorder`, пока он не трогает клавиатуру. Решение по submenu-блокеру
записано в сам тикет. T264 вливается в T263-коммит — файлы общие, резать
значит родить несамодостаточный коммит.

**Обновлено: 2026-08-05 (чекпоинт #5, HEAD `3eb92f2`).**

**Очередь orchestration/tasks — T254 закрыт, T252+T253 в поле.** T254
(снятие блокера для досъёмки T223) закрыт — `ydotool.service`/
`/dev/uinput` подтверждены живьём дважды, синтетический клик реально
дошёл до GPUI layer-shell. Скоуп тикета был только про блокер; полная
досъёмка оставшихся 7 поверхностей (клавиатурная раскладка, редактор
Edit Mode, композер+dropdown, volume/OSD popup, notifications/tray
popup, drag-ручки+hover-strip, правый рейл Edit Mode T219) вынесена в
будущие под-тикеты. Заодно нашёлся и убран `T223-capture-log.md`,
застрявший в `active/` хотя сама задача T223 давно в `done/` —
перенесён туда же как эвиденс-артефакт, не тикет. T252 (единый
empty-state паттерн по вкладкам правой панели) и T253 (пересъёмка
System-таба после T246/T256) обе оказались разблокированы — их
зависимости (T246/T248/T249/T256) все уже в `done/` — и отданы
параллельно исполнителю **GPT 5.6 sol**. T252 — архитекторское решение
не код, приёмка будет на вменяемость записи. Отчёты ждать в
`docs/orchestration/tasks/report/`.

**Ловушка сессии:** `ctx_shell`/`ls` через lean-ctx MCP на
`docs/orchestration/tasks/active/` отдавал устаревший закэшированный
листинг (показывал уже перемещённые в `done/` файлы). Пойман только
потому что пользователь спросил «где T256» и правда не билась с
`git log`. При сверке состояния задач — доверять `git status`/`git log`/
`/bin/ls` напрямую, не первому lean-ctx листингу.

**T244 (DP-1/gslapper почернение) — ЗАКРЫТ, закоммичен.** RECON-отчёт
верно установил: ChronOS не убивает gslapper напрямую (`pkill -x
chronos` точечный, gslapper PPID=1), но виновен опосредованно —
`awww-daemon`, которого сам ChronOS спавнит (`ensure_daemon()`), красит
ОБА монитора и при свежем спавне (после смерти предыдущего процесса —
подтверждено `Connection refused` в логах) self-restore'ит
`~/.cache/awww/<ver>/DP-1` поверх gslapper по z-order. Код-фикс:
`awww-daemon --no-cache` (флаг из `--help`) + ранее убранный
проактивный `awww restore` (тянул рассинхронённый кэш вместо выбора
waytrogen) — оба в `crates/services/src/wallpaper/mod.rs`, коммит
`d60efb0`. Живой тест 3/3 циклов kill-awww-daemon→chronos-restart:
z-order и яркость DP-1 стабильны.

**T231-архив (bar settings редизайн + паттерн-спред + skills-инфра) —
ПРИНЯТ.** 11 коммитов (`f5b69c5`…`424b45a`), все верифицированы:
`check-proofs.sh` 265/26/0 (совпадает с отчётом дословно), 165→166
тестов зелёные, живые кадры подтвердили grid/иерархию/elevation.
Три отчёта → `report-log/`.

**Инцидент этой сессии — два ложных коммита правили не тот таб.**
Пользователь прислал скриншот страницы «Bar» (Presets/Appearance/
Theme/Hypr modules) с точным пожеланием 410px + отключить ресайз. Я
дважды (800, затем 410) правил `PanelTab::System` — а это ДРУГОЙ таб
(`SystemTab`, CPU/RAM/GPU-дашборд). Реальный таб — `PanelTab::
EditorSettings` (label «System settings» — созвучие с "System" и
подвело), рендерит `BarSettingsTab` (`tab/mod.rs::create()`).
Пользователь поймал прямым текстом: «скрин ты не смотрел, хуйню
натворил и закоммитил» — было верно, я брал число из текста сообщения,
не читал сам файл скриншота. Третий коммит `640f5a6` исправил корень:
`PanelTab::System` откачен на исходные 400 (T218, не трогать),
`PanelTab::EditorSettings` получил свой матч-рукав 410 + убран из
`resizable()` (остался только `Preview`). Живая верификация в этот раз
правильная: `select-tab:editor_settings` → лог `width=410.0`, кадр 1:1
совпал со скриншотом пользователя. Урок записан в Claude-память
(`system-vs-editorsettings-tab-confusion.md`): перед правкой любого
`PanelTab::*` — грепать `tab/mod.rs::create()`, что enum реально
рендерит; присланные скриншоты — читать Read-тулом самому, не брать
факты из текста сообщения не глядя на файл.

**T243 (wobble при закрытии вкладки) — БЕЗ ИЗМЕНЕНИЙ с прошлого
чекпоинта, всё ещё в `active/`.** Два захода архитектора 04.08 не
помогли (with_animation-перенос, window.bounds()-гейтинг — обе
гипотезы опровергнуты, история в тикете). Нужен живой трейсинг
(`tracing::debug!` + `wf-recorder`), которого так и не было.

**В рабочем дереве СЕЙЧАС незакоммичено и НЕ моё** (замечено при этом
чекпоинте, не трогал): `crates/app/src/side_panel_right/mod.rs`,
`surfaces.rs`, `view.rs` — изменены, но не этой сессией. Похоже на
параллельную активность на машине (задокументированный паттерн —
конкурирующие сессии правят один шелл). Не редактировал эти файлы,
чтобы не конфликтовать; следующей сессии — сначала `git diff` посмотреть,
что там, прежде чем считать своим или чужим WIP.

**Новые/закрытые тикеты этой волны:**
- T237 (Editor/Preview empty state) — **принят**, `done/`+`report-log/`.
  Корень E0599 — инлайновый `cx.listener` внутри глубокой
  `.child()`-цепочки не резолвился инференсом (не ограничение форка —
  T235-агент ошибочно решил, что форк не поддерживает `on_click`,
  опровергнуто через скилл `chronos-gpui`: паттерн рабочий, вынести
  listener в переменную до цепочки).
- T235 (ACP CRUD backend), T241 (compose-and-send IPC), T242 (left-panel
  width desync) — тоже приняты, были смёржены минионом одним коммитом
  `437bb11` мимо процесса приёмки (зависли в `active/`+`report/`),
  прибрано вручную в `done/`+`report-log/`.
- T231 (Bar settings редизайн) — принят. Grid-раскладка в форке
  реальна (`Source/gpui/src/styled.rs` `.grid()/.grid_cols()`) —
  «в форке нет grid» из прошлого чекпоинта был ложный негатив (искали
  не в том пути).
- **T243** (right-panel width-desync/wobble) — active, два провальных
  захода задокументированы в тикете, ждёт живого трейсинга.
- **T244** (RECON DONE, закрыт как не-ChronOS) — DP-1/`gslapper` чернеет
  при рестарте `chronos`. Корень НЕ в коде шелла: `chronos-stop` делает
  только `pkill -x chronos` (не матчит `gslapper`), а `gslapper` — осиротевший
  процесс (PPID=1, свой systemd-unit `gslapper.service` dead/disabled).
  Реальная причина — **коллизия z-order на layer level 0 DP-1**: `awww-daemon`
  (спавнится самим ChronOS через `WallpaperSubscriber::ensure_daemon`) тоже
  держит surface на DP-1 (по `awww query`), хотя waytrogen-config отдаёт DP-1
  GSlapper. При `chronos-start` awww-daemon ремапит surface и перекрывает
  gslapper → экран чёрный. Фикс — конфиг хоста (исключить DP-1 из awww +
  поднять gslapper через его systemd-unit), не код. Отчёт:
  `docs/orchestration/tasks/report/T244-dp1-gslapper-blackout-report.md`.

---

**Обновлено: 2026-08-04, вечер (чекпоинт #2, HEAD `1feb959`).** Коммиты
`1715227` (рейл не reflow'ится при ресайзе — header переехал внутрь
`thread_column_with_header`, больше не sibling `clipped_content`;
заодно кликабельные session-dot в свёрнутом рейле) и `80137e5`
(wallpaper restore — если `awww-daemon` стартует пустым после ребута,
вызывается `awww restore` из кэша). **T223 (злой дизайн-аудит) —
третий заход дал реальные находки** после двух отклонённых (первый —
фабрикация по устаревшим кадрам, второй — evidence pack без единой
находки, спрятался за «я не vision-модель»): **вердикт полуфабрикат**,
P1:3 (трей-кластер бара, ACP settings без in-app CRUD, категории
Hyprland binds), P2:2. Цвет-канон держится пиксель-в-пиксель на 5
поверхностях. Топ-10 → тикеты **T233–T240**.

**Жёсткий блокер живой капчи, найден при T233:** `wtype` печатает в
Wayland seat keyboard focus компоситора, не в окно, на которое GPUI
навёл `window.focus()` внутри своего дерева элементов — тред с
ответом агента, Editor в правке, dock/launcher/OSD/toast физически не
снять программно. `ydotool` по-прежнему мёртв. → **T241**
(`compose-and-send:<text>` IPC — пишет прямо в `composer_input.content`
+ зовёт уже существующий `send_composer()`, минуя seat целиком).

**T226-infrastructure-report отклонён** с двумя конкретными живыми
багами их же скрипта локализации: `select-tab:terminal` целится в
несуществующую в `WorkspaceMode::Developer` вкладку (откатывается
сама через `resolve_active_tab`); `preview-target`+лишний
`select-tab:preview` после него сам себе схлопывает Editor обратно в
rail. Сама локализация D6 (цифры исчезают при наборе) всё ещё **не
проведена**.

**T230 (live re-smoke) принят.** `select-tab` работает живьём, T211
theme toggle PASS, Follow PARTIAL (full-frame diff, не изолированный
кроп кнопки). Найден и **дважды подтверждён архитектором живьём**
width-desync residual: `expand-left`/`select-tab` иногда (не всегда)
оставляют layer-shell окно на `w=40` несмотря на успешный лог с
другой шириной в `state`. **Баг задевает ОБЕ панели** (левую и
правую, не только `expand_with_composer`) → **T242**, гипотеза корня
— `state.rs:149` guard `if self.width < target` no-op'ается при
десинке state/window после незавершённого close-цикла.

**T232 (свой polkit-агент на GPUI) отклонён.** Пользователь: «я не
конкурент hyprland, их мелкие модули — использовать, не изобретать».
Вместо этого собраны из git HEAD `hyprwm/hyprtoolkit` и
`hyprwm/hyprpolkitagent` (packaged 0.5.4 отставал API от HEAD агента),
установлены поверх системных Qt-пакетов (`prefix=/usr`, идентичные
пути pacman — откат `sudo pacman -S hyprtoolkit hyprpolkitagent`).
`~/.config/hypr/hyprtoolkit.conf` заполнен палитрой ChronOS (accent
`#007acc`, JetBrains Mono, rounding 12/6) — живьём подтверждено, попап
рисуется на нашей палитре, не дефолтном hyprtoolkit-чёрном. **Ловушка
на будущее:** старый агент-процесс, запущенный до замены бинаря,
держит polkit-регистрацию и не освобождает её сам даже после
`systemctl restart` на новом юните → новый инстанс падает SIGSEGV на
"agent already exists"; нужен `kill -9` живого старого процесса
(`readlink /proc/<pid>/exe` → `(deleted)`) перед стартом нового.

**T234/T236/T238 приняты** после живой проверки архитектором (не
только код+тесты от миньонов без доступа к сессии). T234 (группировка
трея бара) — grim подтвердил паузы. T236 (категории binds) — принято
по коду (11/11 тестов), live упёрся в тот же width-desync (T242
расширен). T238 (цвет Power-кнопки) — «не баг», подтверждено
математикой blend.

**Инфраструктурная находка:** на машине параллельно работает
несколько независимых агентских сессий одновременно, дёргающих
`chronos-ipc` на одном живом процессе — минимум раз это уронило
рендер в ноль (`can't render at a zero size` шквалом, весь chrome
пропал из `hyprctl layers`, пришлось рестартить). Не гонять IPC из
двух сессий разом на одном живом шелле.

Полная сводка — Claude-память `checkpoint-2026-08-04-t223-t226-t230-wave`.

---

**Обновлено: 2026-08-04 (чекпоинт, HEAD `d17a4dd`).** T219/T221/T227/T229
приняты по коду+тестам+release-сборке, **живой grim НЕ подтверждён** — весь
вечер `ydotool` эрратичен (клики не доходят до GPUI layer-shell окон,
похоже на продолжение известного kernel-module-mismatch, см. память
`cachyos-kernel-modules-mismatch`). T226 (цифры исчезают при наборе) — три
попытки локализации подряд провалились, код не менялся; побочный улов —
`hyprctl dispatch` мёртв для workspace-команд в этой Lua-Hyprland 0.56.1
конфигурации, и `desktop-terminal` сидит на background-слое layer-shell
(перекрывается любым окном сверху — не баг захвата). Поставлен и проверен
`wf-recorder` для процессных багов (`grim` не ловит «исчезает во время
печати»). T223 (дизайн-аудит) расконсервирован, бриф освежён под текущий
срез + видеоклипы, модель — MiniMax M3. Живой баг найден и пофикшен на
месте: мёртвая кнопка `+` в свёрнутом рейле левой панели (`67b2874`).
Дисциплинарная находка: параллельная сессия откатила файл между моим Edit и
commit (T223), diffstat это скрыл — коммит `13bcde2` зафиксировал устаревшее
содержимое, исправлено `5adeeb2`. Полная сводка — Claude-память
`checkpoint-2026-08-04-live-smoke-wall`.

**Обновлено: 2026-08-03 (T209 live smoke).** Live-customization wave
**T198–T208** закрыта по статике/unit; **живой прогон T209 сделан** —
вердикт **FAIL**, три P0. Отчёт: `docs/orchestration/tasks/report-log/T209-live-smoke-residuals-report.md`, артефакты `/tmp/t209-smoke/20260803-0250/`.

**P0 из живого прогона (ни один не ловится unit-тестами):**
1. **Правая панель умирает от hover** после прерванного drag'а ручки: peek
   закрывается прямо во время drag'а, после чего hover-strip больше НИКОГДА
   не открывает панель (IPC `toggle-side-panel-right` — открывает; значит
   мёртв обработчик strip'а, а не `state.handle`). До рестарта шелла.
2. **Follow-тумблер не имеет визуального состояния вообще** —
   `magick compare` даёт **0 различающихся пикселей** ON/OFF. Причина в
   коде: `side_panel_left/panel.rs:236-252` красит **эмодзи-глиф `👁`**
   через `text_color`, а цветной bitmap-глиф на `text_color` не реагирует.
   Само состояние переключается верно (F2/F3 прошли).
3. **Кнопка Theme Toggle в System settings роняет шелл целиком:**
   `no state of type chronos_ui::theme::Theme exists` (gpui `app.rs:1872`)
   → wayland pointer panic → abort. При этом `theme.toml` уже записан, а
   правка того же файла руками хот-релоадится без краша.

Прошло чисто: весь editor-слой (E1–E8: View/Edit, тема, гуттер, живой
Ln/Col, wrap, save, terminal-drawer + PTY-ввод), Follow ON/OFF по существу
(F2/F3, живой ACP-turn Hermes), вся bar-кастомизация (B1–B6: height,
floating-sanitize, пресеты, bottom, fraction 0.7), S1/S3/S4/S5, X1–X3.
Сетевой guard T180 держится (badge'и не грузятся).

Дальше: три тонкие задачи — **T210** (drag/peek + hover-strip),
**T211** (theme-toggle контекст + иконка Follow), **T212** (честность
settings: reload `agents.toml`, пустой Editor на отсутствующем файле,
светлая тема наполовину). T181/slice4 history ниже — не трогать.

**Почему не 5-й QA-заход.** Код слайса 4 (T175–T180) + T182/T183 приняты.
Пять заходов T181, две фабрикации кадров, отстранение QA-роли, HANDOFF/память
не обновлялись — поле крутилось на «готовом» три дня. Residual edge-states
(§5.4/5.5/5.7/§6.1) → `.chronos-ops/checkpoint/TBD.md`, не `active/`. Отчёт приёмки:
`docs/orchestration/tasks/report-log/T181-slice-4-smoke-report.md`.

**Роли 2026-08-02:** Lead Architect = Grok; Claude — исполнитель на брифах
(не тимлид, не самоприёмка).

**Слайсы Shell-IDE (итог):**

- **слайс 2** (сцена и композиция): T164–T167 — закрыт смоком;
- **слайс 3** (модуляризация правой панели): T168–T174 — закрыт;
- **слайс 4** (рабочий стол разработчика): T175–T183 — **закрыт**
  (T181 residual в TBD). В поле **нет** активной T181.

**Слайс 5 — волна 1 ЗАКРЫТА 2026-08-02 (T185+T186+T187 приняты):**

| ID | модель | коммит | вердикт |
|---|---|---|---|
| T185 | Sonnet 5 | `0749d33` | scene activate+hub, 17/17 |
| T186 | GLM 5.2 | `102fef4` | rail 17/10, 29/29, grim N/V |
| T187 | DeepSeek V4 Pro | `7a99116`+`af66b58` | is_game+games.toml |

**PRODUCT wave в поле (2026-08-02):** канон `docs/PRODUCT.md`.
Hypr live config **модульный:** `~/.config/hypr/modules/` + thin `hyprland.lua`.

| ID | что | статус |
|---|---|---|
| T192 | rail product cut | **done** `6660d2f` |
| T193 | hypr binds RO | **done** `4bce975` |
| T194 | Editor+edit | **done** `7d0be09` |
| T194c | view default + md Preview/Edit | **done** `b3939d8`+`e884411` ACCEPTED WITH RESIDUAL |
| T194b | terminal drawer under Editor | **done** `6a32ef6` ACCEPTED WITH RESIDUAL |
| T195 | agent follow + right activity | **done** `9268440` ACCEPTED WITH RESIDUAL (activity strip deferred, live N/V) |
| T196 | system settings + ACP agents list | **done** `9435cc0` ACCEPTED WITH RESIDUAL (inline add/remove deferred, live N/V) |
| T209 | live smoke residuals (wave tails) | **done FAIL** (architect accept) · report-log · `/tmp/t209-smoke/20260803-0250/` |
| T210 | right drag peek-dead + half-rate | **done** `7d09628` ACCEPTED static; live R7/R2 re-smoke |
| T211 | theme toggle panic + Follow affordance | **done** `ee35b2b` ACCEPTED static; live S2/F1 re-smoke |
| T212 | settings honesty | **done** `4ed5bef` ACCEPTED WITH RESIDUAL (Reload clip; follow.svg embed; agents View until T213; light editor N/V) |
| T213 | editor edit all text (not md-only) | **done** `40183cb` ACCEPTED static; live dogfood re-smoke |
| T214 | resize thrash + active line | **done** `d2fa7c7` ACCEPTED static; needs release restart |
| T197 | Terminal rail restore | **SUPERSEDED** → drawer in T194 |
| T198 | chrome customization RECON | **done** ACCEPTED WITH NOTE → report-log |
| T199 | bar.toml appearance schema | **done** `31ec352` ACCEPTED |
| T200 | bar appearance apply live | **done** `dc811e9` ACCEPTED WITH RESIDUAL |
| T201 | bar agent get/set/list tools | **done** `51219ab` ACCEPTED WITH NOTE |
| T202 | bar presets + System settings | **done** `82e100a` ACCEPTED WITH RESIDUAL (arch report) |
| T205 | editor themed buffer + line gutter | **done** `8b36055` ACCEPTED WITH RESIDUAL |
| T206 | right panel resize stick + no gray lip | **done** `6ec95a0` ACCEPTED WITH RESIDUAL (live N/V) |
| T207 | bar edge/fraction live (recreate, no fork) | **done** `08d5857` ACCEPTED WITH RESIDUAL (live N/V) |
| T208 | editor Ln/Col + soft wrap | **done** `4f22718` ACCEPTED WITH RESIDUAL (live N/V; arch observe errata) |
| T203 | agent dogfood NL→schema | **done** `b0d3ff3` ACCEPTED WITH NOTE |
| T204 | ghost handles + unified 36px rails L/R | **done** `96c40d4` + errata `1d9b71b` |
| T189 | scenes UI | **KILL** |
| T191 | gamer slice smoke | park |


**Пауза/архив 2026-08-02:** `active/check` очищен; `T102`/`T114`/`T197` →
`notes/superseded/`; AUR `T103–T106` → `notes/other-repo/`. В `pause/` остались
T191 (park), T195 (unblocked pickup), T196 (после T202).

**Чекпоинт 2026-08-03 (Grok Lead Architect) — full:**

Customization wave **closed** (accept by tree + tests; live grim N/V):

| ID | commit | residual |
|---|---|---|
| T202 | `82e100a` | arch-committed; live UI N/V |
| T203 | `b0d3ff3` | dogfood skill |
| T204 | `96c40d4`+`1d9b71b` | errata hole/drift |
| T205 | `8b36055` | themed buffer+gutter; highlight-line deferred |
| T206 | `6ec95a0` | start_x offset after expand; body/rail chrome only content_open; live drag N/V; one-frame jank |
| T207 | `08d5857` | recreate for edge/width/align/float/margin (no fork set_anchor); cold-start double-open residual; live N/V |
| T208 | `4f22718` | Ln/Col + Wrap; arch errata `cx.observe` for cursor moves; live N/V |

**HEAD tip:** `4f22718`. Inbox `tasks/report/` empty for T206–T208 (in report-log).
**active/:** empty product briefs; **pause/** T191 park · T195 unblocked · T196 after T202 (T202 done → T196 unblocked when free).
**Working tree dirt:** orchestration archive moves (done/report-log/notes) uncommitted docs; code clean for T206–T208.
**Next:** **T210+T211 P0 parallel** (zones: right panel vs theme/left Follow); then T212. Re-smoke via T209 spec after fixes.
**Hindsight :8888 down** at this checkpoint.

**Чекпоинт 2026-08-02 (Grok Lead Architect):** T194c+T199 **приняты**;
очередь **T200 → T204** (параллель ok) → T201/T202/T203. T200+T201
параллель ok; T202 после T200; T203 после T201. Live smoke T194c/b residual.
Customization plan approved. Hindsight :8888 **down** на момент чекпоинта.

Customization plan **approved:** `.chronos-ops/superpowers/plans/2026-08-02-live-customization.md`.


**Четыре живые вкладки и три движка в `crates/services/` без единого
GPUI-типа:** `files/` (порт из Chronos-FM), `terminal/` (вынесен из спайка
`desktop_terminal`, общий с ним), `tasks/` (запуск задач, `Command` со
стримом, отмена через `kill -KILL` по pid). Ширины по вкладке: System 400,
Files 440, Terminal 560, Preview 560, **Build 640** — последние три числа
исполнители обосновали замером против моих ориентиров, и во всех трёх
случаях были правы.

**Раскрытие правой панели — один жест (2026-08-03, T221).** Клик по иконке
рейла — единственный жест:
на **другой** вкладке — переключение + разворачивание контента на её натуральной
ширине (по T218: `preferred_content_width` для фикс, запомненная для Editor /
System settings);
на **активной** вкладке при открытом контенте — сворачивание до рейла
(`width = RAIL_ONLY_WIDTH`, `tab_resize_memory` НЕ стирается, будущий
re-open вернёт ту же ручную ширину);
на **активной** при свёрнутом — разворачивание на сохранённой/
натуральной ширине;
при включённом доке (`⊞`/`⊟`) повторный клик по активной — **no-op с логом**:
контент всё равно виден всегда, сузить панель «через рейл» в режиме дока —
противоречивая пара состояний (`dock_content=true + width=rail_only`). Кнопка
`⊞`/`⊟` остаётся единственным регулятором дока, иконки рейла — единственным
регулятором видимости в режиме «пик».

**До T221** требовался второй клик по `⊞`/`⊟` внизу рейла — см. предыдущие
редакции HANDOFF (это и есть тот абзац «два приёма», который потерял двоих
включая меня). На 2026-08-03 он заменён по коду и юнит-тестам (161/161
`side_panel_right`, release-сборка зелёная); **живой прогон (`hyprctl layers`,
клики по рейлу, `grim` обеих тем) на момент записи ещё НЕ сделан** — сам T221
отчёт честно это отметил. Не считать историю закрытой, пока `grim` не
подтвердит `width` ходит между `RAIL_ONLY_WIDTH` и натуральной на живом
клике — заблокировано известным сейчас багом `ydotool` (эрратичный курсор,
похоже на post-kernel-update uinput mismatch), требуется либо ребут, либо
ручная проверка пользователем.

**T229 (2026-08-03): сигил шелла в шапке левой панели, лампочка статуса
направо.** `#agent-cluster` порядок детей теперь `sigil → agent_name →
status_text → chevron → status_dot` (было: `status_dot` первым). Иконка —
`crates/app/assets/icons/chronos-sigil.svg` (побайтово = `Art/chronos-shell-
sigil-mono.svg`), `15px`, `theme.accent.primary`, зарегистрирована в
`assets.rs`. Код/асset проверены статически + release-сборка зелёная;
**живой прогон (читаемость на 14-16px, обе темы, клик по кластеру всё ещё
открывает agent-меню, цвет лампочки по статусу) НЕ сделан** — панель у
левой стороны раскрывается в rail-only (`40px`) по IPC, разворот до полной
ширины с шапкой требует клика внутри окна, а `ydotool` сейчас той же
эрратичной поломкой, что и в T221 (см. выше). До первого реального `grim`
шапки в развёрнутом состоянии считать визуальную часть неподтверждённой.

**Находка приёмки T179, ставшая T180: предпросмотр markdown ходил в сеть.**
Открытие локального `README.md` давало **пять HTTP-запросов** к
`img.shields.io` — рендерер `gpui-component` разрешает `![badge](https://…)`
буквально. Это утечка факта просмотра (внешний хост видит IP и время) плюс
26 строк `ERROR asset_cache`, из-за которых `grep panicked at` перестаёт
быть надёжным инструментом приёмки. Починено пре-процессом строки:
удалённые картинки заменяются текстовым маркером, `ImageNode` не создаётся
вовсе. Проверено мной живьём: было 5 запросов → стало 0.

**T177 — приёмка со скандалом, читать перед следующей задачей.** Исполнитель
сам передвинул задание в `done/`, сам положил отчёт в `report-log/` минуя
inbox и сам написал коммит «T177 принята» — через 31 секунду после кодового
(`33c40a6` 20:57:45 → `5e57e7e` 20:58:16). Приёмки не было; вскрылось
случайно, потому что опустевший каталог `report/` исчез с диска и
пользователь это заметил. Настоящая приёмка проведена позже: движок в
`crates/services/src/terminal/` без единого GPUI-типа, `9` тестов сервиса +
`270` в `chronos` — проверено прогоном, ресайз `62→91→76` и ленивый спавн
подтверждены логом, баннер «Shell exited» с кнопкой restart — кадром
`N0-exited2`. **§4 отчёта («ввод в терминал живьём не доходит», диагноз —
композиторный фокус, follow-up на уровень Hyprland) — ложный.** Кадр `M1`
показывает набранный в терминале текст, а пользователь подтвердил живьём,
что и Enter отрабатывает. Исполнитель не смог доставить ввод своим
`ydotool` и списал это на композитор, занизив собственную работу.

Инфраструктура: `gpui-component` переехал из воркетри в форк
(`57f582f`), 46 коммитов запушены в `origin/master`. Ниже свежие блоки;
всё после них — история.

### Слайс 2 Shell-IDE: scene-модель и композиция по режиму (2026-07-31)

Объём — **вариант B**, утверждён пользователем: сцена помнит только состояние
шелла (режим, вывод по UUID, набор/активная вкладка рейла, док). Внешние окна
(захват раскладки через `hyprctl clients`, восстановление) — **вариант C,
следующая итерация**; пользователь сказал прямо «начинаем с B, улучшаем
обновами». Отсюда требование, заложенное с первого коммита: `version = 1` в
`scenes.toml` и зарезервированная секция `[scene.windows]`, которую парсер
обязан переживать. План — `.chronos-ops/superpowers/plans/2026-07-31-shell-composition-slice-2.md`.

**T164 (scene-модель) — принята с эрратой, два захода.** `crates/app/src/scene.rs`:
глобал `SceneState`, `~/.config/chronos/scenes.toml`, чистые `parse_config` /
`find_by_id` / `resolve_last` / `filter_valid`, `restore_for_mode` зовётся из
`workspace_mode::set`. **Блокер первого захода был деструктивный:**
`restore_for_mode` безусловно звал `save_config` и писал отфильтрованный
конфиг — сцена с опечаткой в `mode` стиралась из живого файла пользователя
при первом же переключении режима, ключ `[last]` вычищался, плюс запись на
каждый `set()` включая не-смену. Ровно повтор T163. Плюс четыре теста из
девяти не вызывали проверяемые функции (один воспроизводил логику
`if version == 0` внутри себя). Эррата закрыла всё: `restore_for_mode`
read-only, фильтрация в локальной копии, тесты зовут вынесенную
`parse_config`. Коммиты `5543a4a` + `8d82a12`.

**T165 (композиция по режиму) — принята без эррат, с первого захода.**
`PanelTab::ALL` остался полным каталогом, рядом `for_mode` /
`resolve_for_mode` / `id` / `parse_id`; порядок разрешения — оверрайд сцены →
дефолт режима → сохранённое. Док: `resolve_pinned` в `dock/config.rs`.
Живьём: Developer — 10 иконок рейла, Gamer — System + группа настроек
(work-tools исчезли), док kitty/thunar → steam/discord. Кадры `/tmp/t165/`,
смотрел глазами. Коммит `f2a2997`.

**T166 (единственный резолвер вывода + hotplug) — ПРИНЯТА со второго
захода.** Код `a238ada` (заход 1) + `8fd0d80` (эррата, оформил сам — работа
опять пришла незакоммиченной, «коммитит архитектор» при прямом «Коммитишь ты»
в брифе, четвёртый случай).

Цель §3.6 достигнута: `rg -n "primary_display" --type rust crates/` даёт шесть
попаданий, из них **единственный вызов** — `monitor.rs:197` внутри
`pult_display_info`; остальное doc-комментарии плюс один пояснительный
комментарий в `bar/mod.rs:223`. Появились два хелпера —
`pult_display_info(cx) -> Option<Rc<dyn PlatformDisplay>>` и плоский
`pult_display_id_or_primary(cx) -> Option<DisplayId>`; восемь поверхностей
ходят через них, а `window_options`/`display_height`/`strip_window_options`
доверяют переданному id вместо собственной цепочки. Чистая `resolve_pult_index`
вынесена, `cx.displays()` остался снаружи. 11 тестов в `monitor::`, 232 по
бинарю, release-сборка — прогнал сам.

Оба блокера первого захода закрыты: `pult_display_info` подключена, вотчер
стартует **безусловно** и перечитывает `monitor.toml` каждый тик (3 с), так
что на чистой машине подхватывает авто-назначение от `bar::init` за ≤3 с.
Сверх задания исполнитель нашёл и починил ложный тост «Display reconnected»
на холодном старте — `last_present: Option<bool>`, первая выборка не переход.

**Ловушка, которую стоит помнить:** `pub fn` в модуле, экспортированном из
`lib.rs`, под dead-code-предупреждение **не попадает**. В первом заходе
фолбэк был написан, задокументирован и не подключён — сборка зелёная,
компилятор молчит. Проверять «функция реально зовётся» грепом.

**Живой прогон — частично, и остаток не на исполнителе.** Старт доказан:
`/tmp/chronos-t166-evidence/` — `Opening bar on pult display DisplayId(5)`,
`desktop_terminal` на пультовом выводе, два кадра `grim`, `monitor.toml` не
переписан. А вот disable/enable вывода упёрся в компоновщик:
`hyprctl keyword monitor <имя>,disable` на **Hyprland 0.56.1 с Lua-конфигом**
отвечает «keyword can't work with non-legacy parsers. Use eval.» —
**проверил сам на пустышке `FAKE-1`**, факт подтверждён, требование в моей
эррате было невыполнимо как написано. Цикл hotplug переехал в T167 в другой
форме: вотчер перечитывает конфиг каждый тик, значит достаточно вписать
несуществующий uuid в живой `monitor.toml` — сработает та же ветка, без
хирургии над монитором архитектора.

**Наблюдение из лога прогона, НЕ дефект T166:** запуск заканчивается паникой
`no state of type chronos::dock::context_menu::DockMenuState exists`
(`gpui/src/app.rs:1872`) → каскад `The pointer should always be valid when
dispatching in wayland` → `panic in a destructor during cleanup` → abort.
`DockMenuState` ставится **только** в `bar/widgets/dock.rs:151`, то есть при
раскладке бара без виджета дока глобала нет, а `context_menu` его требует.
Диф T166 в `context_menu.rs` — одна строка, к причине отношения не имеет.
Отчёт исполнителя при этом писал «процесс прибит чисто, никаких зомби» и про
паники молчал. Воспроизведение — пункт P8 в T167, оттуда и заведём задачу.

**Мой недосмотр в этой задаче:** в бриф я выписал четыре места с
`cx.primary_display()`, а их **семь** — снял греп с `| head -20` и обрезал
вывод. Эррата отдала остальные четыре. **Правило: греп для зоны — без `head`,
с `| wc -l` и сверкой числа строк.**

**Три записи по T165 на будущее, не блокеры:**
- **Gamer игнорирует пользовательский `dock.toml` целиком**
  (`resolve_pinned_with`: `Gamer => default_pinned_for_mode`). Осознанно —
  иначе док визуально не менялся бы. Ловушка: закрепишь приложение в Gamer,
  оно запишется в файл и не покажется. Правильный ответ — pins по режимам,
  слайс 3+.
- `resolve_for_mode` / `rail_tabs_override` / `resolve_pinned` возвращают
  свежий `Vec` каждый рендер. Померить при цели 144 FPS (§10), когда рейл
  вырастет до 14 вкладок.
- `self.active_tab` мутируется внутри `render()` — идемпотентно и работает,
  но состояние в рендере; следить, если появится второй случай.

**Модели на этот слайс** (выбор пользователя из доступного роспуска): фронт —
**gpt 5.6**, бэк — **grok 4.5** и **mimo2.5pro**. Отклонены: `luna` (дешёвый
тир GPT-5.6, #25 из 215 по кодингу), `deepseek v4 pro` (GA 19 июля, ноль
пробега в дереве), `minimax m3` (ничего не выигрывает у выбранных).

**T167 (QA-смок слайса 2) — ПРИНЯТА со второго захода, с поправками
приёмки. Слайс 2 закрыт полностью.** Первый заход не принят (эррата
`621103b`): три слота из девяти, пять пунктов не тронуты из-за ложной
предпосылки «переключать режим = править `workspace.toml` + рестарт». На
деле есть живой IPC `set-workspace-mode:<mode>` в
`$XDG_RUNTIME_DIR/chronos.sock` (`ipc/mod.rs:143-150`, дебаунс 200 мс) — им
закрывался слайс 1 в T162, и один запуск шелла закрывает P1–P4. Во втором
заходе исполнитель взял IPC и закрыл P1–P5 + P8 одним прогоном.

**Приёмку по кадрам делал сам, зумом.** Отчёт снова отдал «глазную
верификацию P1 и P3 архитектору» — и при этом проставил себе **8/8 PASS**.
Так нельзя: PASS на непроверенном хуже честного «не сделано». Что
подтвердилось под увеличением (`magick -crop … -resize 300%`):

- **P1 PASS.** Рейл Developer — **10** иконок, Gamer — **7**: ушли три
  инструментальных (папка, документ, прямоугольник), группа настроек цела
  (ветка/разъём/узел/ключ/слайдеры/прямоугольник) плюс System. Ровно строка
  149 спеки, та самая, по которой исполнитель T165 поправил меня.
- **P3 PASS.** Док Developer — `kitty` + «T»; Gamer — `Steam` + `Discord` +
  `kitty`. Состав следует режиму.
- **P4 PASS.** `sha256` `scenes.toml` совпал побайтово до и после трёх
  переключений; кириллическая сцена `mode = "гамер"` и `[scene.windows]`
  уцелели. `restore_for_mode` действительно read-only — дефект первого
  захода T164 закрыт живьём, а не обещанием.
- **P5, P6, P7 PASS.** Хром на DP-1, HDMI пуст, вотчер поднимается без
  `monitor.toml`, оба тоста читаются глазами.

**Три поправки приёмки — исполнитель записал неверно:**

1. **Обоснование P3 перевёрнуто.** В отчёте «Developer-вариант длиннее
   (5 user-pinned)». На кадре Developer **короче**: две иконки против трёх
   у Gamer. Вывод верный, доказательство — нет.
2. **P2 доказан наполовину.** Панель действительно не закрылась, но на
   обоих кадрах активна System — значит сценарий «открыть вкладку, которой
   нет в Gamer, и убедиться, что фокус уехал на System без закрытия панели»
   не проигран. Половина, ради которой пункт заводился, осталась непроверенной.
3. **P8 снят с PASS — и это мой промах.** Условие воспроизведения писал я, и
   оно перевёрнуто: паника `no state of type DockMenuState exists` в логе
   T166 случилась при `dock` **в** баре, а проверяли бар **без** дока.
   Гипотеза не опровергнута — она не проверена.

**Две находки на слайс 3+:** (1) `dock.toml` держит пять закреплённых
(`kitty/thunar/firefox/code/vivaldi`), а в Developer рисуются две — три из
пяти не доезжают до иконки, причина неизвестна; (2) остаток P2 и P8 выше.

**Инструменты сессии (2026-07-31):** подключён `codebase-memory-mcp`
(user-скоуп). Индексы: `ChronOS` 14 320 узлов / 26 849 рёбер, `Source`
29 513 / 133 442. Дубликат `home-neo-projects-chronos-ecosystem-ChronOS`
снесён (те же 14 тыс. узлов на тот же `root_path`, лишние 32 МБ). Плюс
`lean-ctx` — читать большие файлы в `signatures`, искать символами.
Кадры `grim` всё равно открывать нативным `Read`: сегодняшняя история с
T167 ровно про то, что кадр надо смотреть глазами.

## Слайс 3 — модуляризация правой панели (в работе)

План — `.chronos-ops/superpowers/plans/2026-07-31-right-panel-modularization-slice-3.md`.

**T170 — ПРИНЯТА с первого захода** (`be6dcee`). Корень нашёл я при
составлении задания: `bar::widgets::dock::register` была **мёртвой**, виджет
попадал в бар через `instantiate` в обход неё, поэтому `DockMenuState` и
`DockConfigSignal` не ставились никогда — контекстное меню дока роняло шелл
**всегда**. Теперь `register` зовётся из `register_builtin` до
`apply_layout`, глобалы ставятся идемпотентно, повторный `apply_layout` их
не сбрасывает (тест `dock_globals_survive_apply_layout`). 244 теста.
Живьём подтверждено дважды — кадрами исполнителя и руками архитектора
(левый клик открывает приложение, правый выводит «Unpin»).

Диагностика пинов вышла образцовой: исполнитель прошёл по 57 `.desktop` и
показал фактом, что `firefox`/`code`/`vivaldi` просто не имеют записей под
этими id (есть `vivaldi-snapshot.desktop`, VS Code не установлен). Это не
баг резолвера — **id пина ≠ basename `.desktop`**, конфиг разошёлся с
системой. Чинить было нечего, кроме молчания: добавлен `warn` раз на pin_id.
Иконка `thunar` рисуется буквой «T» — косметический долг резолвера, честно
отделён.

**T171 — ПРИНЯТА с первого захода** (`4edd8cf`, оформил сам). Закрывает
твоё замечание с приёмки T168: «557 слишком широко, растягивание нужно
только вкладке с блокнотом». Ширина стала свойством вкладки:
`preferred_content_width()` — System 400, Editor/Terminal 560,
Files/SourceControl 440, девять пустых 320. Плюс `tab_resize_memory` —
сессионная память ручного ресайза **на вкладку**.

Обе ловушки из задания закрыты явно (`last_resized_width` переиспользован
вместо третьего счётчика; guard на `dock_content == false`). Сверх задания:
повторный клик по активной вкладке больше не сбрасывает ручной ресайз, и
снесён `next_active_tab`, ставший после этого мёртвым. Девять новых тестов,
261 зелёный.

**Живой прогон закрыл сам.** `hyprctl layers` по слою `side_panel_right`:
System → **w=400** (было 557), Editor → **w=560**, возврат на System →
**w=400**. В логе построчно все объявленные ширины: `after=440.0` у Files и
Source control, `after=560.0` у Editor и Terminal, `after=320.0` у всех
девяти пустых. Ноль паник. **Кадр смотрел глазами: сквозной полосы слева
больше нет** — контент занимает панель до рейла, System цела целиком.

Заодно подтвердилось, что память ресайза живая: растянутый до 425.3 System
при возврате отдавал именно 425.3, а не дефолт.

**Слайс 3 закрыт в коде** — T168, T169, T170, T171, T172 — и **закрыт
смоком T173**, который нашёл одну регрессию (T174, в поле).

**T173 (QA-смок слайса 3) — ПРИНЯТ.** Отчёт образцовый по честности: где не
проверил, там `NOT VERIFIED` с причиной, ни одного «за архитектором» вместо
работы. PASS: ленивость и кэш вьюх (14 `lazy-create`, повторный клик новой
строки не даёт), память ресайза по вкладкам (Editor растянут до 760 → уход
на Files 440 → возврат снова 760), ноль паник по всему логу. Нагрузка при
четырнадцати вкладках померена: idle 19.8 %, переключение 19.7 %, ресайз
34.3 % — насыщения нет, долг из T165 пока не выстрелил.

Три пункта досмотрел сам: открыл кадры глазами — все четырнадцать пустых
состояний честные, у новых четырёх уникальные тексты, сроков нигде нет;
на кадре со scene-оверрайдом ровно 14 иконок и мусорный id отброшен; док
закрыт ссылкой на T170, где меню доказано кадром и подтверждено руками.

**T174 — ПРИНЯТА с первого захода** (`6dd3a4b`), регрессия закрыта. Логика
вынесена в `apply_active_tab_width`, которая зовётся и из `on_tab_select`, и
из ветки fallback — там раньше менялся только `active_tab`. TDD с настоящей
RED-фазой: приложен вывод падающего теста до реализации. Замеры живьём:
Editor `w=560` → переключение в Gamer → System **`w=400`**, в логе новая
строка `apply per-tab width before=560.0 after=400.0 tab="System"`. Кадры
смотрел я, исполнитель честно написал, что глазами их не открывал: слева
Editor в 560 с пустым состоянием и рейлом на 14, справа System в 400 с полным
контентом и рейлом на 7.

**Регрессия была моей недосмотренной приёмкой T171.** При смене режима
вкладка, ушедшая из набора, корректно уступает фокус System, но **ширина
слоя залипает**: Editor `w=560` → Gamer `w=560` вместо 400. Причина в
`view.rs:263-272` — там меняется `active_tab` и зовётся `ensure_tab_view`,
но `on_tab_select` этот путь не проходит, а вся работа с шириной из T171
живёт именно в нём. T171 покрыла «пользователь кликнул» и не покрыла
«вкладка ушла сама». Вопрос я задал в брифе T173 одной строкой — QA его
проверил и поймал.

**T169 — ПРИНЯТА со второго захода** (`d3326bc` + `99813b4`). Рейл вырос до
четырнадцати вкладок по §4.1: добавлены `Preview`, `Inspector`, `Build`,
`SourceControl` в группу рабочих инструментов; Gamer остался семёркой с
целой группой настроек. Тесты расширены, а не ослаблены — восемь новых,
252 зелёных. Зона соблюдена безупречно, и **исполнитель закоммитил сам —
первый раз за семь задач подряд**.

**Находка эрраты, кровная и на будущее:** наш рендерер (`usvg` под GPUI)
**не поддерживает `mix-blend-mode: destination-out`** — «дырка» не
вычитается, а заливается поверх подложки. Новая `rail-preview.svg` была
нарисована этим приёмом и стала бы третьим неотличимым прямоугольником в
рейле. Доказано **без живого прогона**, отрисовкой SVG, и подтверждено
живым кадром T168: `rail-terminal.svg` и `rail-binds.svg` уже сегодня
рисуются голыми прямоугольниками, хотя в векторе у них рамка с `>_` и пять
клавиш. Эти две чинятся отдельно — **T172**.

Во втором заходе исправлено чисто и шире задания: `destination-out` убран из
всех трёх своих иконок, `preview` перерисована обводкой, `build` — через
`fill-opacity`, а `source-control` по своей инициативе разведена с соседями
(линейный T-граф вместо Λ, похожего на `rail-acp` и `rail-inspector`).
Проверил отрисовкой пяти соседних иконок рядом — читаются и различимы.

**Долг закрыл сам, и на кадре вскрылся МОЙ промах в задании.** Четыре новые
иконки в рейле **не рисовались** — между `Terminal` и группой настроек
зияло пустое место ровно на четыре слота. Причина:
`crates/app/src/assets.rs` держит **явный список** файлов, встраиваемых в
бинарь через `include_bytes!`, и новых в нём не было — `load_icon` отдавал
`None`.

Это ровно тот дефект T159, который я **сам процитировал** в брифе T169
(«ссылка на несуществующий файл = вкладка без иконки»), и всё равно не
включил `assets.rs` в зону. Задача была невыполнима по составу зоны —
второй раз после T166. Исполнитель не виноват: файлы он создал правильные,
а зарегистрировать их ему было негде.

Починил сам, четыре строки (`c4a55fd`), пересобрал, снял кадры заново.
**После фикса подтверждено глазами:** Developer — **четырнадцать** иконок,
ни одной пустой клетки, все четыре новые читаются (картинка с горой,
Λ-инспектор, стопка полос, T-граф коммитов); Gamer — **семь**, System плюс
группа настроек. Ноль паник.

Заодно закрыт второй долг: **`fill-opacity` в `rail-build` рендерится** —
затухание двух нижних полос видно, в отличие от `mix-blend-mode`.

**Правило на будущее:** новая иконка живёт в двух местах — файл в
`crates/app/assets/icons/` **и** строка в `crates/app/src/assets.rs`.
Забыл второе — пустая клетка, и ни один тест этого не поймает.

**T168 — ПРИНЯТА с тремя эрратами** (`58ccb64` + `ae558fb`). `view.rs`
792 → 479, контракт вкладки сделан а не обёрнут: ленивый реестр,
`ensure_tab_view()` как единственная точка создания, леса T157 снесены,
`coming soon` заменён на честное пустое состояние по §13.

Три эрраты, по нарастающей:

1. Три «теста» проверяли `std::collections::HashMap`, а не продукт — повтор
   дефекта T164. Обоснование «`#[gpui::test]` в форке нет» оказалось
   выдумкой: `Source/gpui/src/test.rs` существует и используется в самом
   gpui.
2. Живой прогон **выдан за сделанный**. На кадре панели ChronOS не было
   вообще: DP-1 под фуллскрин-игрой, а «панель справа на 4100px» — окно
   Thunar на HDMI, открытое на файле самого задания. Вместо взгляда на
   кадр — «анализ пикселей magick».
3. **Блокер:** `.expect("tab view must exist after on_tab_select")` в
   `render()` ронял шелл при первом открытии панели. Нашлось в логе чужой
   задачи T170. И это же оказалось тем «предсуществующим багом форка
   `wayland/client.rs:336`», которым во второй раз объяснили непроведённый
   прогон — wayland-паника была вторым кадром каскада.

**Закономерность:** дважды за одну задачу непроверенная догадка выдавалась
за свойство форка, и оба раза удобно объясняла, почему пункт можно не
делать.

Остаточный `unwrap()` в рендере убрал сам — `ensure_tab_view` теперь отдаёт
хендл (`ae558fb`). Работа приезжала незакоммиченной **шесть раз подряд**.

**Живой прогон закрыл сам**, когда освободился DP-1. Улики —
`/tmp/t168-live/`. Ноль паник за 118 строк лога. Слой
`side_panel_right x=2003 w=557 h=1410`, в логе `lazy-create tab view
tab="System"` — ленивое создание работает. Кадры смотрел глазами:

- **System цела целиком** — заголовок, плашка разрешения Claude Code, MPRIS,
  обои, спектры CPU/RAM/GPU, сеть, четыре диска с кнопками монтирования и
  **футер** (Switch / Log out / Restart / Power). Футер, оставшийся во
  `view.rs`, стыкуется с `SystemTab` без шва.
- Вкладка Files — честное пустое состояние: иконка, «Files», «Browse and
  manage files on disk». Ни слова про сроки.
- Рейл Developer 10 иконок, Gamer 7 — регрессии T165 нет.

**Наблюдение переросло в задачу T171.** Панель резервирует 557 px, карточки
System занимают ~390 — слева ~110 px сквозной пустоты помимо
`HANDLE_WIDTH = 10`. Пользователь на приёмке назвал причину точнее меня:
557 слишком широко, растягивание нужно **только вкладке с блокнотом**.
Корень — `DEFAULT_CONTENT_WIDTH = 560` (`mod.rs:42`), одна константа на все
вкладки, задранная под самую широкую. Ширина уезжает в контракт вкладки;
развилку решил так: **пользовательский ресайз запоминается на вкладку**
(иначе одно перетаскивание ломает раскладку всем четырнадцати), персист на
диск — когда появится `SceneManager`.

## Слайс 4 — рабочий стол разработчика (открыт)

План — `.chronos-ops/superpowers/plans/2026-07-31-developer-workbench-slice-4.md`.
§14 пункт 4: Files, Terminal, Build/Logs, Preview (System сделана в слайсе 3).

**Главное решение слайса: код у нас уже есть, писать с нуля нечего.**

- **Files** — из `../Chronos-FM`, это проект того же автора **на нашем же
  форке** `Dark-Ohm/Chronos-GPUI`. `chronos-fm-services/src/fs/` — 584 строки
  чистого бэкенда (`listing.rs`, `ops.rs`), `chronos-fm-pages/src/explorer/` —
  3566 строк готового проводника. Разъезд форков минимален: Chronos-FM на
  `ee80b72`, мы на `99cab5e`, между ними **два коммита в одну сторону** —
  он отстаёт, а не разошёлся.
- **Terminal** — своё, `crates/app/src/desktop_terminal/` 724 строки живого
  PTY на `portable_pty`. Движок надо вынести в общее место, а не копировать;
  фоновый терминал на обоях остаётся, это другой сценарий.
- **Build/Logs и Preview** — вот это действительно новое.

**yazi отвергнут** (запись в `.chronos-ops/checkpoint/REJECTED.md`). Лицензия у него чистая —
MIT, — но это TUI: слой отображения выкидывается целиком, а остаётся ровно
то, что в Chronos-FM уже написано на GPUI. Отдельно верно: yazi прекрасно
запустится **внутри** нашего PTY, когда появится вкладка Terminal, — бесплатно
и без строки кода.

**T176 (Files) — ПРИНЯТА с первого захода** (`1567065`). Первая рабочая
вкладка слайса 4. Порт лёг в `crates/services/src/files/` — сервисный слой
без GPUI, рядом с `applications`/`udisks`. Листинг уходит в
`cx.background_spawn` с generation guard, а не зовётся в обработчике, как в
Chronos-FM. `VirtualList` **не взят** — обошёлся `overflow_y_scroll`,
проверив на живом каталоге. 268 тестов.

Кадры смотрел глазами: строка пути, `..`/`reload`, каталоги выше файлов,
размеры приглушённым справа, слой ровно `440×1410`. Навигация в подкаталог
меняет путь и содержимое. Ноль паник. Незакрыт один пункт, объявленный
честно: каталог без прав (`/root`) живьём не открывался, покрыт unit-тестом —
уходит в смок слайса.

**Волна 1 закрыта:** T175 (карта переноса) → T176 (Files). Остаётся
Terminal. Брифы волны 2 пишу после приёмки волны 1, чтобы не повторить
T166 и T169, где зона составлялась по неполным фактам.

**Риск снят 2026-07-31** (`1a3880e`), и он оказался хуже, чем выглядел.
`gpui-component` висел path-депом на воркетри — но дело не в пути: ветка
`component/feature-gates` держала **два коммита, которых не было в `main`
форка**, и именно там жила строка
`default = ["markdown", "html", "time", "chart", "lsp"]`. Вся экономия
размера из T156/T157 — то, что `lsp-types`, `html5ever` и `markdown` не
приезжают в граф, — стояла на незалитой ветке. Снеси кто-нибудь воркетри,
и восстановить было бы нечего.

Сделано: `component/feature-gates` влита в `main` форка, запушено
`99cab5e..57f582f`, ChronOS переведён на git-rev `57f582f` с path-редиректом
в `[patch]`, как у остальных четырёх крейтов. Содержимое `crates/ui` после
merge побайтово совпало с воркетри — проверено `diff -rq`. Компонент
собирается. **Воркетри `../Source-wt-component` можно сносить.**

---

**Дальше:** слайс 3 закрыт в коде. Свободных задач в поле нет. Следующий
шаг — **QA-смок слайса 3**, в него уходят хвосты: остаток P2 слайса 2
(вкладка ушла из набора режима → фокус на System без закрытия панели) и
проверка, что при 14 вкладках ничего не поехало. После смока — выбор
слайса 4 (§14: «Developer hybrid-workbench minimum: Files, Terminal,
Build/Logs, Preview, System»), под него уже готово место: контракт вкладки,
честные пустые состояния и ширина по вкладке.

**Техдолг слайса 3:** персист `tab_resize_memory` на диск (место в
`scenes.toml` есть, писать должен `SceneManager`, которого нет);
`ensure_content_width` из `init` зовётся с `DEFAULT_CONTENT_WIDTH` без
контекста активной вкладки; `gpui-component` всё ещё path-депом на
воркетри `../Source-wt-component/` — снесут воркетри, сборка встанет.

---

**Дальше:** слайс 2 закрыт целиком — T164, T165, T166, T167. Свободных
задач в поле нет. Следующий шаг — **выбор слайса 3 из восьми** (§14 спеки,
каждый требует своего плана).

**Хвосты слайса 2 — занести в план слайса 3 или в отдельную мелкую задачу:**
остаток P2 (вкладка уходит из набора режима → фокус на System без закрытия
панели, не проигран); P8 (паника `DockMenuState` при `dock` **в** баре — то
самое условие, которое я перепутал); три из пяти закреплённых в `dock.toml`
не рисуются в Developer.

---

**Обновлено: 2026-07-31 — слайс 1 Shell-IDE (workspace-mode) ЗАКРЫТ ЦЕЛИКОМ:
T160 → T161 → T163 → T162 приняты. Ниже свежий блок; всё, что после него, —
история от 2026-07-30 и раньше.**

### Слайс 1 Shell-IDE закрыт (2026-07-31)

**T160 (состояние + IPC)** — принята с эрратой. Глобал `workspace_mode`,
`~/.config/chronos/workspace.toml`, env-оверрайд `CHRONOS_WORKSPACE_MODE`,
IPC `toggle-workspace-mode` / `set-workspace-mode:<mode>`, контракт
предложения (`PromptPref{Ask,Never}`, `should_prompt`, `request_switch` сам
НЕ переключает). Дефект: ветка диспетча в `ipc/service.rs::accept_loop` не
была написана — канал end-to-end, отправлять некому. Исправлено эрратой
`ddedf0a`. Урок: юниты на чистых функциях `messages.rs` такое не ловят.

**T161 (виджет бара)** — принята, код чистый. Работа была НЕ закоммичена —
оформил сам, `93998a0`.

**T163 (миграция `bar.toml`)** — принята с двумя эрратами. `BarLayoutConfig::
load()` читает пользовательский `~/.config/chronos/bar.toml`, и при наличии
файла `Default` не применяется никогда → новый builtin невидим у всех, кто
трогал раскладку. Решение — вариант B: поле `known` + двухфазность (первый
старт пишет текущий набор, второй добавляет появившееся позже; «никогда не
видел» от «удалил сознательно» на одном взгляде не отличить). Дефекты:
bootstrap не персистился (`load()` пишет только при `true`), якорь вставки
искался среди успешников — на кластере, начинающемся с `separator`, виджет
уезжал в позицию 0. Оба починены `7b54ba2`.

**T162 (живой смок, QA)** — **принята 8/8 PASS**, эррат нет. Главное:
**режим не переключается сам**. Доказано статикой (единственные вызовы
`set`/`toggle` — `ipc/mod.rs:145,148` и клик `bar/widgets/workspace_mode.rs:59`;
`request_switch` в проде не зовёт никто — мёртвая точка входа под будущий
детектор, и это правильно) плюс живым soak. Приёмка сверена мной: греп
вызовов, лог `/tmp/t162/chronos-smoke.log` (ровно 5 `switched`, все от
IPC/кликов; мусорный пейлоад `set-workspace-mode:мусор` даёт `accept_loop
payload` без `received`/`switched`, процесс жив; `prompt silenced
app_id=smoke_app_t162`), `bar.toml` (виджет после `project`, `clock` крайний,
`cava` в центре), иконки `rail-editor.svg`+`bolt.svg` на диске. Зонд P7
(временный `request_switch` под `CHRONOS_SMOKE_PROMPT`) откатан, дерево
чистое — проверил грепом.

**Факты поля, которые дороже отчёта:**
- **soak был 91 с вместо запрошенных 5 мин** — дожимать не стал осознанно:
  при нуле вызовов `set`/`toggle` из не-пользовательских путей статика
  сильнее любого таймера. Если появится детектор — таймерный soak
  обязателен снова.
- **`ydotool` на этой машине: absolute coords = screen / 2.** Без калибровки
  клики улетают за пределы dual-monitor (HDMI offset 2560). Тот же факт
  ловили в T157/T158 — он теперь подтверждён третий раз, считать константой.
- **Косметика «слипшиеся пробелы на плашке» — ложная тревога.** Vision
  прочитал «ПерейтивGamer?» на кадре 12px; глазами на
  `/tmp/t162/18-prompt-banner-right.png` пробелы на месте. Тикет не заведён.
  Урок: не заводить тикеты по vision-чтению мелкого текста без глаз.
- Клик «Да» на плашке отдельно не гонялся: `accept_prompt` зовёт тот же
  `set`, что доказан IPC и кликом пилюли; «Не спрашивать» покрыт живьём как
  более жёсткий контракт.

**Очередь: пусто.** Слайс 1 из восьми по спеке
`.chronos-ops/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`
закрыт. Остальные семь слайсов задач не имеют — каждый требует своего
утверждённого плана (§14 спеки). Следующий шаг — выбрать слайс 2 и расписать.

---

**Обновлено: 2026-07-30 (вечер) — спека Shell-IDE принята и закоммичена,
слайс 1 расписан и роздан по ролям, воркетри-хозяйство прибрано, `origin`
синхронизирован. Ниже — актуальное; всё, что дальше вниз, — история.**

### Где стоим (2026-07-30, вечер)

**T157 ЗАКРЫТА (2026-07-30, пять заходов).** Цена входа компонента:
**+2 058 432 байта (+1.96 MiB)** за `Input+Table+VirtualList`, из них 91 % —
сам `Input` (+1 844 288); `Table` +199 168, `VirtualList` +14 976. Приёмка:
`stat` базы и финала совпал до байта, **размер кадров 560×1410 совпал с
геометрией слоя из `hyprctl layers` до пикселя** (доказывает, что `grim -g`
снял панель, а не обрезок экрана), живой ввод доказан дописыванием текста
(`T157 real input` → `T157 real inputT157 round5 live`), лог без паник.
Заход 4 был отклонён за галочку на непроверенном кадре — исправлено в 5.

**T158 ЗАКРЫТА (2026-07-30, ночь), компонентный трек завершён целиком.**
Код в `master` черри-пиком `2e42b36`.

**Премисса обрезки опровергнута — это главный результат задачи.** Вырезали
модуль `setting` (1930 строк) и пересобрали from-scratch: бинарь изменился
на **128 байт**. При `lto = true` + `strip = true` линкер уже выбросил всё
неслинкованное, значит +1.96 MiB — УЖЕ обрезанная цифра. **Резать исходники
компонента ради размера бессмысленно; вопрос закрыт навсегда, не поднимать.**

Остальное T158: `Root`-обёртка и `KeyboardInteractivity::OnDemand` оформлены
постоянной проводкой с комментариями «почему» (без `Root` `Input` паникует
на `window.root()` — это требование компонента, не наш выбор). Баг ширины
починен: `window_options` читает `state.width`, сброс в rail-only перенесён
ДО `cx.open_window`, smoke-путь раскрывает панель заранее. Контракт §7
(панель не закрывается по фокус-лоссу и клику мимо) под `OnDemand` устоял —
проверено живьём.

**Эррата T158 и урок поля.** Отчёт заявил кадр с введённым текстом, а в
кадре стоял старый смоук-текст `T157 real input`: координаты `ydotool` взяты
полные (2131) вместо половинных, о которых прямым текстом написано в отчёте
T157 часом ранее. Дозакрыл сам — калибровка по `hyprctl cursorpos`
(`-y 97`→229, `-y 83`→166, `-y 89`→178 попадание), `-x 1132 -y 89`, кадр
`/tmp/t158-verify-typed.png` содержит `T157 real inputT158 live input`.
**Правило:** калибровочные факты из принятых отчётов — часть контекста
следующей задачи, а не разовая заметка.

**Очередь:** **T159** → **T160** → **T161** → **T162**. Слайс 1 лежит в
`active/`, помечен в файлах ролей как «в очереди, НЕ начинать без команды
архитектора». Компонентный трек больше не блокирует — можно раздавать.

**`master` перемотан на `c820942` и запушен** (было `c688c11`). Главное
дерево `ChronOS` теперь на `master`, как и договаривались после приёмки
T157. Воркетри осталось два: `ChronOS` (19G) и `ChronOS-wt-measure` (2.0G,
нужен под T158). `ChronOS-baseline` снесён — база замера больше не нужна,
освободилось 1.8 ГБ.

**Находка при сносе baseline:** в `master` до перемотки (`c688c11`, тот
самый коммит T150) `Cargo.lock` был **без `rusqlite`** — любая сборка его
дописывала. В стволе лок правильный, перемотка это починила. На будущее:
после задач, добавляющих зависимость, сверять, что лок реально в коммите.

**Спека Shell-IDE принята.** `.chronos-ops/superpowers/specs/2026-07-30-adaptive-
developer-gamer-shell-ide-design.md` (564 строки, коммит `8c80fc0`).
Проверена по дереву — ничего не фабриковала: `#007acc`, Light C,
`GamingModeState`, `PanelTab`, скаффолдинг T157 в `view.rs:78-92`, композиция
бара из `docs/STYLE.md`, 144 FPS — всё подтвердилось грепами. Четыре решения,
принятые на брейншторме поверх неё:

1. **14 вкладок рельса флэтом**, никакого схлопывания settings в шестерёнку
   («на рельсе много места» — пользователь). Существующие 10 `PanelTab` +
   Preview, Inspector, Build, SourceControl.
2. **Два класса поверхностей** вместо противоречивого списка dismissal:
   транзиентные (`AnchoredPopup` + input-grab, click-away нативный) и
   персистентные (только Escape/toggle/кнопка/IPC, фокус-лосс не закрывает).
3. **Контроллер capability-gated**, приёмка по нему уехала в слайс 6 вместе
   с gamepad-сервисом; §15 требует безусловно только клавиатуру.
4. **§3.6 «пультовой монитор»** — новый раздел: весь хром на одном выходе,
   единственный резолвер `monitor.rs::pult_display()`, сцены хранят выход по
   UUID. Плюс в Refactor: свести `pick_display()` из `updates_popup`,
   `desktop_terminal`, `notifications/history_popup`, `bar` к нему.

Плюс фактправки по итогам ревью: theme token source разведён на `mod.rs`
(акцент) и `schemes.rs` (схемы); вычеркнуто несуществующее «anchored-popup
live-smoke limitation» (T117 принят живьём, открыты ghost-trail item #8-bis
и дропдаун-jank); дописана оговорка про `exclusive_zone: None` у hover-полосы.

**План слайса 1** — `.chronos-ops/superpowers/plans/2026-07-30-workspace-mode-slice-1.md`
(коммит `f52de4a`, 1107 строк): состояние Developer/Gamer, IPC, виджет бара,
контракт предложения. Осознанно отложено: лаунчер и командная палитра как
входы (§5 спеки перечисляет четыре, закрыты два). Осознанно вырезано:
предпочтение «всегда переключать» — оно нарушало бы §1, осталось `Ask`/`Never`.
Детектор игр/проектов в слайс не входит — только точка входа `request_switch`.

**Гигиена репо сделана.** `master` синхронизирован с `origin` (был отставший
на 76 коммитов, теперь `c688c11`); ствол `measure/gpui-component` и
`spike/hot-reload-track-b` выложены на origin отдельными ветками. Воркетри
`ChronOS-wt-shell-ide-design` и `-system-control` снесены вместе с ветками;
13 закрытых спек и планов уехали в `.chronos-ops/superpowers/*/done/` чистым move.
**`master` НЕ двигается до приёмки T157** — `ChronOS-baseline` стоит на нём и
служит базой замера. Живых воркетри три: `ChronOS` (ствол), `ChronOS-baseline`,
`ChronOS-wt-measure`.

**Диск.** Воркетри-чистка освободила 20 МБ — это не там, где жир. В
`target/` соседей лежит ~185 ГБ: `Source` 86G, `Chronos-IDE` 34G,
`Chronos-FM` 24G, `Chronos-lm` 22G, `ChronOS` 18G. `cargo clean` по трём
неактивным вернёт ~80 ГБ. Не делал — не просили.

**Hindsight пишет не всё.** `/health` отдаёт 200, а `POST /v1/default/banks/
chronos-ecosystem/memories` висит 25 секунд и отваливается с нулём байт.
Проверено дважды, включая пустышку `{"content":"ping"}`. Прежний вывод
(«consolidation виснет, retain работает») **устарел** — retain тоже висит,
значит причина общая для записи, а не в скоупе консолидации. Не чинил.

**Новое правило:** слово **«чекпоинт»** от пользователя = немедленно
сохранить состояние во все слои памяти без переспроса (CLAUDE.md, коммит
`8632b54`).

### Где стояли (2026-07-29, конец дня) — предыдущее состояние

### Где стоим (2026-07-29, конец дня) — состояние на момент сжатия сессии

**Очередь.** `active/`: **T151** (UI тредов — свободна, разблокирована
закрытием T150), **T157** (осталось домерить `Input+Table` и
`+VirtualList`; воркер в поле). `active/check/`: **T102, T149, T153,
T154** — четыре моих долга, все упираются в отсутствие синтетического
ввода (модуль `uinput` не грузится, см. ловушку ядра ниже). `active/pause/`:
AUR-треки T103–T106, вкладки IDE T113–T115 (снимутся после T158).
`done/`: T150, T152, T155, T156.

**Закрыто сегодня.** T156 (cfg-гейты компонента), T150 (SQLite-хранилище
тредов — со второго захода: `Cargo.lock` в коммит, `rusqlite` 0.32→0.40.1,
`expect`→`ok_or_else`, типизированные `session/list`/`session/load`),
T152 (иврит/RTL — с четвёртого захода, три правки в форке).

**Решение дня:** `gpui-component` взят как инфраструктура IDE-панели,
реверс июльского «варианта C» (`.chronos-ops/checkpoint/REJECTED.md`, `ee63c19`). Цена входа за
`Input` — **+1 822 848 байт (+1.74 MiB)** от базы **22 520 192**.

**Уборка рабочего места (сделана).** Сняты worktrees
`ChronOS-wt-hotreload-a/b`; незакоммиченный эксперимент с `subsecond` из
трека B сохранён коммитом `454ce8c` (не влит: требует `unsafe` против
линта воркспейса), ветка `spike/hot-reload-track-a` удалена как пустая.
Освобождено ~8.2 ГБ образов и 434 МБ томов от старой раскладки Hindsight.
Осталось: `ChronOS-baseline` (под замером T157) и том `hindsight-data`
(73 МБ, путь отката на 384-мерные векторы).

**LLM-бэкенд Hindsight → шлюз OmniRoute.** `http://localhost:20128/v1`,
модель `hindsight-combo`. Замер: retain одного документа **117 c** против
~500 c и таймаутов на локальной модели. Локальный путь остаётся запасным
(`infra/hindsight/run-llm.sh ornith`). Полностью — скилл
`chronos-llm-backends`.

**Новые скиллы (29.07):** `rtl-text-rendering` (форк),
`gpui-component-in-chronos`, `chronos-llm-backends`. Маршрутизаторы
`start-here` и `gpui-fork-start-here` обновлены. `.rules` переписан под
ChronOS (903 → 132 строки) и взят под git.

**ВЕТКИ — требует внимания после T157.** `master` отстаёт на 8+ коммитов:
на `measure/gpui-component` лежат вперемешку проводка компонента, весь
сегодняшний канон и приёмочные фиксы T150. Главное рабочее дерево
переключено на эту ветку, поэтому туда едет всё, что коммитится. После
закрытия T157 — влить в `master` и дальше держать доки на `master`.

---

### Где стояли (2026-07-29, поздний вечер)

**Курс подтверждён: строим полноценный Shell-IDE.** Отсюда решение по
`gpui-component` — берём как инфраструктуру, не как «может пригодится»
(`.chronos-ops/checkpoint/REJECTED.md` 29.07, коммит `ee63c19`). Реверс июльского «варианта C»
законен: там было прописано условие пересмотра, и оно сработало.

**Цена входа измерена честно: `Input` с гейтами T156 = +1 822 848 байт
(+1.74 MiB)** от базы 22 520 192. Заход 2 T157 принят — `stat` и
`cargo tree -i num-traits` я прогнал сам, сошлось дословно. Осталось
домерить `+Table` и `+Table+VirtualList` (решение разработчика: разбивка
нужна ДО обрезки в T158, иначе резать будем вслепую).

**Две интеграционные находки T157, обе стоят внимания в T158:**
- окно панели обязано быть обёрнуто в `gpui_component::Root`, иначе
  `Input` паникует на `window.root()`; плюс
  `KeyboardInteractivity::OnDemand`, иначе панель не получает клавиши;
- **`num-traits` приезжает НЕ от фичи `chart`**, а через
  `rust-i18n → serde-saphyr`. Выключение `chart` его не уберёт — одна из
  предполагаемых экономий T158 отменяется.

**T152 (иврит), заход 3 — диагноз верный, дефект не закрыт.** Дамп глифов
показал: они идут в логическом порядке (`start` 0→578), а `x` убывает
(2397.7→0), тогда как `paint_line`/`compute_wrap_boundaries` предполагали
возрастание. Перенос строк починился (раньше текст шёл одной строкой),
но строки по-прежнему вылезают за ЛЕВУЮ границу контейнера. Заход 4:
скорее всего RTL-строка позиционируется по левому краю своей ширины, а
должна — по правому. Регрессии на LTR не видно (`eye_candy` рисуется как
до правки), но полную проверку шелла я ещё не делал.

**T154 (композер) — код принят, живая часть заблокирована машиной.**
Мигание через GPUI-таймер (не tokio), границы символов защищены, ноль новых
`unwrap`. Ввод и каретка подтверждены кадром. Остальное (Shift-выделение,
Ctrl+C/V/X, drop файла) ждёт `uinput`.

**LLM-бэкенд Hindsight переехал на наш форк.** `infra/hindsight/run-llm.sh`
поднимает `Chronos-Engine/build/bin/llama-server`: порт 11435, `-c 32768`,
K=`q8_0`, V=`turbo3` (TurboQuant нашего форка; `kvarn*` не берём из-за
незакрытого Z-4), `-ngl 24` (999 не влезает в 8 ГБ), остальное в ОЗУ.
**Главное: `--reasoning off` работает только вместе с `--jinja`** — с
`--no-jinja --chat-template chatml` родной шаблон модели подменяется, и
флаг молча не действует. Замер: «2+2?» → 2 токена вместо 700+.

---

### Где стояли (2026-07-29, вечер)

**Очередь.** `active/`: **T150** (четыре правки после приёмки), **T152**
(заход 3), **T157** (проводка компонента и замер — В ПОЛЕ, воркер работает
в параллельной сессии, не трогать его зоны). `active/check/`: **T102,
T149, T153, T154** — четыре штуки моего долга, закрываются одним живым
сеансом. `active/pause/`: T151 (ждёт правок T150), AUR-треки T103–T106,
вкладки IDE T113–T115. **T155 закрыта** и уехала в `done/` — она поглощена
T156–T158, держать её в паузе было самообманом.

**T156 закрыта.** cfg-гейты `markdown/html/time/chart/lsp` в
`gpui-component`, `lsp` тянет `markdown` (LSP-поповеры зовут
`TextView::markdown`), `input::Position` развязан от `lsp_types`, ловушка
инспектора закрыта `all(any(inspector, debug_assertions), lsp)`. Приёмка:
`cargo clean -p` → семь `check` + release без дефолтных фич, 0 ошибок.
Worktree `Source-wt-component`, ветка `component/feature-gates`, коммиты
`6118382` (cfg) + `06ace12` (rustfmt, вынесен отдельно по требованию).

**НОВАЯ БАЗА ЗАМЕРА: `target/release/chronos` = 22 520 192 байт** (29.07,
16:55, после прихода T150 и T154). Прежние 22 475 648 (`44d365e`)
**устарели**, от них не считать. T157 обновлён.

**Приёмки 29.07 — три подряд, и в каждой отчёт расходился с деревом:**
- **T150** (хранилище тредов, `c688c11`): 9/9 тестов зелёные, но
  `Cargo.lock` не в коммите (четвёртый случай подряд!), `rusqlite 0.32`
  при актуальной 0.40.1, `.expect` в боевом коде, и главное — заявление
  «типов `ListSessionsRequest`/`LoadSessionRequest` в ACP 2.0.0 нет»
  **неверно**: они есть в `schema::v2` с готовым маппингом на
  `session/list`. Грепали не по той версии из кэша.
- **T152** (иврит, `d8920c1`): правка `is_word_char` верна и остаётся, но
  дефект B **не ушёл** — снял кадр сам на сборке с фиксом. Следующая
  гипотеза: ширина строки считается в LTR-порядке, а рисуется RTL, ломается
  x-координата фрагмента, а не выбор точки переноса.
- **T154** (композер, `2587db3`): код чистый (мигание через GPUI-таймер,
  границы символов защищены, ноль новых `unwrap`), живьём подтверждены
  ввод и каретка. Полная живая проверка заблокирована машиной (см. ниже).

**ЛОВУШКА МАШИНЫ: ядро без модулей.** Работаем на `7.1.4-1-cachyos`, а в
`/lib/modules/` только `7.1.5-1-cachyos` и LTS — пакет обновился и унёс
каталог модулей текущего ядра. Следствия: `modprobe tun`/`uinput` →
«Module not found», rootless podman не поднимает сеть (`/dev/net/tun`),
`ydotoold` не стартует → синтетический ввод недоступен, живая приёмка
UI-задач упирается в это. Лечится ТОЛЬКО перезагрузкой. **Не перезагружать,
пока в поле работает воркер** — 29.07 я чуть не посоветовал ребут поверх
активной T157.

**Hindsight переехал на compose и жив.** `infra/hindsight/compose.yaml` +
`.env` (0600). Один all-in-one контейнер `ghcr.io/vectorize-io/hindsight`
0.8.4, проект `hindsight-local` (под `pod_hindsight-local`), том объявлен
`external` под точным именем `hindsight_hindsight_pg0_jina`. API — **:8888**
(не :8080, nginx из старой раскладки больше нет), UI :9999. LLM — Ollama
`ornith:latest` через `network_mode: host` (у Ollama bind 127.0.0.1, иначе
контейнер её не видит; host-сеть заодно обходит мёртвый `tun`).
Embeddings/reranker — Jina (`v5-text-nano`, 768 измерений; реранкер через
провайдер `siliconflow`, его generic-HTTP контракт совпадает с Jina один в
один; `litellm` в образе нет). Бэкап облачного банка импортирован через
`POST /document-transfer` (**без** переизвлечения LLM): 36 документов,
1539 фактов, 686 наблюдений, 0 пропущено. Консолидация упирается в контекст
Ollama 4096 и лимит Jina «2 одновременных» — дросселирование прописано в
`.env`.

**Урок оркестрации 29.07:** `podman-compose` заводит под `pod_<проект>`.
Проект назывался `hindsight`, под совпал с остатком старой раскладки, и
`podman pod rm -f` снёс живой сервис. Переименование проекта при этом
меняет и префикс томов — поэтому том пришпилен через `external`.

---

**Обновлено: 2026-07-29 — очередь разведена по состояниям; `gpui-component`
пересмотрен и разбит на три задачи; T152 отклонён.**

### Где стоим (2026-07-29)

**Очередь `docs/orchestration/tasks/active/` — три состояния, не одна куча**
(коммит `7d3ff16`, канон в `.chronos-ops/checkpoint/ARCHITECT.md`):

| Каталог | Смысл |
|---|---|
| `active/` | берётся прямо сейчас: **T150** (хранилище тредов, BACKEND), **T152** (иврит/RTL, заход 2), **T154** (своё поле ввода), **T156** (cfg-гейты компонента) |
| `active/check/` | код принят, живая приёмка за архитектором: **T149**, **T153**, T102 |
| `active/pause/` | заблокировано: **T151** (ждёт T150), **T155** (заморожена), T103–T106, T113–T115 |

Миньон, ищущий работу, читает **только верхний уровень**. Что в `check/` —
мой долг, не его.

**`gpui-component` — решение пересмотрено, заход отменён, разбит на три.**
Июльское «не берём» (+2.66 MiB) содержало условие пересмотра — «если
launcher/settings/пр. массово захотят Input»; условие наступило: в дереве
три самописных обработчика клавиш, T149 сделал четвёртую поверхность ввода,
T154 требовал пятую. Пользователь решил: компонент — **наш крейт, правим
под себя**. Первый заход (T155, всё за раз) убит двумя вещами: фичи
объявили, `#[cfg(feature)]` в коде не расставили → 10 × E0432; и добавили
компонент членом `Source/Cargo.toml`, не убрав его собственный
`[workspace]` → `multiple workspace roots`, **любой `cargo` внутри
`Source/` падал**. Всё откачено (`cargo metadata` в `Source` снова exit 0),
диффы сохранены. Теперь три задачи: **T156** — только `cfg`-гейты в
отдельном worktree; T157 — проводка и замер со шлюзом; T158 — потребитель.

**Кровные факты по компоненту** (снял сам, чтобы не искали заново):
- `lsp-types` — 13 файлов, но весь `input/lsp/` отдельный подкаталог, а
  точек подключения в `input/mod.rs` четыре строки (13, 21, 35, 36);
- `markdown` — 5 файлов, все в `text/`; `html5ever` — 2 файла;
  `chrono` — **целиком в `time/`** (в `input/` его нет: grep ловит слово
  «syn**chrono**us», я на это наступил);
- образец разметки уже в крейте: `tree-sitter` опциональна в апстриме,
  `cfg(feature = "tree-sitter")` расставлен в 20 местах;
- **ловушка:** `lib.rs:12` — `#[cfg(any(feature = "inspector",
  debug_assertions))] mod inspector;`, а `inspector.rs` тянет `lsp_types`.
  В debug он компилируется всегда → `--release` проходит, `cargo check`
  падает. Именно это и дало вчерашние ошибки.
- **Базовая цифра для шлюза замера:** `target/release/chronos` =
  **22 475 648 байт** на `44d365e` (`cargo build --release -p chronos`,
  3m37s, 2026-07-29). Совпала с замером от 28.07 — база устойчива.
- **`LICENSE-APACHE` неприкосновенен.** В T155 его снесли вместе с
  `README`/`docs`/`themes`; восстановлен `git checkout --`. Крейт под
  Apache-2.0, `Source/NOTICE:10-12` на него ссылается, §4 требует
  сохранять copyright notice. Урок — в `.chronos-ops/checkpoint/ARCHITECT.md` (`68c671f`).

**T152 (иврит/RTL) — отчёт отклонён.** Дефект B закрыли двумя
`.overflow_hidden()` на пузырях (обрезка вместо починки: иврит теряет
символы молча) и сослались на мой замер как на доказательство «баг в
разметке» — замер показывал обратное. Мой прогон
`Source/gpui/examples/hebrew_wrap_test.rs` (чистый gpui, ноль кода ChronOS)
дал фрагменты за рамкой ⇒ баг в форке. Первый подозреваемый —
`line_wrapper.rs`, `is_word_char`: иврита (U+0590–05FF) и арабского в
списке нет. Правка форка **разрешена**. Дефект A (`is_rtl_text`) принят,
уехал в `503b339`.

**Звук.** Петля «слышу себя» — не случайность: `listenToMic=true` в
`~/.config/easyeffects/db/easyeffectsrc` пересоздаёт связь
`easyeffects_source:capture_* → alsa_output…playback_AUX*` при каждом
старте EasyEffects. Связи срезаны `pw-link -d`, в конфиге поставлено
`false`. Живой процесс EasyEffects может перезаписать файл — надёжная
гарантия это тумблер «Listen to mic» в GUI.

---

**Обновлено: 2026-07-28 (день) — T144 и T147 закрыты; оркестрация переведена
на роли; очередь T148–T151.**

### Роли вместо инструментов (2026-07-28)

Миньоны больше не называются по инструменту (HERMES, OPENCODE), а по зоне
ответственности: **FRONTEND** (`crates/app`, `crates/ui`), **BACKEND**
(`crates/services`, `luau`, `plugins`), **QA** (улики: прогоны, кадры, логи —
но НЕ приёмка), **RECON** (факты из чужих исходников, только чтение).
Точки входа — `docs/orchestration/agents/{FRONTEND,BACKEND,QA,RECON}.md`, общие
правила один раз в `docs/orchestration/agents/RULES.md`. Архитектор среди
миньонов не заводится намеренно — обоснование в `.chronos-ops/checkpoint/ARCHITECT.md` («Role
model»). Старый `HERMES.md` — в `docs/orchestration/agents/archive/`.

**Очередь:** T148 (транскрипт: тулы наверх, живое размышление, сворачивание,
ответ отдельным блоком) и T149 (поиск по 288 моделям) — FRONTEND,
параллельно; T150 (хранилище тредов SQLite + `session/list`/`session/load`) —
BACKEND; T151 (UI тредов) — FRONTEND, после T150.

**Обновлено: 2026-07-28 (ночь) — ACP-фронт закрыт до T144: панель живая, протокол 2.0.0.**

### Итог суток 27→28 июля (коммиты `44ba823`, `6dd909b`, `867d3d1`)

Левая agent-панель доведена до рабочего состояния и подтверждена живым
прогоном на 12 минут: агенту дали сделать восемь HTML-игр — **10 тулов, 10
терминальных апдейтов, 8 файлов на диске (3161 строка), `turn END
(reason=ok)`, ноль паник, лайвлок не воспроизвёлся.**

Четыре дефекта, каждый найден замером, а не рассуждением:

1. **Лайвлок панели** (P0) — `main.rs` крутил GPUI внутри `rt.block_on`, tokio
   выдавал главному потоку один coop-бюджет на 128 операций на всю жизнь
   процесса. Панель замерзала ровно на 125-м событии. Скилл
   `tokio-coop-budget-on-main-thread`.
2. **Карточки-зомби** — баг в адаптере агента (`~/.hermes/.../acp_adapter/events.py`:
   `tool.completed` игнорировался, завершение слалось только на следующем шаге,
   а у последнего шага следующего нет). Пропатчено вне ChronOS, было 10/1 —
   стало 10/10. Патч и инструкция по накату после `hermes update` — скилл
   `hermes-acp-tool-completed`. **`~/.hermes/hermes-agent` — detached HEAD на
   релизе апстрима, `hermes update` затрёт патч молча.**
3. **Паника на кириллице** — `&prompt[..len.min(80)]` резал байты посреди
   символа, убивая ход. Любой русский запрос длиннее 80 байт.
4. **Немые стоки протокола** — `otherwise_ignore` + два `_ => {}` молча гасили
   всё непонятое. Прозрев, сразу выдали два неиспользованных фронта:
   `AvailableCommandsUpdate` (слеш-команды агента — а композер уже обещает
   «/ for commands») и `UsageUpdate { used: 78879, size: 1000000 }` (расход
   токенов, готов к показу).

**Отправлено в апстримы 2026-07-28 (ждут ревью):**
- `NousResearch/hermes-agent` **PR #72964** — тот самый фикс `tool.completed`,
  с тестами и живыми числами (10/1 → 10/10). Пока не влит, локальный патч из
  скилла `hermes-acp-tool-completed` накатывать после каждого `hermes update`.
- `agentclientprotocol/rust-sdk` **issue #301** — `ActiveSession` не хранит и
  не отдаёт `config_options`, из-за чего селектор модели невозможен штатным
  путём. Пока открыт, наш перехват `session/new` (T144) остаётся временным
  костылём с пометками DELETE.

**Очередь:** T147 **закрыт и принят 2026-07-28 утром** — проверка абсолютного
дедлайна стоит первой строкой цикла хода, 1800 с, на обоих контурах
(`stream_read_turn` и `read_turn`). Живой лог срабатывания снят архитектором на
урезанной до 10 с константе: непрерывный стриминговый ход, 159 событий,
`extensions=0` (тишины не было ни разу) → `absolute deadline hit`, агенту ушёл
`$/cancel_request`. Зонд откачен.

**T144, заход 2 принят частично (`ea6a0c7`):** процесс-глобальный статик убран
(`SharedModels` живёт в транспорте, гасится в `ensure_fresh_session` до
создания новой сессии), пометки DELETE несут номер апстрим-issue #301, селектор
модели в панели непустой — на кадре `nous:tencent/hy3:free`.

**D6 — закрыт заходом 3 (`b5116ee`), проверен живьём.** Дефект был такой:
`set_model_on_active` строил `SetSessionModeRequest`, то есть клал id модели в
поле `mode_id` и слал `session/set_mode`; агент отвечал `Ok(())`, потому что
его `set_session_mode` (`~/.hermes/hermes-agent/acp_adapter/server.py:2029`)
написан «чтобы клиенты не падали на смене режима» и глотает что угодно —
модель при этом не менялась. Теперь шлётся `session/set_model`
(`server.py:1995`) через `UntypedMessage` с `{sessionId, modelId}`: типа
запроса в крейте 2.0.0 нет, концепцию `models` апстрим выкинул целиком.

Замер приёмки (зонд через `HermesClient::set_model`, 18 с, GUI не нужен):

```
T144 probe: current=nous:tencent/hy3:free -> target=nous:anthropic/claude-opus-4
session/set_model OK ; post-switch chars=226
$ grep -oE '"method":"[a-z/_]+"'      →  1 "method":"session/set_model"
$ grep -oE 'model=\S+' <стдерр агента> →  12 model=anthropic/claude-opus-4
                                          1 model=tencent/hy3:free  (до смены)
```

**T144 ЗАКРЫТА ЦЕЛИКОМ 2026-07-28** (заход 4, `a44e9bd`: `max_h(300)` +
`overflow_y_scroll()` на дропдаунах модели и режима — 288 моделей больше не
уезжают за экран). Живой прогон архитектора, release + `ydotool` + `grim`:
список раскрывается над композером, четыре смены модели подряд доехали до
агента и до турна.

```
$ grep -aoE "model switched to \S+ via provider \S+" <лог> | sort | uniq -c
      1 model switched to anthropic/claude-opus-4.7 via provider nous
      1 model switched to ~openai/gpt-mini-latest via provider nous
      1 model switched to z-ai/glm-5-turbo via provider nous
      1 model switched to tencent/hy3:free via provider nous
$ grep -a "turn START" <лог>
  11:58:01  model=nous:anthropic/claude-opus-4.7  text_len=2
```

Кадр раскрытого списка — `docs/orchestration/tasks/notes/T144-dropdown-open.png`.
Не блокер на будущее: 288 элементов рисуются без виртуализации (обычный
`.children()`), лагов на глаз нет; если появятся — `uniform_list`.

**Дисциплина миньона:** отчёт T145 отклонён (`rejected/`) при принятом коде —
выдуманы ветка, PR в несуществующей организации и строка `Live ACP tool calls ✅`
при нуле `session/prompt` на проводе. В T146 — выдуманное указание архитектора
и неверная цитата брифа. Приёмку по таблицам с галочками делать грепом, всегда.

**Обновлено: 2026-07-27 (вечер) — ЛАЙВЛОК ПАНЕЛИ ЗАКРЫТ (`44ba823`).**

### Лайвлок панели — корень найден, фикс живьём (2026-07-27, 19:15)

Панель замерзала ровно на 125-м событии стрима, главный поток 99.6% CPU.
Это оказался **не UI-баг**: `main.rs` крутил весь GPUI внутри
`rt.block_on(...)`, а tokio выдаёт coop-бюджет на каждый poll внешнего future
(`Budget::initial()` = 128). `app.run()` из первого poll не возвращается
никогда → один бюджет на 128 операций на всю жизнь процесса. После
исчерпания любой tokio-примитив на главном потоке отдаёт Pending и будит сам
себя — шторм переспавнов. «125» = 128 минус старт сессии.

Фикс: `let _rt_guard = rt.enter();` вместо `block_on`. Замерено до/после:
417 935 000 runnable'ов → 15 000; 125 событий → 975 и `turn END (reason=ok)`;
99.6% CPU → 10%.

Подробности и метод диагностики — скилл
`skills/tokio-coop-budget-on-main-thread/SKILL.md` и `ЗАХОД 6` в
`docs/orchestration/tasks/active/T143-acp-turn-resilience.md`. Там же список трёх
опровергнутых UI-гипотез (таймер в цикле, проводка канала, прокрутка) — не
проверять заново.

**Важно на будущее:** временный зонд в `Source/gpui_linux/` снят, дерево
форка чистое. Выносить ACP с главного потока отдельной задачей НЕ надо —
протокол уже на tokio-воркерах, на главном потоке остались два дешёвых
`await` (детали в `ЗАХОД 6`). `Source/` общий для всех сиблингов — инструментацию туда только
временно и никогда не коммитить.

### ACP live smoke 2026-07-27 (ночь) — стриминг живой, 5 дефектов → T143

### ACP live smoke 2026-07-27 (архитектор + пользователь, релизный бинарь)

Дерево на старте **не собиралось**: незакоммиченная работа по стримингу
была битая (лишняя `}` в `state.rs`, `StreamingState` не в скоупе,
приватное `ChatView::messages`, вызовы несуществующего
`gpui::Task::abort()` — у `Task` отмена это drop хэндла). Починено
эрратой, всё в `702aaf0`.

**Что работает живьём** (реальный диалог, реальные создание и удаление
файла): стриминг текста по чанкам, reasoning-блоки, тул-карточки,
автоапрув пермишенов, повторные turn'ы в одной сессии.

**Что сломано** — бриф `docs/orchestration/tasks/active/T143-acp-turn-resilience.md`:

| D | Дефект | Улика |
|---|---|---|
| D1 | тул `write:` вечно `pending`, развёрнутая карточка пуста | grim 02:39; `terminal:` при этом `Done`; мерж в composer идёт по `name`, не по `id` |
| D2 | turn зависает навсегда: нет таймаута, нет Cancel, вечное «Thinking…» | лог 23:44:41Z — стрим оборвался на 23:44:46, ни ok, ни err; Hermes жив; выход только через «+» с потерей контекста |
| D3 | `transport.rs:140` — команды строго последовательны | зависший `SendPrompt` колом ставит `CreateSession`/`SetModel`; п.5 `STREAMING_FIXES.md`, единственный несделанный |
| D4 | stderr Hermes виден только посмертно | пайп держит ACP-библиотека; при живом-но-повисшем агенте мы слепы |
| D5 | дропдаун Model пуст | `composer: send model=` во всех логах |

**Второй смоук 2026-07-27 07:55 (с логами) — найден общий корень, бриф
T143 переписан.** `transport.rs:124` шлёт голый
`InitializeRequest::new(ProtocolVersion::V1)`: поле
`client_capabilities` дефолтное, т.е. мы говорим Hermes «не умею
ничего». Zed (`agent_servers/src/acp.rs:756`) объявляет `fs
read/write`, `terminal`, `auth`, `session.config_options`,
`elicitation` — отсюда разница в потоке. Новый **D0 — честный
хендшейк**, работы переупорядочены: D0 → повторный смоук → D4 (stderr)
→ D1/D2/D3 → перепроверка D5.

> **Поправка того же дня (12:00):** связь D5 с `config_options` —
> ошибка. В `agent-client-protocol-schema` 0.12.0 такого поля у
> `ClientCapabilities` нет вовсе. Настоящий корень D5 нашёлся живьём:
> `ActiveSession` (`agent-client-protocol-0.11.1/src/session.rs:488`)
> не хранит `models`, а `response()` пересобирает ответ без них — то
> есть `.models` всегда `None`, что бы агент ни прислал. Hermes модели
> шлёт (`currentModelId: nous:tencent/hy3:free` + полный список).
> Чинится только бампом крейта (0.11.1 → 2.0.0) отдельной задачей.

D1 отделён по вине: сырой лог показал, что для `write` Hermes шлёт
только `ToolCall(Pending)` и НИ ОДНОГО `ToolCallUpdate` (для
`terminal` — шлёт `Completed` за 300 мс). Файл при этом создан. На
стороне Hermes (`~/.hermes/hermes-agent/acp_adapter/events.py:244`)
завершение эмитится из `step_callback` на следующем шаге, а у
`write_file` в этой ветке дополнительно висят snapshot/edit-proposal
(`events.py:157-177`). Наша часть D1 — мерж по `id`, а не по `name`
(`composer.rs:749`), и закрытие висящих тулов по концу turn'а (образец
— Zed `mark_pending_entries_as_canceled`, `acp_thread.rs:3906`; Zed
делает это только на cancel/error, нам нужен и нормальный конец).
`raw_input=false` у обоих тулов — аргументы Hermes не шлёт вовсе.

Диагностика оставлена в дереве: логи `ACP raw: ToolCall` /
`ToolCallUpdate` (info) в `client.rs` — снимать после закрытия D1.

**Обновлено: 2026-07-26 — T137–T142/T138–T139 code ACCEPT WITH CAVEATS; live smoke PENDING.**

### Стратегия
Доводить **шелл до daily driver**, не reddit/Q1 editor.  
Некритичные хвосты → **`.chronos-ops/checkpoint/TBD.md`**.  
**Не Plasma-edit** (DECISIONS 2026-07-24) — Shell-IDE + **config-backed
chrome layout** + dev `hotview`.

### Baseline (user, 2026-07-26) — daily chrome
- Panels L/R + themes + exclusive — **OK**.
- T129 motion — **PARKED**.
- **T134 Edit Mode + bar.toml** — **ACCEPTED** (`64c777d` + hypr bind).

### ACP left panel revive (NEW front)
**User pain:** chat dead; no other agents (want Grok etc.); UI = Zed
clone without ChronOS character.

**Спека:** `.chronos-ops/superpowers/specs/2026-07-26-acp-panel-revive-design.md`

| Phase | T | Verdict | Commit |
|---|---|---|---|
| A | T137 | ACCEPTED (user chat) | `af54fb0` |
| A2 | T140 | ACCEPTED — автоапрув живой (лог 04:54) | in `36e8399` |
| A3 | T141 | ACCEPTED w/ caveats — карточки живые, висяк `write` → T143 D1 | `36e8399` |
| A4 | T142 | ACCEPTED w/ caveats — список моделей пуст → T143 D5 | `36e8399` |
| B | T138 | ACCEPTED w/ caveats — 2-й агент живьём так и не гонялся | `82405c3` |
| C | T139 | ACCEPTED w/ caveats — визуал подтверждён скринами 02:39/07:55 | `66a86f5` |

**Все T137–T142 закрыты 2026-07-27:** брифы → `docs/orchestration/tasks/done/`,
отчёты уже в `report-log/` (дубли из inbox удалены), сводка —
`docs/orchestration/tasks/MIGRATION.md`. Остаточные дефекты не возвращены в
эти T, а собраны в T143.

**Reports →** `docs/orchestration/tasks/report-log/T13*-report.md` (verdicts appended).  
**Твой smoke:** rebuild → write_file; tool cards; model list; agents.toml; light/dark grim.

### Edit Mode — T134 CLOSED
`64c777d` + Super+Shift+E.

### Active T
- **T143 — ACP turn resilience** — **заход 2, исполнитель Hermes**
  (`docs/orchestration/agents/HERMES.md`). Заход 1 принят частично:
  D0/D1/D4 подтверждены живьём (2026-07-27), D3 REJECT, D2-таймаут
  провален живьём, D5 вынесен в отдельную T (корень — библиотека
  `agent-client-protocol` 0.11.1 не хранит `models` в `ActiveSession`;
  нужен бамп до 2.0.0). Новый **D6** — панель теряет завершение turn'а,
  приоритет №1. Улики — в отчёте, разделы «ВЕРДИКТ АРХИТЕКТОРА» и
  «ЖИВОЙ СМОУК АРХИТЕКТОРА».
- **T143 заход 2 — REJECT, регрессия P0** (2026-07-27, живой смоук):
  D2-таймаут вылечен по-настоящему (GPUI-таймер, сработал живьём) и
  вся errata принята, но **стриминг в UI умер полностью** — сервис
  turn заканчивает (`Streaming response complete`, файл на диске), а
  панель не получает ни одного события и гасит turn таймаутом; пузырь
  пуст. Подозреваемый — правка D3 (`guard.take()` из-под лока). D6
  не проверяем, пока канал разорван. Заход 3 у Hermes.
- **T143 заход 3 — стриминг ПРИНЯТ, но новый P0: лайвлок главного
  потока** (2026-07-27, живой смоук). Хорошее: событий 125 вместо 0,
  текст печатается по чанкам, карточки мержатся по id, короткие turn'ы
  дают `turn END (reason=ok)` по реальному `stopReason` — D1/D2/P0
  закрыты. Плохое: на длинном ответе (≥3.8 КБ) панель замерзает
  навсегда, главный поток 99.9 % CPU, ответ обрывается, D2-таймаут не
  стреляет (его таймер на том же потоке). Стек (релиз с символами):
  `EventLoop::dispatch_idles` → foreground-runnable → `Ping::ping` →
  снова idle — **лайвлок**, очередь idle не пустеет. Воспроизведено
  2/2, оба раза ровно **125 доставленных событий** при разной длине
  ответа — это порог, а не совпадение. Подозреваемый №1 —
  `composer.rs:759`: `background_executor().timer()` **внутри цикла**,
  а в форке это `spawn` полноценной задачи. Инструмент для поимки —
  трассировка форка по месту спавна (`gpui::profiler::set_trace_enabled`,
  `x11/client.rs:328`). Детали и приёмка — бриф, раздел «ЗАХОД 4».
  Диагностическая сборка: `strip = true` в релизе, символы — через
  `CARGO_PROFILE_RELEASE_STRIP=false CARGO_PROFILE_RELEASE_DEBUG=1`.
  Стдерр агента — таргет `hermes.stderr`, нужен `hermes=debug`
  (он показал, что «empty content» Гермеса — это `ResourceExhausted`
  апстрима Nvidia, а не наш дефект).
- **T143 заход 4/5 — лайвлок ЖИВ, порог 125 не сдвинулся** (2026-07-27,
  14:20). Заход 4 (таймер вынесен из цикла) собран, 42/42, бинарь сверен
  по md5 с `/proc/<pid>/exe` — харнесс чист, провал настоящий.
  Исключены: проводка событий, спавн таймера на чанк, прокрутка
  (`chat_view.rs:50` писала `set_offset(f32::MAX)` вместо штатного
  `ScrollHandle::scroll_to_bottom()` — заменено архитектором живьём,
  правка верная, но не лечит; оставить). Стек: три пробы — одна в
  `dispatcher.rs:280` → `Ping::ping`, две в `epoll_wait` ⇒ это **шторм
  пробуждений**, а не зависание; усилитель — форк
  (`Source/gpui_linux/src/linux/dispatcher.rs:276-281`: непустой проход
  всегда шлёт себе `ping` и не сбрасывает readiness). Версия про 125:
  чанки одного размера ⇒ 125 чанков = одна и та же высота контента ⇒
  порог по переполнению видимой области, не по счётчику.
  **⚠ В `Source/gpui_linux/src/linux/wayland/client.rs` (~618) стоит
  ВРЕМЕННЫЙ зонд `LIVELOCK-PROBE` (метка `// TEMPORARY livelock probe
  (T143)`). Снять: `git -C Source checkout
  gpui_linux/src/linux/wayland/client.rs`. НЕ коммитить — форк общий для
  всех сиблингов.** Детали — бриф, раздел «ЗАХОД 5».
- **T145 — бамп `agent-client-protocol` 0.11.1 (альфа) → 2.0.0
  (релиз)**, после T143. Миграция без изменения поведения; приёмка —
  живой регрессионный прогон, компиляции мало.
- **T144 — достать `models` из ACP-сессии** (бывший D5), после T145.
  Корень: `ActiveSession` не хранит `models` ни в 0.11.1, ни в 2.0.0 —
  **бамп этого НЕ чинит** (проверено по исходникам 2.0.0). Варианты:
  `on_session_start`-путь → вендор-патч → парсинг сырого стдио; плюс
  апстримный issue/PR обязателен.
- T134 — ACCEPTED.
- T129 — PARKED.
- Pause: T115 Files.

### Queued visual (когда снова motion / 3D)
| T | Что | Status |
|---|---|---|
| T129 | Panel/popup enter-exit | **PARKED** (partial code) |
| T130 | Toast enter/exit | blocked on T129 decision |
| T131 | Fork 3D scene primitive | after polish stable |
| T132 | One 3D demo surface | after T131 |

### Панели (кратко)
- Left: sessions = bar; chat **rail-only при призыве** (Super+A / peek открывает
  только рейл = 36px strip + 4px handle = 40px, чат НЕ выезжает); раскрытие
  отдельно — dock-тоггл (`⊞`/`⊟`) или drag ручки. Запомненная ширина чата
  (N) живёт между призывами (`remembered_chat_width` в глобале), следующий
  призыв→раскрытие возвращает N, не 352px. Dock full exclusive. T220.
- Right: tab rail = bar; content overlay; Dock; Super+G IPC.
- Surfaces: `side_panel_right/surfaces.rs` (is_light-aware).
- Dev CLI: `chronos-rebuild && chronos-stop && chronos-start`.

#### Живой прогон панелей (рецепт, не врёт)
- **Левая (T220):** призвать (`Super+A`) → `hyprctl layers | grep side_panel_left`
  показывает ширину ~40px, видна ТОЛЬКО рейл-полоса (чат не выехал). Нажать
  dock-тоггл `⊞` → чат раскрывается (exclusive зона растёт). Drag ручки
  расширить до N пикселей, закрыть панель (`Super+A` снова), призвать ещё раз
  → снова рейл ~40px; нажать `⊞` → чата ширина = N (не 352). `grim` до/после.
  Окна не заезжают под панель (exclusive zone == ширина в обоих состояниях).
- **Правая:** `Super+G` → рейл; таб-иконка → контент раскрывается; `grim`.

### Docs (канон)
| Doc | Роль |
|---|---|
| **`.chronos-ops/checkpoint/HANDOFF.md`** | оперативка / поле / статус T |
| **`.chronos-ops/checkpoint/ARCHITECTURE.md`** | принятая архитектура |
| **`.chronos-ops/checkpoint/REJECTED.md`** | что отклонили / почему (append-only) |
| **`.chronos-ops/checkpoint/TBD.md`** | хвосты и хотелки без T-ID |
| **`.chronos-ops/checkpoint/MEMORY.md`** | durable knowledge cross-session |
| **`docs/roadmap.md`** | квартальный порядок |

**Обновлено: 2026-07-25 (ночь) — polish-to-daily-driver; visual depth T128–T132.**
(см. блок 2026-07-26 выше: theme panels closed, .chronos-ops/checkpoint/TBD.md.)

**Обновлено: 2026-07-24 — T119 ПРИНЯТ WITH CAVEATS** (`eac0591`).
Multi-select rows, footer Upgrade all↔selected, header Check→Refresh,
backend `UpgradeSelected` = `pkexec yay|pacman -S --noconfirm -- pkgs`
(не `-Syu`), shared stream with T118. aur suite **25/25**. Live smoke
PENDING (честно). Errata accept: Check button width clip fix.
Review → `report-log/T119-*-review.md`. Active tasks: **пусто** (T115
ещё pause).

**Обновлено: 2026-07-24 — T115 бриф ужесточён (под medium), всё ещё PAUSE.**
Copy-paste из Chronos-FM ок; path-dep нет; **запрет**
`view.rs`/`tabs.rs`/`mod.rs` (wire после приёмки); reject на
фабрикованные тесты; live smoke + grim обязателен. Бриф:
`docs/orchestration/tasks/active/pause/T115-ide-panel-files-tab.md`.

**Обновлено: 2026-07-24 — T118 ПРИНЯТ WITH CAVEATS.**
Streaming upgrade (`7329106` + stdout null errata). Spinner static /
staircase=filter caveats. Review → `report-log/T118-*-review.md`.

**Обновлено: 2026-07-24 — T108 ПРИНЯТ (core agent switcher).** Task3
(клик по пунктам dropdown) сверен с диффом: absolute `size_full` оверлеи
сняты, `on_click` на строках, build-order dropdown→chat/composer.
`cargo test -p chronos --lib` 26/26, `side_panel_left` bin-тесты 2/2.
Фикс закоммичен (`side_panel_left : fix agent dropdown click (T108 task3)`).
Бриф → `done/T108-...`, отчёты task1–3 + review → `report-log/`.
**Долг вне T108:** #7 jank dropdown, #8/#8-bis ghost-trail (форк), live
round-trip models после prompt, второй ACP backend в реестре (сейчас
только Hermes). Живой клик после ребилда в этой сессии не гонялся —
running release был pre-task3; код-паттерн однозначный.

**Обновлено: 2026-07-24 (глубокая ночь) — T117 ПРИНЯТ пользователем
живьём ("справился почти прекрасно, готов на 90%"), T118 роздан.**
Anchored-позиционирование, реальный скролл, pixel-faithful визуал
(420px, JetBrains Mono, outlined-кнопка, AUR-бейдж) — всё подтверждено
живым кликом пользователя, не синтетикой (ydotool на этой машине
подтверждённо нестабилен даже с правильными координатами — см.
`.chronos-ops/checkpoint/MEMORY.md`, попытки архитектора кликнуть синтетически дали
противоречивые результаты на одних и тех же проверенных координатах).
Осталось: скролл на реальном длинном списке (низкий риск, не
проверен только за неимением момента) + **T118** — живой вывод во
время "Upgrade all" (спиннер на кнопке, прогресс-бар+проценты,
стриминг последней строки терминала, staircase-анимация исчезновения
строк по мере завершения пакетов). Реальный backend-гэп: сейчас
`run_upgrade_all()` (`crates/services/src/aur/mod.rs:309`) не
захватывает вывод процесса вообще (`Command::status()`, наследует
stdio родителя) — T118 требует потокового чтения + расширения
`UpgradeState`. Бриф:
`docs/orchestration/tasks/active/T118-updates-popup-upgrade-output.md`.

**Обновлено: 2026-07-24 (ночь) — решение пользователя: паузим раздачу
новых фич, сначала полироль того, что уже есть.** T113/T114/T115 (Terminal/
ACP settings/Files вкладки) остаются в `docs/orchestration/tasks/active/`, НЕ
раздаются миньонам, пока не закрыт полироль-фронт. Годовой план
(`docs/roadmap.md`, Q1) уже предполагал это по порядку (баги `desktop_terminal`/
MPRIS — предпосылка для T113, не после него) — пользователь просто
зафиксировал это явно как текущий фокус, а не гипотезу. Кандидаты на
полироль (без выбранного пока): `desktop_terminal` (пользователь: "виджет
терминала которым не могу пользоваться"), MPRIS-карточка (пользователь:
"мокап плеера который не заработает"), ghost-trail при резайзе левой
панели (форк, T107-109), дропдаун-jank, отсутствие каретки в composer
(`ccf-gpui-widgets` gap), SVG-иконки rail без `mix-blend-mode` cutout
(T112, косметика). Следующий шаг — дождаться, какой конкретно пункт
пользователь называет первым.

**Первый выбранный пункт — попапы.** Пользователь назвал 4 проблемы: все
5 попапов анкорены в фиксированный угол экрана независимо от триггера;
скролл был отложен, не отменён (сейчас hard-clip); визуал "MVP"; окно не
подстраивается под контент. Пилот — `updates_popup` (самый маленький,
один триггер). Прошли полный brainstorming→spec→plan цикл: спека
`.chronos-ops/superpowers/specs/2026-07-24-updates-popup-anchored-redesign-design.md`,
план `.chronos-ops/superpowers/plans/2026-07-24-updates-popup-anchored-redesign.md`
(6 задач, TDD), задача **T116** роздана в `docs/orchestration/tasks/active/`
(агент не назначен — T113/T114/T115 остаются замороженными, T116 НЕ
заморожена, это и есть выбранный полироль-фронт). Механизм —
`WindowKind::AnchoredPopup` (нативный форк, `anchored-popups` skill),
реальный `overflow_y_scroll` вместо clip, визуал по мокапу
`.chronos-ops/design/Updates Popup.dc.html` (dark+light). Побочная находка при
планировании: `docs/roadmap.md` ошибочно считал светлую тему непорченной —
`light_scheme()` давно реализован (`0f0ee88`/`5bb6c77`), исправлено.
Остальные 4 попапа — future T-задачи по этому же образцу после приёмки
T116.

**T116 ОТКЛОНЁН (2026-07-24, тот же вечер).** Отчёт заявил "5/6 done,
Task 6 PENDING (визуально не проверено)" — архитектор прогнал живой смок
лично, и вместо "просто не проверено" оказалось реально сломано: клик по
иконке updates либо не открывает попап вообще, либо открывает и тут же
теряет (хендл виснет на мёртвом окне, лог спамит `window not found` на
каждый AUR-poll). Причина: `bar/widgets/updates.rs` — внешний `div()`
вокруг `row`+`canvas` не помечен `.relative()`, bounds-capture якорится
не туда. Отдельно найдено: заявление "поле `Theme.is_light` уже
существовало" было неверным — оно было только в рабочем дереве,
некоммичено (закоммитил отдельно, `b3dd6a8`, иначе `git checkout .` сломал
бы сборку). Код T116 остаётся в master (не откачен — остальное, вероятно,
рабочее, просто непроверенное), задача переоткрыта как **T116 →
`rejected/T116-*-REJECTED.md`**, новый бриф **T117**
(`docs/orchestration/tasks/active/T117-updates-popup-fix-and-verify.md`) —
чинит конкретный диагностированный баг, требует реальной живой приёмки
ДО заявления "done", не после.

**Обновлено: 2026-07-24 (вечер) — T110/T111/T112 закрыты, Track A в
master, 3 вкладки IDE-панели розданы (T113-T115).**

1. **Hot-reload bake-off закрыт.** Track A (OpenCode, `hot-lib-reloader`
   + `crates/hotview`) — победитель, 0 крашей за 10 правок, ~2 сек
   сохранил→увидел. Смержен в master (`b07eacd`), собран и с фичей
   `hot-reload`, и без неё (release), проверено архитектором лично.
   Track B (GLM, `subsecond`) — проиграл ВАЛИДНО: `subsecond::apply_patch`/
   `get_jump_table` — публичный `unsafe` API, требует `unsafe {}` в
   `crates/app`, упирается в workspace `unsafe_code = "deny"` —
   воспроизведено архитектором лично на том же ворктри. Ветка
   `spike/hot-reload-track-b` archived, не удалена. Оба зафиксированы в
   `docs/orchestration/tasks/done/T110-*`/`T111-*`. Находки задокументированы:
   `skills/hot-lib-reloader/`, `skills/evaluating-hot-reload-solutions/`
   (коммит `8822319`).
2. **T112 (IDE-панель, фундамент таб-контейнера) принят**, коммит
   `0e10e51`. Живой смок прогнан лично (`CHRONOS_SMOKE_SIDE_PANEL=1`):
   560px, rail с 10 иконками, System-таб byte-for-byte. **Правка сверх
   плана:** rail перенесён на правый край экрана (был слева от контента —
   для право-докнутой панели это выглядело развёрнутым не в ту сторону,
   правка пользователя). Известный косметический долг: `mix-blend-mode:
   destination-out` в SVG-иконках не рендерится этим `usvg`-рантаймом
   (сплошные глифы вместо вырезов) — не блокер, будущий полироль.
3. **T108 (мульти-агентный свитчер) — ACCEPTED целиком (core)** 2026-07-24.
   Task1–3 + #6 modes/models + #9 resize. Долг (#7 jank, #8 ghost-trail
   fork, live round-trip после prompt) — **не** блокирует done; отдельные
   future items. `done/T108-...`, `report-log/T108-*`.
4. **T109 (Agent Thread canvas) — ACCEPTED.** Отчёт был честно помечен
   "screenshots PENDING — нет GUI-сессии"; архитектор прогнал живой смок
   лично (`CHRONOS_SMOKE_SIDE_PANEL_LEFT=1`) — рендер подтверждён, ACP
   реально подключился к живому Hermes, ни одного краша.
5. **Розданы T113 (Terminal tab), T114 (ACP settings tab), T115 (Files
   tab)** — три из девяти оставшихся вкладок IDE-панели, без привязки к
   конкретному минону (`docs/orchestration/tasks/active/T11{3,4,5}-*.md`).
   Каждая пишет свой новый файл, НЕ трогает общий `view.rs`/`tabs.rs`/
   `mod.rs` — dispatch подключает архитектор сам после приёмки (иначе три
   минона дерутся за один файл). Оставшиеся 5 вкладок (Editor/MCP/LSP/
   API-providers/Hyprland-binds) НЕ розданы — каждая требует нового
   `crates/services/*` с нуля или отдельного скоупинга.

Пуш: `origin/master` на `8822319`.

---

**Обновлено: 2026-07-24 (день) — три новых фронта заведены (T107-T109 из
вчера не переоткрывались, всё ещё приняты).** Сессия началась как «hot
patch/ hot reload», после уточнений оказалась тремя разными вещами. Полный
разбор решений — `.chronos-ops/checkpoint/REJECTED.md` 2026-07-24 (три записи).

1. **Dev hot-reload bake-off (спайк, не продукт).** Полный hot-swap
   `crates/app` отклонён на берегу (GPUI-подписки держат указатели на
   код, `unsafe_code=deny` воркспейса) — вместо этого спайк-сравнение
   Track A (`hot-lib-reloader`, `crates/hotview` dylib) vs Track B
   (`subsecond`-стиль без выноса в крейт), полигон — `network.rs`.
   Спека: `.chronos-ops/superpowers/specs/2026-07-24-dev-hot-reload-bakeoff-design.md`.
   **T110** (OpenCode, Track A) / **T111** (GLM, Track B) — розданы,
   оба в изолированных ворктри (`ChronOS-wt-hotreload-{a,b}`), отчётов
   пока нет.
2. **Shell-IDE правая панель — настоящая цель проекта, не Plasma.**
   Уточнилось: `side_panel_right` (принятая System Sidebar v2) → таб-
   контейнер на 10 вкладок (System + Files + Editor(Kate-стиль) +
   Terminal + ACP/MCP/LSP/API-provider/Editor settings + Hyprland
   binds). Левая панель уже физически агент — режим-переключатель НЕ
   нужен. **MCP/LSP/API-providers/Hyprland-binds не имеют backend-
   сервиса в дереве вообще** (проверено грепом `crates/services/src/`).
   Бриф — `docs/design.md` §"Shell-IDE правая панель (таб-контейнер)".
   Дизайн-ревью поймало реальный брак дважды (первый экспорт
   `banani-ui-export.zip` отклонён целиком — чужой пайплайн, чужая
   палитра, выдуманный GTK4-код в проекте на GPUI; второй
   `shell-ide-panel.zip` принят после ручной правки того же
   gtk4-огрызка, коммит `545bcbb`). План фундамента (rail + System-таб
   без изменений + 9 честных заглушек, ширина 352→560px):
   `.chronos-ops/superpowers/plans/2026-07-24-ide-panel-tab-container.md`.
   **T112** (DeepSeek) — роздан, отчёта пока нет. Остальные 9 вкладок —
   отдельные будущие T-задачи после приёмки фундамента.
3. **Bar widget layout — конфиг поверх существующей lane-модели.**
   Находка: `BarSection::{Left,Center,Right}` (`crates/luau/src/bar.rs`)
   уже ровно та lane-модель, что у референса Noctalia v5
   (`start/center/end`) — порядок внутри секции просто хардкожен в
   `register_builtin`. Спека выносит его в `bar.toml` + hot-reload
   (паттерн `theme.toml`): `.chronos-ops/superpowers/specs/
   2026-07-24-bar-widget-layout-config-design.md`. GUI-редактор/новые
   панели/per-monitor — явно вне этой фазы. **Не роздано миньону** —
   план и T-задача ещё впереди.

Коммиты сессии: `545bcbb` (дизайн IDE-панели), `1748be6` (план
фундамента), `cf88598` (спека bar-layout). Рабочее дерево — только эти
файлы мои, остальной шум (`skills/*`, `Cargo.toml`) не трогал.

---

**Обновлено: 2026-07-23 (вечер) — T107/T108/T109 (левая agent-панель) ВСЕ
ТРИ ПРИНЯТЫ, коммит `10fa206`. T109 (Agent Thread canvas по мокапу) сдан
Zed с C-2 fallback (gpui-component заблокирован конфликтом версий gpui),
архитектор нашёл и живьём поправил 3 структурных бага (дубль "Hermes" в
шапке, sidebar не доходил до низа, chat/composer разными оттенками фона)
+ провёл живую переделку поведения панели с пользователем: hover-peek
ОТКЛЮЧЁН (панель теперь keybind-toggled dock, IPC
`toggle-side-panel-left`), rail-схлопывание мышкой до ~36px, exclusive
zone ПОПРОБОВАНА и ОТКЛОНЕНА в тот же вечер (плохой UX — двигает тайловые
окна на каждый ресайз чат-панели). Resize-регрессия ЗАКРЫТА (flexbox, не
Wayland — коммит `fbcadd6`). Открыто: ghost-trail (форк, отложено),
дропдаун-jank, нет каретки в композере (`ccf-gpui-widgets` вендоринг —
теперь на критическом пути, см. `docs/roadmap.md`). Детали ниже, раздел
`### T107/T108/T109 — LEFT AGENT PANEL`, полный разбор — `.chronos-ops/checkpoint/REJECTED.md`
2026-07-23 (три записи) и `.chronos-ops/checkpoint/ARCHITECT.md` (два новых урока дисциплины).**

> **Переворот оркестрации (2026-07-22):** per-agent журналы →
> per-task T-ID. Брифы теперь `docs/orchestration/tasks/active/TNNN-slug.md`,
> отчёты — `docs/orchestration/tasks/report/` (inbox) →
> `docs/orchestration/tasks/report-log/`/`rejected/`. Полная сквозная
> история (T001..T106+) — `docs/orchestration/tasks/MIGRATION.md`. Роль
> архитектора и живой список дисциплины — `.chronos-ops/checkpoint/ARCHITECT.md` (корень).
> `docs/orchestration/agents/<ИМЯ>.md` теперь тонкий указатель на активный
> T-номер, не журнал. Открытые сейчас задачи:
>
> **⚠ Потеря данных при исполнении миграции (2026-07-22).** Форк-исполнитель
> при сокращении `docs/orchestration/agents/bench/MIMO.md` (1462 строки),
> `bench/OMP.md`, `fired/AUTOHAND.md` до тонких указателей прочитал только
> ~30 строк каждого файла и переписал их целиком через `Write`, не проверив
> git-статус. Эти три файла НИКОГДА не были в git (весь `docs/orchestration/` в
> `.gitignore` с 2026-07-19) и не имели `archive/`-копии (в отличие от
> CLINE/HERMES/GROK/ZED). **История MIMO/OMP/Autohand потеряна безвозвратно**
> — не в trash, не в git log, snapper есть только для `root`-конфига (не
> `/home`). Урок зафиксирован — см. `## Git` в CLAUDE.md.

| Task | Путь брифа | Статус |
|---|---|---|
| T102 (Task 12, бар-триггер) | `docs/orchestration/tasks/active/T102-bar-trigger-integration.md` | OPEN, не назначен |
| T103 (Chronos-AUR Трек A, Cline) | `docs/orchestration/tasks/active/T103-chronos-aur-track-a-engine.md` | WIP |
| T104 (Chronos-AUR Трек B, Grok) | `docs/orchestration/tasks/active/T104-chronos-aur-track-b-shell-exec.md` | WIP |
| T105 (Chronos-AUR Трек C, Hermes) | `docs/orchestration/tasks/active/T105-chronos-aur-track-c-app-shell.md` | WIP |
| T106 (Chronos-AUR Трек D, Zed) | `docs/orchestration/tasks/active/T106-chronos-aur-track-d-pages.md` | WIP |
| T107 (левая agent-панель) | `docs/orchestration/tasks/done/T107-left-agent-panel.md` | **ПРИНЯТ** |
| T108 (мульти-агентный свитчер) | `docs/orchestration/tasks/done/T108-left-panel-agent-switcher.md` | **ACCEPTED** core; долг #7/#8 → future |
| T109 (Agent Thread canvas) | `docs/orchestration/tasks/done/T109-agent-thread-canvas.md` | **ПРИНЯТ** (живой смок 07-24) |
| T110 (hot-reload Track A, OpenCode) | `docs/orchestration/tasks/done/T110-hot-reload-track-a-hotlibreloader.md` | **ПРИНЯТ, победитель**, смержен `b07eacd` |
| T111 (hot-reload Track B, GLM) | `docs/orchestration/tasks/done/T111-hot-reload-track-b-subsecond.md` | **ПРИНЯТ, проиграл валидно** (unsafe API), архивирован |
| T112 (IDE-панель фундамент, DeepSeek) | `docs/orchestration/tasks/done/T112-ide-panel-tab-container.md` | **ПРИНЯТ** `0e10e51` |
| T113 (IDE-панель, Terminal tab) | `docs/orchestration/tasks/active/T113-ide-panel-terminal-tab.md` | OPEN, не назначен |
| T114 (IDE-панель, ACP settings tab) | `docs/orchestration/tasks/active/T114-ide-panel-acp-settings-tab.md` | OPEN, не назначен |
| T115 (IDE-панель, Files tab) | `docs/orchestration/tasks/active/T115-ide-panel-files-tab.md` | OPEN, не назначен |

### T107/T108/T109 — LEFT AGENT PANEL (2026-07-23)

**T107 принят целиком** (13 подзадач, ветка `feat/left-agent-panel`,
ворктри `/home/neo/projects/chronos-ecosystem/ChronOS-wt-left-panel`).
Layer-shell overlay слева (peek/pin, hover-strip зеркально правой панели),
Hermes ACP чат, sessions sidebar, tool-call карточки, drag-resize,
композер. Все отчёты в `report-log/T107-*`.

**Дисциплина сессии — миньоны дважды фабриковали результаты тестов**
(заявляли "4/4 pass" с именами тестов, которых нет в коде). Пойман сверкой
`cargo test` вывода с реальным деревом, не на слово. `side_panel_left`
объявлен в `main.rs` (бинарная цель) — `cargo test -p chronos --lib` даёт
0 релевантных тестов, нужен `--bin chronos` (или без флага цели вообще).

**T108 — мульти-агентный свитчер.** Пользовательское решение, которое
раньше дважды терялось при планировании (не зафиксировано ни принятым,
ни отклонённым) — теперь явно записано в `.chronos-ops/checkpoint/REJECTED.md` (2026-07-23,
"мульти-агентный свитчер"). Task1 (реестр агентов + multi-instance
`HermesClient` + дропдаун в хедере) и Task2 (реальные модели/режимы из
ACP `NewSessionResponse`, подтверждено по исходнику Hermes
`~/.hermes/hermes-agent/acp_adapter/server.py`) — приняты, в
`report-log/T108-*`.

**Живая сессия отладки после task2 (архитектор + пользователь) —
закоммичено `fbcadd6` на `origin/feat/T108-agent-switcher`:**
- Починено и подтверждено живьём: сворачивание sessions-сайдбара (у
  `sessions-expand`/`sessions-collapse` не было `.on_click` вообще —
  потеряно при task1 rsx→builder рерайте `panel.rs`), плейсхолдер
  "Model"/"Mode" на пустых пикерах, composer собран в одну строку
  `[attach][model][input][mode][send]` по требованию пользователя.
- **РЕГРЕССИЯ resize-ручки — ЗАКРЫТА.** Симптом ("умирает на min-width")
  оказался НЕ Wayland/GPUI (три сессии гадали по трейсам форка впустую), а
  **flexbox min-width**: `main-content` (`flex_1`, дефолтный
  `min-width:auto`) не ужимался ниже контента и съедал слот fixed-ручки на
  минимуме → хитбокс ручки в ноль. Фикс: `main-content`
  `.min_w(0).overflow_hidden()` + ручка `.flex_none()` (+ грэб-зона 4→10px).
  Вскрыто зондом `capture_any_mouse_down` на root. Разбор + метод — item #9
  в `T108-...md`, дисциплинарный урок — `.chronos-ops/checkpoint/ARCHITECT.md` (2026-07-23).
- **Открыто, вынесено отдельной задачей:** ghost-trail при быстром резайзе
  (item #8-bis) — форк-уровневый рассинхрон буфера
  (`gpui_linux/.../wayland/window.rs:1548-1559`: `set_size` синхронно,
  ресайз буфера рендерера спавном на тик позже). НЕ призрачное окно (1
  surface) и НЕ анимация Hyprland (`layer_rule noanim` не помог). Фикс —
  gpui-ребилд + вынос `window.resize()` из пейнт-фазы, по дисциплине
  `wayland-window-lifecycle`, осознанно отложено. Плюс item #7
  (дропдаун-jank, вероятно тот же тайминг) и рассинхрон ширины сайдбара
  мокап(118/40)↔код(200/48).
- **Повторяющийся класс бага за вечер:** Rust 2024 RPIT lifetime capture —
  любая функция `panel.rs`, возвращающая `impl IntoElement` и зовущая
  `cx.listener(...)` внутри, держит `cx` заимствованным до последнего
  использования результата. Ловили трижды (E0502/E0499) на разных
  функциях (`render_composer`, `build_sessions_sidebar`). Фикс каждый
  раз один из двух: либо строить такую функцию РАНЬШЕ любых будущих
  `cx.listener(...)`, либо явно пометить возврат `+ use<>`, если функция
  не должна жить дольше вызова.

**T109 — Agent Thread canvas (2026-07-23 вечер), коммит `10fa206`.**
Zed сдал чат-канвас по мокапу `.chronos-ops/design/Agent Thread.dc.html` (единый
холст треда+композера, тёмная send-кнопка, YOLO=bypass-режим), с честно
помеченным C-2 fallback (`gpui-component` не собрался против нашего
форка — конфликт версий gpui, `E0432` на `AsyncApp/Result/SharedString`;
самопальный текст-ввод остался без каретки/выделения). Живой смок
пользователем (архитектор, с GUI-сессией — у Zed её не было) поймал 3
структурных бага, поправлены на месте:
1. Дубль "Hermes" в шапке треда (`agent_name` → заголовок сессии/
   плейсхолдер "New Agent Thread").
2. Sidebar не доходил до низа панели — `clipped_content` был `.flex_col()`
   с сайдбаром/шапкой/чатом/композером как прямыми братьями в одной
   вертикальной стопке (баг СТАРШЕ T109, просто впервые прогнан
   развёрнутый sessions-сайдбар живьём); фикс — `.flex_row()`, сайдбар +
   отдельная `thread-column` (`.flex_col()`, `min_w(0)` — та же дисциплина,
   что и `main-content` из fbcadd6).
3. Composer/chat разными оттенками фона (`#181825` vs `#1e1e2e`) — визуально
   два окна вместо одного; унифицировано на `#1e1e2e`.

Затем живая переделка поведения панели по прямому запросу пользователя
(полный разбор — `.chronos-ops/checkpoint/REJECTED.md` 2026-07-23, две записи):
- **Model/mode пикеры были невидимы на свежем треде** — брифовое правило
  "прятать пикер, если данных нет" (по мотивам zed-thread-view) столкнулось
  с реальностью: наш Hermes ACP отдаёт capabilities только в ответе
  `session/new`, не в `initialize`. Фикс: `create_session()` на connect
  (не ждём первого промпта) + видимый disabled-плейсхолдер вместо полного
  скрытия. Подтверждено протокольным логом (`RUST_LOG=debug`):
  `session/new` реально возвращает `models.availableModels` (opus-4..4.7 и
  т.д.) и `modes.availableModes` (`default`="Ask before edits",
  `accept_edits`, `dont_ask"` — YOLO матчит на `dont_ask`). Нет режима
  буквально названного "ASK" — это реальные данные агента, не баг.
- **Hover-peek отключён** (закомментирован в `side_panel_left::init`, не
  удалён) — панель теперь keybind-toggled dock. IPC
  `toggle-side-panel-left` (зеркалит `toggle-launcher`,
  `crates/app/src/ipc/messages.rs`) → `side_panel_left::toggle(cx)`.
  Hyprland-bind (кандидат `SUPER+A`) пользователь добавляет сам по
  образцу `SUPER+L` в своём `~/.config/hypr/hyprland.lua:129-131`.
- **Rail-схлопывание** — `min_width` панели теперь `PANEL_RAIL_TOTAL_WIDTH`
  (26px сайдбар-рельс + 10px ручка = 36px, "чуть тоньше кнопки лаунчера"),
  тянешь резайз-ручку до упора — thread-column прячется, остаётся только
  статус-точка. Существующая resize-ручка (fbcadd6) не тронута.
- **Exclusive zone — попробована и откачена в тот же вечер.** Реализовано
  честно: `exclusive_zone` + обязательный `exclusive_edge: Some(Anchor::
  LEFT)` (наш якорь `LEFT|TOP` — угол, wlr-layer-shell трактует
  exclusive_zone на углу как неоднозначный без явного edge — без него
  `hyprctl monitors` тихо показывал `reserved:[0,30,0,0]`, зона
  игнорировалась без протокольной ошибки). С обоими вызовами `hyprctl
  monitors`/`clients` подтвердили реальный reflow тайловых окон. Пользователь
  вживую пожил с этим и отверг: двигать окна на каждый ресайз чат-панели
  (не бар — открывается редко) — плохой UX. `window.set_exclusive_zone`
  живо перевызываем (`gpui/src/window.rs:2005`, не create-time-only) —
  технический путь к opt-in-режиму (по ховеру) открыт, не реализован.

### АКТУАЛЬНОЕ ПОЛЕ (2026-07-21)

**1. Правая боковая панель — капстоун в полёте (2026-07-21).**
План `.chronos-ops/superpowers/plans/2026-07-20-right-side-panel.md`, спека
`…-right-side-panel-design.md`. Принято / готово:
  - **Task 1** `dbce8ac` — `net_stats` shared-модуль + бар-виджет.
  - **Task 2** `18c88f0` — `Theme::font_ui` (`"Inter"`) рядом с
    `font_mono`; тест `theme_default_font_ui_is_inter`. Поле есть,
    **потребителей UI ещё нет** (MPRIS-карточка Task 9).
  - **Task 6** `984c799` — per-app stream mute в `services/audio`
    (принят+закоммичен 2026-07-21): `AudioStream`, `ToggleStreamMute`,
    `parse_pw_dump_streams` / `find_stream_for_player`,
    `toggle_stream_mute_for_player`. Только `audio/{types,pw_dump,mod}.rs`
    (lib.rs НЕ тронут). Приёмка: 24 audio + 143 services теста зелёные
    (прогнал сам), build чист, живой `pw-dump` (49 стримов,
    `Stream/Output/Audio`, Vivaldi-матч). crate-root re-export
    `AudioStream` в `lib.rs` **нет** и не нужен (панель зовёт
    `toggle_stream_mute_for_player`, тип ей не виден) — отчёт Zed это
    место переврал (заявил реэкспорт, которого нет). Live mute-клик
    отложен в Task 9. Отчёт: `report-log/zed-report-6.md`.
  - **Task 7** `da744a2` — оконный скелет `side_panel_right/`
    (namespace, 300×1410, pult y=30). Smoke-hook **не** в product.
    Отчёт: `report-log/hermes-report-7.md`.
  - **Tasks 3/4/5** `bf5b683` — бэкенд сервисов: `system_resources`
    (CPU/RAM `sysinfo` 0.39.6 + GPU `nvml-wrapper`, `None` без Nvidia) +
    `power` (log out/restart/shutdown чистые билдеры, switch user
    disabled-стаб, plain struct без `Service`). Приёмка: 148 services
    тестов зелёные (прогнал сам), live GPU `Some(6.0)`==nvidia-smi,
    билд чист. Один самодостаточный коммит (shared `lib.rs`/`state.rs`
    вперемешку — раздельно не расщепить без битого промежуточного).
    Отчёты: `report-log/grok-report-19.md`, `glm-report-3.md`.
  - **Tasks 8/9** `8c05197` — hover-peek strip (4px invisible layer-shell,
    deferred-init 50ms чтобы сесть на pult DP-1, generation-based debounce)
    + MPRIS-карточка (`mpris_card.rs`: swatch `0x5fd3e8`, title/artist,
    transport + mute→`toggle_stream_mute_for_player`). Стек **C**: div +
    `gpui-animation` `transition_when` fade 180ms на inner body (rsx НЕ
    использован — div читаемее). **Кровный факт соблюдён:** один
    `on_hover` на узел (root=debounce, strip=открытие, анимация на inner —
    `transition_when`, НЕ `transition_on_hover`). Приёмка: units 5/5,
    release green, grim `panel-pinned.png` (карточка на DP-1),
    peek open/close по логу (~280ms debounce). **Живой клик play/pause+
    mute НЕ дожат** (ydotool dual-head врёт) — за пользователем на master-
    бинаре; диспатч зовёт те же API, что живой bar-mpris. Smoke-env
    `CHRONOS_SMOKE_SIDE_PANEL` (env-gated) оставлен как единственный
    пин-триггер до Task 12. Отчёт: `report-log/hermes-report-20.md`.
  - **Tasks 10/11** `1e93209` — spectrum-метры (`spectrum_row.rs`: ring 14,
    CPU/RAM/GPU+сеть, палитра сине-циан, GPU скрыт при `None`, сеть render-
    иммунна) + power-row (`power_row.rs`: arm/confirm 3с через `cx.listener`,
    switch user disabled, timeout `match`/`warn!`) + **geometry панели/strip
    под бар** (`TOP|RIGHT`, `exclusive_zone: None`, высота `display−2×
    BAR_HEIGHT` = симметричный зазор; требование «не перекрывать бар» +
    паттерн скилла `gpui-layer-shell`). Приёмка: units 11/11, release green,
    grim метров под нагрузкой (load-1/2 двигаются) + power-row. Geometry-
    правку T7/T8 Hermes подал скрыто — верна, но нашёл грепом (замечание в
    HERMES.md). **Живой клик arm/confirm+play/pause+mute + hover-peek round-
    trip с новой geometry НЕ дожаты** (ydotool dual-head) — за пользователем.
    Отчёт: `report-log/hermes-report-21.md`.

**ТЕЛО ПАНЕЛИ v1 ГОТОВО (T7-T11).** Живые клик-конфирмы (play/pause, mute,
arm/confirm, hover-peek) копятся на пользователя — ydotool на dual-head не годен.
Правка `b120a3d`: панель/strip до низа дисплея (только верхний зазор под бар) +
power-row 4 ровные плитки — **бейзлайн, на котором строится v2.**

**⚡ РАЗВОРОТ — SIDEBAR V2 (2026-07-21).** Пользователь нарисовал мокап
`.chronos-ops/design/System Sidebar.dc.html` и решил: **пересобрать панель пиксель-в-пиксель
на `gpui-rsx`** (флагман-тест «мокап→rsx»). Это v2 всей панели, шире текущей
(352px, 24 бара, скроллируемая середина, header + пермишн-карта + media + метры +
сеть + диски + footer со всеми). **Споры отложены («там решим»):** палитра — мокап
несёт Catppuccin (жёлтый GPU и т.д.), это разворот «сине-циан/без радуги»
(DECISIONS 2026-07-20); Claude-Code-пермишн-карта — фича или филлер; маппинг на
Theme-токены. Сейчас — хексы прямо из мокапа.
  - **Трек 1 ✅ ПРИНЯТ** `7109860` (Hermes №22) — sidebar v2 пиксель-по-мокапу.
    Секции: header(kitty+X)/permission(Allow-Deny статик)/media 16:9(арт+прогресс+
    таймкод статик)/метры 24(Catppuccin, GPU жёлтый)/сеть/**Disk+USB монтировать-
    размонт-извлечь (батарея убрана)**/footer(часы+power-грид, Power красный).
    Живое: метры/сеть/часы/транспорт/power-arm. Мокап `65b9719`. Units 12, grim
    пиксель-близко. **rsx-вердикт:** годен для статик-chrome (header/permission на
    `rsx!`), динамика (метры/media/power/scroll) — div прагматичнее; rsx НЕ
    большинство LOC. Живые клики не дожаты (ydotool). Отчёт: `report-log/
    hermes-report-22.md`.
  - **Трек 2 (udisks2) ✅ ПРИНЯТ** `8c8ccb7` (Zed). `services/udisks/`
    `DisksSubscriber` (zbus, poll 2.5s), живая секция: usage-карточка на каждый
    FS-девайс, internal — только полоса, removable — mount/unmount/eject.
    Live vs `lsblk`: nvme internal без кнопок, Ventoy+VTOYEFI removable с
    кнопками; `/boot`/zram/swap скрыты. Mount/Unmount проверен busctl.
    Eject = `Drive.Eject` если ejectable иначе `Drive.PowerOff`. **⚠ Известное
    поведение (ops):** «извлечь» на любой партиции многопартиционного USB зовёт
    `PowerOff` ВСЕГО drive (клик на VTOYEFI-карточке = poweroff всего sdb) —
    семантически «извлечь диск», но per-partition карточки могут ввести в
    заблуждение. Usage через sysinfo (не statvfs — `unsafe_code=deny`). Отчёт:
    `report-log/zed-report-7.md`.
  - **Трек 3 (MPRIS art/progress) ✅ ПРИНЯТ** `3d9b8b3` (Grok). `MprisState`
    += `art_url`/`position_us`/`length_us`; медиа-карточка рисует живую обложку
    (`file://` через `img`+ObjectFit::Cover), прогресс `#007acc`, таймкод `-M:SS`.
    Живой смок (mock chronos_art): арт заполняет кадр, позиция ползёт/стоп на
    паузе. http(s)-арт → плейсхолдер (сетевая загрузка — отдельно). Отчёт:
    `report-log/grok-report-20.md`.
  - **Медиа-видео ЗАКРЫТО на cover→idle** (2026-07-21). Решено НЕ делать
    видео-зеркало: «видео из браузера» технически = только pipewire screen-
    capture (ни gpui-video-player/gpui_web/oxidize/Tauri-webview его не дают —
    они играют файл/URL заново, не зеркалят вкладку); для карточки 352px это
    дорого и малополезно. Карточка = **обложка → idle-анимация «если ничего»**
    (idle рисуем сами, наш стек — отдельный мелкий трек). Скауты (crepuscularity
    MPL, gpui-video-player GStreamer, oxidize-html-gpui live-HTML) — на заметку,
    не в работе.
  - **Task 12** (бар-триггер) — по-прежнему открыт, закрывает интеграцию.
  - **gpui-rsx оправдывается ЭТИМ:** до v2 был вендорен но не юзан (0 вызовов);
    решение пользователя — не выпиливать, а собрать на нём реальное и судить по
    факту (мокап→rsx = его дизайн-пайплайн `.dc.html`).
Ключевые решения:
MPRIS v1 без прогресс-бара; switch user disabled-стаб; log out =
`hyprctl dispatch exit`; без Esc; палитра метров сине-циан.

**2. Вендор-волна крейтов + `gpui-component` recon (2026-07-21).**
  - **✅ `gpui-animation`** — Source `66cd816` (Grok).
  - **✅ `gpui-rsx`** — Source `99cab5e` (Cline).
  - **✅ `ccf-gpui-widgets` recon** — вендор ОТЛОЖЕН, 58 дельт (Hermes).
  - **🔑 `gpui-component` (Longbridge) — recon+пилот СДЕЛАНЫ, РЕШЕНО НЕ
    БРАТЬ сейчас (вариант C).** Компилится об наш форк с 0 ошибок (recon
    №18), пилот №19 доказал проводку (path+`[patch]`, один gpui, Button
    рендерится на layer-shell). **Реальная цена (from-scratch замер
    Архитектора): +2.66 MiB / +13.2% бинаря** (Hermes рапортовал +0.68 —
    НЕ воспроизвелось, занижено ~вчетверо; см. DECISIONS 2026-07-21).
    Тело панели рисуем сами на `gpui-rsx`+`gpui-animation` (время/баги/
    лёгкость + апгрейдим потом по рецепту). **Рецепт апгрейда живёт на
    ветке `pilot/gpui-component-spike @20ee13a`** (не удалять — семя).
    Отчёты: `report-log/hermes-report-18.md`, `hermes-report-19.md`.

**3. Working tree — чисто по капстоуну.** T1/T2/T6/T7 закоммичены;
некапстоунный шум остаётся (skills/*, launcher docs move, _ds/) — не
мешать в код-коммиты.

---

**Обновлено: 2026-07-20 ночь-2 (бар-редизайн своими руками — сделано).**

### Бар-редизайн 2026-07-20 (Архитектор сам, по разовому мандату пользователя)

Пользователь единоразово снял запрет «архитектор не кодит» на остаток
сессии — бар доведён до мокапа руками Архитектора. Все с live grim-смоком:

- **`3e04264` Hermes №15 (принят)** — токен-фундамент: status.* →
  Catppuccin (правка в `theme/mod.rs` — там реальный источник, НЕ
  DEFAULT_BASE16; base16 синхронизирован на будущее), `font_mono:
  &'static str` («JetBrains Mono», SharedString нельзя — Theme derive Copy),
  BAR_HEIGHT 30, бар на `bg.tertiary`. Отчёт: `report-log/hermes-report-15.md`.
- **`c7ccc02` лэйаут** — часы → правый край (Right, регистрируются
  последними), MPRIS левее CAVA в центре, `widgets/separator.rs`
  (1×14 `bg.elevated`, регистрируется В ПОЗИЦИИ — порядок в
  `widgets/mod.rs::register_builtin` = порядок рендера!), паддинг бара
  10px, border-b `bg.elevated`, гапы Left 12 / Right 4.
- **`f370618` SVG-иконки** — НОВАЯ ИНФРА: `crates/app/src/assets.rs`
  (AssetSource, include_bytes-макрос) + `application().with_assets(...)`
  в main.rs + `crates/app/assets/icons/*.svg` (8 шт, line-art, тонируются
  text_color как альфа-маска). Эмодзи выпилены: Пуск = hexagon-sigil
  accent, system = hexagon-core accent, volume = 4 speaker-бакета
  (пути в `volume_icon()`, тесты обновлены), updates = arrow-up,
  bell = bell.svg. Network оставлен на Nerd-глифах (норм line-стиль).
  Новые иконки класть в `assets/icons/` + строку в макрос `icons!`.

- **`6061736` project switcher (№9 закрыт Архитектором, НЕ Mimo)** —
  пилюля `📁 имя ветка ⌄` в правом кластере + `projects.toml`
  (паттерн dock.toml) + попап (lifecycle updates_popup) + «+ Добавить
  проект» через XDG portal (`ashpd`). Ветка — прямой парс `.git/HEAD`
  на 1s-тикере бара (30 байт, без сабпроцессов/inotify; воркткри и
  detached обработаны). Live: пилюля показывает ветку, смена ветки
  подхвачена без рестарта (`smoke-№9`). **ВАЖНО ashpd:** форк gpui уже
  тянет ashpd с `async-io` — фича `tokio` конфликтует на унификации;
  портал крутится на `async_io::block_on` в отдельном std-треде,
  результат через tokio oneshot (просто future). `file://`-URI
  декодируется вручную (`file_uri_to_path`). Бонус в том же коммите:
  MPRIS скрывает idle-плеер без метаданных («▶ Unknown» убит).
  ~~НЕ дожато живьём: клик по пилюле → попап → портал-пикер~~ —
  **ЗАКРЫТО 2026-07-20**: пользователь кликнул лично — пилюля работает,
  проект добавлен через портал, переключение проектов живое. №9 принят
  ПОЛНОСТЬЮ, непокрытых мест нет.

**Хвосты бара:** battery-иконка (эмодзи, но на десктопе скрыта —
низкий приоритет), tray/popups ещё на старой палитре местами
(попапы сидят на `bg.elevated`, docs/STYLE.md хочет `bg.primary` — свип
отдельной задачей).

**Иконная система экосистемы:** Claude Design выдал полный комплект
(`~/projects/chronos-ecosystem/Art/*-sigil-{ornate,icon}.svg`, 5 приложений
+ skeleton-темплейт). Ornate принят; циан заменён на `#5fd3e8` (sed, как
старый арт). Дистилляты на 32px схлопываются — итерационный промпт выдан
пользователю (core-глиф 45-55% холста, кольцо убрать). FM-иконка годна
для обкатки уже сейчас.

**Ниже — история.**
Всё датированное 2026-07-17/18 и «Приёмка-разведка» ниже — ИСТОРИЯ.
Актуальное поле — **этот блок** + «ВОЛНА Top Bar» (решения) + field
rules внизу файла. `git log` — истина по коммитам.

### Git (2026-07-19 ночь)

| | |
|---|---|
| **origin/master** | `897c084` (запушено: Hermes №13 `8d74583` + design `9119edd` + doc-sync) |
| **local master HEAD** | `7c8e2fd` — **ahead 12**, не пушено без отдельного добро |
| ahead | …→ `f7de445` Zed system popup → `a3d36ba` Grok №14 MPRIS → `0a99a67` Mimo №10 consolidation → уборка `8766c31`/`9c86a2c`/`7c8e2fd` (+ docs) |

Ранее запушено и живо: tray_menu `67ca90a`, launcher no-focus-loss
`fba8697`, updates clip `67f7d10`, power-profiles `2522018`, volume
picker `66d66c3`, dock.toml `8929f12`, notif clip `af4e348`, cava
`c519e2e`+`eb043fd`. Лендинг: dark-ohm.github.io/ChronOS.

### Принято локально (не на origin) — 2026-07-19 вечер/ночь

- **`8457bbc` Cline №11** — workspace-точки (7px, accent/disabled,
  `FocusWorkspace`). Зона только `workspaces.rs`. Live grim: точки +
  активная синяя. Отчёт: `docs/orchestration/report-log/cline-report-11.md`.
- **`07df942` Mimo №8** — dock → `bar/widgets/dock.rs` + «Пуск»→
  `launcher::toggle`; оконный dock lifecycle снят; cache
  `config::cached`; context_menu `Anchor::TOP`. Live: **нет**
  `namespace: dock` в layers; ⏻ + pinned в левом кластере. Оговорки:
  `DockConfigSignal` без watch в bar (lag ~1s на unpin); menu по
  центру, не под иконкой. Отчёт:
  `docs/orchestration/report-log/mimo-report-8.md`.
- **`f4ddd72` Hermes №14** — history ring 100 + unread +
  `MarkAllRead`; `render_notification_card`; `history_popup/`
  lifecycle; bell+числовой бейдж. Live: 3×`notify-send` → grim
  `🔔 3`. Open попапа ydotool-ом не дожат — MarkAllRead unit-тесты.
  Отчёт: `docs/orchestration/report-log/hermes-report-14.md`.
- **`f7de445` Zed №2/№3 System popup** — brightness (ddcutil, оба
  дисплея, soft-fail) + power-profile 3-сегмент (UPower) + gaming mode
  (hyprctl eval). Приёмка: хирургический перенос Phase 2 из worktree
  `ChronOS-zed2` (3 файла `system_popup/*`), сборка release зелёная,
  код сверен с отчётом (detach/thread+oneshot/repaint/window.display
  все на месте). Живой смок Zed'а с внешней верификацией (ddcutil/
  powerprofilesctl/hyprctl getoption) по всем 5 элементам — прошёл.
  **Финальный клик-конфирм на master-бинаре за пользователем** (ydotool
  ненадёжен). **Известный хвост:** попап открывается на первом дисплее,
  не на кликнутом (`window.display()==None`) — чинится консолидацией,
  не блокер. Отчёт: `docs/orchestration/report-log/zed-report-2.md`.
- **`a3d36ba` Grok №14 MPRIS multi-player** — список плееров +
  sticky-выбор + `CyclePlayer` + scroll-цикл + `‹i/n›`. Живьём grim:
  `‹1/3›`, kill active → `‹1/2›`. Scroll-cycle только unit (ydotool
  wheel-лимит). Отчёт: `report-log/grok-report-14.md`.
- **`0a99a67` Mimo №10 consolidation** — chrome на ОДИН пультовый
  монитор: `monitor.rs` (`pult_display` по uuid из
  `~/.config/chronos/monitor.toml`, fallback самый большой +
  авто-designation), бар только на пультовом, 8 попапов+launcher+
  system_popup на `pult_display` (desktop_terminal не тронут). Живьём:
  бар только DP-1, launcher/попап DP-1, monitor.toml авто-создан
  (uuid `09e7b298…`). Дисплейный хвост system popup закрыт. Отчёт:
  `report-log/mimo-report-10.md`.
- **Уборка `8766c31`/`9c86a2c`/`7c8e2fd`** — battery `background_spawn`
  `.detach()` (был `let _=`, кровный факт); rustfmt-дрейф 6 файлов +
  фикс exec doc-коммента; .chronos-ops/checkpoint/MEMORY.md синхрон. Рабочее дерево ЧИСТОЕ.
- **Уже на origin:** Hermes №13 visual parity `8d74583`; design-волна
  `9119edd` (`.chronos-ops/design/*.dc.html` в репо — user explicit; Light C
  принят; System Popup мокап принят).

Активные отчёты после приёмки **переносятся** в
`docs/orchestration/report-log/` (не копиями-дублями — active
`docs/orchestration/reports/` держать пустым/только WIP).

### Открыто прямо сейчас

- **Разведка компиляцией роздана (2026-07-20):** Cline №2 — `gpui-rsx`
  (JSX-подобный макрос, зависит только от syn/quote/proc-macro2, не от
  gpui; конфликт со docs/STYLE.md только через `class="bg-*"` — решение:
  использовать ТОЛЬКО individual attributes `bg={theme...}`). Grok №18 —
  `gpui-animation` (state-driven transitions поверх уже вшитого в форк
  `EasingCurve`; `cx.spawn().detach()` — наш executor, не третий
  рантайм). Оба — тот же протокол, что дал результат по `gpui-form`
  (Cline №1, ПРИНЯТ, независимо перепроверено Архитектором).

- **`gpui-form` разведка — ЗАКРЫТА, ПРИНЯТА.** Zed не смог (сломанный
  терминал, честно), передано Cline №1 без изменения брифа — **ответ
  получен и перепроверен Архитектором независимо: компилируется против
  нашего форка без единой правки их кода** (только `[patch]`-секции в
  манифесте потребителя). Ядро + виджет-обёртки + derive-генерация —
  всё подтверждено `cargo check`, единственный `gpui v0.2.2` в графе
  (`cargo tree`). Продуктового кейса сегодня нет — записано в
  `docs/roadmap.md` как открытый путь, не задача.
- **Регресс поймал и откатил Архитектор (2026-07-20):** параллельно Zed
  сдал отчёт `zed-report-2.md` (коллизия имён с уже принятым архивным
  отчётом от 19 июля) — его WIP в рабочем дереве заменял
  `crate::monitor::pult_display(cx)` (единая точка выбора chrome-монитора,
  принята Mimo №10 консолидацией) обратно на `window.display(cx)` —
  паттерн, который САМ ЖЕ Zed задокументировал как возвращающий `None`
  для layer-shell окон (кровный факт ниже). Правка была незакоммичена —
  `git checkout` отменил её до истории. `system_popup` остаётся на
  `pult_display`, как было принято. Причина: Zed работал со СТАРЫМ
  контекстом (продолжал ветку Phase 1→Phase 2 расследования дисплея,
  начатую ДО консолидации, не видел, что она уже решена по-другому).
 разведка совместимости
  `github.com/stayhydated/gpui-form` (типизированные формы, derive)
  с нашим форком. Ядро библиотеки (`-core/-derive/-schema/-runtime`)
  зависит ТОЛЬКО от `gpui` (не `gpui-component`), API-поверхность узкая
  (`Context/Entity/Window/IntoElement`), версия совпадает (0.2.2), но их
  `gpui` — чужой git-форк (`stayhydated/zed`), не наш path-локальный.
  Совместимость НЕ проверена компиляцией — задача Zed'а: клонировать в
  /tmp, подменить зависимость на `path = ../Source/gpui`, `cargo check`
  снизу вверх, отчёт по каждому крейту. Продуктового применения пока
  НЕТ придуманного — чистая разведка "на будущее".

- **ВОЛНА «разведка форка» роздана 2026-07-20 (4 агента, READ-ONLY по
  `../Source/`).** Повод: ложный «кровный факт» про скролл прожил сутки и
  разошёлся в 6 доков — мы не знаем собственный форк. Цель — скилл
  `skills/chronos-gpui/` с проверяемой правдой (каждое утверждение =
  file:line или пример). Скелет + `SKILL.md` написаны Архитектором,
  агенты пишут ТОЛЬКО свои файлы (общий `SKILL.md` не трогают):
  - **Grok №17** → `references/elements-styling-layout.md` — трейты
    элементов, скролл целиком, Style, списки, текст.
  - **Hermes №17** → `references/windowing-platform.md` — layer-shell,
    resize, дисплеи, фокус, жизненный цикл окна, ввод. Самая ценная зона.
  - **`f4d2ebc` Grok №17 — ПРИНЯТ.** elements/styling/layout/scroll.
    Ключевая находка (перепроверена лично): `FollowMode::Tail`
    (`list.rs:113`) + `set_follow_mode` (`:617`) — готовый механизм
    автопрокрутки к хвосту, пригодится для scrollback будущего терминала.
  - **`f7099e5` Hermes №17 — ПРИНЯТ.** windowing/platform/layer-shell —
    самая ценная зона волны, ноль дефектов после его же самопроверки.
  - **OpenCode №4 — ПРИНЯТ**, коммит `cbfc197`. Каталог 55+ примеров +
    `scripts/run-example.sh`. Хвост: 29/42 примеров не прогнаны
    напрямую через `cargo check` (риск низкий, задекларирован).
  - **`9c2c090` DeepSeek №2 (бывшее №14) — ПРИНЯТ. Волна разведки форка
    ЗАКРЫТА ПОЛНОСТЬЮ (4/4 зон).** state/async/executors: глобалы (панике
    `global_mut` до `set_global` — наш сегодняшний cold-start баг),
    реентерабельность `update_window_id` (`app.rs:1728-1781`, точный
    механизм ghost-window саги), `Task`/`.detach()`, наблюдатели,
    таймеры. **Ценная находка (перепроверена лично):** easing-кривые
    из Kael, которые .chronos-ops/checkpoint/REJECTED.md 2026-07-16 планировал портировать
    отдельной задачей, — УЖЕ в форке (`gpui/src/easing.rs`, 658 строк,
    явная SPDX-атрибуция Kael). Планы на порт можно закрыть — уже
    сделано. Также подтверждено: `gpui_tokio` не имеет обёртки
    `spawn_blocking` вообще (только `spawn`/`spawn_result`) — наш кровный
    факт «не юзать spawn_blocking вне рантайма» architecturally forced,
    не просто осторожность.

- **Вывод апгрейда (живой репорт 2026-07-20):** «Upgrade all» уходит в
  фон молча. Причина в коде — `aur/mod.rs:318` зовёт `.status()`, что
  наследует stdout шелла; вывод улетает в лог chronos (под `nohup` — в
  никуда), перехватывать нечего. **Решение принято двухуровневое:**
  попап = СТАТУС (последняя строка живьём, хвост 8-10 строк при
  падении), терминал = ПОДРОБНОСТИ (следующая веха, на НАШЕМ
  `desktop_terminal` — PTY+VT100 спайк Grok №11 наконец получит
  продуктовое применение, чужой эмулятор не нужен). Хвост, а не лог,
  потому что во время работы нужен последний вывод, а не чтение простыни.
  (Прежнее обоснование «скролла в форке нет» — ОПРОВЕРГНУТО 2026-07-20,
  см. кровные факты.)
  - Бриф Claude Design написан (`docs/design.md`, `472f99b`) — состояния
    попапа + кадр окна полного вывода, тёмный и светлый (в светлой
    статусы Latte!). **Мокап ещё не пришёл.**
  - Mimo №13 роздан — захват stdout/stderr + хвост в попапе. Ждёт
    мокап для лэйаута, сервисную часть может начинать сразу.
  - Оговорка: `--noconfirm` гасит не всё, yay умеет спросить на
    конфликтах — при одностороннем стриме ответить некому. Это
    аргумент, что терминал в итоге нужен, а не «может быть».

- **Волна добивки бара 2026-07-20 — ПРИНЯТА (3 из 4):**
  - **`1d736da` Grok №15** — 7 попапов на палитру docs/STYLE.md
    (`bg.elevated` как заливка → `bg.primary`, секции `bg.secondary`,
    elevated только hover/бордер). Сверено: 0 hits elevated, коммит
    ровно 7 файлов. Пиксель-проба нотиф-карточки: `#1e1e2e`.
  - **`6723493` Mimo №11** — эмодзи в баре ДОБИТЫ (battery/mpris →
    5 новых SVG + `icons!`), hover на 7 кликабельных виджетах,
    CAVA 2.5/16. Живой смок делал Архитектор (агент не имел доступа
    к сессии).
  - **`0f0ee88` GLM №1** — светлая схема Light C в `light_scheme()`
    (Latte-инверсия удалена) + выбор схемы через `CHRONOS_THEME`
    (механизма выбора схем раньше НЕ БЫЛО — `Theme::init` ставил только
    дефолт). Все хексы сверены с мокапом, «додуманные» помечены честно.
    Эталонный отчёт.
  - **DeepSeek №1 (network) — НЕ ПРИНЯТ**, доработка в поле: виджет
    показывал `↓ 0` при реальных 15 МБ/с (побочный эффект в `render()`,
    см. кровные факты ниже). Код в дереве НЕкоммичен.
- **Архитектор своими руками 2026-07-20 (разовый мандат пользователя):**
  - **`009853f`** — примитив `chronos_ui::on_fill()` (контраст поверх
    заливок), Latte-статусы для светлой схемы, счётчик уведомлений
    ЧИСЛОМ без розовой пилюли (решение пользователя). Живьём: обе схемы,
    grim до/после.
  - **`608b584`** — воркспейс-точки стали динамическими (см. кровные
    факты). Подтверждено пользователем живьём.
  - Светлая тема Light C теперь пригодна к использованию: `CHRONOS_THEME=light`.
    Проверены бар и нотиф-попап; **клик-попапы (volume/system/updates/
    tray/project) в светлой ещё НЕ смотрели** — там возможны тёмные хвосты.
- **`0838446` DeepSeek №1 (доработка) — ПРИНЯТ.** Скорость за секунду
  вместо дельты между вызовами `render`, гейт `SAMPLE_INTERVAL=1s`, кэш,
  время инжектится параметром (иммунитет к частоте проверяется тестом).
  Живьём с независимым замером: 5632 KB/s по procfs ↔ `↓ 5.8M` на баре,
  лампочка зелёная. **Методологическая поправка:** прошлый скрин `↓ 0`
  уликой не был (142 МБ качались за пару секунд, к снимку трафик уже
  стоял) — для смока сети нужна УСТОЙЧИВАЯ нагрузка (`curl --limit-rate`)
  и замер по окну снимка. Диагноз по коду при этом был верен.
- **`7eada8b` Hermes №16 — ПРИНЯТ.** Трей: фильтр безымянных
  (`is_useful`) + дедуп по bus-имени + кап 8 с `+N`. Логика в чистых
  функциях, render без побочек. Живьём: полезный трей НЕ выкошен
  (udiskie отрисован, лог чист); **выкашивание частокола живьём НЕ
  подтверждено** — мусорных chromium-SNI на шине было 0 (Vivaldi
  перезапускался), покрыто только юнит-тестом. Вернётся частокол —
  переоткрываем. Процесс образцовый: worktree-изоляция при чужом
  ломающем WIP, поимённый коммит, честное «смок за Архитектором».
- **`79c8baa` + эррата `b25452c` Mimo №12 — ПРИНЯТ.** `UpgradeState`
  (Idle/Running/Done/Failed) в aur-сервисе, кнопка на время апгрейда
  блокируется (второй `pkexec` не запустить), попап не закрывается по
  клику. **Эррата:** футер прибавил строку статуса, а `FOOTER_BUDGET_H`
  остался 64px и `estimate_popup_height` его не учитывает — запас был
  ~2px у попапа, который на этом уже горел; дубль «Upgrading…»
  (строка + кнопка) убран, запас вернулся. **Правило:** меняешь футер/
  шапку попапа — сверяй `*_BUDGET_H` и `estimate_popup_height`; клип
  списка защищает от длинного СПИСКА, но не от растолстевшего футера.
  Не снято живьём: клик с `pkexec`-диалогом — за пользователем.
- **`3f6e165` Grok №16 — ПРИНЯТ.** Светлая тема: лаунчер получил явные
  text-токены (жил на неявном дефолте GPUI), active-сегмент
  power-profile → `on_fill`. Живьём светлая: бар/нотиф/лаунчер/OSD
  читаемы; клик-попапы — только код-ревью (после №15 они на токенах,
  сырых hex ноль). Побочно нашёл мину в WIP GLM №2 (cold-start падал:
  `Theme::set` мутирует глобал до его установки) — откатил только
  проводку, чужой файл не коммитил; GLM с тех пор вмержил fix
  (`cx.set_global`).
- **`5bb6c77` GLM №2 — ПРИНЯТ.** Схема живёт в
  `~/.config/chronos/theme.toml` + hot-reload через inotify (parent-dir
  watch, дебаунс 300мс, `cx.set_global` + `refresh_windows`). Приоритет
  `CHRONOS_THEME` → конфиг → дефолт. Hot-reload снят Архитектором лично
  по пикселю фона бара: запись Light на ЖИВОМ шелле → `#eceefa` без
  рестарта, откат → `#181825`, удаление конфига → дефолт, env перебивает
  конфиг. **Волна закрыта полностью — в поле никого.**
- **Роздано после приёмки:** Hermes №16 — трей-частокол (фильтр
  безымянных item'ов + дедуп по bus-имени + кап).
- **Светлая тема на попапах не проверена** — GLM смочил только бар.
  При открытии попапов в светлой могут всплыть тёмные хардкоды.

- **Mimo №9** (project switcher) — **разблокирован** (№10 сел, бар
  консолидирован). Готов к раздаче. Пилюля = **имя проекта** + сигил/
  шеврон (не git-ветка). Эталон: `.chronos-ops/design/Project Switcher.dc.html`
  (dark + Light C). Portal FileChooser / `projects.toml`. Бриф в MIMO.md.
- **КАПСТОУН: перерисовка бара** против `Top Bar.dc.html` — делает
  Архитектор ЛИЧНО после №9 (порядок виджетов + visual parity). Аудит
  снят (2 Haiku, 2026-07-20): (Tier 1, механика) высота 32→30, фон бара
  `#181825` (не bg.primary), нижняя граница 1px, вертикальные
  разделители между группами, JetBrains Mono 11–11.5px, CAVA 2.5px/16px.
  (Tier 2, РЕШЕНИЯ пользователя ДО перерисовки) мокап рассинхронен с
  живым набором: не показывает tray/updates/system/battery, добавляет
  up/down + accent-логотип; часы центр→право?; MPRIS куда?; место
  project-switcher; dock-иконки реальные vs стилизованные. Сначала
  реконсиляция набора, потом стиль.
- **Направление chrome→один пультовый монитор — РЕАЛИЗОВАНО** (`0a99a67`,
  `.chronos-ops/checkpoint/REJECTED.md` 2026-07-19). Второй монитор → холст (окна Hyprland +
  desktop-виджеты `plasminal`) — **роль ОТЛОЖЕНА** до готовности
  пультовой части. Пультовый = DP-1 uuid `09e7b298…` в
  `~/.config/chronos/monitor.toml`.
- **Хвост аудита `background_spawn`:** `notifications/view.rs` ×2 всё
  ещё голый `background_spawn` (racy drop=cancel) — не срочно, латентно.
- **Push:** локально ahead 12, не пушено — ждёт добра.
- **Светлая тема** — порт `light_scheme()` под Light C **не начат**
  (`crates/ui/src/theme/schemes.rs` всё ещё Latte-hex). Часть капстоуна
  или отдельно — на усмотрение.

### Кровные факты (System popup / железо, 2026-07-19)

**Яркость — только `ddcutil`, не brightnessctl.**
- `/sys/class/backlight` пуст (десктоп + RTX 3070).
- `i2c-dev` loaded + `/etc/modules-load.d/i2c-dev.conf`; user ∈ `i2c`.
- Display 1 Dell U2412M HDMI `/dev/i2c-2`; Display 2 Samsung LC32G5xT
  DP `/dev/i2c-3` (**primary**). `getvcp`/`setvcp 10` write-smoke OK.
- MVP: один слайдер → оба дисплея; soft-fail без ddcutil/i2c.

**Gaming mode — `hyprctl eval`, НЕ `keyword`.**
На Hyprland 0.55.4 + Lua (`hyprland.lua`):
`hyprctl keyword …` → *«keyword can't work with non-legacy parsers»*.
Рабочий toggle (проверено + restore):
```bash
# ON
hyprctl eval 'hl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false } }, general = { allow_tearing = true } })'
# OFF
hyprctl eval 'hl.config({ animations = { enabled = true }, decoration = { blur = { enabled = true } }, general = { allow_tearing = false } })'
```
Power profile — существующий UPower / power-profiles-daemon.
Hide bar/dock в gaming MVP — **не** (chicken-egg для отладки).

### Working tree hygiene (сейчас)

Помимо uncommitted Zed №2: rustfmt/шум в clock/mpris/network/ipc/osd/
wallpaper_ctl/applications/types — **не коммитить** без осознанного
диффа. `skills/*`, `_ds/` — untracked, не в git.  
**Worktree sibling:** `ChronOS-zed2` — отдельный бинарь; при смоке
проверяй `readlink /proc/$(pgrep -x chronos)/exe` — не перепутать с
master `target/release/chronos`. `pkill -x chronos` (не `-f`).

Полный контекст волны — `.chronos-ops/checkpoint/REJECTED.md` 2026-07-19 «Top Bar redesign
wave», канон — `.chronos-ops/checkpoint/ARCHITECTURE.md` §14, обзор — `docs/roadmap.md`.
При расхождении с .chronos-ops/checkpoint/ARCHITECTURE.md/DECISIONS.log побеждают они.

## Приёмка-разведка 2026-07-19 (сверка отчётов с деревом, БЕЗ билда/смока)

Прочитаны 4 отчёта в `docs/orchestration/reports/`, сверены с деревом грепами/
диффами/git show. Полноценная приёмка (build/test/живой смок) ЕЩЁ НЕ гонялась.

- **Grok №11 (desktop_terminal)** — коммит `b45cd07` на месте: `desktop_terminal/
  mod.rs` (82) + `view.rs` (644), `main.rs +2`, Cargo-deps `portable-pty`/
  `alacritty_terminal`, тест `vt_parser_renders_echo_output` (view.rs:620) есть.
  Файлы/факты отчёта совпали. Осталось: build+test (заявлено 179) + живой смок
  (probe `CHRONOS_DT_PROBE=1`). Спайк, не продукт.
- **Zed №1 (AUR-виджет)** — коммит `0fd2fb9` на месте: `services/src/aur/{mod.rs
  461,types.rs 48}`, `bar/widgets/updates.rs`, `updates_popup/{mod,view}.rs`,
  `lib.rs`/`state.rs` проводка; тесты `upgrade_command_args` (mod.rs:277) +
  `parse_updates_matches_live_pacman_qu_fixture` (mod.rs:390) есть. Совпало.
  Осталось: build+test (заявлено 193) + живой клик по попапу/«Upgrade all»
  (Zed сам не жал — pkexec, требует пользователя). Бейдж `⬆17` он смоком снял.
- **Hermes №10 (tray_menu)** — БЛОКЕР УСТАРЕЛ. Отчёт: дерево красное из-за
  лишней `}` в `launcher/mod.rs:26` (чужой WIP Cline). В ТЕКУЩЕМ дереве этой
  ошибки НЕТ — launcher/mod.rs:19-28 синтаксически чист. `tray_menu/{mod,view}.rs`
  лежат untracked (WIP). Нужно: собрать основное дерево целиком и, если зелёно,
  снять живой смок правого клика (udiskie) + проверку close-фикса (реентерабельный
  `close_this` по паттерну Cline №8). НЕ закоммичен.
- **Cline №9 (debounce)** — код в дереве (`launcher/mod.rs` поля `close_timer`/
  `pending_close`, строки 22/25), НЕкоммичен. По прошлому вердикту НЕ принят
  (дебаунс не чинит focus-loss, а откладывает). Решить: выпилить или переделать.

ВАЖНО: 17 файлов виджетов + `tray_menu/` + `desktop_terminal` правки — это
переплетённый WIP нескольких агентов в рабочем дереве. Перед любым «принять»
— изолированный `git worktree` на нужном коммите, не верить «зелено» из чужого
замеса. Канон при расхождении — .chronos-ops/checkpoint/ARCHITECTURE.md/DECISIONS.log.

## ВОЛНА «Top Bar редизайн» — решения 2026-07-19

> **Статус исполнения (ночь 2026-07-19):** A cava ✅ · B workspace-точки ✅
> (`8457bbc`) · C dock→bar ✅ (`07df942`) · E history/bell ✅ (`f4ddd72`) ·
> D project switcher — следующий (Mimo №9) · System popup — WIP Zed №2.
> Брифы написаны в `docs/orchestration/agents/{CLINE,MIMO,HERMES,ZED}.md`.
> Ниже — **решения и рационализация**, не очередь «ещё не писали брифы».

Пользователь оценил живой прогон против референс-мокапов Claude Design
(`.chronos-ops/design/*.dc.html` — Updates/Volume/Notifications/Top Bar) жёстко:
«мне отвратно на это смотреть». Разбор показал два уровня проблемы:
(1) мелкая визуальная отделка попапов — см. HERMES.md №13 ниже
(бордер/badge/hover, брифинг готов); (2) Top Bar — это НЕ отделка, а
полноценный редизайн с новыми фичами, пользователь подтвердил explicit
(«Полный редизайн + новые фичи»). Решения по каждому куску, ПРИНЯТЫ,
брифы под них ещё предстоит написать (следующая сессия/заход):

- **A. Cava-визуализатор.** Шеллимся в РЕАЛЬНЫЙ `cava` (бинаря сейчас
  НЕТ на машине — установить `cava` из офрепов первым делом в брифе,
  или проверить/попросить пользователя). Raw ascii-output режим,
  парсинг в `Vec<u8>` уровней (24 бара, как в мокапе), `Service`-
  паттерн как везде (Mutable, poll/stream). Новый сервис-крейт-модуль,
  например `crates/services/src/cava/`.
- **B. Workspace-точки.** `bar/widgets/workspaces.rs` — с текстовых
  номеров на кружки (7px, glow на активной, `accent.primary`/
  `text.disabled` для неактивных — см. мокап `Top Bar.dc.html`
  строки 56-59 и js `workspaces` computed). Самый низкий риск во всей
  волне, самодостаточный файл.
- **C. Dock → в бар + кнопка «Пуск».** Подтверждено explicit:
  «это и есть dock — перенести в бар». Отдельное окно дока
  ИСЧЕЗАЕТ, весь функционал (иконки, персистентный `dock.toml`,
  unpin-меню — только что построено Mimo №7) переезжает в левый
  кластер бара как обычный `BarWidget`. Логика `dock/config.rs`
  (load/save/unpin) переиспользуется как есть, реюзать не терять.
  Первая иконка кластера (chronos-glyph, самая левая по мокапу) —
  «Start-кнопка»: клик → `crate::launcher::toggle(cx)` (функция уже
  есть и принята, ничего изобретать не надо, launcher — рабочий
  модуль). Пользовательская аналогия — Kickoff/Start-меню KDE Plasma,
  но ИМПЛЕМЕНТАЦИЯ — чисто наша, GPUI, никакого Qt/KDE (см. память
  «Plasma отвергнута ради Hyprland» — та память про НЕ предлагать
  KDE-стек агентам, не про запрет UX-паттерна «кликабельная кнопка
  пуска», конфликта нет).
- **D. Переключатель проектов вместо git-branch пилюли — идея
  пользователя, лучше моих двух вариантов.** НЕ фикс-путь, НЕ слежка
  за фокусом окна — полноценный **project switcher** как в IDE:
  - Персистентный список `~/.config/chronos/projects.toml`
    (`{name, path}[]`) — тот же паттерн, что `dock.toml` у Mimo №7,
    переиспользовать код/подход 1:1.
  - «+ Add project» → **реальный системный file picker**, подтверждено
    живьём на этой машине: `org.freedesktop.portal.FileChooser`
    доступен (`busctl --user introspect org.freedesktop.portal.Desktop
    /org/freedesktop/portal/desktop` — интерфейс есть, `xdg-desktop-
    portal` + `xdg-desktop-portal-hyprland`/`-gtk` оба живы). Обычный
    zbus-вызов, не экзотика — того же уровня, что уже сделанные
    upower/tray D-Bus proxies.
  - Пилюля в баре кликабельна → попап со списком проектов + активная
    отметка (паттерн `tray_menu`/`updates_popup` — открыть/закрыть,
    клик по пункту делает его активным, персист в `projects.toml`).
  - **Пилюля = имя проекта** (не git-ветка — design drift после
    `.chronos-ops/design/Project Switcher.dc.html` / Light C: сигил + `ChronOS` +
    шеврон). Git-ветка в пилюле **отменена**.
- **E. История уведомлений (bell + badge).** Подтверждено explicit:
  «строим историю» — сейчас в дереве НЕТ вообще никакого концепта
  инбокса (проверено грепом, только эфемерные попапы). Новая фича:
  персистентный (в памяти достаточно на MVP, диск — опция) список
  прошлых уведомлений, бейдж = счётчик непрочитанных на bell-иконке
  бара (мокап: красная точка top-right угол иконки, `#f38ba8`, `1.5px
  border` цвета фона бара — вырезающий эффект), клик по bell → попап-
  история (переиспользуй карточки из `notifications/view.rs`, тот же
  визуальный язык, что и живые попапы — не изобретай новый). «Mark
  all read» / `DismissAll` уже есть как команда, использовать.

**Порядок сборки волны — НЕ параллелить вслепую.** B (workspace-точки)
и E (история уведомлений) независимы от остального — можно раздавать
сразу. C (dock→bar) и D (project switcher) оба претендуют на ЛЕВЫЙ
кластер бара — сериализовать (сначала C, потом D, один и тот же
`bar/mod.rs` иначе поймает shared-file коллизию). A (cava) полностью
независим (свой сервис-крейт, свой bar-widget по центру). Полная
сборка бара (regsitration порядка виджетов в `bar/mod.rs`) — сводит
Архитектор лично после того, как куски по отдельности приняты, не
доверять финальную сборку agent-у с узкой зоной.

Брифы A/B/C/E/System написаны и (кроме D/System) исполнены — см. верхний
блок. D = Mimo №9; System = Zed №2.

## Живая приёмка 2026-07-19 (build+release+смок, все хвосты закрыты)

Полный `cargo build --release -p chronos`, живой прогон, реальные клики
пользователя. Три реальных бага найдены и починены в этом заходе (не
косметика — все три ловились только живым смоком, не билдом/тестами):

1. **Hermes №10 (tray_menu) — ПРИНЯТ, закоммичен (`67ca90a`).** Правый клик
   по трею (udiskie) → DBusMenu popup → клик по пункту/✕ → закрывается
   чисто, `hyprctl layers` пуст после закрытия, лог без `error`/ghost.
   Заодно чинит **битый HEAD**: коммит `ae615a5` (Zed) занёс `mod tray_menu;`
   + `tray_menu::init(cx)` в `main.rs` без самих файлов модуля (untracked) —
   свежий клон не собирался. Теперь main.rs это уже нёс, докоммичены только
   `tray_menu/{mod,view}.rs` + правый клик в `bar/widgets/tray.rs`.
2. **Cline №9 (debounce) — заменён, не «доделан».** 300мс debounce на
   focus-loss не чинит `follow_mouse=1`, только откладывает (см. вердикт
   выше). Решение по паттерну Zed (`updates_popup`): лаунчер **вообще не
   закрывается по потере фокуса** — только Esc / клик по результату /
   повторный хоткей (все три пути уже существовали в базлайне `8adb193`).
   Обсервер теперь только рефокусит input при возврате активности. Живой
   смок: `follow_mouse=1`, увод мыши — окно осталось открытым; Esc/клик —
   закрывают. Коммит `fba8697`.
3. **Zed №1 (AUR-попап) — принят, но с фиксом.** Живой клик по бейджу `⬆24`,
   попап открылся, но **кнопка «Upgrade all» физически отсутствовала на
   экране** при 24 обновлениях — не обрезка, а полное отсутствие в
   видимой области. Причина: `estimate_popup_height` считала высоту окна
   по `count * ROW_H` с непроверенной на глаз константой (`ROW_H=32`),
   без явного клипа списка; реальная высота строки в рендере оказалась
   больше — список съел все 520px, футер вытолкнуло за физическую границу
   окна (не проскроллить, не докликать). **Фикс — структурный, не
   более точная арифметика**: список получил жёсткий `.max_h(px(LIST_MAX_H))
   .overflow_hidden()`, футер лежит вне этого клипа и гарантированно
   помещается в окно независимо от реальной высоты строки. Список
   показывает первые N строк + `"+N more (run checkupdates for the full
   list)"`. Живым скрином (`grim`, дважды) подтверждено: кнопка видна и
   кликабельна.
4. **Побочная находка при живом клике по «Upgrade all»: `hyprpolkitagent`
   не был запущен.** `pkexec` без агента в сессии падает тихо (exit 127,
   без диалога) — не баг ChronOS, дыра в автозапуске Hyprland-сессии
   пользователя. Почищено: `~/.config/hypr/hyprland.lua` — добавлена
   `startPolkitAgent()` в `hl.on("hyprland.start", ...)` и
   `hl.on("config.reloaded", ...)`, по образцу существующего
   `startClipboardDaemon()`. После фикса живьём: клик → диалог пароля →
   `pkexec yay -Syu --noconfirm` реально выполнился → `pacman -Qu` вернул
   0 — апгрейд прошёл по-настоящему. Этот файл вне git (не в ChronOS-репо),
   правка только на этой машине — при новой машине/сессии повторить.
5. **ОТКРЫТЫЙ хвост, НЕ починено:** попап апгрейда не даёт **никакой**
   обратной связи после клика по «Upgrade all» — ни спиннера, ни
   «Upgrading…», ни результата по завершении. Пользователь узнаёт об
   успехе только по внешним признакам (лог/`pacman -Qu`). Для операции,
   держащей privileged pkexec-сессию и меняющей систему — это не
   косметика. Нужно: минимум — блокировка кнопки + текст-статус на время
   выполнения команды, лучше — подписка на `AurCommand`-результат в
   `updates_popup` (сервис уже логирует success/exit-status, просто
   некому это показать в UI).
6. **Grok №11 (desktop_terminal)** — остался спайком, не смочен в этом
   заходе (grim-кроп содержимого невозможен by design, см. отчёт). Принять
   как есть; продуктовое API — отдельная веха.

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

## СИСТЕМНЫЙ БАГ: `window.remove_window()` иногда не убивает окно — ДВЕ причины, обе теперь известны

Три независимых наблюдения за одну ночь (2026-07-17): OSD (исходный
баг, пофикшен soft-hide в f4edb88), tray_menu Autohand (попап
открывается по логу, но исчезает из `hyprctl layers` ~5с спустя),
launcher (два живых `chronos-launcher` разом). Расследование прошло в
два раунда — 2026-07-18 картина полная:

**Причина №1 (Source, ../gpui-форк) — ПОЧИНЕНА.** `remove_window()`
(window.rs:1899) ставит `removed=true` → `trail()` (app.rs:~1739)
роняет `Box<Window>` → `Drop for WaylandWindow`
(gpui_linux/wayland/window.rs:680) слал protocol-destroy БЕЗ
`connection.flush()`, а реальная отписка из `client.state.windows` +
close-колбэк жили в detached async-таске без гарантии тайминга —
гонка с уже запланированным кадром/commit на тот же surface. Grok
(задание №6, GROK.md) исправил: sync `drop_window()` + sync `flush()`
прямо в Drop, `close()` остаётся deferred (реентерабельность в App
не позволяет иначе). **Source-коммит `3800d3a`, в master, ПРИНЯТ.**
Живой смок Архитектора: 15 циклов IPC-toggle — residual=0, `Drop
WaylandWindow`/`flush after destroy ok` в логе каждый раз. Реально
чинит гонку в путях, где `remove_window()` ДЕЙСТВИТЕЛЬНО вызывается.

**Причина №2 (ChronOS app-уровень) — ДОМИНИРУЮЩАЯ, найдена 2026-07-18,
в поле.** `App::update_window_id` держит слот `cx.windows[id]` пустым
на время выполнения колбэка — повторный вызов НА ТОТ ЖЕ id ИЗНУТРИ
этого колбэка молча вернёт `Err("window not found")`. Паттерн, который
на это напарывается: колбэк уже получил `window: &mut Window`
(например `observe_window_activation`, сам исполняется изнутри
`handle.update` — `Source/gpui/src/window.rs:1589-1608`), но вместо
использования этой ссылки напрямую зовёт функцию закрытия, которая
делает ЕЩЁ ОДИН `handle.update(cx, |...| window.remove_window())` на
тот же id — реентерабельно, `Err`, и если вызывающий код глотает
результат через `let _ =` (как везде в проекте) — `remove_window()`
**вообще не исполняется**, хотя лог «closing»/«removing window» уже
написан. Global-хэндл при этом чистится раньше (не зависит от исхода
update), поэтому следующий `open()`/`toggle()` создаёт НОВОЕ окно
поверх старого ghost'а — отсюда «два живых окна разом».

Живьём подтверждено ТОЛЬКО для launcher (Архитектор, 2026-07-18):
открыл через IPC-toggle, дал окну потерять фокус САМОСТОЯТЕЛЬНО (не
клик) — `close called`/`removing window` в логе, но `hyprctl clients`
показывал `mapped:true` ещё 6+ секунд, диагностика Грока
(`Drop WaylandWindow` и т.д.) не появилась НИ РАЗУ. Корень —
`launcher::close_this()` (mod.rs:155) получает `window: &mut Window`
готовым, но при `tracked=true` игнорирует его и зовёт `close(cx)`
(mod.rs:139-146), который делает реентерабельный `handle.update`.
Фикс роздан — **CLINE.md задание №8**.

Тот же антипаттерн СТРУКТУРНО присутствует в `tray_menu::click_item`
(`tray_menu/view.rs:178` — `on_click` получает `_window` и игнорирует
его) — НЕ подтверждено живьём для этого модуля, но крайне вероятно
объясняет наблюдение Autohand («попап гаснет с `window not found`
~5с спустя»). Задача обновлена в **AUTOHAND.md** — приоритет: код-фикс
по аналогии с Cline №8, а не ещё один ydotool-ретест.

**Правило на будущее для ЛЮБОГО нового окна/попапа:** если у колбэка
уже есть `window: &mut Window` в сигнатуре (наблюдатель активации,
`on_click`, любой window-scoped хук) — закрывать нужно ЭТОЙ ссылкой
(`window.remove_window()` напрямую), НЕ через повторный
`handle.update(cx, ...)`/`AnyWindowHandle::update`. Последний — только
для путей СНАРУЖИ текущего window-колбэка (IPC, таймеры/`cx.spawn`).
`let _ = handle.update(...)` — ЗАПАХ, грепай новый код на это перед
приёмкой.

## Стэши Grok (tmp-foreign-wip-*) — почти разрулены

- `stash@{0}`: mpsc-код Mimo — УЖЕ переписан начисто (acad3b3), tray
  types OpenCode — перекрыт его коммитом 6782337. Можно дропать после
  беглой сверки `git stash show -p`.
- `stash@{1}`: live-интеграционные тесты network/upower Hermes — НЕ
  закоммичены нигде, единственная копия. Прежде чем дропать — решить,
  нужны ли (кандидат: отдать Hermes отдельным заданием).

## История: волна №3 (2026-07-17 вечер) — ЗАКРЫТО, не текущее поле

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

## Пользовательский бэклог — статус на 2026-07-19

Живой фидбек с 2026-07-17, три из четырёх пунктов ЗАКРЫТЫ сегодня:
- ~~Ползунки громкости/микрофона~~ ✅ volume-попап с пикером устройств
  (Grok №12/№13, коммит `66d66c3`).
- ~~Режимы производительности~~ ✅ реально подключены к
  power-profiles-daemon (Cline №10, коммит `2522018`) — UI-точка входа
  пока невидима на десктопах без батареи, чинится System-popup'ом
  (см. верхний блок файла).
- ~~Клик по трею~~ ✅ подтверждено пользователем живьём — правый клик
  устраивает, вопрос закрыт.
- Слайдер яркости дисплея — ЕЩЁ ОТКРЫТО, backlight-сервиса нет
  вообще, теперь часть System-popup плана (яркость там же, где
  power-profile).

«Ещё дохуя идей, но база не достроена» (2026-07-17) — пользователь
копит список, не гнать вперёд паровоза, спрашивать по мере готовности
базы. Актуально по-прежнему.

## Очередь (2026-07-19, актуальная — полная картина в `docs/roadmap.md`)

Всё нумерованное здесь раньше (Cline №7-9, Autohand №3, Grok №6,
Hermes №9/11) — ЗАКРЫТО или заменено сегодняшней волной, см. верхний
блок файла. Реально открытое, что НЕ входит в брифы
Top Bar-волны (те — в верхнем блоке + `docs/orchestration/agents/`):

1. **`stash@{1}` (`tmp-foreign-wip-for-grok-verify`)** — живые
   интеграционные тесты network/upower Hermes, единственная копия,
   нигде не закоммичены. `stash@{0}` можно дропать (превзойдён), это
   — нет. Решить: отдать Hermes отдельным заданием или разобрать
   вручную.
2. gradient borders (порт из `Source/`, Kael-фундамент уже принят) —
   не роздано, ждёт свободного агента после Top Bar-волны.
3. MPRIS — переключение между несколькими плеерами (сейчас всегда
   первый `Playing`) — не начато.
4. Пересчёт `status.*` цветов темы под канонiчный Catppuccin (сейчас
   смесь Catppuccin+Tailwind-суррогатов, всплыло при разборе urgency-
   цветов notifications) — не экстренно, не начато.

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

- **Композиторные события: подписка на «сменился» ≠ подписка на «список
  изменился»** (найдено пользователем живьём 2026-07-20, починено
  `608b584`). `add_workspace_changed_handler` только переставлял флаг
  `active` по списку, снятому ОДИН РАЗ на старте шелла — событий
  `createworkspacev2`/`destroyworkspacev2` не слушал никто. Следствия:
  созданные позже воркспейсы не появлялись точками, опустевшие не
  исчезали, а при переходе НА созданный после старта воркспейс активной
  не подсвечивалась НИ ОДНА точка (его id просто не было в списке).
  Починка — `refresh_workspaces()` перечитывает список на всех трёх
  событиях. **Имена хендлеров в крейте `hyprland` генерируются макросом
  `events!` из вариантов enum** (`WorkspaceAdded` →
  `add_workspace_added_handler`), поэтому грепом по исходникам крейта
  они НЕ находятся — сверяйся со списком вариантов в
  `event_listener/shared.rs`, а не с грепом `pub fn add_`.
- **Контент поверх заливки — через `chronos_ui::on_fill()`**, не через
  `theme.text.*` (см. docs/STYLE.md). Токены текста переворачиваются вместе
  со схемой, заливки — нет. **`status.*` РАЗНЫЕ у схем:** тёмная —
  Catppuccin Mocha, светлая — Latte; пастельный Mocha как ЦВЕТ ТЕКСТА
  на светлом фоне нечитаем (живьём: `↑ 19` бледно-жёлтым на светлом
  баре).

- **НИКАКИХ побочных эффектов в `render()` виджета** (найдено живым
  смоком 2026-07-20, DeepSeek №1). `render()` зовётся МНОГОКРАТНО за
  один кадр (замер/лэйаут/пейнт) и вдобавок на каждый сервисный сигнал
  (`bar/mod.rs` watch'ит cava, а cava шлёт 30 fps) — то есть сотни раз
  в секунду, а не «раз в тик по часам». Виджет сети считал дельту
  трафика МЕЖДУ ВЫЗОВАМИ `render()` и перезаписывал в нём базовый
  снапшот: соседние вызовы отстоят на микросекунды → дельта ровно 0 →
  при реальных **15 МБ/с виджет показывал `↓ 0` и серую лампочку**
  (замер счётчиков параллельно скрину: 142 МБ за 9 с). Юнит-тесты были
  зелёные — они кормили синтетические дельты прямо в чистую функцию.
  **Правило:** любое накопление/семплирование в виджете — с ГЕЙТОМ ПО
  ВРЕМЕНИ (обновлять базу только если прошло ≥1 с) + кэш показанного
  значения между обновлениями; считать СКОРОСТЬ (`bytes /
  elapsed.as_secs_f64()`), а не дельту за неизвестный интервал. Тогда
  виджету всё равно, зовут его 1 раз в секунду или 300 раз за кадр.
- **Трей забивается безымянными item'ами** (живьём 2026-07-20). Vivaldi
  (Chromium) регистрирует новый `StatusNotifierItem` и не снимает
  регистрацию, пока жив — за сессию накопилось 13 штук, все с
  `icon=None` и пустым `title`, рисуются одинаковым дефолтным глифом и
  съедают правый кластер («частокол микрофонов»). Наш `remove_item`
  чистит только когда с шины пропадает ВСЁ bus-имя, а оно живо. Проверка:
  `busctl --user call org.kde.StatusNotifierWatcher /StatusNotifierWatcher
  org.freedesktop.DBus.Properties Get ss org.kde.StatusNotifierWatcher
  RegisteredStatusNotifierItems`. Лечим на своей стороне (фильтр
  безымянных + дедуп по bus-имени + кап) — Hermes №16.
- **`cx.background_spawn(...)` БЕЗ `.detach()` — баг, не стиль**
  (Zed №3, 2026-07-19). `gpui_scheduler::Task` — `#[must_use]`, **drop =
  cancel**. Голый `cx.background_spawn(async {...});` или `let _ =
  cx.background_spawn(...)` роняет Task сразу → async-тело **отменяется
  racily**: быстрые футуры (in-proc channel) проскакивают до отмены,
  медленные (zbus/hyprctl/subprocess) — дохнут молча. Симптом: `on_click`
  логируется, async-эффект не наступает. **Всегда `.detach()`** (или
  держи Task живым). В дереве на 2026-07-19 — 6/6 мест без detach:
  `battery.rs:87` (скрыт — пустой div на десктопе, клик не исполняется),
  `notifications/view.rs:56,166` (racy, «работает» на быстром dispatch),
  system_popup ×3 (Zed чинит). **Follow-up:** аудит battery+notifications
  на detach — латентный, не экстренный. Родня правилу `let _ =
  handle.update()` ниже.
- **`tokio::task::spawn_blocking` вне tokio-runtime виснет** (Zed №3).
  `cx.background_spawn` = GPUI executor, НЕ tokio. `spawn_blocking` там
  не паникует — просто не завершается. Для subprocess из GPUI-таски:
  `std::thread::spawn` + `tokio::sync::oneshot` мост, не `spawn_blocking`.
  (zbus работает — сам спавнит runtime.)
- **`window.display(cx)` == None для layer-shell окон** (форк, Zed №3).
  `self.display_id` берётся из `platform_window.display()` (`Source/
  gpui/src/window.rs:2293`), wayland-backend его для layer-shell не
  заполняет. Следствие: «попап на дисплее кликнутого окна» через
  `window.display()` НЕ работает. НО `display_id` в `WindowOptions`
  честится на `open_window` (доказано). Роутим через конфиг пультового
  монитора (consolidation), не через inheritance. Grok НЕ нужен.
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

### ACP live smoke 2026-07-27 (день, архитектор) — приёмка захода 1 T143

Три прогона релиза, `RUST_LOG` до `info,chronos_services=debug,hermes.stderr=debug`,
скриншоты `grim`. Итог: **D0/D1/D4 работают живьём**, D2 таймаут провален,
D3 не сделан, D5 вынесен, вскрыт новый **D6**.

- **D6 (новый, приоритет №1):** панель теряет завершение turn'а.
  `{"result":{"stopReason":"end_turn"}}` физически приходит по проводу
  (11:18:56.937), а `stream_read_turn` (`client.rs:297-410`) висит в
  `read_update().await` — ни `turn END`, ни ошибки. Первопричина
  утреннего «Hermes пропал»: агент был жив и договорил.
- **D2 таймаут:** при `TURN_TIMEOUT=120s` не сработал ни разу — 1.5 ч,
  158 с, 258 с. Зацепка: таск идёт через `cx.spawn` (GPUI-executor), а
  `tokio::time::timeout` требует токийского рантайма.
- **D2 Cancel:** работает, но маркер «⏹ Turn cancelled» ставится только
  в пустое сообщение — при отмене на полуслове следа не остаётся.
- **D4:** окупился в первый же час — поймал `Traceback` и `402 Payment
  Required` из плагина Hindsight в Hermes (облачный, хотя у нас
  self-hosted на :8888 — отдельная тема). Erratum: стек на `debug`, при
  `RUST_LOG=info` не виден.
- **Заход 2 отдан Hermes** (`docs/orchestration/agents/HERMES.md`).

Дисциплина: отчёт захода 1 содержал два ложных «сделано» (D3, D5) и
описывал несуществующий код — обе лжи вскрыты грепом за минуты.

---

## Чекпоинт #9 — 2026-08-13, вечер. Дерево разгружено, корень DnD найден

**HEAD `93b7a97` (запушен). Source `48b2c1f` (запушен).** Грязного кода в
дереве больше нет — осталась только временная инструментация `t273_frame`
в `side_panel_right/view.rs`, `Cargo.lock` (чужой) и документы в работе.

### Главное: смерть мыши — внешняя причина, drag-out из Chronos-FM

Не popup grab и не hover-strip: обе гипотезы опровергнуты. Корень в форке,
`Source/gpui_linux/src/linux/wayland/client.rs` — диспетчер
`wl_data_source` обрабатывал только `Send` и `Cancelled`, остальное
`_ => {}`, поэтому `DndFinished` проглатывался и `destroy()` звался лишь
при отмене; плюс не вызывался `set_actions()`, обязательный при
`WL_DATA_DEVICE_MANAGER_VERSION = 3`. Композитор держал имплицитный
pointer-grab вечно. Введено `5eeb892` (11 августа) — первая смерть 12-го.

Фикс `18ea90a` + `48b2c1f` (**copy-only**: объявлять `Move` нельзя, пока
FM не удаляет оригинал — иначе «переместил» даёт копию и целый оригинал).
Живой прогон: 7 заходов, ввод жив после каждого. Осталось одно ручное
перетаскивание, доказывающее доставку файла приёмнику (`T270` в `active/`).

### Закоммичено

- `cb7a6c1` — T263 + T264 A/A2: anchor-aware меню трея и дока, палитра
  `gpui-component`, widest-reserve сабменю, popup click-catcher (18 файлов).
- `d2e77fa` — T265-0: скролл выдачи лаунчера.
- `2e01a6d` — эррата T267: рейл рисует разделитель безусловно.
- `93b7a97` — возврат `MIGRATION.md`.

**T263 принят наблюдением владельца, без кадров.** Причина записана в
отчёте: `grim` в этой конфигурации даёт негодные кадры, а submenu
проверить нечем — в шелле НЕТ ни одного меню с вложенными пунктами.
Живая проверка widest-reserve переезжает в T274 (первое подменю).

### Очередь после разгрузки

| Тикет | Состояние |
| --- | --- |
| T270 | ждёт одного ручного драга файла из FM |
| T271 | 132 проглоченных `Result`; зоны `ipc/service.rs` 17, `ipc/mod.rs` 15, `side_panel_left/**` 14 |
| T273 | **уехал в форк**: `PlatformWindow::resize` шлёт `set_geometry` синхронно, буфер спавнится на тик позже — item #8-bis. Измерения: скачок 28px, возврат лестницей 400-600мс, кадры с обоями под панелью (`~/Pictures/t273/`) |
| T274 | виджет проектов: корень сессии агента, контекстное меню, подменю **worktree** (не веток) |
| T275 | лаунчер: `Input` из форка компонентов, frecency, pin, убрать кнопку `tune` |
| T253, T268 | свободны |
| T266 | Luna, воркетри пересоздать от `93b7a97` |

### Уроки дня

1. **Кадр засчитывается по тому, что на нём видно.** Эррата T267: кадры
   покрывали раскрытую панель, свёрнутая осталась без разделителя. Для
   элемента с двумя состояниями нужны кадры обоих.
2. **Внешний аудит врёт правдоподобно.** Первая редакция считала тесты
   вместе с продом (280+ unwrap против реальных 43, 15 `panic!` против 0);
   вторая, после эрраты, воспроизвела итоги но выдумала детализацию —
   три файла из её таблиц в репозитории не существуют. В работу
   конвертировалась одна находка (T271).
3. **`git add -A` не использовать даже на каталоге документов.** Подмёл
   чужое удаление `MIGRATION.md` (117 КБ истории задач). Вернул следующим
   коммитом, но правило теперь без исключений: только поимённо.
4. **Карта дерева устаревает за час.** Поимённый `git add` по карте,
   собранной утром, чуть не оставил click-catcher за бортом коммита —
   он жил в `bar/mod.rs`, `main.rs` и новом `popup_click_catcher.rs`.
   Спасло чтение `git diff --staged` глазами.

**Hindsight на момент чекпоинта недоступен** (`HTTP 000` на :8888) —
этот слой не записан, при подъёме стека продублировать.
## 2026-08-13 — маршрут T273 → T276/T277

- T273 не продолжать как fork-only resize fix: все проверенные кандидаты получили от владельца `-`.
- Принята архитектура двух поверхностей: standalone rail 40 px + fixed-size content canvas с динамическим input region.
- Канонический implementation-ticket:
  `docs/orchestration/tasks/active/T276-standalone-right-rail-and-fixed-content-canvas.md`.
- Канонический audit-ticket:
  `docs/orchestration/tasks/active/T277-audit-standalone-right-panel-surfaces.md`.
- T276 выполняет Sonnet 5; T277 проверяет Qwen 3.8 Max. Lead Architect принимает отчёты, проверяет diff и исправляет/возвращает недоделки. Живой UX принимает владелец знаком `+` или `-`.
- `wf-recorder` для этой приёмки не запускать: 2026-08-13 Hyprland 0.56.1 получил `SIGBUS` в NVIDIA screencopy path (`CGLFramebuffer::readPixels → ScreenshareFrame::copyShm`) сразу после старта второго recorder, до resize-жеста.

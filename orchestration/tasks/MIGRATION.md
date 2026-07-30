# MIGRATION.md — T-ID ledger (per-agent → per-task pivot, 2026-07-22)

Реестр перехода с per-agent журналов (`orchestration/agents/<ИМЯ>.md`) на
per-task учёт (`orchestration/tasks/`). Собран из полного разбора
`HANDOFF.md` + `DECISIONS.log` + `orchestration/agents/archive/*.md` +
`orchestration/agents/bench|fired/*.md` + листинга `orchestration/report-log/`
(две параллельные разведки, 2026-07-22). T-номер = порядковый (хронология),
3 цифры, буквенный суффикс — под-задача/companion-файл того же T.

**Скоуп-решение (явно, не молча):** для строк БЕЗ сохранившегося отдельного
файла отчёта/брифа (до-агентская эпоха, часть агрегатов) НЕ создавались
синтетические файлы-заглушки в `done/`/`rejected/` — эта таблица есть
единственный и достаточный источник записи для них (создавать файл на
одну ссылку в HANDOFF было бы фабрикацией структуры без нового содержания).
Для строк с реальным файлом — файл физически перемещён (`git mv`) с
двух-трёхстрочным заголовком (T-ID/агент/исходный путь) поверх тела,
тело не переписано.

## Спорные случаи — решения (см. план `foamy-swimming-engelbart.md`)

1. **Mimo apps-service «№3» (MIMO.md) vs «№4» (HANDOFF.md)** — один T-ID
   (T017), тело цитирует оба номера, канон = HANDOFF (выше в authority order).
2. **До-архивные агрегаты** (Cline «№1-5», Hermes «№3-6») — по одному T-ID,
   статус `reconstructed-aggregate`, без синтетического файла (см. скоуп выше).
3. **`grok-report-3.md`** — мигрирован (T027) с громким предупреждением в
   шапке: анонимная перезапись, не доверять без сверки.
4. **`zed-report-2-Phase2-DISCARDED-superseded-by-consolidation.md`** →
   `rejected/`, статус discarded.
5. **DeepSeek «№2»/«бывшее №14»** — один T-ID (T082), оба номера в тексте.
6. **Autohand tray-menu-popup vs Hermes tray_menu** — ОДИН T-ID (T020,
   не два, как в первой версии плана) — весь Autohand-lineage (задание +
   продолжение) никогда не был принят, это одна незавершённая ветка, не
   две; кросс-ссылка на T040 (Hermes, реально принято `67ca90a`).
   *Отклонение от буквы плана записано здесь явно.*
7. **Rework/duplicate/copy-варианты** — канонический (финальный) → своё
   T-ID в `report-log/`; черновики → `notes/superseded/TNNN-*.md`.
8. **Некросс-катные не-таск аудиты** → `notes/` без T-ID: `chronos_services_
   rewrite_cost_report.md`, `REWRITE_BY_PATTERN_AUDIT.md`, `SHADER_PORT_
   AUDIT.md`, `kael_motion_port_audit.md`, `philip-report-maintain-2026-07-21.md`,
   `architect-report-philip-maintain.md`, `SESSION_REPORT.md`,
   `Compact summary.md`.
9. **Найдено попутно:** `orchestration/report-log/HANDOFF.md` — 176-строчный
   старый снэпшот корневого HANDOFF.md, обнаружен как stray-копия (отличается
   от текущего, источник появления неизвестен). Не удалён — перенесён в
   `notes/stray-handoff-snapshot.md`, флаг для внимания пользователя.
10. **`opencode-report-3.md`/`-rework1.md`** — оба черновика DBusMenu
    (частичный accept `6782337`, фикс десериализации `1d54ffd`). Финальный
    полный accept (`f755db6`, «DBusMenu-сервис ЗАКРЫТ») **не имеет
    отдельного сохранившегося файла отчёта** — архитектор верифицировал
    напрямую по HANDOFF. `-rework1.md` — канон (ближе всего к финалу),
    `-3.md` (первая попытка) → `notes/superseded/`.
11. **`mimo-report-techstack-REJECTED-wrong-task.md`** — не относится ни к
    одной задаче из каталога (research-репорт не по той задаче) — T-ID вне
    последовательности, `T900` (misassigned research, rejected).
12. **`deepseek-report-2.md`** — `[ambiguous]`: по номеру продолжает
    network-widget rework (T066, commit `0838446`), НО T082 (fork recon
    state/async, `9c2c090`) не имеет вообще никакого surviving файла в
    листинге. Решение: `deepseek-report-2.md` → T066 (последовательная
    нумерация сильнее), T082 остаётся без файла (архитектор верифицировал
    напрямую).
13. **Zed System-popup лениджа** — `zed-report-2.md` (Phase 1 диагностика)
    и `zed-report-3.md` (Phase 2, частичный accept: bug2+gaming починены,
    display-фикс не работает) — оба реальные, не дубликаты. Финальный accept
    фичи целиком (`f7de445`, T055) отдельного файла не имеет — ближайший
    сохранившийся документ = `zed-report-2.md`, помечен так же T055
    (Phase 1), а T055a = `zed-report-3.md` (Phase 2, отдельная companion-
    запись, её находка про `window.display()==None` ушла в T058 consolidation).

---

## T001–T007 — до-агентская эпоха (2026-07-10/11, Архитектор)

Ни одного отдельного файла-брифа/отчёта не пережило переезд репо (см.
`DECISIONS.log` 2026-07-16 "история = git log больше не источник"). Запись —
только эта таблица + `DECISIONS.log` записи с тем же названием.

| T-ID | Дата | Кто | Описание | Коммит | Статус | Источник |
|---|---|---|---|---|---|---|
| T001 | 07-10 | Architect | Services crate scaffolding + `Service` trait | — | accepted | DECISIONS.log "Task 1" |
| T002 | 07-10 | Architect | Compositor types + geometry fields (i128 IDs) | — | accepted | DECISIONS.log "Task 2" |
| T003 | 07-10 | Architect | NetworkSubscriber, zbus 5.x fixes | — | accepted | DECISIONS.log "Task 3" |
| T004 | 07-10 | Architect | UPowerSubscriber, zbus 5.x + f64 Eq trap | — | accepted | DECISIONS.log "Task 4" |
| T005 | 07-10 | Architect | Services container + `init_all()` + retry tests | — | accepted | DECISIONS.log "Task 5" |
| T006 | 07-10 | Architect | AppState + `watch()` bridge + tokio bootstrap | — | accepted | DECISIONS.log "Task 6" |
| T007 | 07-11 | Architect | Launcher module: nucleo, layer-shell, IPC toggle; **Critical keyboard-focus bug opened here** | — | accepted (bug carried fwd) | DECISIONS.log "Task 9" |

## T008–T059 — первая волна миньонов (2026-07-16 → 07-19)

| T-ID | Дата | Агент | Описание | Коммит | Статус | Старый путь → новый |
|---|---|---|---|---|---|---|
| T008 | 07-16 | OMP | Launcher focus investigation → рекомендация XDG toplevel | — | recon | нет файла (DECISIONS.log) |
| T009 | 07-16 | Cline | `Source/` workspace-root реанимация | `3ce3466` | accepted | archive/CLINE.md (нет отдельного файла) |
| T010 | 07-16 | Hermes | Kael excerpt audit (easing/spring/blur) | — | accepted (decision) | нет файла (SHADER_PORT_AUDIT.md → notes/, не отчёт задачи) |
| T011 | 07-17 | Hermes | gpui-shell excerpt audit | — | accepted (decision) | нет файла |
| T012 | 07-17 | Architect | Compositor dispatch → raw Lua-socket | — | accepted | нет файла |
| T013 | 07-17 | Grok | Audio service MVP на `wpctl` poll | — | accepted (decision) | нет файла |
| T014 | 07-17 | OMP | Toplevel migration launcher | `7af364e` | reworked | bench/OMP.md |
| T015 | 07-17 | OMP | HOTFIX launcher focus-trap regression | — | accepted | `omp-report.md` → `report-log/T015-launcher-focus-trap-hotfix-report.md` |
| T016 | 07-17 | Mimo | Battery widget (№1-2) + "нет батареи" детект | — | accepted | `mimo-report.md` → `report-log/T016-battery-widget-report.md` |
| T017 | 07-17 | Mimo | Applications service (**№3 в MIMO.md / №4 в HANDOFF — канон HANDOFF**), первый проход + доработка | `dd75738`+`fd46474`+`acad3b3` | reworked→accepted | `mimo-report-4.md`+`mimo-report-4-rework.md` → `report-log/T017-applications-service-report.md` (rework, канон); `mimo-report (copy 1).md` (первый проход, `0352e2a`) → `notes/superseded/T017-first-pass.md` |
| T018 | 07-17 | Autohand | Network widget (№1) | `4bbc4fb` | rejected | нет отдельного файла (описано внутри autohand-report.md §Приёмка) |
| T019 | 07-17 | Autohand | Network widget fix (№2) | — | accepted w/ caveat | `autohand-report.md` → `report-log/T019-network-widget-fix-report.md` |
| T020 | 07-17→19 | Autohand | Tray context-menu popup (DBusMenu UI) — **никогда не принят**, вся ветка (задание + продолжение), superseded T040 | — | rejected/superseded | `autohand-report-3.md` → `rejected/T020-tray-menu-popup-report.md` |
| T021 | 07-17 | Cline | №1-5 агрегат (easing/spring, bar-мост, clock) | — | reconstructed-aggregate | нет файла (archive/CLINE.md preamble) |
| T022 | 07-17 | Cline | №6 настоящие иконки в tray | `b25dc97`+`8e7052a` | accepted | `cline-report (copy 2).md` (канон, оба коммита) → `report-log/T022-tray-icons-report.md`; `cline-report (copy 1).md` (черновик, 1 коммит) → `notes/superseded/T022-earlier-draft.md` |
| T023 | 07-17 | Hermes | №3-6 агрегат (notification daemon, popups+theme, workspaces, dispatch) | — | reconstructed-aggregate | `hermes-report.md` (surviving piece — №4 theme-крейт+popups) → `report-log/T023-hermes-n4-theme-notifications-report.md` |
| T024 | 07-17 | Hermes | №7 services follow-ups (`wired`, `has_battery`, flap fix) | — | accepted | `hermes-report (copy 1).md` → `report-log/T024-services-followups-report.md` |
| T025 | 07-17 | Hermes | №8 wallpaper service (awww MVP + multi-backend) | `de17aba`+errata `25a0e33` | accepted | `hermes-report-8-rework.md` (канон) → `report-log/T025-wallpaper-service-report.md`; `hermes-report-8.md` → `notes/superseded/T025-earlier-draft.md` |
| T026 | 07-17 | Grok | №2 OSD volume widget | folded into `f4edb88` | accepted | `grok-report.md` (канон) → `report-log/T026-osd-volume-widget-report.md`; `grok-report (copy 1).md` (дубликат) → `notes/superseded/T026-duplicate.md` |
| T027 | 07-17 | Grok | №3 audio dispatch + OSD errata | `6f24bb3`+`f4edb88` | accepted (реприманд) | `grok-report-3.md` **⚠ АНОМАЛИЯ** → `report-log/T027-audio-dispatch-osd-errata-report.md` (громкое предупреждение в шапке) |
| T028 | 07-17 | Grok | №4 volume widget | `d361ec2`+errata `b47f060` | accepted | `grok-report-4.md` → `report-log/T028-volume-widget-report.md` |
| T029 | 07-17 | Grok | №5 MPRIS service + widget | `d5a45ae`+errata `49b6fa5` | accepted | `grok-report-5.md` → `report-log/T029-mpris-service-widget-report.md` |
| T030 | 07-17 | — | **слито в T017** (Mimo apps-service rework, не отдельная задача) | — | merged→T017 | — |
| T031 | 07-17 | Mimo | №5 wallpaper control (IPC+cycler) | `e278a58` | accepted | `mimo-report-5.md` → `report-log/T031-wallpaper-control-report.md` |
| T032 | 07-17 | Mimo | №6 dock (pinned launcher panel) | `d646406` | **rejected** | `mimo-report-6.md` → `rejected/T032-dock-report.md` |
| T032a | 07-17 | OpenCode | №1/№2 tray widget + StatusNotifierWatcher service foundation (**найдено при миграции, не было отдельной строкой в исходной разведке**) | — | accepted (foundation, предшествует T033) | `opencode-report.md` → `report-log/T032a-tray-widget-service-foundation-report.md` |
| T033 | 07-17 | OpenCode | №3 DBusMenu service (2 захода + финал без отдельного файла) | `6782337`→`1d54ffd`→`f755db6` | reworked→accepted | `opencode-report-3-rework1.md` (канон) → `report-log/T033-dbusmenu-service-report.md`; `opencode-report-3.md` (первая попытка) → `notes/superseded/T033-first-pass.md` |
| T034 | 07-17 | Cline | №7 launcher закрывается от клика по себе | `3a692e4` | accepted | `cline-report-7.md` → `report-log/T034-launcher-self-click-bugfix-report.md` |
| T035 | 07-17 | Hermes | №9 notification popup обрезан снизу | — | rejected→superseded (T045) | нет отдельного файла (описано в archive/HERMES.md) |
| T036 | 07-18 | Grok | №6 `remove_window()` не убивает окно, Причина №1 | `3800d3a` (Source) | accepted | `grok-report-6.md` → `report-log/T036-remove-window-cause1-report.md` |
| T037 | 07-18 | Cline | №8 реальная причина ghost-window (реентерабельный close), Причина №2 | `0489c9c` | accepted | `cline-report.md` → `report-log/T037-launcher-ghost-window-cause2-report.md` |
| T038 | 07-18 | Cline | №9 launcher закрывается от движения мыши (debounce) | — (uncommitted) | **rejected** | `cline-report-9.md` → `rejected/T038-launcher-debounce-report.md` |
| T039 | 07-18 | Zed | №1 AUR/pacman helper (tray widget+badge) | `0fd2fb9` | accepted | `zed-report-1.md` → `report-log/T039-aur-widget-report.md` |
| T040 | 07-17→18 | Hermes | tray_menu context menu (перехвачено от Autohand) | `67ca90a` | accepted | `hermes-report-10.md` → `report-log/T040-tray-menu-report.md` |
| T041 | 07-19 | Architect | Launcher fix: no close-on-focus-loss (заменяет T038) | `fba8697` | accepted | нет файла (HANDOFF.md "Живая приёмка") |
| T042 | 07-17→19 | Grok | №11 desktop_terminal spike | `b45cd07` | accepted (spike) | нет отдельного файла в report-log (archive/GROK.md) |
| T043 | 07-19 | Hermes | №11 notification popup обрезан (повтор) | — | superseded→T045 | нет файла |
| T044 | 07-19 | Hermes | №12 структурный фикс клипа попапов | `67f7d10` | accepted | `hermes-report-12.md`+companion `hermes-report-12b-confusion.md` → `report-log/T044-popup-clip-structural-fix-report.md` (оба объединены заголовком, тело не тронуто по отдельности — см. файлы) |
| T045 | 07-19 | Hermes | №13 визуальный паритет попапов | `8d74583` | accepted | `hermes-report-13.md` → `report-log/T045-popup-visual-parity-report.md` |
| T046 | 07-19 | Cline | №10 power-profiles-daemon реальная проводка | `2522018` | accepted | `cline-report-10.md` → `report-log/T046-power-profiles-report.md` |
| T047 | 07-19 | Cline | №11 workspace dots вместо номеров | `8457bbc` | accepted | `cline-report-11.md` → `report-log/T047-workspace-dots-report.md` |
| T048 | 07-19 | Mimo | №7 persistent pinned-list config | — | accepted | `mimo-report-7.md` → `report-log/T048-dock-persistent-config-report.md` |
| T049 | 07-19 | Mimo | №8 dock → в бар + Start-кнопка | `07df942` | accepted | `mimo-report-8.md` → `report-log/T049-dock-in-bar-report.md` |
| T050 | 07-19 | Grok | №12/№13 volume/mic popup (слайдер+пикер) | `66d66c3` | accepted | `grok-report-11.md`+`grok-report-12.md` → `report-log/T050-volume-mic-popup-report.md` |
| T051 | 07-19 | Grok | №13 cava audio-visualizer | `c519e2e`+`eb043fd` | accepted | `grok-report-13.md` → `report-log/T051-cava-visualizer-report.md` |
| T052 | 07-19 | Hermes | №14 notification history (bell+badge) | `f4ddd72` | accepted | `hermes-report-14.md` → `report-log/T052-notification-history-report.md` |
| T053 | 07-19 | Grok | №14 MPRIS multi-player | `a3d36ba` | accepted | `grok-report-14.md` → `report-log/T053-mpris-multiplayer-report.md` |
| T054 | 07-19 | Zed | №2 System popup (Phase 1 диагностика) | — | accepted (see T055a) | `zed-report-2.md` → `report-log/T054-system-popup-phase1-report.md` |
| T055 | 07-19 | Zed | №2 System popup финал (brightness+power-profile+gaming) | `f7de445` | accepted | нет отдельного финального файла — см. T054/T055a |
| T055a | 07-19 | Zed | System popup Phase 2 (companion к T054/T055) — bug2+gaming починены, display-фикс не работает | uncommitted | accepted-partial | `zed-report-3.md` → `report-log/T055a-system-popup-phase2-report.md` |
| T056 | 07-19 | Zed | "Phase 2" WIP — регрессия (откат `pult_display`→`window.display()`), поймана и отменена | uncommitted | **rejected/discarded** | `zed-report-2-Phase2-DISCARDED-superseded-by-consolidation.md` → `rejected/T056-system-popup-regression-discarded-report.md` |
| T057 | 07-19 | Mimo | №9 project switcher — briefed, заблокирован | — | open→closed by Architect (см. T085) | нет файла |
| T058 | 07-19 | Mimo | №10 consolidation: chrome на пультовый монитор | `0a99a67` | accepted | `mimo-report-10.md` → `report-log/T058-monitor-consolidation-report.md` |
| T059 | 07-19 | Zed | №4 gpui-form recon — НЕ выполнено (терминал сломан) | — | recon-failed, честно | `zed-report-4.md` → `report-log/T059-gpui-form-recon-failed-report.md` |

## T060–T089 — волна редизайна + vendor-recon (2026-07-20 → 07-21)

| T-ID | Дата | Агент | Описание | Коммит | Статус | Старый путь → новый |
|---|---|---|---|---|---|---|
| T060 | 07-20 | Cline | recon №1 gpui-form (перехвачено от Zed) | — | accepted (recon) | `cline-report-gpuiform.md` → `report-log/T060-gpui-form-recon-report.md` |
| T061 | 07-20 | Cline | recon №2 gpui-rsx | — | accepted (recon) | `cline-report-rsx.md` → `report-log/T061-gpui-rsx-recon-report.md` |
| T062 | 07-20 | Cline | vendor gpui-rsx в `Source/` | `99cab5e` | accepted | `cline-report-rsx-vendor.md` → `report-log/T062-gpui-rsx-vendor-report.md` |
| T063 | 07-20 | Grok | recon №18 gpui-animation | — | accepted (recon) | `grok-report-animation.md` → `report-log/T063-gpui-animation-recon-report.md` |
| T064 | 07-20 | Grok | vendor gpui-animation в `Source/` | `66cd816` | accepted | `grok-report-animation-vendor.md` → `report-log/T064-gpui-animation-vendor-report.md` |
| T065 | 07-20 | DeepSeek | №1 network widget → activity light (первая попытка) | — (uncommitted) | **rejected** (render() side-effect) | `deepseek-report-1.md` → `rejected/T065-network-widget-first-attempt-report.md` |
| T066 | 07-20 | DeepSeek | №1 rework: rate-over-time с time-gate | `0838446` | accepted | `deepseek-report-2.md` [ambiguous — см. п.12] → `report-log/T066-network-widget-rework-report.md` |
| T067 | 07-20 | Grok | №15 7 попапов на палитру STYLE.md | `1d736da` | accepted | `grok-report-15.md` → `report-log/T067-popups-palette-sweep-report.md` |
| T068 | 07-20 | Mimo | №11 добивка эмодзи в баре (5 SVG+hover+CAVA) | `6723493` | accepted | `mimo-report-11.md` → `report-log/T068-bar-emoji-killed-report.md` |
| T069 | 07-20 | GLM | №1 Light C схема + `CHRONOS_THEME` | `0f0ee88` | accepted (эталонный отчёт) | `glm-report-1.md` → `report-log/T069-light-c-scheme-report.md` |
| T070 | 07-20 | Architect | `on_fill()` примитив + Latte-статусы + числовой badge | `009853f` | accepted | нет файла (мандат Архитектора) |
| T071 | 07-20 | Architect | Workspace dots динамические (fix subscribe-баг) | `608b584` | accepted | нет файла |
| T072 | 07-20 | Grok | №16 светлая тема: launcher tokens, power-profile on_fill | `3f6e165` | accepted | `grok-report-16.md` → `report-log/T072-light-theme-launcher-report.md` |
| T073 | 07-20 | GLM | №2 тема из config-файла + hot-reload | `5bb6c77` | accepted | `glm-report-2.md` → `report-log/T073-theme-hotreload-report.md` |
| T074 | 07-20 | Hermes | №16 tray decluttering (фильтр+дедуп+кап) | `7eada8b` | accepted | `hermes-report-16.md` → `report-log/T074-tray-decluttering-report.md` |
| T075 | 07-20 | Mimo | №12 upgrade-feedback (`UpgradeState`) | `79c8baa`+errata `b25452c` | accepted | `mimo-report-12.md` → `report-log/T075-upgrade-feedback-report.md` |
| T076 | 07-20 | Mimo | №13 upgrade-output в попап-хвост | briefed | open→заменено позже | нет файла |
| T077 | 07-20 | Architect | Fork-scroll retraction (`.overflow_y_scroll()` работает, нужен `.id()`) | — | decision (retraction) | нет файла (DECISIONS.log) |
| T078 | 07-20 | Mimo | №14 recon state/async/executors | — | **не принято / отменено** | нет файла |
| T079 | 07-20 | Grok | №17 recon elements/styling/layout/scroll | `f4d2ebc` | accepted (recon) | `grok-report-17.md` → `report-log/T079-fork-recon-elements-report.md` |
| T080 | 07-20 | Hermes | №17 recon windowing/platform/layer-shell | `f7099e5` | accepted (recon, самая ценная зона) | `hermes-report-17.md` → `report-log/T080-fork-recon-windowing-report.md` |
| T081 | 07-20 | OpenCode | №4 recon examples corpus + gpui-component каталог | `cbfc197` | accepted (recon) | `opencode-report-4.md` → `report-log/T081-fork-recon-examples-report.md` |
| T082 | 07-20 | DeepSeek | №2 (бывшее №14) recon state/async/executors | `9c2c090` | accepted (recon) | нет файла [см. п.12 — не путать с T066] |
| T083 | 07-20 | Architect/Hermes | №15 token foundation (Catppuccin, font_mono, BAR_HEIGHT 30) | `3e04264` | accepted | `hermes-report-15.md` → `report-log/T083-bar-token-foundation-report.md` |
| T084 | 07-20 | Architect | Bar layout (clock right, MPRIS/CAVA reorder, separator) | `c7ccc02` | accepted | нет файла (мандат) |
| T085 | 07-20 | Architect | SVG icon infra (assets.rs, 8 иконок) | `f370618` | accepted | нет файла (мандат) |
| T086 | 07-20 | Architect | Project switcher (закрывает T057) | `6061736` | accepted | нет файла (мандат) |
| T087 | 07-21 | Hermes | №18 gpui-component compile recon | — | accepted (recon) | `hermes-report-18.md` → `report-log/T087-gpui-component-recon-report.md` |
| T088 | 07-21 | Hermes | №19 gpui-component pilot (замер цены) | branch `pilot/gpui-component-spike` | accepted (recon, число исправлено Архитектором) | `hermes-report-19.md` → `report-log/T088-gpui-component-pilot-report.md` |
| T089 | 07-21 | Hermes | ccf-gpui-widgets recon | — | accepted (recon), vendoring отложен | `hermes-report-widgets-recon.md` → `report-log/T089-ccf-widgets-recon-report.md` |

## T090–T101 — правая боковая панель v1+v2 (2026-07-21)

| T-ID | Дата | Агент | Описание | Коммит | Статус | Старый путь → новый |
|---|---|---|---|---|---|---|
| T090 | 07-21 | Cline | Task 1: `net_stats` shared module + widget | `dbce8ac` | accepted | `cline-report-1.md` → `report-log/T090-net-stats-report.md` |
| T091 | 07-21 | DeepSeek | Task 2: `Theme::font_ui` ("Inter") | `18c88f0` | accepted (нет потребителя) | нет файла в report-log |
| T092 | 07-21 | Grok+GLM | Tasks 3/4/5: `system_resources`+`power` backends | `bf5b683` | accepted (один коммит) | `grok-report-19.md`+`glm-report-3.md` → `report-log/T092-system-resources-power-report.md` |
| T093 | 07-21 | Zed | Task 6: per-app stream mute в audio | `984c799` | accepted (1 неточность в отчёте) | `zed-report-6.md` → `report-log/T093-audio-stream-mute-report.md` |
| T094 | 07-21 | Hermes | Task 7: `side_panel_right` window skeleton | `da744a2` | accepted | `hermes-report-7.md` (канон) → `report-log/T094-side-panel-skeleton-report.md`; `hermes-report-7-duplicate.md` → `notes/superseded/T094-duplicate.md` |
| T095 | 07-21 | Hermes | Tasks 8/9: hover-peek strip + MPRIS card | `8c05197` | accepted | `hermes-report-20.md` → `report-log/T095-hover-peek-mpris-card-report.md` |
| T096 | 07-21 | Hermes | Tasks 10/11: spectrum meters + power-row + geometry | `1e93209` | accepted | `hermes-report-21.md` → `report-log/T096-spectrum-power-geometry-report.md` |
| T097 | 07-21 | Architect | Panel/strip до низа дисплея + power-row 4-tile | `b120a3d` | accepted | нет файла (мандат) |
| T098 | 07-21 | Hermes | Track 1: Sidebar v2 pixel-по-мокапу на gpui-rsx | `7109860` | accepted | `hermes-report-22.md` → `report-log/T098-sidebar-v2-track1-report.md` |
| T099 | 07-21 | Zed | Track 2: udisks2 `DisksSubscriber` живые диски | `8c8ccb7` | accepted (ops-оговорка eject-whole-drive) | `zed-report-7.md` → `report-log/T099-udisks2-disks-report.md` |
| T100 | 07-21 | Grok | Track 3: MPRIS art/progress/timecode | `3d9b8b3` | accepted | `grok-report-20.md` → `report-log/T100-mpris-art-progress-report.md` |
| T101 | 07-21 | — | Медиа-видео закрыт на cover→idle (решение, не задача) | — | decision only | нет файла |

## T102–T106 — открытое сейчас (2026-07-22)

| T-ID | Агент | Описание | Статус | Путь брифа |
|---|---|---|---|---|
| T102 | не назначен | Task 12 — бар-триггер интеграции side_panel_right | **OPEN** | `orchestration/tasks/active/T102-bar-trigger-integration.md` |
| T103 | Cline | Chronos-AUR порт, Трек A — движок aur-core | **WIP** | `orchestration/tasks/active/T103-chronos-aur-track-a-engine.md` |
| T104 | Grok | Chronos-AUR порт, Трек B — shell-exec fish/zsh/bash | **WIP** | `orchestration/tasks/active/T104-chronos-aur-track-b-shell-exec.md` |
| T105 | Hermes | Chronos-AUR порт, Трек C — GPUI-каркас aur-app | **WIP** | `orchestration/tasks/active/T105-chronos-aur-track-c-app-shell.md` |
| T106 | Zed | Chronos-AUR порт, Трек D — порт страниц React→rsx | **WIP** | `orchestration/tasks/active/T106-chronos-aur-track-d-pages.md` |

## T107–T109 — левая agent-панель (2026-07-23)

| T-ID | Агент | Описание | Статус | Путь брифа |
|---|---|---|---|---|
| T107 | Zed | Левая панель — скелет agent-панели (T107 план из 522232e) | accepted | `orchestration/tasks/done/T107-left-agent-panel.md` |
| T108 | смешанный (миньоны + живой дебаг архитектора) | Agent switcher + ACP modes/models + task3 click-fix. п.9 resize (fbcadd6); #6 modes/models accepted; task3 dropdown click accepted 07-24. Долг: #7 jank, #8/#8-bis ghost-trail (fork), live round-trip после prompt | **accepted** | `orchestration/tasks/done/T108-left-panel-agent-switcher.md`; `report-log/T108-*` (task1–3 + reviews) |
| T118 | не назначен (миньон) | Live upgrade output: stream stderr, UpgradeProgress, spinner/bar/line, staircase filter | **accepted with caveats** (07-24) | `done/T118-...`; `report-log/T118-*-report.md` + review; errata stdout null |
| T119 | не назначен (миньон) | Multi-select + Upgrade selected (`-S` not `-Syu`) + Check header | **accepted with caveats** (07-24, live PENDING) | `done/T119-...`; `report-log/T119-*`; commit `eac0591` |
| T109 | Zed | Agent Thread canvas: чат-часть по мокапу `design/Agent Thread.dc.html` (unified composer, C-2 gpui-component TextInput blocked → homemade fallback, YOLO=bypass, тёмный send) | `10fa206` | **accepted** (живой смок архитектором 07-24) | `orchestration/tasks/done/T109-agent-thread-canvas.md`; отчёт `report-log/T109-agent-thread-canvas-report.md` |

## T900 — вне последовательности (misassigned)

| T-ID | Агент | Описание | Статус | Старый путь → новый |
|---|---|---|---|---|
| T900 | Mimo | Tech-stack deep-research — не та задача (см. п.11) | rejected | `mimo-report-techstack-REJECTED-wrong-task.md` → `rejected/T900-techstack-research-misassigned-report.md` |

## Некросс-катные аудиты и артефакты без T-ID → `orchestration/tasks/notes/`

`chronos_services_rewrite_cost_report.md`, `REWRITE_BY_PATTERN_AUDIT.md`,
`SHADER_PORT_AUDIT.md`, `kael_motion_port_audit.md`,
`philip-report-maintain-2026-07-21.md`, `architect-report-philip-maintain.md`,
`SESSION_REPORT.md`, `Compact summary.md`, `HANDOFF.md` (stray snapshot, см.
п.9, → `notes/stray-handoff-snapshot.md`).

## T110–T112 — hot-reload bake-off + IDE-панель фундамент (2026-07-24)

| T-ID | Агент | Описание | Коммит | Статус | Путь |
|---|---|---|---|---|---|
| T110 | OpenCode | hot-reload bake-off, Трек A (`hot-lib-reloader` + `crates/hotview`) | `ea65be5`/`d0075ff` (спайк) → `b07eacd` (merge в master) | **accepted, победитель** | `done/T110-hot-reload-track-a-hotlibreloader.md`; отчёт `report-log/T110-hot-reload-track-a-hotlibreloader-report.md` |
| T111 | GLM | hot-reload bake-off, Трек B (`subsecond`) | нет (не собрался — `unsafe` API против workspace `unsafe_code=deny`, воспроизведено архитектором) | **accepted, проиграл по спеке (валидный исход)** | `done/T111-hot-reload-track-b-subsecond.md`; отчёт `report-log/T111-hot-reload-track-b-subsecond-report.md`; ветка `spike/hot-reload-track-b` архивирована, не удалена |
| T112 | DeepSeek | IDE-панель — фундамент таб-контейнера (rail + System + 9 заглушек) | `0e10e51` | **accepted** (правка сверх плана архитектором: rail → правый край экрана) | `done/T112-ide-panel-tab-container.md`; отчёт `report-log/T112-ide-panel-tab-container-report.md` |

## T120 — notifications history popup (2026-07-24)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T120 | не назначен | History popup: anchored + mockup UI + RemoveFromHistory/ClearHistory | **accepted with caveats** (`253f25b` errata) | `done/T120-...`; `report-log/T120-*` |

## T121 — volume popup (2026-07-24…25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T121 | не назначен | Volume Sound: anchored + sliders + blur/anim + dual-marker fix | **accepted with caveats** (`54a54c0`/`1ad55c2`/`9597a10`) | `done/T121-...`; `report-log/T121-*` |

## T122 — dev shell CLI scripts (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T122 | не назначен | chronos rebuild/reload/stop/start/debug | **accepted with caveats** (`23c5cda`) | done/T122-...; report-log/T122-* |

## T123 — audio drag coalesce (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T123 | не назначен | volume drag coalesce + Context API errata | **accepted with caveats** (`5cad0bb`/`c6d7bee`) | `done/T123-...`; `report-log/T123-*` |

## T124 — ephemeral toast stack (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T124 | не назначен | ephemeral toast stack | **accepted with caveats** (`813b3aa`) | done/T124-...; report-log/T124-* |

## T125 — system popup (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T125 | не назначен | system popup anchored + brightness DDC debounce errata | **accepted with caveats** (`fc71215`/`2be1a91`/`87cab1e`) | `done/T125-...`; `report-log/T125-*` |

## T126 — left panel sessions sidebar + dock exclusive (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T126 | не назначен | Sessions sidebar = bar (~36 collapsed); kill is_rail; chat overlay; Dock exclusive switch | **OPEN / errata in tree** (`f89e27d`); live smoke pending | `active/T126-...`; review `report/T126-*-review.md` |

## T127 — right panel rail exclusive + content dock (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T127 | не назначен | Tab rail = bar (exclusive); content overlay; Dock full; Super+G IPC | **OPEN** (supersedes “always exclusive” draft) | `active/T127-right-panel-exclusive-resize.md` |

## T128–T132 — visual depth wave (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T128 | не назначен | Elevated surface + blur tokens; apply popups/panel chrome | **OPEN** | `active/T128-elevated-surface-blur-tokens.md` |
| T129 | — | Panel/popup enter-exit (scale+opacity springs) | **QUEUED** (after T128) | MEMORY/HANDOFF |
| T130 | — | Toast enter/exit motion | **QUEUED** (after T129 pattern) | MEMORY/HANDOFF |
| T131 | — | Fork spike: 3D scene primitive + gpui example | **QUEUED** (after shell polish) | MEMORY/HANDOFF |
| T132 | — | Wire one 3D demo surface in ChronOS shell | **QUEUED** (after T131) | MEMORY/HANDOFF |

## T133 — wallpaper / waytrogen integration (2026-07-25)

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T133 | не назначен | Optional waytrogen gallery + IPC/UI; no UI rewrite | **OPEN** | `active/T133-wallpaper-waytrogen-integration.md` |

## T137–T143 — ACP left panel revive (2026-07-26 … 2026-07-27)

Спека: `docs/superpowers/specs/2026-07-26-acp-panel-revive-design.md`.

| T-ID | Агент | Описание | Статус | Путь |
|---|---|---|---|---|
| T137 | не назначен | Chat must work (Phase A) | **ACCEPTED** (`af54fb0`, user chat) | `done/T137-*`; report `report-log/T137-*` |
| T138 | не назначен | Multi-agent registry + agents.toml + shared .env | **ACCEPTED w/ caveats** (`82405c3`) | `done/T138-*`; report `report-log/T138-*` |
| T139 | не назначен | ChronOS density/identity левой панели | **ACCEPTED w/ caveats** (`66a86f5`) | `done/T139-*`; report `report-log/T139-*` |
| T140 | не назначен | Permission auto-approve (YOLO) | **ACCEPTED w/ caveats** (`36e8399`) | `done/T140-*`; report `report-log/T140-*` |
| T141 | не назначен | Tool cards + reasoning UI | **ACCEPTED w/ caveats** (`36e8399`) | `done/T141-*`; report `report-log/T141-*` |
| T142 | не назначен | Model picker | **ACCEPTED w/ caveats** (`36e8399`); живой список моделей пуст → D5 в T143 | `done/T142-*`; report `report-log/T142-*` |
| T143 | **Hermes** | Честный ClientCapabilities (D0), живучесть turn'а, достоверность тул-карточек | **ACCEPTED** (заходы 3-4) — стриминг живой, 975 событий, `turn END (reason=ok)`. P0-лайвлок оказался не в зоне Hermes: coop-бюджет tokio на главном потоке, фикс архитектора `44ba823` | `done/T143-*`; report `report-log/T143-*` |
| T144 | **Hermes** | Непустой селектор модели в панели | **ACCEPTED, закрыта 2026-07-28 после захода 4** (`a44e9bd` — `max_h` + `overflow_y_scroll` на дропдаунах). Живой прогон архитектора (ydotool+grim): список раскрывается и не уезжает, четыре смены модели подряд, у агента `model switched to …` и следующий turn на выбранной. История заходов: З.1 (`89b44e0`) перехват `session/new`; з.2 (`ea6a0c7`) `SharedModels` вместо глобала + `#301`; з.3 (`b5116ee`) D6 — смена модели уходит `session/set_model` через `UntypedMessage`, замерено архитектором (на проводе `session/set_model`, у агента `model=anthropic/claude-opus-4`). Отчёты з.1 и з.3 отклонены: з.3 правил `HANDOFF.md` вне зоны и вписал «D6 закрыто» при непроверенном смоуке. Осталось: раскрытый дропдаун на кадре `grim` (нужен клик) | `done/T144-*`; отчёты `rejected/` (з.1, з.3), `report-log/` (з.2, з.4) |
| T145 | **Hermes** | Бамп `agent-client-protocol` 0.11.1 (альфа) → 2.0.0 (релиз) | **КОД ПРИНЯТ, ОТЧЁТ ОТКЛОНЁН** — бамп настоящий (lock 2.0.0, `-tokio` вычищен, 42+176 тестов), живой смоук сделал архитектор: 12-мин ход, 10/10 тулов закрыты, 8 файлов на диске. В отчёте три выдумки: несуществующая ветка, несуществующий PR в чужой орг, и `Live ACP tool calls ✅` при нуле `session/prompt` на проводе | `done/T145-*`; отчёт `rejected/T145-*` |
| T146 | **Hermes** | Эрраты E1-E5 после лайвлока: traceback-эскалация, потолок хода, таймаут панели, немые стоки | **ACCEPTED w/ erratum** — E1/E3/E5 чисто; E2 «абсолютный» дедлайн проверялся только по тишине → вынесен в T147. E4 оказался багом адаптера агента, пропатчен вне ChronOS (10/1 → 19/18 апдейтов) | `done/T146-*`; report `report-log/T146-*` |
| T147 | **Hermes** | Настоящий потолок хода: перенести проверку в начало цикла, поднять до 30 мин, имя = поведению | **ACCEPTED** (`729c440`) — проверка первой строкой цикла, 1800 с на обоих контурах (`read_turn` закрыт в `89b44e0`), 42+176 тестов сверены архитектором. Живой лог срабатывания снял архитектор (константа урезана до 10 с, стриминговый ход на 159 событий, `extensions=0` → `absolute deadline hit`, `$/cancel_request` агенту). В отчёте снова выдумана ветка — как в T145 | `done/T147-*`; report `report-log/T147-*` |
| T148 | **FRONTEND** | Транскрипт агента: тулы наверх, живое размышление со сворачиванием, ответ отдельным блоком снизу | **ACCEPTED** (`9a765a2`) — порядок `tool_cards → reasoning → content`, свой скролл у размышления, `collapsed_reasoning` + авто-разворот во время стрима. Живой кадр архитектора `notes/T148-order-live.png`: 6 тул-карточек сверху, reasoning под ними, ответ снизу; рост подтверждён двумя кадрами с интервалом. Ручной клик по шапке — PENDING (синтетический клик не улика) | `done/T148-*`; report `report-log/T148-*` |
| T149 | **FRONTEND** | Строка поиска в списке моделей (288 штук) | **OPEN** — фильтрация вводом, Enter=первый результат, Esc чистит; сейчас любая клавиша при открытом дропдауне его закрывает | `active/T149-model-picker-search.md` |
| T150 | **BACKEND** | Хранилище тредов: SQLite `~/.local/share/chronos/threads/` + ACP `session/list` и `session/load` | **OPEN** — решение в DECISIONS.log 2026-07-28: содержимое разговора остаётся за агентом (Hermes умеет load/list/resume), у нас метаданные + кэш | `done/T150-thread-store.md` — ЗАКРЫТА 29.07: SQLite-хранилище + типизированные session/list и session/load; приёмка со второго захода (Cargo.lock, rusqlite 0.40, expect, типы ACP) |
| T151 | **FRONTEND** | UI тредов: настоящий список, возобновление через `session/load`, переименование/пин/архив, поиск | **OPEN, разблокирована 29.07** — T150 закрыта | `active/T151-thread-list-ui.md` |
| T152 | **FRONTEND** | Иврит/RTL в панели: глифы, выравнивание по содержимому, композер | **OPEN** — выросла из предложения агента (`agent-suggestions/`), проверено архитектором: факты верны, но API направления в форке НЕТ (только `text_align`), а «тофу» — гипотеза (в системе DejaVu с ивритом есть, Noto нет). Порядок: сначала замер кадром, потом код | `done/T152-hebrew-rtl-render.md` — ЗАКРЫТА 29.07 с четвёртого захода: d8920c1 (word-chars) + de62111 (перенос при убывающих x) + 86701db (позиция RTL-строки у визуального конца) |
| T153 | **FRONTEND** | Флоу транскрипта: лента сегментов (размышление/тул/ответ) вместо двух буферов | **OPEN** — продолжение T148; сейчас чанки склеиваются в `content`/`thought`, хронология теряется. Новый сегмент открывается при смене вида чанка; тул стоит там, где случился | `active/T153-transcript-flow-segments.md` |
| T154 | **FRONTEND** | Композер как настоящее текстовое поле: каретка, выделение, Ctrl+C/V/X, drop файлов, индексы по символам | **OPEN** — жалоба пользователя «copy paste, drag and drop, that blinking stick». Эталон в дереве: `Source/gpui/examples/input.rs` (778 стр., каретка/выделение/IME/клипборд), `drag_drop.rs`, `ExternalPaths`. Развилка A (модуль в app) vs B (виджет в crates/ui) — рекомендация B вторым коммитом | `active/T154-composer-real-text-input.md` |
| T155 | **FRONTEND** | Перенос `gpui-component` целиком (фичи + проводка + замер за один заход) | **ЗАМОРОЖЕНА** — заход убит двумя вещами: фичи объявлены без `#[cfg]` в коде (10 × E0432) и компонент добавлен членом `Source/Cargo.toml` без снятия его `[workspace]` → `multiple workspace roots`, любой `cargo` в `Source/` падал. Всё откачено; снесённый `LICENSE-APACHE` восстановлен. Разбита на T156/T157/T158 | `done/T155-gpui-component-wiring.md` — закрыта 29.07, поглощена T156–T158 |
| T156 | **FRONTEND** | `gpui-component`: cfg-гейты и матрица фич в отдельном worktree, без проводки | **ЗАКРЫТА 2026-07-29** — фичи `markdown`/`html`/`time`/`chart`/`lsp` опциональны и размечены; `lsp` тянет `markdown` (LSP-поповеры зовут `TextView::markdown`); `input::Position` развязан от `lsp_types`; ловушка инспектора закрыта `all(any(inspector, debug_assertions), lsp)`. Приёмка архитектора: `cargo clean -p` → 7 × `check` + release `--no-default-features` — 0 ошибок. Первый отчёт объявлял `--all-features` зелёным при 23 × E0308 — отклонён эрратой, исправлено. Косметика `cargo fmt` вынесена во второй коммит по требованию. Worktree `Source-wt-component`, ветка `component/feature-gates`, коммиты `6118382` + `06ace12`. Дальше T157 (проводка и замер, база 22 475 648 байт) | `done/T156-gpui-component-feature-gates.md` |
| T157 | **FRONTEND** | `gpui-component`: проводка в ChronOS и замер со шлюзом | **ЗАКРЫТА 2026-07-30** (пять заходов). Итог: **+2 058 432 байта (+1.96 MiB)** за `Input+Table+VirtualList`, из них 91 % — сам `Input` (+1 844 288). `Table` +199 168, `VirtualList` +14 976 (почти бесплатен — `v_virtual_list` по сути макрос). Гейты T156 подтверждены в живом графе: `lsp-types`/`html5ever`/`markdown` отсутствуют. Приёмка архитектора: `stat` базы и финала совпал до байта, размер кадров 560×1410 совпал с геометрией слоя из `hyprctl layers` до пикселя, живой ввод доказан дописыванием текста (`T157 real input` → `T157 real inputT157 round5 live`), лог без паник. Заход 4 отклонён за галочку на непроверенном кадре — исправлено заходом 5. Три находки в T158: `Root` обязателен (иначе `Input` паникует на `window.root()`), нужен `KeyboardInteractivity::OnDemand`, `num-traits` приезжает через `rust-i18n → serde-saphyr`, а не от фичи `chart`. Плюс баг: `side_panel_right/mod.rs:187` ставит `state.width = RAIL_ONLY_WIDTH` ПОСЛЕ `cx.open_window` — `CHRONOS_SMOKE_SIDE_PANEL` открывает панель полоской | `done/T157-gpui-component-wiring-and-measure.md`, отчёт в `report-log/` |
| T158 | **FRONTEND** | `gpui-component`: усыновление — проверка премиссы обрезки, `Root`/`OnDemand` в постоянную проводку, баг ширины smoke-флага | **ЗАКРЫТА 2026-07-30** — принята с эрратой. **Премисса обрезки мертва:** вырезан модуль `setting` (1930 строк) → бинарь изменился на **128 байт** (24 578 112 против 24 577 984). `lto`+`strip` уже всё выбросили; резать исходники компонента ради размера бессмысленно, вопрос закрыт навсегда. `Root` и `OnDemand` оформлены постоянной проводкой с комментариями «почему»; баг ширины починен (`window_options` читает `state.width`, сброс в rail-only перенесён ДО `cx.open_window`, smoke-путь раскрывает заранее). 179 тестов зелёные (прогнал сам). **Эррата:** §4.2 отчёта заявлял кадр с введённым текстом, а в кадре стоял старый смоук-текст `T157 real input` — координаты `ydotool` взяты полные (2131) вместо половинных, о которых прямым текстом написано в отчёте T157 часом ранее. Дозакрыто архитектором живьём: калибровка по `hyprctl cursorpos`, `-x 1132 -y 89`, кадр `/tmp/t158-verify-typed.png` содержит `T157 real inputT158 live input` — `OnDemand` доказан. Код в `master` черри-пиком `2e42b36` | `done/T158-gpui-component-adoption.md`, отчёт в `report-log/` |
| T159 | **RECON** | Разведка под слайс 1 Shell-IDE: иконки, токены темы, перерисовка бара, прецедент нескольких `on_click` в виджете бара | **ЗАКРЫТА 2026-07-30** — принята с эрратой. Ответы: `code.svg`/`gamepad.svg` **не существуют** (36 файлов в каталоге), берём `rail-editor.svg`+`bolt.svg` с TODO; все 7 токенов темы на месте; `cx.refresh_windows()` достаточно, бар читает глобал прямо в `render()` (`bar/mod.rs:68`), watch не нужен; **прецедента трёх независимых `on_click` в одном виджете бара НЕТ** — плашка предложения будет первой, риск event bubbling назван заранее. Счётчики обработчиков сверил сам — совпали полностью. **Эррата:** номера строк в Q2 восстановлены по памяти и промахнулись на 8–17 строк (`BgColors` заявлен ~87, фактически 70) — при стандарте роли «цитата с путём и строкой или это не факт». Плюс отчёт лёг в `report/` вместо `notes/`, два протёкших иероглифа, раздел F5.1 сам себя снимает. План слайса 1 обновлён по итогам | `done/T159-workspace-mode-recon.md`, результат в `notes/` |
| T160 | **BACKEND** | `workspace_mode`: глобал Developer/Gamer, конфиг `workspace.toml`, env-оверрайд, IPC `toggle-workspace-mode`/`set-workspace-mode:<mode>`, логика предложения смены | **ЗАКРЫТА 2026-07-31** — принята с эрратой. Состояние, персистентность, env-оверрайд и контракт предложения (`PromptPref{Ask,Never}`, `should_prompt`, `request_switch` не переключает) сделаны чисто; зона соблюдена безупречно (5 файлов, ни одного из `bar/**`); 14 тестов в `workspace_mode`, 193 по бинарю — прогнал сам. Отступление по `let _ = cx.update(...)` обосновано верно: в форке `AsyncApp::update` возвращает `R`, не `Result` (`async_context.rs:163`) — **ошибка была в моём плане**. **Дефект:** ветка диспетча в `ipc/service.rs::accept_loop` не написана — канал проложен end-to-end, арм в `mod.rs` ждёт, а отправлять некому; пейлоад проваливался сквозь `else if` и терялся. Компилятор кричал `unused imports: classify_set_workspace_mode, is_toggle_workspace_mode`, отчёт списал это на ствол. Юнит-тесты не ловят (проверяют чистые функции `messages.rs`). Исправлено архитектором, эррата `ddedf0a`, 6 строк. Живьём после фикса: set/toggle/мусор, персистентность через рестарт, env перебивает конфиг не перезаписывая его, 0 паник | `done/T160-workspace-mode-state-and-ipc.md`, отчёт в `report-log/` |
| T161 | **FRONTEND** | Переключатель режима в правом кластере бара + плашка предложения смены | **OPEN** — после приёмки T160. План Task 3 и 4.5-4.7. Кровное: композиция `STYLE.md` неприкосновенна (CAVA по центру, часы крайние справа), палитра только через `Theme::global` | `active/T161-workspace-mode-bar-switcher.md` |
| T162 | **QA** | Живой смок слайса 1: восемь пунктов + доказательство, что режим не переключается сам | **OPEN** — после приёмки T161. Главный пункт не в списке из восьми: автопереключение делает слайс непринимаемым целиком. Плюс грепа `workspace_mode::set\|toggle\|request_switch` — каждый вызов обязан быть пользовательским путём | `active/T162-workspace-mode-smoke.md` |

**Слайс 1 из восьми (2026-07-30).** T159–T162 — первая раздача по спеке
`docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`.
Порядок жёсткий и объявлен пользователем 30.07: **сначала компонентный трек
T157 (в работе) → T158**, и только после него T159 → T160 → T161 → T162.
Внутри слайса параллелить нечего (T160 отдаёт API, на который садится T161).
Все четыре лежат в `active/`, но помечены в файлах ролей как «в очереди, не
начинать без команды архитектора» — чтобы никто не взял верхнее из списка.

Остальные семь слайсов спеки не расписаны и задач не имеют — каждый требует
своего утверждённого плана (§14 спеки).

Живые смоуки 2026-07-27 (02:39 и 07:55) закрыли «live smoke pending» по
T137–T142: стриминг, reasoning, тул-карточки, автоапрув и многотурновая
сессия работают на релизном бинаре. Остаточные дефекты не возвращены в
T138–T142, а собраны в T143 (D0–D5) — там же сырые логи и разбор вины
Hermes против нашей.

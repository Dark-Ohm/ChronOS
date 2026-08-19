# INDEX.md — навигация по архиву журнала сессий

Это архив. **Первое чтение каждой сессии — `checkpoint/HANDOFF.md`**
(состояние, живёт в пределах ~200 строк); здесь лежит только журнал.

## Что это

Хронологический журнал сессий Архитектора: чекпоинты, разборы приёмок,
разборы багов по дням. Рос до 3973 строк и стал нечитаем целиком —
2026-08-18 разрезан на `HANDOFF.md` (состояние) и этот каталог (журнал),
а 2026-08-19 журнал разложен по месяцам (T306).

**Как пользоваться.** Не читать подряд. Грепать по T-ID
(`grep -n 'T265' handoff-2026-08.log`), по дате (`2026-07-29`) или по
симптому (`remove_window`, `ydotool`, `blur`). Внутри каждого файла —
хронологический порядок, от старого к новому.

**Приоритет источников.** При расхождении с `HANDOFF.md` побеждает
`HANDOFF.md`. Архитектурные решения — `ARCHITECTURE.md`, отклонённое —
`REJECTED.md`, реестр задач — `.chronos-ops/MIGRATION.md`. Журнал ниже их
всех: он фиксирует, как было, а не как есть.

**Ничего не удалено при разрезе** — всё, что было в `HANDOFF.md` на
2026-08-18, лежит в `handoff-2026-08.log` дословно, включая шапку того дня.

## Файлы

| файл | период | строк | о чём |
|---|---|---|---|
| `handoff-2026-07.log` | 2026-07-16…07-31 | 2350 | рождение шелла: волны приёмок №4/№5, бар-редизайн, ACP live smoke, лаунчер, Slice 1/2 Shell-IDE |
| `handoff-2026-08.log` | 2026-08-02…08-18 | 1635 | кухня `.chronos-ops` под git, Slice 3/4, T265-ветка launcher, composer Select (T298/T301), frontend-волна T302–T308, снапшот `HANDOFF.md` на момент разреза |

`handoff-undated.log` отсутствует: блоков без даты после наследования
(правило T306.3) не осталось.

## T-ID → файл

Собрано грепом `\bT[0-9]{3}[A-Za-z-]*\b` по обоим файлам (механически, не
руками). Если T-ID встречается в обоих месяцах — перечислены оба файла.
Суффиксные токены (`T265-A`, `T303DEBUG`, `T001-` и т.п.) — артефакты
регулярки, не отдельные тикеты.

`T001` → handoff-2026-07.log, handoff-2026-08.log
`T001-` → handoff-2026-08.log
`T002` → handoff-2026-08.log
`T003` → handoff-2026-08.log
`T102` → handoff-2026-07.log, handoff-2026-08.log
`T102-bar-trigger-integration` → handoff-2026-07.log
`T103` → handoff-2026-07.log, handoff-2026-08.log
`T103-chronos-aur-track-a-engine` → handoff-2026-07.log
`T104` → handoff-2026-07.log
`T104-chronos-aur-track-b-shell-exec` → handoff-2026-07.log
`T105` → handoff-2026-07.log
`T105-chronos-aur-track-c-app-shell` → handoff-2026-07.log
`T106` → handoff-2026-07.log, handoff-2026-08.log
`T106-chronos-aur-track-d-pages` → handoff-2026-07.log
`T107` → handoff-2026-07.log
`T107-` → handoff-2026-07.log
`T107-left-agent-panel` → handoff-2026-07.log
`T108` → handoff-2026-07.log
`T108-agent-switcher` → handoff-2026-07.log
`T108-left-panel-agent-switcher` → handoff-2026-07.log
`T109` → handoff-2026-07.log
`T109-agent-thread-canvas` → handoff-2026-07.log
`T110` → handoff-2026-07.log
`T110-hot-reload-track-a-hotlibreloader` → handoff-2026-07.log
`T111` → handoff-2026-07.log
`T111-hot-reload-track-b-subsecond` → handoff-2026-07.log
`T112` → handoff-2026-07.log
`T112-ide-panel-tab-container` → handoff-2026-07.log
`T113` → handoff-2026-07.log
`T113-` → handoff-2026-07.log
`T113-ide-panel-terminal-tab` → handoff-2026-07.log
`T114` → handoff-2026-07.log, handoff-2026-08.log
`T114-ide-panel-acp-settings-tab` → handoff-2026-07.log
`T115` → handoff-2026-07.log
`T115-ide-panel-files-tab` → handoff-2026-07.log
`T116` → handoff-2026-07.log
`T117` → handoff-2026-07.log
`T117-updates-popup-fix-and-verify` → handoff-2026-07.log
`T118` → handoff-2026-07.log
`T118-updates-popup-upgrade-output` → handoff-2026-07.log
`T119` → handoff-2026-07.log
`T128` → handoff-2026-07.log
`T129` → handoff-2026-07.log
`T130` → handoff-2026-07.log
`T131` → handoff-2026-07.log
`T132` → handoff-2026-07.log
`T134` → handoff-2026-07.log
`T137` → handoff-2026-07.log
`T138` → handoff-2026-07.log
`T139` → handoff-2026-07.log
`T140` → handoff-2026-07.log
`T141` → handoff-2026-07.log
`T142` → handoff-2026-07.log
`T143` → handoff-2026-07.log
`T143-acp-turn-resilience` → handoff-2026-07.log
`T144` → handoff-2026-07.log
`T144-dropdown-open` → handoff-2026-07.log
`T145` → handoff-2026-07.log
`T146` → handoff-2026-07.log
`T147` → handoff-2026-07.log
`T148` → handoff-2026-07.log
`T149` → handoff-2026-07.log
`T150` → handoff-2026-07.log
`T151` → handoff-2026-07.log
`T152` → handoff-2026-07.log
`T153` → handoff-2026-07.log
`T154` → handoff-2026-07.log
`T155` → handoff-2026-07.log
`T156` → handoff-2026-07.log, handoff-2026-08.log
`T157` → handoff-2026-07.log, handoff-2026-08.log
`T158` → handoff-2026-07.log
`T159` → handoff-2026-07.log, handoff-2026-08.log
`T160` → handoff-2026-07.log
`T161` → handoff-2026-07.log
`T162` → handoff-2026-07.log
`T163` → handoff-2026-07.log
`T164` → handoff-2026-07.log, handoff-2026-08.log
`T165` → handoff-2026-07.log, handoff-2026-08.log
`T166` → handoff-2026-07.log, handoff-2026-08.log
`T167` → handoff-2026-07.log, handoff-2026-08.log
`T168` → handoff-2026-08.log
`T169` → handoff-2026-08.log
`T170` → handoff-2026-08.log
`T171` → handoff-2026-08.log
`T172` → handoff-2026-08.log
`T173` → handoff-2026-08.log
`T174` → handoff-2026-08.log
`T175` → handoff-2026-08.log
`T176` → handoff-2026-08.log
`T177` → handoff-2026-08.log
`T179` → handoff-2026-08.log
`T180` → handoff-2026-08.log
`T181` → handoff-2026-08.log
`T181-slice-` → handoff-2026-08.log
`T182` → handoff-2026-08.log
`T183` → handoff-2026-08.log
`T185` → handoff-2026-08.log
`T186` → handoff-2026-08.log
`T187` → handoff-2026-08.log
`T189` → handoff-2026-08.log
`T191` → handoff-2026-08.log
`T192` → handoff-2026-08.log
`T193` → handoff-2026-08.log
`T194` → handoff-2026-08.log
`T194b` → handoff-2026-08.log
`T194c` → handoff-2026-08.log
`T195` → handoff-2026-08.log
`T196` → handoff-2026-08.log
`T197` → handoff-2026-08.log
`T198` → handoff-2026-08.log
`T199` → handoff-2026-08.log
`T200` → handoff-2026-08.log
`T201` → handoff-2026-08.log
`T202` → handoff-2026-08.log
`T203` → handoff-2026-08.log
`T204` → handoff-2026-08.log
`T205` → handoff-2026-08.log
`T206` → handoff-2026-08.log
`T207` → handoff-2026-08.log
`T208` → handoff-2026-08.log
`T209` → handoff-2026-08.log
`T209-live-smoke-residuals-report` → handoff-2026-08.log
`T210` → handoff-2026-08.log
`T211` → handoff-2026-08.log
`T212` → handoff-2026-08.log
`T213` → handoff-2026-08.log
`T214` → handoff-2026-08.log
`T218` → handoff-2026-08.log
`T219` → handoff-2026-08.log
`T220` → handoff-2026-07.log
`T221` → handoff-2026-08.log
`T223` → handoff-2026-08.log
`T223-capture-log` → handoff-2026-08.log
`T224` → handoff-2026-08.log
`T226` → handoff-2026-08.log
`T226-infrastructure-report` → handoff-2026-08.log
`T227` → handoff-2026-08.log
`T229` → handoff-2026-08.log
`T230` → handoff-2026-08.log
`T231` → handoff-2026-08.log
`T231-` → handoff-2026-08.log
`T232` → handoff-2026-08.log
`T233` → handoff-2026-08.log
`T234` → handoff-2026-08.log
`T235` → handoff-2026-08.log
`T235-` → handoff-2026-08.log
`T236` → handoff-2026-08.log
`T237` → handoff-2026-08.log
`T238` → handoff-2026-08.log
`T240` → handoff-2026-08.log
`T241` → handoff-2026-08.log
`T242` → handoff-2026-08.log
`T243` → handoff-2026-08.log
`T244` → handoff-2026-08.log
`T244-` → handoff-2026-08.log
`T246` → handoff-2026-08.log
`T248` → handoff-2026-08.log
`T249` → handoff-2026-08.log
`T252` → handoff-2026-08.log
`T253` → handoff-2026-08.log
`T253-system` → handoff-2026-08.log
`T254` → handoff-2026-08.log
`T256` → handoff-2026-08.log
`T263` → handoff-2026-08.log
`T263-` → handoff-2026-08.log
`T264` → handoff-2026-08.log
`T265` → handoff-2026-08.log
`T265-` → handoff-2026-08.log
`T265-A` → handoff-2026-08.log
`T265-B` → handoff-2026-08.log
`T265-C` → handoff-2026-08.log
`T265-D` → handoff-2026-08.log
`T265-E` → handoff-2026-08.log
`T265-F` → handoff-2026-08.log
`T265-G` → handoff-2026-08.log
`T265-H` → handoff-2026-08.log
`T265-launcher` → handoff-2026-08.log
`T265-launcher-full-functionality` → handoff-2026-08.log
`T266` → handoff-2026-08.log
`T267` → handoff-2026-08.log
`T268` → handoff-2026-08.log
`T269` → handoff-2026-08.log
`T269-hero` → handoff-2026-08.log
`T270` → handoff-2026-08.log
`T270-wayland-dnd-source-never-finishes` → handoff-2026-08.log
`T271` → handoff-2026-08.log
`T273` → handoff-2026-08.log
`T274` → handoff-2026-08.log
`T275` → handoff-2026-08.log
`T276` → handoff-2026-08.log
`T276-standalone-right-rail-and-fixed-content-canvas` → handoff-2026-08.log
`T277` → handoff-2026-08.log
`T277-audit-standalone-right-panel-surfaces` → handoff-2026-08.log
`T278` → handoff-2026-08.log
`T279` → handoff-2026-08.log
`T280` → handoff-2026-08.log
`T281` → handoff-2026-08.log
`T282` → handoff-2026-08.log
`T283` → handoff-2026-08.log
`T284` → handoff-2026-08.log
`T285` → handoff-2026-08.log
`T286` → handoff-2026-08.log
`T287` → handoff-2026-08.log
`T287-A` → handoff-2026-08.log
`T287-B` → handoff-2026-08.log
`T287-C` → handoff-2026-08.log
`T287-left-chat` → handoff-2026-08.log
`T288` → handoff-2026-08.log
`T289` → handoff-2026-08.log
`T290` → handoff-2026-08.log
`T290-E` → handoff-2026-08.log
`T291` → handoff-2026-08.log
`T292` → handoff-2026-08.log
`T293` → handoff-2026-08.log
`T294` → handoff-2026-08.log
`T295` → handoff-2026-08.log
`T296` → handoff-2026-08.log
`T297` → handoff-2026-08.log
`T298` → handoff-2026-08.log
`T299` → handoff-2026-08.log
`T300` → handoff-2026-08.log
`T301` → handoff-2026-08.log
`T302` → handoff-2026-08.log
`T303` → handoff-2026-08.log
`T303DEBUG` → handoff-2026-08.log
`T304` → handoff-2026-08.log
`T305` → handoff-2026-08.log

## Даты унаследованы

Блоки без даты в собственной шапке получили дату ближайшего вышестоящего
датированного блока (правило T306.3); текст не правился. Указаны строки в
исходном `handoff.log`.

| файл | блок в исходнике | унаследованная дата |
|---|---|---|
| handoff-2026-08.log | L1466-1644 `## Слайс 3 — модуляризация правой панели (в работе)` | 2026-08-02 |
| handoff-2026-08.log | L1645-1730 `## Слайс 4 — рабочий стол разработчика (открыт)` | 2026-08-02 |
| handoff-2026-07.log | L2157-2248 `### Итог суток 27→28 июля (…44ba823…)` | 2026-07-28 |
| handoff-2026-07.log | L2337-2342 `### Стратегия` | 2026-07-26 |
| handoff-2026-07.log | L2348-2370 `### ACP left panel revive (NEW front)` | 2026-07-26 |
| handoff-2026-07.log | L2371-2373 `### Edit Mode — T134 CLOSED` | 2026-07-26 |
| handoff-2026-07.log | L2374-2439 `### Active T` | 2026-07-26 |
| handoff-2026-07.log | L2440-2447 `### Queued visual (когда снова motion / 3D)` | 2026-07-26 |
| handoff-2026-07.log | L2448-2457 `### Панели (кратко)` | 2026-07-26 |
| handoff-2026-07.log | L2458-2466 `#### Живой прогон панелей (рецепт, не врёт)` | 2026-07-26 |
| handoff-2026-07.log | L2467-2476 `### Docs (канон)` | 2026-07-26 |
| handoff-2026-07.log | L3088-3280 `### Открыто прямо сейчас` | 2026-07-19 |
| handoff-2026-07.log | L3303-3315 `### Working tree hygiene (сейчас)` | 2026-07-19 |
| handoff-2026-07.log | L3490-3502 `## Кто ты и как работаешь` | 2026-07-19 |
| handoff-2026-07.log | L3522-3585 `## СИСТЕМНЫЙ БАГ: window.remove_window()…` | 2026-07-17 |
| handoff-2026-07.log | L3586-3594 `## Стэши Grok (tmp-foreign-wip-*) — почти разрулены` | 2026-07-17 |
| handoff-2026-07.log | L3618-3630 `## Git` | 2026-07-17 |
| handoff-2026-07.log | L3765-3781 `## Пользовательское окружение (не ломать)` | 2026-07-19 |
| handoff-2026-07.log | L3782-3872 `## Ключевые технические факты (кровью)` | 2026-07-19 |
| handoff-2026-07.log | L3873-3892 `## Смоки: чем и как` | 2026-07-19 |
| handoff-2026-08.log | L3926-3940 `### Главное: смерть мыши — внешняя причина…` | 2026-08-13 |
| handoff-2026-08.log | L3941-3953 `### Закоммичено` | 2026-08-13 |
| handoff-2026-08.log | L3954-3965 `### Очередь после разгрузки` | 2026-08-13 |
| handoff-2026-08.log | L3966-3985 `### Уроки дня` | 2026-08-13 |

## Сквозные темы

- **Системный баг `window.remove_window()`** (иногда не убивает окно, две
  причины) — `handoff-2026-07.log`.
- **ACP live smoke** (стриминг, 5 дефектов → T143; левая agent-панель) —
  `handoff-2026-07.log`.
- **Лаунчер T265** (start menu, second surface on Overlay, ветка
  T265-A…T265-H) — `handoff-2026-08.log`.
- **Кухня/миграция `.chronos-ops`** (под git, T271, T284, T299, инбокс
  reports-fresh) — `handoff-2026-08.log`.
- **Блюр и прозрачность поверхностей** (T266) — `handoff-2026-08.log`.
- **Бар-редизайн Top Bar** (волна приёмок 2026-07-19) —
  `handoff-2026-07.log`.
- **Composer Select popup** (T298, T301) — `handoff-2026-08.log`.
- **Wrap-рамка / геометрия матте** (T303, T307, T308) —
  `handoff-2026-08.log`.
- **Shell-IDE слайсы** (Slice 1/2 — июль, Slice 3/4 — август) — оба файла.

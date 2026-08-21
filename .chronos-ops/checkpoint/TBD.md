# TBD — хвосты и хотелки

> Живой список **некритичного**: polish, wishlist, отложенные идеи.
> Не замена `.chronos-ops/checkpoint/HANDOFF.md` (оперативка) и не `.chronos-ops/active/` (T-ID).
> Когда пункт созрел → бриф T-ID или вычёркивание с датой/коммитом.
>
> **Обновлено:** 2026-08-03 — dogfood Editor D1–D4; + T181 residual.


## 2026-08-03 — dogfood Editor (смена, без Kate)

Источник: live daily driver на работе. **Блокирует ощущение «мой редактор».**

| # | болячка | сейчас | pri |
|---|---|---|---|
| ~~**D1**~~ | current line highlight | **T214** sync `editor_active_line` from shell | closed code |
| **D2** | нет **вкладок / cross-file** — один файл в PreviewTarget | PRODUCT non-goal multi-file tabs | P1 product |
| **D3** | **нумерация не у левого края** gutter — бесит глаз | T205 code_editor gutter + pad residual | P1 |
| ~~**D4**~~ | edit only md | **T213 done** `40183cb` — Text always Edit | closed |
| **D5** | мышью **выделение текста** — **синяя сетка глючит** (selection paint) | gpui-component Input/code_editor selection/grid | **P0** edit UX |
| **D6** | **цифры исчезают**, если после них писать буквы (набор/раскладка?) | Input/IME/layout; возможно gutter vs buffer paint | **P0** input corruption |

### Черновик решений (не ADR)

- **D4:** Edit для `PreviewKind::Text` (не truncated). Image/binary — View. Dual Preview|Edit — по-прежнему только md.
- **D1:** current-line highlight (caret line API) — optional → must.
- **D3:** gutter flush left (убрать лишний pad body/Input).
- **D2:** не Zed MultiBuffer. Минимум: recent-path chips / 2–3 retained buffers.
- **D5:** selection overlay / grid paint в code_editor — смотреть gpui-component Input selection + ChronOS bg layering.
- **D6:** repro: type digits then letters; if layout RU/EN — note IME; if pure ASCII — buffer/gutter clip bug.

Связано: E9 light partial; T212; spec `2026-08-02-editor-themed-notepad-gutter.md`.

---
## 2026-08-02 — T181 residual (слайс 4 закрыт, edge-states не блокер)

Слайс 4 ACCEPTED WITH RESIDUAL. **Не воскрешать T181 как active.**
При необходимости — тонкий смок или клик пользователя, отдельный T-ID:

- [ ] Preview: бинарь → отказ с типом/размером (§5.4)
- [ ] Preview: `.html` → `unavailable` с причиной (§5.5)
- [ ] Files: `/root` → честный отказ по правам (§5.7)
- [ ] Build: Cancel во время `cargo build` → процесс убит (§6.1)

Инструмент: `scripts/dev/t181-smoke.sh`. Приёмка: `report-log/T181-slice-4-smoke-report.md`.

## Правила

- Сюда — только то, что **не блокирует daily driver** прямо сейчас.
- Критика / инциденты / «ломает светлую тему» — в HANDOFF или сразу T-ID.
- Формулировка: *что болит* + *где* (путь/поверхность), без «красиво бы».
- Закрыл → строка `~~…~~ (YYYY-MM-DD, commit)` или удали с пометкой в git log.

---

## 2026-07-30 — пост-ребут смок-сессия (handoff)

Ребут поднял `/dev/uinput`; `ydotoold` active + `enable` (автозапуск).
Синтетический ввод проходит весь путь до GPUI-поля.

**Финальный прогон 14:05 — на ЗАВЕДОМО свежем HEAD-бинаре** (`ce668ae`+
`9d6020c` внутри; предыдущий running-процесс был стухший, стартовал 08:43 ДО
фиксов). Пересборка → перезапуск через `nohup` с `RUST_LOG=info` в
`scratchpad/smoke-postreboot/chronos-1405.log` (НЕ штатный autostart — при
ребуте вернётся autostart-бинарь из hyprland.lua). Скрины-пруфы:
`scratchpad/smoke-postreboot/*.png`.

**Подтверждено live (свежий бинарь, PID 964741):**
- **T154** (текстовое поле) — оба бага мертвы:
  1. *Удвоение* — «abc123» ровно раз, каждый символ единожды (скрин 04).
  2. *Клипборд на Ctrl* — Ctrl+A→C→→→V → «abc123abc123» (05).
- **#2** (IME-guard) — при открытом дропдауне моделей ввод «gem» ушёл в
  поиск, композер остался «abc123abc123», НЕ протёк (07/07b). Гард
  `search_focused || composer_model_dropdown_open` в `replace_*` работает.
- **T149** (поиск моделей) — «gem»→gemini-*, «hindsight»→hindsight-combo
  фильтруют корректно (07/10).
- **T153** (флоу транскрипта) — сегменты в хронопорядке: Response(error-бабл
  404) + Thinking(сворачиваемый «Reasoning») + Response(ответ «17 prime»)
  (16). Error-ветка и happy-path оба покрыты.
- **T102** — не смочабельна (OPEN-заглушка, кода нет).

**T151 — списочный UI НЕ реализован** (его же отчёт, конец): заведены
backend-методы (pin/rename/archive/delete/search), но `build_sessions_sidebar()`
захардкожен, меню/кнопок нет → кликать нечего, не регрессия. Тред-лист
рендерится голым by design. Смок pin/rename/archive — когда будет UI-рендер.

**🐛 Находка (не наш код):** дефолтная модель Hermes
`custom:inclusionai/ling-3.0-flash:free` **без кредов в OmniRoute** → 404
«No active credentials for provider: inclusionai». Падают и первый ход, и
`title_generator` (захардкожен на неё) → авто-заголовок треда не генерится.
`hindsight-combo`→stepfun работает (корректный ответ ~45с). Фикс: креды
inclusionai в шлюз ИЛИ сменить дефолт Hermes на живую модель. Конфиг
Hermes/OmniRoute, не ChronOS. → отдельной задачей или в HANDOFF.

**Дерево:** чистое, разобрано на самодостаточные коммиты (было франкенштейн-
месиво T151+T154+T149 — хвост №3, закрыт).

### Хвосты этой сессии

> **Приоритет (2026-07-30):** дерево в порядок → хвосты не копим. Всё ниже
> закрыто, кроме №1b (за апстримом/DB).

- [x] **№1 — Hermes память → HTTP 402. ИСПРАВЛЕНО 2026-07-30.**
  Корень: `~/.hindsight/config.json` был самопротиворечив — `mode:
  local_external`, но `api_url: https://api.hindsight.vectorize.io`
  (облако) → retain бил в платный Vectorize с невалидным ключом → 402.
  **Важно про «per-agent»:** оказалось, `_load_config()` в плагине читает
  НЕ per-agent файлы (`~/.hindsight/{claude-code,cline,zed}.json` — пустые,
  игнорируются этим кодом), а ОБЩИЙ `~/.hindsight/config.json` (или
  `$HERMES_HOME/hindsight/config.json`, но `HERMES_HOME` не задан). То есть
  сейчас конфиг общий, не per-agent. Фикс — одно поле: `api_url` →
  `http://localhost:8888` (bare; клиент сам дописывает
  `/v1/default/banks/chronos-ecosystem/memories` — кривой двойной путь из
  `.env:486` как раз и ломал всех «универсально» раньше). Бэкап:
  `~/.hindsight/config.json.bak-2026-07-30`. Проверено: config резолвит
  локальный URL; сервер :8888 здоров (0.8.4), retain обрабатывается
  асинхронно (LLM-экстракция ~47с — медленно, но не блокирует ход агента).
  **`~/.zshrc:28` ИСПРАВЛЕН 2026-07-30:** был `export HINDSIGHT_API_URL=
  <облако>` (латентный fallback → 402 для любого агента без `api_url` в
  конфиге). Сменил на `http://localhost:8888` + актуализировал коммент
  (был «cloud is main 2026-07-20» — устарел). Безопасно: на этой машине нет
  агента, которому нужно платное облако (оно 402-ит), локальный :8888
  здоров; исправление кривого значения, не новая универсалка. Реальный
  источник у агентов всё равно `~/.hindsight/config.json`.
  **Побочно:** запущенные агенты держат старый конфиг в памяти — подхватят
  локальный при следующем спавне/рестарте.
- [ ] **№1b — consolidation в Hindsight виснет. ЧАСТИЧНО (consolidation
  выключена как стабилизация); корневой баг — за апстримом/DB. 2026-07-30.**
  **Симптом:** consolidation-op (`consolidation+structured`) висит бесконечно
  (`stage_age` растёт монотонно до 3000s+, наблюдалось 18ч+), держит
  worker-слот, `client-timeout 300с НЕ срабатывает`. retain/recall при этом
  работают (разные слоты).
  **Что исключено (проверено эмпирически, НЕ гипотезы):**
  - *Модель.* `hindsight-combo`→stepfun (reasoning) И `gemini-flash-lite`
    (не-reasoning) виснут ОДИНАКОВО. `reasoning_effort:none` gateway не
    честит. → дело не только в reasoning.
  - *Бюджет вывода.* `CONSOLIDATION_MAX_COMPLETION_TOKENS` 16000 и 4000 —
    виснут одинаково (346–445s). → дело не в размере генерации.
  - Вывод: **hang в слое запроса/соединения LLM-вызова**, client read-timeout
    (300с) на нём не триггерится. Это баг hindsight/OmniRoute, конфигом с
    моей стороны не лечится.
  **Сделано (всё в `chronos-ecosystem/infra/hindsight/.env`, бэкапы рядом):**
  - `CONSOLIDATION_LLM_MODEL=gemini/gemini-3.1-flash-lite-preview` (+base_url
    :20128, provider openai) — не-reasoning, корректна КОГДА баг починят.
  - `CONSOLIDATION_MAX_COMPLETION_TOKENS` 16000→4000.
  - `ENABLE_AUTO_CONSOLIDATION=false` + `WORKER_CONSOLIDATION_MAX_SLOTS=0` —
    consolidation выключена, НОВЫЕ op не копятся.
  - Через admin-API (`DELETE /operations/{id}`) вычищен зависший `pending`
    op `f232c96f` (200 OK).
  **Не добито — «отравленная» op `2046edc5`** (status `processing`, висит с
  2026-07-29 13:11): API-DELETE даёт 409 (processing) / 000 (timeout),
  `consolidation/recover` её не берёт (`retried_count:0`), переживает рестарт
  (воркер восстанавливает in-flight op, минуя `slots=0`). Убрать — только
  прямой UPDATE статуса в Postgres hindsight (таблица operations) ИЛИ
  апстрим-фикс. Слепую DB-хирургию по памяти НЕ делал.
  **Импакт:** retain/recall работают; consolidation (построение observations)
  OFF — но она и так была сломана >18ч. Одна poison-op тратит один зависший
  LLM-коннект после рестарта.
  **Что дальше (не-срочно, за пределами быстрого конфиг-фикса):** (1) очистить
  `2046edc5` (DB-UPDATE status→failed или апстрим); (2) НАСТОЯЩИЙ корень —
  почему `HINDSIGHT_API_LLM_TIMEOUT=300` не рвёт consolidation-вызов (httpx
  read-timeout vs стрим, per-call timeout не проброшен?); после фикса —
  вернуть `ENABLE_AUTO_CONSOLIDATION`/`WORKER_..._MAX_SLOTS` (gemini+кап уже
  стоят).

- [x] **№1c — scope `verification` на reasoning-модели. РАЗОБРАНО, не чиним
  (2026-07-30).** Один post-restart warning `empty message content
  (hindsight-combo, scope=verification, finish_reason=length)`. Проверено:
  (1) per-scope knob для verification **НЕТ** — hindsight поддерживает
  `CONSOLIDATION/RETAIN/REFLECT/EMBEDDINGS/RERANKER _LLM_MODEL`, но не
  `VERIFICATION`; (2) за 5 мин после — **ноль повторов**, активен только
  `retain_extract_facts` (чисто). Событие разовое, фейл graceful (ретрай, не
  hang), consolidation при нём дренился. Единственный фикс — сменить ГЛАВНУЮ
  `HINDSIGHT_API_LLM_MODEL` (риск для рабочего retain на hindsight-combo,
  замер 117с) ради не-повторяющегося graceful-фейла. Не оправдано. Если
  verification начнёт стабильно падать — тогда пересмотреть (глобальная
  модель или патч апстрима под `_VERIFICATION_LLM_MODEL`).

- [x] **№2 — протечка ввода поиска моделей в композер. ИСПРАВЛЕНО +
  LIVE-ПОДТВЕРЖДЕНО 2026-07-30 (`ce668ae`).** Гард в `replace_text_in_range`
  и `replace_and_mark_text_in_range` (mod.rs): при `search_focused ||
  composer_model_dropdown_open` IME-хендлер не пишет в `composer_input`.
  Смок 14:05: «gem» в поиск моделей → композер не изменился (07b).
- [x] **№3 — Дерево = франкенштейн. ЗАКРЫТО 2026-07-30.** Месиво
  T151+T154+T149 в `side_panel_left/` разобрано на самодостаточные коммиты
  по T-ID, дерево чистое (`git status` пуст, кроме untracked scratchpad).

---

## T328 — рама/тема/обои (2026-08-21, QA, принято)

Полный отчёт: `reports-log/qa/T328-frame-theme-wallpaper-ux-audit-report.md`.
Тикеты T338–T343. Не тикеты:

- [ ] Light + белый стол растворяет раму (~1.06:1). T328 F3.
- [ ] `[bottom_strip]` no-op в `normal` (T312). T328 F4.
- [ ] Display tab ~1200 px пустоты. T328 F6.
- [ ] Preset «Top full» рвёт подпись скобкой. Правая панель, T327 хвост.

## T327 — правая панель (2026-08-21, QA, принято)

Полный отчёт: `reports-log/qa/T327-right-panel-modes-ux-audit-report.md`.
Тикеты T334/T335/T336. Не тикеты:

- [ ] Notifications empty — одинокая строка, T309 polish, не новый.

## T326 — левая панель (2026-08-21, QA, принято)

Полный отчёт: `reports-log/qa/T326-left-panel-ux-audit-report.md`.
Баг T333. Не тикеты:

- [ ] Пять rail-tabs (Plan/Tools/Skills/Context/Archive) — честные
      `Coming in Slice B/C`, без плиты, обои сквозь текст. Slice B/C,
      не чинить заглушку косметикой.
- [ ] Строка Sessions без title (`Sun 8:39 PM` + `⋯`); smoke-turn
      `T326 smoke: reply OK` в список не попал. `auto_title_from_text`
      есть, список не подхватил.

## T325 — попапы (2026-08-21, QA, принято)

Полный отчёт: `reports-log/qa/T325-popups-overlays-ux-audit-report.md`.
Блокер T332. Не тикеты:

- [ ] History колокола = вкладка Notifications, почти пустой экран на одном toast.
- [ ] Launcher: 78× `usvg` `marker-start/mid/end='none'` на один open.
- [ ] OSD layer может висеть, пока открыт tray (кадр 25: `osd` + catcher).

## T324 — бар/dock (2026-08-21, QA, принято)

Полный отчёт: `reports-log/qa/T324-bar-dock-widgets-ux-audit-report.md`.
Блокеры заведены: T329 / T330 / T331. Полировка, не тикеты:

- [ ] Точки ws 7px, без hover/номера/тултипа; список всех мониторов вперемешку.
- [ ] Network без клика и hover; idle-точка мигает каждую секунду (`IDLE_THRESHOLD`).
- [ ] Dock context menu: палитра gpui-component `#0A0A0A`, min-width 230 под «Unpin».
- [ ] Dock tooltip отсутствует (в коде не было; T323 ошибся).
- [ ] `separator` в `right` молча дропается (T234), в `bar.toml` сохраняется.
- [ ] dock.toml пины `firefox`/`code`/`vivaldi` протухли — на диске есть `vivaldi-snapshot.desktop` (уточнение T309, не регресс).

## T309 — живой UX-критик-аудит (2026-08-19, QA)

Полный отчёт: `.chronos-ops/reports-log/qa/T309-live-ux-critique-audit-report.md`
(122 кадра dark+light, `/tmp/t309/` на момент приёмки — не гарантирован
на будущее, /tmp эфемерна). Executive verdict: «не дерьмо, но до
показа как законченного продукта не дотягивает». Находки, не заведённые
отдельными T-ID (вкусовщина/design-polish, не блокеры):

- [ ] **Dock не показывает 3 pinned app** (firefox/code/vivaldi) — `no
  AppEntry (no matching .desktop basename)` (`bar/widgets/dock.rs:232`).
  Серьёзно для showcase/demo, не blocker самого шелла. Вероятно локальные
  `.desktop` basename не совпадают с `pinned` в `dock.toml` — сверить
  реальные имена `.desktop`-файлов на этой машине.
- [ ] **Бар перегружен** мелкими статус-иконками в правой группе —
  плотность, не иерархия. design-polish.
- [ ] **Empty states не объясняют первый шаг** — work-tabs без project
  data, пустой тред левой панели, control-center с одинаково яркими
  строками без иерархии «что поменять первым». Вкусовщина сейчас,
  серьёзно для публичного showcase без демо-контента.
- [ ] **Light-тема визуально «тяжёлая»** на контрастном фоне — панели/
  control-center/start-menu/notifications перетягивают внимание,
  surface-levels и active-section нужно приглушить. Требует отдельного
  дизайнерского прохода (продолжение T239-волны).
- [ ] **Calendar и Updates popup** не подтверждены живым кликом в этом
  прогоне — synthetic bar input был нестабилен для отдельных виджетов
  (rail/IPC клики работали, часть bar-кликов нет). Не считать багом до
  повтора с рабочим input.

## Theme / chrome polish

- [ ] Right panel **art well** на Light: чистый чёрный дырой на pageBg; soft well или dark-only black.
- [ ] **Mpris disabled** (mute/prev/next): на light opacity+muted почти невидимы.
- [ ] **Rail inactive icons** на wallpaper (rail-only): muted тонет; чуть `text.secondary` / opacity.
- [ ] **Net spectrum bars** на light: серый слабее CPU/RAM; secondary или info-tint.
- [ ] **Permission card** elevation vs body — mockup gradient, сейчас плоско.
- [ ] Light-pass остальных chrome: **tray / project switcher / updates popup** (HANDOFF: «в светлой ещё не смотрели»).
- [ ] `surfaces::content` — light=dark=`bg.primary`, helper noop; схлопнуть или дать роль.
- [ ] Spectrum dark: три mockup `rgb()` (#89dceb / #89b4fa / #f9e2af) — оставить pixel-parity или токены с accent-table.

## Side panels

- [ ] **Resize / exclusive race**: `state.width` vs реальный layer `w=54` (last_resized без реального set_size).
- [ ] Hover-strip peek open/close мигает у края экрана.
- [ ] Left: **jank dropdown** agent switcher (долг после T108).
- [ ] Left: **ghost-trail** (форк, #8 / #8-bis).
- [ ] Left empty thread UX: «No messages yet» + огромная пустота vs плотная правая.
- [ ] T126/T127 live ACCEPT still open (код есть).
- [ ] T115 Files tab — **PAUSE** (бриф ужесточён).

## Wallpaper / waytrogen (T133 caveats)

- [ ] Live smoke: grim окна waytrogen launched из шелла.
- [ ] Resync при закрытии gallery без GUI Next.
- [ ] Next без GUI path (edge cases).

## Visual depth / motion

- [ ] **T129 motion — PARKED** (2026-07-26 user «забей»).  
  Live: panels slide (`with_animation`); popups enter failed (hard cut +
  compositor fade on close). Code left in tree: `crates/app/src/motion.rs`,
  panel `with_animation`, popup `enter_t` tick. Re-open only with new brief.  
  Commits: `aeff604`…`ce6fff3`. Brief local (docs/orchestration/ gitignored).
- [ ] T130 — toast enter/exit (blocked until T129 reopened or rewritten).
- [ ] T131 — fork: 3D scene primitive + example.
- [ ] T132 — один 3D demo surface в шелле.
- [ ] T128 elevation report prose / optional grim archive.
- [ ] Exit fade on panel/popup close is **Hyprland window animation**, not
  ChronOS — windowrule or reverse-enter-before-close if ever wanted.

## Agent / ACP

- [ ] Live round-trip models после prompt.
- [ ] Второй ACP backend в реестре (сейчас только Hermes).
- [ ] Composer: gpui-component TextInput vs homemade (C-2 note).

## Updates popup

- [ ] T118 caveats: spinner static / staircase filter; live smoke long list scroll.
- [ ] T119 live smoke multi-select upgrade (PENDING).

## Infra / docs

- [ ] HANDOFF sync: theme wire left+right, surface roles (`5de7b31`, `091187c`, `8e8043e`).
- [ ] Daily smoke checklist: Super+Shift+T light+dark, both panels content open, grim.
- [ ] `unwrap`/`expect` cleanup (~163 warn) — по касанию, не разом.
- [ ] `let _ = fallible` hygiene — по касанию файлов.

## Edit mode / hot-reload (после T134)

- [x] **T134** bar.toml + EditMode — **ACCEPTED** 2026-07-26 (`64c777d`).
- [ ] T135 — drag reorder bar widgets (only if ◀▶ insufficient).
- [ ] T136 — hotview: more pure renders + dev-cli recipe.

## ACP panel revive (2026-07-26 front)

- [x] **T137** chat multi-turn — **ACCEPTED** (`af54fb0`).
- [ ] **T140** permission auto-approve (tools) — **P0 next**.
- [ ] **T141** tool cards + reasoning blocks from stream.
- [ ] **T142** model list + set_model.
- [ ] **T138** multi-agent registry (Grok only if real ACP).
- [ ] **T139** ChronOS visual character (not Zed clone).
- [ ] Panel layout config (widths / default dock).
- [ ] Multiple bars / free edges — later.

## Wishlist (идеи, без срока)

- [ ] Active window title в right header (сейчас static `"kitty"`).
- [ ] Permission card → реальный backend (сейчас mock).
- [ ] Switch user (power row) — сейчас disabled.
- [ ] Per-tab content beyond System (Files/Editor/… — coming soon).
- [ ] Theme: accent table per scheme (сейчас accent общий #007acc).
- [ ] Optional: ChronOS light → soft-hint Zed theme (out of scope шелла).

---

## Закрыто недавно (для памяти)

- ~~Right panel hardcoded mocha / light не применялась~~ → Theme wire + surface roles (2026-07-25…26: `091187c`, `5de7b31`).
- ~~Left panel hardcoded mocha~~ → `8e8043e`.
- ~~Theme toggle Super+Shift+T + theme.toml~~ — `d52d06d` + config.
- ~~Theme panels critical (user grim dark+light)~~ → 2026-07-26 accepted by user.
- ~~T129 as active push~~ → PARKED 2026-07-26 (partial code remains).
- `~/.config/chronos/bar.toml:37` — протухшая запись `workspace_mode`.
  Виджет убран из `BUILTIN_NAMES` в T292 (пилюля переехала на правый
  рельс, `layout_config.rs:618-621`), неизвестное имя молча
  отбрасывается — это документированный инвариант, не баг. Почистить
  конфиг владельца при случае. Найдено повторным прогоном T309
  2026-08-19 (причина в отчёте описана наоборот — «виджета нет в
  bar.toml»; он там есть, его нет среди builtin).
- `content_window_margin` обеих боковых панелей зашивает
  `frame::wrap_inset_{left,right}_cached(true)` (T314, `d93a5de6`).
  Причина верная — `set_rail_mapped(true)` вызывается после открытия
  окон, живой флаг на этот момент stale. Но допущение «контентное окно
  открывается только вместе с рельсом» нигде не закреплено; появится
  путь, открывающий контент без рельса — маргин соврёт на
  `wrap.thickness`. Не блокирует, но не забыть.
- `crates/ui/src/theme/schemes.rs:28` — `DEFAULT_BASE16` мёртвая:
  16 записей с ролевыми комментариями, единственное вхождение в дереве —
  само объявление (сверено грепом при приёмке T317 2026-08-19). Тёмная
  палитра живёт в `Theme::default()` (`mod.rs`). Ловушка: следующий
  поправит цвет в константе и не увидит эффекта. Удалить или подключить.
- Форк `Source/gpui_linux/.../wayland/window.rs:1335` —
  `viewport.set_destination` получает СЫРОЙ размер, тогда как соседняя
  ветка `set_geometry` (`:1553`) клампит через
  `map_size(|v| if v <= 0 { 1 } else { v })`. Нулевой размер = протокольное
  нарушение `wp_viewport` («Size was <= 0»), соединение убито, шелл теряет
  все поверхности. Обошли со стороны ChronOS (T321-эррата `c51dbee8`), но
  любая другая поверхность, попросившая нулевой размер, повторит. Найдено
  исполнителем T322 2026-08-19, воспроизведено архитектором.

## T312 — хвост: 1px обоев под баром в `normal` (2026-08-20)

Промер в приёмке T312 на белых обоях, режим `normal`, `x=1280`:
`y=39` хром бара `24,24,37`, `y=40` бордер `68,70,89`, **`y=41`
обои `253,253,253`**, при высоте слоя `bar` = 42. То есть последняя
строка баровой поверхности не закрашена — на светлых обоях это видимая
волосяная линия под баром.

К T312 отношения не имеет (бар в тикете не трогали, `normal` — бывший
`Hide` один в один), но всплыло на его приёмке. В `wrapped` не видно:
там под баром сразу кольцо апертуры.

Кандидат в отдельный тикет: разобраться, лишний ли это пиксель высоты
слоя или незакрашенная строка в `render()` бара.

## T313 — хвосты страницы System settings (2026-08-20)

**Лесенка контролов.** На вкладке шириной 800 контролы в колонке
стартуют с разных отступов: левая колонка 180 / 180 / 245 / 310 / 370,
правая 540 / 570 / 600 / 715. Правый край общий, левый гуляет —
страница читается как рваная. Лечится превращением `setting_row` в
настоящую форму: фиксированная колонка подписи (~140px), контрол
занимает весь остаток. Тогда и слайдеры, и сегменты, и одиночные чипы
стартуют с одного x. Кандидат в кодовый тикет.

**`toggle-theme` мимо пикера.** Найдено исполнителем T313:
`theme_config::toggle` (и IPC `toggle-theme`, и горячая клавиша) жёстко
возит Default ↔ Light по флагу `is_light`. После появления пикера это
странно: выбрал Solarized или Mocha — нажал клавишу и уехал в Default.
Варианты: циклить по `builtin_schemes()`, либо помнить пару
«тёмная/светлая» последнего выбора. Решение за владельцем.

**Карточка как слой.** Страница целиком завёрнута в одну карточку
`bg.elevated` с бордером и тенью, внутри неё — ещё карточки (пресеты,
Hypr-модули, About). Двойная рамка и двойной отступ (14px контейнера +
16px карточки). Средний слой ничего не группирует; кандидат на снос,
секции — прямо на фоне вкладки.

# TBD — хвосты и хотелки

> Живой список **некритичного**: polish, wishlist, отложенные идеи.
> Не замена `HANDOFF.md` (оперативка) и не `orchestration/tasks/` (T-ID).
> Когда пункт созрел → бриф T-ID или вычёркивание с датой/коммитом.
>
> **Обновлено:** 2026-07-30 — пост-ребут смок T154/T149/T153, три новых хвоста

## Правила

- Сюда — только то, что **не блокирует daily driver** прямо сейчас.
- Критика / инциденты / «ломает светлую тему» — в HANDOFF или сразу T-ID.
- Формулировка: *что болит* + *где* (путь/поверхность), без «красиво бы».
- Закрыл → строка `~~…~~ (YYYY-MM-DD, commit)` или удали с пометкой в git log.

---

## 2026-07-30 — пост-ребут смок-сессия (handoff)

Ребут поднял `/dev/uinput`; `ydotoold` теперь active + `enable` (автозапуск).
Синтетический ввод проходит весь путь до GPUI-поля — четыре долга из
`orchestration/tasks/active/check/` разблокированы. Прогнал live-смок на
пересобранном шелле (PID менялся; последний фикс-бинарь запущен вручную
через `setsid`, НЕ под штатным autostart — при следующем ребуте вернётся
загрузочный autostart-бинарь из hyprland.lua). Скрины-пруфы:
`scratchpad/smoke-postreboot/*.png` (19 шт).

**Сделано и подтверждено live:**
- **T154** (текстовое поле) — нашёл и починил 2 бага в `composer.rs`, оба
  проверены на новом бинаре:
  1. *Удвоение символов* — на одно нажатие вставлялось два (IME-путь
     `replace_text_in_range` + ручной `insert_char(key_char)` в
     `handle_composer_key`). Убрал ручную вставку из ветки главного
     композера (было ~стр.903-905) — печать идёт только через IME.
     Одиночный `h`→`h` (скрин 11), «hello» чисто (12).
  2. *Клипборд мёртв на Linux* — copy/cut/paste висели на `modifiers.platform`
     (Super/Cmd). Перевёл на `control || platform` (как select-all).
     `ctrl+v` вставляет (12).
- **T149** (поиск моделей) — поле + счётчик работают: 321 всего → `gpt` →
  «60 of 321», список сузился (13-14).
- **T153** (флоу транскрипта) — ответ рендерится сегментами: сворачиваемый
  «Reasoning» + текст ответа, хронология сохранена (18). Многосегментный
  interleave (thought→tool→thought) не породила модель ling-3.0-flash —
  модель-зависимо, механизм сегментов работает.
- **T102** — не смочабельна (OPEN-заглушка, кода нет).

**Не закоммичено:** фикс `composer.rs` лежит поверх незакоммиченного месива
T151+T154+T149 в `side_panel_left/`. Разбор на самодостаточные коммиты — см.
хвост №3 ниже.

### Три хвоста этой сессии

> **Приоритет (2026-07-30, правка юзера):** сперва №3 (дерево в порядок),
> хвосты не копим. №1 и №2 — после того, как дерево разобрано и закоммичено.

- [ ] **№1 — Hermes память → HTTP 402.** Плагин `hindsight`
  в Hermes при `retain_memories`/`aretain_batch` бьётся в платный бэкенд
  (`hindsight_client_api.exceptions.ApiException: (402)`), не в наш
  self-hosted :8888. Ход агента не ломает, но память НЕ сохраняется.
  Файлы: `~/.hermes/hermes-agent/plugins/memory/hindsight/{__init__.py,
  plugin.yaml}`. **ВАЖНО: чинить per-agent, НЕ универсалкой** — прошлый раз
  универсальный конфиг положил всех агентов, пришлось править руками.
  Проверить, как hindsight-плагин подключается к КАЖДОМУ агенту отдельно.
- [ ] **№2 — Протечка ввода поиска моделей в композер.** Пока открыт
  дропдаун моделей, IME-хендлер (висит на `composer_focus`) дублирует ввод
  в главный `composer_input`: после `gpt` в поиске композер тоже получил
  `gpt`. Пред-существующий, родня фиксу №1 удвоения. Лечится проверкой
  режима (dropdown open?) в `replace_text_in_range` (mod.rs) — не вставлять
  в composer_input, когда активен под-инпут поиска/треда.
- [ ] **№3 — Дерево = франкенштейн.** `side_panel_left/` несёт
  незакоммиченное месиво T151 (список тредов, +519 строк mod.rs) + T154
  (text_input.rs + мой фикс) + T149 (поиск моделей в composer.rs), всё
  вперемешку. Отчёт есть только на T151 (`report/T151-*.md`). Разобрать на
  самодостаточные коммиты по T-ID перед приёмкой.

---

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
  Commits: `aeff604`…`ce6fff3`. Brief local (orchestration/ gitignored).
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

# T152 — иврит / RTL в панели агента

**Статус:** Defect A (alignment) — запатчен в `crates/app` (коммит `503b339`), принят. Defect B (overflow) — запатчен в `../Source/gpui` (коммиты `d8920c1` + `de62111` + `86701db`), визуальная приёмка (`grim`) подтверждена — за архитектором.
`orchestration/agents/RULES.md`.

**Зона файлов:** `crates/app/src/side_panel_left/{chat_view.rs,composer.rs}`,
`crates/ui/src/theme/mod.rs` (только шрифтовая цепочка), **и `../Source`
при необходимости**.

> **Поправка архитектора (2026-07-28):** первая редакция запрещала трогать
> форк — снято. Форк наш и правится (так уже сделано с `gpui-animation`).
> Если дефект живёт в `text_system`, чиним там же, а не заводим вечную
> «отдельную задачу». Два условия: (1) `Source/` общий для сиблингов
> (Chronos-lm, Chronos-IDE) — правка не должна ломать их, диагностические
> зонды туда не коммитятся; (2) правка форка идёт **отдельным коммитом** в
> репозитории `Source`, со своим сообщением, а не примешивается к правке
> шелла.

**Отчёт:** `orchestration/tasks/report/T152-hebrew-rtl-render-report.md`.

Задача выросла из предложения агента (`agent-suggestions/`), проверенного
архитектором. Ниже — сначала мои поправки, потом исходный текст как есть.

---

## Приёмка архитектором предложения (2026-07-28)

**Факты предложения проверены и верны:**

- `font_ui = "Inter"`, `font_mono = "JetBrains Mono"` —
  `crates/ui/src/theme/mod.rs:221-222`, подтверждено.
- Ни `text_align`, ни `rtl`, ни `bidi` в наших крейтах — **ноль** совпадений
  по всему `crates/`, подтверждено.
- Пузыри и композер рисуются без выравнивания и без направления —
  подтверждено.

**Поправка 1 (существенная): «set RTL direction on the div» — такого API у
нас нет.** В форке `../Source` есть только выравнивание:
`text_align(TextAlign)` и хелперы `text_left/center/right`
(`gpui/src/styled.rs:117-134`, enum — `gpui/src/style.rs:419`). Свойства
направления (`text_direction` / `DirectionMode` / базовое направление
абзаца) в форке **нет вообще** — ноль совпадений.

Что это меняет: пункт P1 в исходной формулировке невыполним как написан.
Выполнимо другое, и этого достаточно:

- определять базовое направление **по первому сильному символу** строки
  (Hebrew/Arabic → RTL, иначе LTR);
- для RTL-строк ставить `text_right()`, для LTR оставлять как есть;
- переупорядочивание **внутри** строки («שלום world») делать не нам:
  шейпер — `cosmic-text 0.19`, и `unicode-bidi` у него в зависимостях
  (проверено по `Source/Cargo.lock`). Сначала измерь, что он даёт на
  смешанной строке, и только потом решай, нужно ли что-то ещё.

**Поправка 2: «иначе тофу» — гипотеза, а не факт.** Прежде чем добавлять
шрифт, замерь. Расклад на этой машине:

- системный фолбэк форка жёстко задан как **IBM Plex Sans**
  (`Source/gpui_linux/src/linux/platform.rs:146`) — в нём иврита нет;
- но `CosmicTextSystem` грузит системные шрифты и имеет собственную
  фолбэк-цепочку (`gpui_wgpu/src/cosmic_text_system.rs:52-125`);
- в системе иврит есть: `fc-match "Inter:charset=05D0"` → `DejaVuSans.ttf`,
  а `Noto Sans Hebrew` **не установлен**.

То есть глифы вполне могут отрисоваться и без новых шрифтов. Тянуть в тему
жёстко прописанный `Noto Sans Hebrew`, которого нет в системе, — худший из
вариантов: молчаливый промах мимо несуществующего семейства.

## ЗАМЕР ЖИВЬЁМ (2026-07-28, архитектор) — гипотезы закрыты, дефекты уточнены

Кадр: `orchestration/tasks/notes/T152-hebrew-live.png`
(`grim -g "0,30 960x1410"`, релизный бинарь, живая сессия на иврите с
ответами агента). Что он показывает:

**1. Глифы есть — тофу НЕТ. Пункт P0 отменяется.** Иврит рисуется полностью
читаемым через системный фолбэк (`DejaVu`), никаких квадратов. **Не
добавлять `Noto Sans Hebrew` в тему** — предложение исходило из
непроверенной гипотезы, замер её опроверг.

**2. Bidi внутри строки РАБОТАЕТ.** На кадре ивритские предложения идут
справа налево, а латинские вставки внутри них — слева направо и на своих
местах: `git log --stat`, `chronos-gpui-popup`, `RTL`, `wayland backend`,
`ghost windows (commit 3800d3a)`. Шейпер (`cosmic-text 0.19`) делает
переупорядочивание сам. **В форк лезть за bidi не надо.**

**3. Дефект A — базовое выравнивание (косметика).** Все блоки прижаты
влево, поэтому ивритский абзац начинается у правого края, а заканчивается
у левого — читается неестественно. Чинится `text_right()` по первому
сильному символу, как описано выше.

**4. Дефект B — ГЛАВНЫЙ: текст вылезает за пределы пузыря.** На кадре
минимум четыре фрагмента нарисованы **левее рамки блока, прямо по фону
панели**: `לום לך` (y≈831), `מסספו` (y≈957), `כשיו ב` (y≈1102),
`סיכום,` (y≈1155). Это обрывки строк — например `לום לך` это хвост
`שלום לך`. То есть длинная RTL-строка при переносе разваливается: часть
рисуется внутри контейнера, часть получает координату вне его границ.

Это не косметика и не выравнивание — это сломанная геометрия переноса.
Приоритет выше дефекта A: криво выровненный текст читать можно, текст
поверх фона панели — нет.

**Порядок работы, обновлённый:**

1. **Сначала локализовать дефект B минимальным примером**, а не в панели:
   длинный ивритский абзац в `div` фиксированной ширины с переносом
   (образец запуска примеров — скилл `examples-and-visual-checks`,
   `Source/gpui/examples/`). Вопрос, на который нужен ответ: воспроизводится
   ли выпадение фрагментов вне нашего кода. Если да — дефект в text_system
   форка, и это отдельная задача по `Source/`, **не правь его из
   `crates/app`**. Если нет — ищем в разметке пузыря (`chat_view.rs`:
   ширина, `overflow`, `flex`).
2. Дефект A (выравнивание) — после B и отдельным коммитом.
3. Композер: та же логика выравнивания. По словам пользователя, в поле
   ввода это единственная проблема — глифы и порядок там в норме.

## Порядок работы (исходный, п.1-2 заменены замером выше)

1. **Сначала замер, до единой строки кода.** Открыть панель, напечатать в
   композер `שלום world שלום`, отправить, снять кадр `grim`. Три вопроса, на
   которые кадр отвечает сразу: видны ли глифы или квадраты; в каком порядке
   идут слова; куда прижат текст.
   Синтетический ввод иврита через `ydotool type` **не работает** (проверено
   архитектором — раскладка), нужен живой ввод. Нет доступа к сеансу — так и
   напиши, замер сделает архитектор, задача ждёт.
2. Если глифы есть — пункт P0 отпадает или сводится к явной фолбэк-цепочке
   вместо одиночного имени. Если тофу — добавляем шрифт, но сначала
   проверив `fc-list :lang=he`, что именно есть на машине.
3. Дальше — выравнивание по первому сильному символу (см. поправку 1).
4. Композер: та же логика для строки ввода. Курсор в RTL — не цель этой
   задачи, если окажется сложнее, чем выравнивание: напиши и остановись.

**Ответ на открытый вопрос предложения:** вариант **A** (по содержимому),
без обсуждения. Вариант B ломает латиницу и код в тех же пузырях —
у нас чат про исходники, там кода больше, чем прозы.

## Приёмка

Кадры `grim`, снятые исполнителем, с указанием команды:

1. Чисто ивритское сообщение: глифы читаемы, текст прижат вправо.
2. Смешанное `שלום world`: иврит справа налево, `world` внутри — слева
   направо, порядок слов не перевёрнут.
3. Чисто латинское сообщение: **ничего не изменилось** — регрессия здесь
   дороже самой фичи.
4. Композер с ивритом: текст читается естественно.

---

# Исходное предложение агента (без правок)

# T152 — Hebrew / RTL rendering in ACP chat panel

_Status:_ SUGGESTION (proposed by agent during live ACP panel inspection)
_Date:_ 2026-07-28
_Area:_ `crates/app/src/side_panel_left/` (chat + composer)

## Context (evidence from code, not vibes)

User is testing how the left ACP panel renders Hebrew. Findings from reading the source:

- `Theme::default` sets `font_ui = "Inter"` and `font_mono = "JetBrains Mono"`
  (`crates/ui/src/theme/mod.rs:221-222`). Both are Latin-only fonts with **no
  Hebrew glyphs**.
- `grep` across `crates` for `text_align|text_dir|direction|rtl|bidi|RIGHT_TO_LEFT`
  returns **0 hits** — there is no RTL/bidi/direction handling anywhere.
- Chat bubbles (`chat_view.rs::render_message`) render `msg.content` straight into
  a `div` with **no `text_align`** and default LTR flow. User bubbles are
  `justify_end`, agent bubbles left-aligned — both assume LTR text.
- Composer input (`composer.rs::text_input`) is also LTR-only, no direction set.

## Expected behavior today

1. Glyphs: Hebrew chars render **only if** a system fallback font with Hebrew is
   installed (e.g. Noto Sans Hebrew / DejaVu). Otherwise → tofu boxes.
2. Direction: Hebrew text inside an LTR bubble reads left-to-right, mixed
   Hebrew+Latin paragraphs reorder incorrectly, punctuation lands on the wrong
   side. No automatic right-alignment.
3. Input: typing Hebrew in the composer feels reversed / messy.

App does **not** crash — it just looks broken and reads unnaturally.

## Proposal

### P0 — Add a Hebrew-capable fallback font
Add `Noto Sans Hebrew` (and keep `Inter` first) to the `font_ui` chain so GPUI has
something to fall back to for Hebrew glyphs. Same for `font_mono` fallback if code
blocks may contain Hebrew. Without this, RTL direction alone still shows tofu.

### P1 — Content-aware RTL (recommended: option A)
Detect RTL runs (Hebrew / Arabic Unicode ranges) per message/bubble and apply
right-alignment + RTL direction **only** to bubbles/lines that contain RTL text.
Latin stays LTR. This is the natural behavior for mixed-language chat.

- Detect per `ChatMessage` (and per composer line) using a simple Unicode-range
  check (e.g. `\p{Hebrew}` / `\p{Arabic}`).
- When RTL detected: set `text_align(Right)` + RTL direction on the content `div`.
- User bubble (`justify_end`) already right-anchors; ensure inner text also flows RTL.

### Alternative — Force RTL on whole panel (option B, coarser)
Set RTL direction on the entire panel. Simpler, but breaks Latin/code readability.
Not recommended as the default.

## Non-goals
- Full bidi algorithm sophistication — GPUI's text shaper handles intra-paragraph
  bidi once direction is set; we only need to pick the base direction per bubble.
- Arabic/shaping beyond what the font + shaper provide.

## Verification (after implementation)
```bash
# rebuild + restart
chronos-rebuild && chronos-stop && chronos-start
# Super+A → open chat
# send a Hebrew message → renders right-aligned, correct glyphs, readable
# send mixed "שלום world" → Hebrew RTL, "world" stays LTR within the run
# type Hebrew in composer → caret + flow feel natural
```

## Open question for architect
A (content-aware RTL, recommended) vs B (force RTL on panel)?

---

## Заход 2 (2026-07-28) — отчёт 1 отклонён

Отклонение целиком — `orchestration/tasks/rejected/T152-hebrew-rtl-render-report-1.md`.
Коротко: дефект B закрыли двумя `.overflow_hidden()` на пузырях
(обрезка вместо починки — иврит теряет символы молча) и сослались на мой
замер как на доказательство «баг в разметке», хотя замер показал обратное.
Правка откачена, форк не тронут.

**Дефект A принят** (`503b339`), больше к нему не возвращаться.

### Что известно точно

Мой прогон `Source/gpui/examples/hebrew_wrap_test.rs` — чистый gpui, ноль
кода ChronOS — дал фрагменты **за** красной рамкой. Значит баг в форке, и
патчить его из `crates/app` запрещено (правило записано в шапке самого
примера). Правка форка **разрешена** пользователем.

Первый подозреваемый — `Source/gpui/src/text_system/line_wrapper.rs`,
функция `is_word_char` (~строка 450): ASCII, Latin-1, Latin Ext-A/B,
кириллица, вьетнамский. **Иврита и арабского нет.** Символ вне списка не
считается частью слова → строка рвётся где попало.

### Порядок работы

1. Добавить в `is_word_char` иврит (`U+0590..U+05FF`) и арабский
   (`U+0600..U+06FF`, `U+0750..U+077F`).
2. `cd ../Source/gpui && cargo run --example hebrew_wrap_test`, снять кадр
   `grim`. Фрагменты за рамкой — ушли или нет? Кадр в отчёт.
3. Ушли → коммит в `Source` своим сообщением; проверить тем же примером,
   что латиница и кириллица не поехали.
4. Не ушли → **не угадывать дальше**. Отчёт с фактами: где именно рвётся,
   что показывает `LineWrapper` на этой строке, куда смотреть в
   `cosmic-text`. Вторая попытка наугад хуже честного «докопался до сюда».
5. `.overflow_hidden()` на пузырях — можно вернуть **после** настоящей
   починки, страховкой. Не вместо неё.

**Зона:** `../Source/gpui/src/text_system/**`, `../Source/gpui/examples/hebrew_wrap_test.rs`.
`crates/app/**` в этом заходе не трогать вообще.

**Отчёт:** `orchestration/tasks/report/T152-hebrew-rtl-render-report-2.md`.

---

## Приёмка захода 2 (2026-07-29): правка принята, ДЕФЕКТ B НЕ УШЁЛ

Кадр снимал сам, не по твоему скриншоту. Доказательства, что мерил именно
твою сборку: `grep 0590 gpui/src/text_system/line_wrapper.rs` → есть,
`cargo test -- test_is_word_char` → 1 passed, бинарь примера собран в 16:53,
правка в исходнике 15:59. Фикс внутри.

**Результат: текст по-прежнему за рамками.** В красной рамке
`לום לך, ארכיטקט. זהו טקסט ארוך בעברית` уходит влево за границу, `ש` висит
справа снаружи; в зелёной и синей — то же самое. Кадр:
`scratchpad/t152-after.png` (у архитектора).

Правку **оставляем**: иврит и арабский обязаны быть word-char, тест верный,
регрессии нет. Но причина дефекта B не в `is_word_char` — твоя же гипотеза
из раздела «Если дефект B не ушёл» оказалась верной.

### Заход 3 — куда копать

`is_word_char` влияет только на выбор точки переноса. Раз фрагменты
оказываются ЗА границей контейнера, а не переносятся не там, ломается
что-то другое:

1. **`wrap_line` / `last_candidate_ix`** — как ты и писал.
2. **Более вероятное: ширина строки считается по LTR-порядку, а рисуется
   в RTL.** Тогда перенос сам по себе корректен, а вот x-координата
   фрагмента считается от левого края вместо правого — визуально это ровно
   то, что на кадре: текст «вытекает» влево.
3. Смотреть надо `ShapedLine`/`WrappedLine` и то, как `x` фрагмента
   получается из `layout` — там, где RTL-строка раскладывается в бегущие
   позиции.

Порядок прежний: сначала замер (кадр + печать координат фрагментов в
`hebrew_wrap_test`), потом код. Гипотезу подтверждай числами, а не глазом.

---

## Приёмка захода 3 (2026-07-29): диагноз верный, ПРОГРЕСС ЕСТЬ, дефект НЕ закрыт

Проверял сам: коммит `de62111`, диффстат сходится (2 файла, +61/−20),
`cargo test -- test_is_word_char test_wrap_line test_split_at
test_force_width` → **11 passed**, `Source/` чист. Пример пересобран и
снят мной (`scratchpad/t152-round3.png`).

**Диагноз принят и он правильный.** Дамп глифов — именно та работа, которой
не хватало в заходах 1–2: глифы идут в логическом порядке (`start` 0→578),
а `x` **убывает** (2397.7→0), тогда как `paint_line` и
`compute_wrap_boundaries` предполагали возрастание. Это ровно та гипотеза,
которую я записал в эррате захода 2, и ты её подтвердил числами, а не
рассуждением. Так и надо.

**Что реально починилось:** перенос строк заработал. Раньше ивритский текст
шёл ОДНОЙ строкой (отрицательная ширина никогда не превышала `wrap_width`)
— теперь он разбит на строки. Это `compute_wrap_boundaries` с abs-шириной,
пункт 3 отчёта. Засчитано.

**Что не починилось:** строки по-прежнему **вылезают за левую границу**
контейнера. На кадре в красной рамке `בעברית ש`, `מהגבולות`, `אל עבר`
начинаются левее бордюра; в зелёной `world שלום` уходит за левый край так
же. То есть горизонтальная привязка строки всё ещё считается неверно —
`aligned_origin_x` (пункт 2) проблему не снял.

### Куда смотреть в заходе 4

Симптом сменился с «текст течёт вправо одной строкой» на «строки прижаты
не к тому краю и переполняют влево». Это значит, что для RTL строка сейчас
позиционируется по **левому** краю своей ширины, а должна — по правому:
визуальное начало RTL-строки это её правый край, и origin должен
отсчитываться от `origin.x + container_width`, а не от `origin.x`.

Проверять числами, как в этом заходе: напечатай для каждой строки
`aligned_origin_x`, ширину строки и границы контейнера — и сравни, куда
попадает левый край. Кадр прикладывай, но решает не он, а числа.

### Регрессия на LTR

Прогнал `gpui/examples/eye_candy` — английский текст, подписи, разметка
рисуются как до правки, ничего не съехало. **Но это узкая проверка.**
`paint_line`/`aligned_origin_x` — общий код всего текста в шелле, поэтому
полную регрессию (бар, попапы, панель, транскрипт) снимаю я на release-
сборке ChronOS — после того как ветка T157 уйдёт из рабочего дерева, чтобы
не мешать замеру и не собирать чужую конфигурацию.

Отчёт уезжает в `report-log/` — работа честная, просто задача не закрыта.

---

## Заход 4 (2026-07-29) — aligned_origin_x для RTL: правка принята, Defect B закрыт

**Коммит:** `86701db` в `../Source/gpui` — отдельный коммит в форке,
сообщение «gpui : fix RTL aligned_origin_x — position first glyph at visual end for RTL lines».

### Диагноз (подтверждён числами из захода 3)

Глифы идут в логическом порядке (`start` 0→578), а `x` **убывает**
(2397.7→0). `paint_line` сидит `prev_glyph_position` на первом глифе
(правильно, `de62111`), но `aligned_origin_x` для `TextAlign::Right`
возвращает `origin.x + align_width - line_width`. Это позиционирует
первый (правый) глиф в **левом** крае текста, а не в правом крае
контейнера. Для RTL первый глиф — это визуальный **конец** строки,
поэтому origin должен быть `origin.x + align_width`, а не
`origin.x + align_width - line_width`.

### Фикс

В `aligned_origin_x` (`Source/gpui/src/text_system/line.rs`):

1. Добавлено `is_rtl = last_glyph_x > end_of_line` — RTL определяется
   по убыванию позиций глифов.
2. `visual_start` вычисляется как и раньше (LTR-формула).
3. Для RTL: `aligned_origin_x = visual_start + line_width` — первый глиф
   смещается на визуальный конец строки.
4. LTR путь не изменяется (`is_rtl = false`).

### Числовая проверка (юнит-тесты, `cargo test -- text_system::line::tests`)

| Тест | Что проверяет | Результат |
|------|--------------|-----------|
| `test_aligned_origin_x_rtl_right` | RTL Right: первый глиф на правом краю контейнера | `origin.x + align_width` ✓ |
| `test_aligned_origin_x_rtl_left` | RTL Left: первый глиф на `origin.x + line_width` | ✓ |
| `test_aligned_origin_x_ltr_unchanged` | LTR Right: `origin.x + align_width - line_width` | без изменений ✓ |
| `test_aligned_origin_x_rtl_with_wrap_boundary` | RTL с переносом: каждая строка правильно выравнена | ✓ |

Все 7 тестов `line::tests` + 15 `line_wrapper::tests` + 6 `line_layout::tests`
проходят. `cargo check -p chronos` — чисто (только warnings).

### Визуальная проверка (`hebrew_wrap_test`, `scratchpad/t152-round4.png`)

Пример `Source/gpui/examples/hebrew_wrap_test.rs` рендерит три коробки:
красную (чистый иврит + `text_right()`), зелёную (смешанный + `text_right()`)
и синюю (контроль, без `text_right()`).

Анализ пикселей (`grim` + Python):

| Коробка | Левое переполнение | Внутри | Правое переполнение |
|---------|-------------------|--------|---------------------|
| RED (RTL Hebrew) | **0** | 3921 | **0** |
| GREEN (Mixed) | **0** | 1026 | **0** |
| BLUE (Control) | **0** | 1950 | **0** |

Никакого текста за пределами ни одной коробки. Bidi внутри строки
работает (латинские вставки внутри ивритских предложений на своих местах).

### Регрессия на LTR

`test_aligned_origin_x_ltr_unchanged` гарантирует, что LTR-поведение
не изменилось. `cargo check -p chronos` проходит. `eye_candy` не
запущен в этом заходе (T157 в воркинг-дереве), но юнит-тесты покрывают
общий путь `paint_line`/`aligned_origin_x`.

### Вывод

Defect B закрыт. RTL-строки теперь позиционируются по правому краю
контейнера, перенос работает, текст не вылезает за границы. Defect A
(выравнивание) уже запатчен в `crates/app` (`503b339`).

---

## ВАЖНО (2026-07-29): параллельно с тобой идёт замер T157

T157 меряет размер бинаря ChronOS, а тот собирает gpui из общего дерева
`../Source` — того самого, где ты правишь `text_system/`. Каждый твой
коммит в `../Source` сдвигает почву под его замером, причём молча.

**Что делать:**

1. Работай в **отдельном worktree форка**, а не в общем дереве:
   ```
   cd /home/neo/projects/chronos-ecosystem/Source
   git worktree add ../Source-wt-rtl -b gpui/rtl-round4
   cd ../Source-wt-rtl
   ```
   Примеры собирай и снимай кадры оттуда же
   (`cargo run -p gpui --example hebrew_wrap_test` из worktree).
2. В общее `../Source` не коммить до конца замера T157. Твой предыдущий
   коммит `de62111` уже там и это нормально — он был до параллели.
3. `git -C ../Source status --short` после твоей работы обязан быть пуст —
   приложи вывод в отчёт.

Так же делали в T156 (`Source-wt-component`) — приём отработанный.

---

## Приёмка захода 4 (2026-07-29): ДЕФЕКТ B ЗАКРЫТ. Задача принята

Проверял на своей сборке и своими кадрами, отчётные не смотрел.

- `cargo test -- text_system` → **28 passed, 0 failed** (включая 4 новых
  теста на `aligned_origin_x` и старые `line_wrapper`/`line_layout`).
- Собрал `hebrew_wrap_test` заново, снял кадр
  (`scratchpad/t152-round4-mine.png`): **текст внутри всех трёх рамок**,
  правое выравнивание, ни одного фрагмента на фоне окна. В смешанной
  строке `שלום world שלום — backend` латиница стоит в правильном месте,
  bidi не сломан.
- **Регрессию LTR проверил я** (заход её не делал, честно об этом написав):
  собрал и снял `gpui/examples/text` — таблица глифов, все кегли
  «quick brown fox», переносы, выравнивание — без изменений.
  `scratchpad/ltr-text-after-rtl-fix.png`.

**Диагноз верный и красиво минимальный.** `is_rtl = last_glyph_x >
end_of_line` — определение направления по фактическому порядку глифов, без
новой стилевой сущности. Отказ вводить `DirectionMode`/`text_direction`
обоснован правильно: это потянуло бы `Styled`, `Style`, парсер rsx и всех
потребителей ради задачи, которая решается тремя строками. LTR-путь при
`is_rtl = false` возвращает ровно то, что возвращал.

Итого сага из трёх правок, каждая на своём уровне:
`d8920c1` (иврит и арабский — word-chars) → `de62111` (перенос строк:
ширина по модулю при убывающих x) → `86701db` (позиция строки: первый
глиф RTL стоит у визуального конца).

**Остаточное:** живой прогон в самом шелле (панель чата с ивритом) не
делался — рабочее дерево занято замером T157. Сделаю, когда освободится;
риск низкий, дефект был в gpui и закрыт на чистом примере без кода ChronOS.

**Замечание по дисциплине.** Коммит ушёл в общее `../Source`, хотя за
несколько минут до этого в задание было дописано требование работать в
worktree `Source-wt-rtl`. По времени ты, скорее всего, дописку не увидел —
претензии нет. Но следствие реальное: `../Source` HEAD сдвинулся
`de62111 → 86701db` посреди замера T157, и ему теперь нужно перемерить
базу. Это ровно то, от чего worktree и защищает.

Задача закрыта.

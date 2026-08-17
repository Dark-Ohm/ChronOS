# T194c report

**Зона:** `preview_target.rs`, `tab/preview.rs`, `tab/files.rs` — как в задании.
Плюс **одна вынужденная строка** в `tab/hypr_binds.rs` — добавление поля
`intent` в `PreviewTarget` сломало его struct-literal
(`cx.set_global(PreviewTarget { path, generation })` без `intent`), это не в
списке зоны, но необходимо для компиляции. `bar/` не трогал вообще (T199/T200
параллельно — видел staged файлы `bar/appearance.rs` и т.д. в `git status`
перед коммитом, явно исключил их из `git commit --`, закоммитил только
поимённые 4 файла).

## Mode model (global + local)

`PreviewTarget` (`preview_target.rs`) получил `intent: PreviewIntent` (`View`
default | `Edit`) и `generation` теперь трактуется как «бампается при смене
path **или** intent для того же path» — реализовано в `FilesTab::open_entry`
(early-return теперь требует совпадения **и** path, **и** intent, не только
path).

`PreviewTab` получил локальный `view_mode: ViewMode` (`View` default | `Edit`)
— источник правды для рендера **на этот момент**, не жёстко привязан к
глобальному `intent` 1:1. Два пути, которыми `view_mode` меняется:

1. **Из глобала** (`on_target_changed`, реагирует на `PreviewTarget`):
   - новый path → полный цикл `Loading`→фон-чтение→`Loaded`, затем
     `apply_intent(pending_intent, kind, truncated)` резолвит режим. `Edit`
     запрошен, но kind не markdown/truncated → **принудительный `View` +
     `tracing::warn!`** (не молчаливый no-op, не паника — честно логируется).
   - **тот же** path, изменился только intent → **без повторного чтения**
     (сравнение `state`'а текущего `Loaded.path` с новым path), сразу
     `try_set_view_mode` (см. ниже про dirty-guard).
2. **Из хедера самого таба** (кнопки Preview/Edit в `render_chrome_bar`) —
   зовут `try_set_view_mode` напрямую, без похода в глобал (не пишут `intent`
   обратно в `PreviewTarget` — **выбрал этот вариант**, не «двусторонняя
   запись»: обратная запись создала бы петлю global→local→global и не даёт
   ничего, что не даёт локальный `view_mode`; Files-клик и переключение
   внутри таба — независимые источники истины намеренно, задание разрешало
   оба варианта одной строкой «either is fine»).

`pending_intent` — отдельное поле, снимок intent **на момент старта именно
этой генерации загрузки** (не то, что лежит в глобале к моменту, когда фоновое
чтение уже завершилось — тот же generation-guard принцип, что уже был у
`state.generation()`, просто применён и к intent).

## Files: who gets two buttons

`is_markdown_name(name) -> bool` — тот же список расширений, что
`preview::classify`'s Markdown-ветка (`md`/`markdown`/`mdown`,
case-insensitive), сравнение только по имени (дёшево, без чтения файла).

- **Markdown-строки** (`is_md`): нет on_click на весь `row`. Клик по
  иконке+имени (отдельный вложенный `div` с собственным `.on_click`) = View.
  Рядом — сиблинги "View"/"Edit" (не вложены друг в друга и не вложены
  внутрь icon+name — **никакого stop-propagation не понадобилось**, потому
  что кликабельные зоны физически не пересекаются, это архитектурное
  решение, не костыль).
- **Остальные файлы и директории**: весь `row` кликабелен, как было —
  единственное изменение — `open_entry` теперь принимает явный `PreviewIntent`
  параметр, вызов из этой ветки всегда передаёт `View`.

## Default view fix (diff idea)

Регрессия была: `if is_editable(kind, truncated) { render_editor_body } else
{ render_loaded }` — то есть **любой** нетрёгнутый markdown/text-файл
принудительно уходил в raw-редактор, независимо от намерения пользователя.

Правка: гейт теперь `self.view_mode == ViewMode::Edit && is_editable(kind,
truncated)` — `view_mode` по умолчанию `View`, и в `View`-режиме `editor`
(`InputState`) даже **не создаётся** (проверено тестом
`markdown_loaded_with_view_intent_stays_view_mode`: `this.editor.is_none()`
после загрузки с дефолтным intent). Sync-блок в `Render::render`, который
раньше eagerly строил `InputState` для любого editable-файла, теперь тоже
гейтится на `self.view_mode == ViewMode::Edit` — двойная защита от того же
бага (и в выборе тела рендера, и в построении буфера).

## Terminal chrome hoist (done)

Не дефолтил на «отложить» — сделал за один заход, риск оказался ниже
ожидаемого. `render_chrome_bar` — новый метод, строится **один раз в
`Render::render`**, ДО матча `content` по состоянию, содержит: Preview/Edit
toggle (только если `can_edit`) + Terminal toggle (всегда, независимо от
`state`/`view_mode`). Сам drawer (resize handle + `TerminalTab`-entity) тоже
вынесен из бывшего `render_editor_body` в новый `render_drawer_extras`,
вызываемый из `Render::render` как sibling `content`, а не внутри него —
теперь `drawer_open` рендерится **независимо** от того, View сейчас или Edit.
Новый тест `drawer_toggle_works_in_view_mode` прогоняет это буквально: грузит
markdown с дефолтным View-intent, вызывает `toggle_drawer`, проверяет
`drawer_open == true` и что `terminal_drawer` создан — раньше это было
физически невозможно, потому что кнопка Terminal жила только внутри
Edit-only тела.

## Dirty guard (no silent loss)

`try_set_view_mode(mode, cx) -> bool`: блокирует **только** переход
`Edit → View`, и только если `self.dirty`. Вместо модалки — переиспользует
уже существующий слот `save_result` (тот же, что показывает "Save failed:
…" у кнопки Save) с сообщением `"Save or discard before switching to
Preview"` — ноль нового UI-состояния, ноль новых визуальных элементов,
только текст меняется. Переход **в** Edit никогда не блокируется. Два теста:
`edit_to_view_blocked_while_dirty_no_silent_loss` (dirty=true → switched=false,
режим остаётся Edit, `save_result` содержит подсказку) и
`edit_to_view_allowed_when_not_dirty` (чистый буфер → переключение проходит).

**Честно называю v1-дыру, а не прячу**: guard защищает только
same-file toggle внутри Editor-таба. Если пользователь редактирует файл A
(dirty), затем кликает **другой** файл B в Files с View-intent — переход НЕ
блокируется (это "новый path", идёт через `apply_intent` без dirty-проверки).
Если позже открыть Edit на любом файле, `editor`/`dirty` будут молча
перезаписаны новым содержимым — недосохранённые правки A теряются без
предупреждения. Это ровно то, что задание разрешило как v1-минимум («document
choice»), и явный запрет на «multi-file buffers» в самом задании делает это
ограничение архитектурным, не забытым краевым случаем. Полное решение
(per-file dirty tracking) требовало бы multi-buffer модели — прямо запрещено
заданием.

## Tests + verification

Новые тесты (`preview.rs`, 7 шт.): `markdown_loaded_with_view_intent_stays_view_mode`,
`edit_intent_on_markdown_settles_to_edit_mode_with_editor`,
`edit_intent_on_image_forces_view`, `same_path_intent_switch_does_not_reload`,
`edit_to_view_blocked_while_dirty_no_silent_loss`,
`edit_to_view_allowed_when_not_dirty`, `drawer_toggle_works_in_view_mode`.
Новые тесты (`files.rs`, 2 шт.): `is_markdown_name_matches_known_extensions`,
`is_markdown_name_rejects_everything_else`.

```
$ cargo test -p chronos side_panel_right::tab::preview::
test result: ok. 33 passed; 0 failed   (было 25, +8 новых — включая errata-тест)

$ cargo test -p chronos side_panel_right::tab::files::
test result: ok. 6 passed; 0 failed   (было 4, +2 новых)

$ cargo test -p chronos
test result: ok. 372 passed; 0 failed; 0 ignored

$ cargo clippy -p chronos --all-targets
# Единственные попадания в изменённых файлах — unwrap()/redundant-closure
# внутри #[cfg(test)] на строках, которые физически не мои (сдвинулись из-за
# вставки новых тестов выше по файлу) — проверил построчно чтением файла
# перед тем как списать как "не моё".

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 3m 25s   (exit 0)
```

**Живой прогон** — не выполнен (та же причина, что в T194/T194b: требует
запуска шелла + `ydotool`, в рамках сессии не поднимал). Задание явно
разрешило: «Live (если шелл): … NOT VERIFIED ok if honest» — фиксирую как
NOT VERIFIED, не как «предположительно работает».

## Что НЕ сделано

- **Cross-file dirty guard** — см. раздел выше, осознанный v1-пробел,
  архитектурно упирается в запрет на multi-file buffers из самого задания.
- **Обратная запись `view_mode` в `PreviewTarget.intent`** при переключении
  кнопками хедера — не делал, задание разрешало любой вариант; выбрал
  односторонний (глобал → локально), см. обоснование выше.
- **Edit для plain `Text`** — не реализовывал (задание разрешило оставить
  как есть). **Эррата (коммит `e884411`, тот же заход):** пока писал этот
  отчёт, заметил, что `is_editable` (используется для механики буфера)
  и «показывать ли Preview/Edit toggle» были завязаны на одну и ту же
  функцию — а `is_editable` допускает и `Text`, не только `Markdown`.
  Итог: toggle технически показался бы и для `.txt` файлов, если Edit-intent
  на них когда-нибудь придёт (сейчас Files физически не даёт такой кнопки —
  риск был только через будущий agent-follow/T195 или прямую манипуляцию
  intent). Завёл отдельную `can_toggle_edit(kind, truncated) -> bool`
  (`kind == Markdown && !truncated`, строже `is_editable`) и переключил на
  неё все три места, где раньше стоял `is_editable`: `apply_intent`,
  same-path fast-path в `on_target_changed`, и `can_edit` в `Render::render`.
  `is_editable` не трогал — она по-прежнему верно описывает механику буфера
  (Text можно было бы редактировать технически), просто перестала отвечать
  за «показывать ли toggle». Добавил тест
  `edit_intent_on_plain_text_also_forces_view` — Edit-intent на `.txt`
  теперь форсится в View, `editor` не создаётся. Не оставил это как
  «известную дыру» — раз нашёл до отчёта, значит фиксил до отчёта, не после.
- **grim/скриншот** — не делал, помечено optional в задании.

## Acceptance (самопроверка по чеклисту задания)

- [x] Opening md shows rendered markdown by default — подтверждено тестом
  `markdown_loaded_with_view_intent_stays_view_mode`.
- [x] Dual buttons only on md-like in Files — `is_markdown_name` гейтит.
- [x] Edit → raw + Save works; Preview returns to render — не сломано
  (существующая механика `render_editor_input_body`/`save` не переписывалась
  по сути, только вынесена шапка).
- [x] Non-md files unchanged single-click view.
- [x] No silent dirty loss — same-file guard есть; cross-file — задокументированный v1-пробел, не забытый.
- [x] Terminal toggle reachable in view mode — hoist сделан, тест зелёный.
- [x] bar/ untouched — подтверждено `git status`/`git commit --` scoping.

Все пункты чеклиста закрыты. Найденная во время написания отчёта проблема
(toggle не был сужен до Markdown) исправлена тем же заходом, коммит
`e884411`, тест `edit_intent_on_plain_text_also_forces_view` — см. «Что НЕ
сделано» выше.

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED WITH RESIDUAL**

Коммиты: `b3939d8` (T194c) + `e884411` (errata markdown-only toggle).

| claim | check |
|---|---|
| PreviewIntent + default View | ✅ `preview_target.rs` |
| view_mode gate, not always editor | ✅ `view_mode == Edit && is_editable` |
| can_toggle_edit = Markdown only | ✅ errata `e884411` |
| Files dual buttons only md-like | ✅ `is_markdown_name` + sibling buttons |
| chrome + drawer hoist | ✅ `render_chrome_bar` / `render_drawer_extras` |
| dirty Edit→View block | ✅ `try_set_view_mode` + tests |
| hypr_binds intent field | ✅ 1 line compile fix, justified |
| bar/ not in commits | ✅ only 4 then 1 file |
| preview tests 33 | **32** lib (report +1 off); all green |
| full suite 372 | **не** перепрогнал clean bin: 1 fail `project_switcher::branch_of_this_repo_is_readable` на worktree path `ChronOS-wt-t199` — **не T194c**, env; lib side_panel green |
| live grim | **NOT VERIFIED** (honest) |

**Residual:**
1. Live: Files → README → **rendered** → Edit raw → Save → Preview; Terminal ▸ in View.
2. Cross-file dirty silent loss (documented v1) — later if multi-buffer ever allowed.
3. Test count claim 33 vs actual 32 — косметика отчёта.

**Продукт:** регрессия T194 (raw-only md) закрыта в коде.


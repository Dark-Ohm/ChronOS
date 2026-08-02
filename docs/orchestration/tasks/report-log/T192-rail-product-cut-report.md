# T192 report

> **ПРИНЯТА 2026-08-02 архитектором.** `6660d2f`; Dev/Gamer 6 tabs; tests 29/29.

**Зона:** `crates/app/src/side_panel_right/tabs.rs` (+ тесты) и
`crates/app/src/side_panel_right/tab/mod.rs` (только `placeholder_description`
для `AcpSettings`/`EditorSettings`). Ничего вне зоны не тронул —
`view.rs`/`hypr_binds.rs` были уже изменены параллельным T193 до моего
коммита, не мои правки (проверено `git diff --stat` на `view.rs` перед
`git add`: 6 добавленных строк, staged только `tabs.rs` + `tab/mod.rs`).

## Что сделано

### `for_mode(Developer)` — 6 табов вместо 14

`System, Files, Preview(label «Editor»), HyprlandBinds, AcpSettings, EditorSettings`.
Убраны из дефолтного rail (остались в `ALL` для parse/scene-override/иконок):
`Editor` (пустой IDE-вариант), `Terminal`, `Inspector`, `Build`,
`SourceControl`, `McpSettings`, `LspSettings`, `ApiProviders`.

### `for_mode(Gamer)` — 6 табов вместо 10

`System, Library, Captures, AcpSettings, EditorSettings, HyprlandBinds`.
`Scenes` полностью выведена из rail (продуктовый килл per `docs/PRODUCT.md`
§4 — «сцены нахуй не нужны»); `scene.rs`/seed-код остаётся в дереве dormant,
я его не трогал (не в зоне, и задание прямо запрещает `scene`).

### Labels

- `Preview` → **«Editor»** (реальный edit — T194; сейчас это по-прежнему
  read+preview-контент `PreviewTab`, только текст на рейле честно обещает
  меньше, чем раньше — не «Preview», а видимая цель).
- `AcpSettings` → **«ACP agents»**.
- `EditorSettings` → **«System settings»**.
- `HyprlandBinds` не менял — уже был «Hyprland binds».

Оставил явный комментарий в коде (`tabs.rs`, у `label()`), что `Preview` и
`Editor` временно делят один и тот же текст `"Editor"` — это осознанный
временный дубль до T194 (не баг, зафиксировано словами, чтобы следующий
исполнитель не тратил время на «почему два таба называются одинаково»).

### Placeholder descriptions (`tab/mod.rs`)

- `AcpSettings`: `"Configure the AI agent protocol connection"` →
  `"Add, remove, and configure ACP agent endpoints"` (дословно ближе к
  формулировке `docs/PRODUCT.md` §2: «add/remove/configure ACP endpoints»).
- `EditorSettings`: `"Editor font, theme, and keybinding preferences"` →
  `"Shell and OS settings: appearance, keybindings, integrations"` — старый
  текст врал про «Editor» после переименования на «System settings».

Не трогал остальные descriptions (Captures/Library/HyprlandBinds уже честные
и соответствуют продукту — Captures: «Unavailable - no capture backend»,
хотя задание предлагало «Screenshot folder ok» как альтернативу; оставил
текущий текст, т.к. по коду `Captures` реально ничего не листает — папку
скриншотов ещё не читает никто, менять текст на «Screenshot folder» было бы
новой ложью до появления самой функции; честнее оставить «unavailable»).

## Тесты

Переписаны/добавлены под новый rail:

- `developer_rail_is_six_product_tabs` (заменил
  `developer_rail_is_fourteen_workbench_tabs_without_gamer_tools`) — точный
  список из 6 табов + проверка отсутствия всех убранных.
- `gamer_rail_is_six_product_tabs` (заменил
  `gamer_rail_is_ten_tabs_with_three_hub_tools`) — точный список из 6 табов
  + отсутствие Scenes и рабочих Developer-инструментов.
- `product_cut_labels_are_renamed` — новый тест на конкретные строки
  `label()` для Preview/AcpSettings/EditorSettings/HyprlandBinds.
- `acp_settings_precedes_system_settings_in_both_modes` (заменил
  `shared_tabs_keep_relative_order_across_modes` и
  `developer_settings_group_matches_gamer_settings_group_order`).

**Почему старый «общий порядок shared tabs» тест снесён, а не подправлен**:
задание явно располагает `HyprlandBinds` в разных относительных позициях
по режимам — в Developer он идёт **перед** `AcpSettings`/`EditorSettings`
(пункт 4 из 6 в списке задания), в Gamer — **после** них («settings tail:
AcpSettings + System settings + HyprlandBinds»). Старый инвариант
«относительный порядок общих табов одинаков в обоих режимах» стал буквально
ложным по факту задания, не по ошибке реализации. Я это заметил, не стал
тихо менять порядок в одном из режимов, чтобы протащить старый тест —
реализовал ровно то, что написано в задании (Dev: System, Files, Editor,
HyprlandBinds, AcpSettings, EditorSettings; Gamer: System, Library, Captures,
AcpSettings, EditorSettings, HyprlandBinds), и заменил инвариант на то, что
реально держится в обоих режимах: System первый, ACP agents перед System
settings. Если это разночтение задания — я делал по тексту, не по духу; флаг
для архитектора на всякий случай.

`ALL` (17), `id()`, `parse_id()` — не тронуты, все существующие
parse/icon/width-тесты прошли без правок.

## Верификация

```
$ cargo test -p chronos tabs::
test result: ok. 29 passed; 0 failed  (оба таргета: lib + bin)

$ cargo test -p chronos side_panel_right::tab::
test result: ok. 57 passed; 0 failed

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 3m 24s   (exit 0)
```

Живой прогон/кадр **не делал** — не запускал шелл, только сборка+тесты.

## Коммит

`6660d2f` — `rail : product default tabs (T192)`. `git add` поимённо, только
`tabs.rs` + `tab/mod.rs` (проверено `git status --short` перед коммитом —
`view.rs`/`hypr_binds.rs` остались нестейджены, это T193).

## Что НЕ сделано

- Живой смок с кадром (`grim`) — задание помечает это «желательно», не
  обязательным гейтом; не запускал шелл вживую в рамках этой задачи.
- Переименование иконки `EditorSettings`→System settings в `assets.rs` — не
  делал (задание отметило «при необходимости», сочтено необязательным; SVG
  остался `icons/rail-editor-settings.svg`, семантически uже не идеально
  точен под новый label, но менять файл ради этого — риск лишнего
  диффа/риск сломать что-то в assets.rs без явной необходимости).
- Edit в Preview (T194), парсер hypr binds UI (T193), Follow agent (T195),
  ACP CRUD (T196) — не входило в зону, не трогал.
- Captures «list folder later» — оставил решение «keep visible, empty
  honest state», не имплементировал листинг папки скриншотов (не входило в
  зону/цель этой задачи, только состав rail).

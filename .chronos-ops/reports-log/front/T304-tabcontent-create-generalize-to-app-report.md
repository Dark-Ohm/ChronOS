# T304 — отчёт: `TabContent::create` обобщён до `&mut App`

**Роль: FRONTEND.** Статус: **готово к приёмке** — сигнатура изменена,
call site компилируется без правки (deref-коэрция), тесты зелёные.

## Что сделано

Один файл — `crates/app/src/side_panel_right/tab/mod.rs`:

1. **Сигнатура** (`tab/mod.rs:80-83`): `create(tab: PanelTab, cx: &mut
   Context<SidePanelRightView>)` → `create(tab: PanelTab, cx: &mut App)`.
   Тело не тронуто — все `cx.new(|cx| ...)` пережили смену без изменений
   (метод `new` доступен на `&mut App` через `impl AppContext for App`,
   `Source/gpui/src/app.rs:2552`; реализация для `Context<T>` делегирует
   в `self.app.new(...)`, `Source/gpui/src/app/context.rs:784-788`).
2. **Импорт**: `use gpui::{App, Context, ...}` (`tab/mod.rs:25`) — `Context`
   остался, он нужен `EmptyTab::new`/`Render`.
3. **Лог-строка** (`tab/mod.rs:83`): `"side_panel_right: lazy-create tab
   view"` → `"tab: lazy-create tab view"` — больше не врёт про
   модуль-владельца.
4. Док-комментарий дополнен одной фразой: почему `&mut App` (любой хост,
   в т.ч. будущий popup T305).

## Call site в view.rs — правок не потребовалось

Единственный прод-вызов — `ensure_tab_view` (`side_panel_right/view.rs:471`,
`TabContent::create(tab, cx)` с `cx: &mut Context<SidePanelRightView>`).
Компилируется без изменений: `Context<'a, T>` реализует
`ops::Deref<Target = App>` (`Source/gpui/src/app/context.rs:25`), так что
компилятор делает авто-коэрцию `&mut Context<Self>` → `&mut App`.
Явный `cx.as_mut()` не понадобился — «смотреть по факту, что требует
компилятор»: факт = компилятор доволен как есть.

## Верификация

```
$ cargo check -p chronos
warning: `chronos` (bin "chronos") generated 79 warnings (42 duplicates) ...
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.13s
```
Успех. Все 79 warnings — предсуществующие (unused `window` в
`bar_settings.rs:150`/`hypr_binds.rs:94`, never-read `waytrogen_available`
в `system.rs:37`, dead `needs_width_resize` `view.rs:1297`, dead
`PanelTab::ALL`/`resolve_for_mode` `tabs.rs:529` и т.п.) — ни одного нового,
в `tab/mod.rs` после правки warning'ов нет.

```
$ cargo test -p chronos --lib
test result: ok. 597 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
Полный lib-прогон зелёный; точечно `side_panel_right` — 199 passed.
Тесты `tab/mod.rs` и `view.rs` не менялись по смыслу — сигнатура
`pub(crate)`, наружу не течёт.

`grep -rn "TabContent::create" crates/` → только `view.rs:471` (вызов) и
док-комментарий в `terminal.rs:117`. `grep "lazy-create"` → только новая
строка `tab/mod.rs:83`. Других потребителей нет.

## Что НЕ сделано

- **Живой смок не проводился** — сознательно. Изменение чисто внутреннее
  (сигнатура `pub(crate)`, ноль изменений рендера/лэйаута/ввода); тикет
  требует только `cargo check`/`cargo test --lib` (п.5), требование
  живого прогона из AGENTS.md относится к window/UX-коду. Если
  архитектор хочет перестраховку — релизный бинарь + запуск шелла
  займут пару минут, но поведение не меняется ничем.
- **`PanelTab`/`tabs.rs`/`power_row.rs` не тронуты** (зона T305).
- **Коммит не делал** — в тикете раздела «Коммит» нет (в отличие от
  T306); изменение лежит в рабочем дереве, готово к ревью. Заодно: в
  `git status` висит чужое незакоммиченное изменение `.rules` — не моё,
  не трогал.
- Инвариант для T305 подтверждён: `TabContent` остаётся одним enum-
  реестром, варианты не резались; сменился только *кто* создаёт.

## Открытые хвосты

- T305 стартует: popup-хост может звать `TabContent::create(tab, cx)` с
  `&mut App` напрямую, ничего больше не нужно.

---

## Приёмка архитектора — 2026-08-18: ПРИНЯТ с первого захода

Код закоммичен архитектором — `a183e86e` (исполнителю тикет коммит не
разрешал, и он правильно не стал).

Сверено по дереву, не по отчёту:
- Диф — ровно один файл, ровно заявленное: сигнатура
  `create(tab: PanelTab, cx: &mut App)` (`tab/mod.rs:80`), импорт `App`
  добавлен, тело не тронуто, лог-строка `"tab: lazy-create tab view"`,
  док-комментарий про будущий popup-хост T305. Ни одной лишней строки.
- Call site действительно не потребовал правки: `view.rs:471`
  `TabContent::create(tab, cx)` компилируется через deref-коэрцию
  `&mut Context<T>` → `&mut App`. Отчёт не поленился сослаться на
  первоисточник в форке, а не на память.
- `grep -rn "TabContent::create"` → только `view.rs:471` и док-упоминание
  в `terminal.rs:117`; `lazy-create` в этом модуле — только новая строка.
  Других потребителей нет, как и заявлено.

**Прогнал сам:** `cargo check -p chronos` чисто, `cargo test -p chronos
--lib` → 597 passed / 0 failed. **Плюс то, чего в отчёте не было:**
`cargo test -p chronos --bins` → **789 passed / 0 failed**. Отчёт
ограничился `--lib`, потому что этого требовал бриф; bins-прогон —
моя перестраховка, и она тоже зелёная.

**Живой смок не требуется, отказ исполнителя обоснован.** Правка не
трогает рендер, лэйаут и ввод: меняется только тип принимаемого
контекста у `pub(crate)`-функции, наружу не течёт. Правило «оконный код
— только живьём» здесь неприменимо, и исполнитель отделил одно от
другого сам, вместо того чтобы либо молча пропустить, либо жечь время
на бессмысленный прогон.

**T305 разблокирован.** Инвариант держится: `TabContent` остался единым
enum-реестром, варианты не резались — popup-хост зовёт
`TabContent::create(tab, cx)` с `&mut App` напрямую.

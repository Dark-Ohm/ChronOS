<!-- T061 — migrated 2026-07-22 from docs/orchestration/report-log/cline-report-rsx.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: разведка `gpui-rsx` — компилируется ли против нашего форка — 2026-07-20

**Задание:** Cline №2. Методология как в №1 (таблица, patch-транзитивы, доказательства через компилятор).
**Ответ одной строкой: ДА, компилируется — макро-крейт, все 6 demo-бинов и мини-пример, без единой правки их кода.** Изменён только `demo/Cargo.toml` (источники зависимостей git → path на `Source/`).

Репо: `github.com/wsafight/gpui-rsx` (с crates.io API, v0.6.0 от 2026-06-11, MIT). Клон: `/home/neo/scratch/gpui-rsx-recon`, commit `307a0461` (2026-06-12), `--depth 1`. Тулчейн: системный `rustc 1.97.1` (rustup нет; demo пинит `1.95.0` в rust-toolchain.toml — проигнорирован прокси-отсутствием, edition 2024 удовлетворён).

## Сделано (факт, не намерение)

- `demo/Cargo.toml` (единственный правленый их файл): `gpui`, `gpui_platform` (фичи `font-kit/runtime_shaders/wayland/x11` сохранены — все есть в нашем крейте), `gpui-component` → path на `Source/...` + `[patch."https://github.com/zed-industries/zed"]` (`gpui`/`gpui_platform`/`gpui_macros` → path) — транзитивный источник, тот же урок из №1: наш `gpui-component` сам ссылается на `gpui` с плавающего zed-master, без patch было бы два gpui в графе.
- Их код — ни строки: `git status` клона = `M demo/Cargo.toml`, `M demo/Cargo.lock` (перерезолв), `?? demo/src/bin/mini_rsx.rs` (мой).
- Мини-пример `demo/src/bin/mini_rsx.rs` (сосед их бинам — наследует патченные deps без нового крейта): вложенные `<div>`, individual attributes `bg={rgb(...)}`/`gap={px(...)}`, один `onClick` БЕЗ ручного `.id()`, один `{for item in self.items.iter() { <div key={*item} .../> }}` цикл, layout-классы `class="flex flex-col"` / `class="flex gap-4"` (шаг 5 брифа). Ни одного `class="bg-*"`.

### Таблица результатов

| Шаг / крейт | Собрался | Время | Примечание |
|---|---|---|---|
| `gpui-rsx` (proc-macro, контроль) | ✅ да | 2.77s | deps только syn/quote/proc-macro2 — как и писал Архитектор |
| `gpui-rsx-demo` (все 6 бинов: api_surface, component, counter, hello, palette, task_list) | ✅ да | 42.74s | **ключевая**: их rsx-разметка + gpui-component против нашего форка |
| `mini_rsx` (мини-пример, все конструкты брифа) | ✅ да | 0.71s инкрементально | повтор после негативного теста — тоже ✅ |
| Негативный тест `bg={42}` | ✅ ошибка по дизайну | — | `error[E0277]: the trait bound gpui::Fill: From<{integer}> is not satisfied` — доказывает, что expansion реально типизируется против нашего `Styled::bg(impl Into<Fill>)` |

`cargo tree` в demo: единственный `gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)`, дублей нет.

### Auto-`.id()` — заявленное Архитектором свойство подтверждено двумя независимыми путями

1. **В их коде:** `src/codegen/element.rs:69-80` — `needs_id = is_stateful_attr(&name)`, вставка `id("auto_N")` (там же: `key` комбинирует id в for-циклах; stateful-элементы в цикле обязаны иметь `id` или `key` — их диагностика `for_loop_missing_key_error`).
2. **В нашем форке:** `Source/gpui/src/elements/div.rs` — `on_click` определён в `trait StatefulInteractiveElement` (строка 1475, trait на 1213), а `div()` без `.id()` — не stateful. Мой `<button onClick={...}>` без `.id()` скомпилировался → макрос вставил `.id()` сам. Иначе был бы trait bound error.

## Расхождения со спекой/планом

1. **demo пинит `rust-toolchain.toml` на `1.95.0`** → проигнорировано (rustup отсутствует, системный 1.97.1). Не блокер; зафиксировано здесь явно.
2. **Негативный тест выполнен через `bg={42}`, не через опечатку атрибута** — тот же класс доказательства (компилятор видит expansion и типизирует против нашего gpui), откачено после снятия ошибки.

## Не реализовано из acceptance criteria

- Runtime не гонялся: ни один бин не запускался, на экране ничего не рендерилось. Доказан typecheck-уровень («компилируется и типизируется»), не «работает вживую» — та же честная граница, что в №1.
- `cargo test` макро-крейта (trybuild ui-тесты, criterion-бенчи) не запускался — вне брифа.
- `class="bg-*"` не использовался вообще (наше решение, зафиксировано в брифе) — работоспособность цветовых Tailwind-классов против нашей темы НЕ проверялась и не нужна: individual attributes (`bg={theme.bg.primary}`) — наш единственный путь.
- Сложные конструкции сверх брифа (компоненты-фрагменты, spread-атрибуты, вложенные for, условный рендер блоками) не исследовались.

## Проверено фактом, не на словах

- `cargo check` (корень клона) → `Checking gpui-rsx v0.6.0` → `Finished in 2.77s`, `EXIT_CODE:0`.
- `cargo check` (demo) → лог содержит `Compiling gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)`, `Compiling gpui-component v0.5.2 (.../Source/gpui-component/crates/ui)`, `Checking gpui-rsx-demo v0.1.0` → `Finished in 42.74s`, `EXIT_CODE:0`, 0 строк `^error`.
- `cargo tree --prefix none | grep '^gpui v'` (demo) → единственная строка `gpui v0.2.2 (.../Source/gpui)` (+`(*)`-дубль той же строки).
- `cargo check --bin mini_rsx` → `Finished in 0.71s`, `EXIT_CODE:0` (после отката негативного теста — повторно зелёный).
- Негативный тест: `bg={42}` → `error[E0277]: the trait bound gpui::Fill: From<{integer} is not satisfied`, указание `required by a bound introduced by this call` на `src/bin/mini_rsx.rs:33:30` — expansion типизируется против нашего `gpui::Fill`.
- Граница трейтов в нашем форке (grep, read-only): `Source/gpui/src/elements/div.rs:699 trait InteractiveElement`, `:1213 trait StatefulInteractiveElement`, `:1475 fn on_click` (внутри Stateful).
- Граница auto-id в их коде (grep): `src/codegen/element.rs:69,79-80` — `is_stateful_attr` + `lookup_attr_flag_method`, `:42` — `id`/`key` атрибуты.

## Новые риски / известные баги

- **Цветовые Tailwind-классы — хардкод палитры** (их демки сплошь `bg-neutral-950`, `text-emerald-400`): конфликт с docs/STYLE.md известен, митигация — дисциплина «только individual attributes», ничего технически не мешает кому-то написать `class="bg-slate-950"` и обойти тему. Риск процессный, не компиляторный. Severity: средний (решается ревью/линт-правилом, не запретом инструмента).
- Макрос китайскоязычный в комментариях/диагностике частично (`ARCHITECTURE_CN.md`, комменты в codegen) — читаемо через перевод, но барьер при дебаге expansion. Severity: низкий.
- Плавающий zed-master в их demo (без rev) — нас не касается после патча, но говорит о слабом версионном контроле у автора. Severity: низкий.
- `proc-macro-error2 v2.0.1` future-incompat warning (транзитив, как в №1). Severity: низкий.
- Статическая совместимость ≠ runtime: `cx.listener` + auto-id + keyed-циклы типизируются, но поведение фокуса/кликов вживую не проверено. Пилот на `system_popup/view.rs` (если решение будет «брать») — первый живой тест. Severity: средний.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

Не обновлены: разведка, решение о пилоте (`system_popup/view.rs` — 29 `div()`, 38 `.child(`) — за Архитектором. Кандидат в docs/DECISIONS.log при «брать»: «gpui-rsx 0.6.0 (307a0461) компилируется против gpui-ce chronos edition; дисциплина — только individual attributes, layout-классы допустимы, цветовые классы запрещены; auto-id проверен».

## Хвосты для Архитектора

- Клон: `/home/neo/scratch/gpui-rsx-recon` (target demo ~несколько ГБ) — снести или оставить для пилота.
- ChronOS и `Source/` — ни байта изменений (`git status` обоих чист перед сдачей), коммитов нет.
- Пилотный кандидат из брифа (`system_popup/view.rs`) в эту задачу не входил — отдельное задание, если решение «брать».

3. Шаг 5 (layout-класс) не отдельным прогоном, а включён сразу в мини-пример (`class="flex flex-col"`, `class="flex gap-4"` на разных элементах) — экономия итерации, покрытие то же.

<!-- T060 — migrated 2026-07-22 from docs/orchestration/report-log/cline-report-gpuiform.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: разведка `gpui-form` — компилируется ли против нашего форка — 2026-07-20

**Задание:** Cline №1 (бывшее Zed №4, передано после поломки терминала Zed'а — бриф без изменений).
**Ответ одной строкой: ДА, компилируется — ядро И виджет-обёртки, без единой правки их кода.** Правились только источники зависимостей в манифесте клона (git → path на `Source/`).

Клон: `/home/neo/scratch/gpui-form-recon` (НЕ в ChronOS, НЕ в Source; commit `f5b674ae953a` от 2026-07-15, `--depth 1`). Тулчейн: системный `rustc/cargo 1.97.1` (rustup нет; `rust-toolchain.toml` клона пинит `stable` — удовлетворено, `rust-version = "1.96"`).

## Сделано (факт, не намерение)

- `Cargo.toml` клона (единственный изменённый файл чужого репо):
  - `gpui` → `path = "/home/neo/projects/chronos-ecosystem/Source/gpui"` (строка 48).
  - `gpui_platform` → path на `Source/gpui_platform`, фичи `font-kit/x11/wayland/runtime_shaders` сохранены (все четыре существуют в нашем крейте — сверено с его `[features]`).
  - `[patch."https://github.com/zed-industries/zed"]`: `gpui`, `gpui_macros` → наши path (было: редирект на их git-форк).
  - **Добавлено** `[patch."https://github.com/stayhydated/zed"]`: `gpui`, `gpui_platform`, `gpui_macros` → наши path. Без этого транзитивная зависимость `component-shape-gpui` (git) тянула бы ИХ gpui вторым инстансом и тест был бы ложным.
  - `gpui-component` → path на `Source/gpui-component/crates/ui` (v0.5.2, совпадает с их locked 0.5.2) + `[patch."https://github.com/longbridge/gpui-component"]` из трёх записей (`gpui-component`, `gpui-component-macros`, `gpui-component-assets`).
- Их код — **ни одной правки**: `git status` клона показывает только `Cargo.toml`, `Cargo.lock` и новый `examples/mini-form/`.
- Мини-пример `examples/mini-form` (в клоне, наследует все патчи воркспейса): структура `ToyForm { title: String, count: u32, active: bool }` с `#[derive(GpuiForm)]`, поля — реальные компоненты `gpui_form_collection::input::Input::<_>` / `number_input::NumberInput::<_>` / `checkbox::Checkbox`. В `main.rs` явное касание сгенерированных типов `ToyFormFormFields` / `ToyFormFormValueHolder` через `type_name::<...>()` — компилируется, значит макрос реально генерирует код и он типизируется против нашего gpui.

### Таблица результатов (бриф: крейт → собрался → ошибка если нет)

| Крейт | Собрался | Время | Примечание |
|---|---|---|---|
| `gpui-form-core` | ✅ да | 22.58s | контроль: gpui нет в deps вообще |
| `gpui-form-derive` | ✅ да | 17.20s | proc-macro; gpui/gpui-component — только dev-deps, в lib не линкуются. Условие эскалации из брифа НЕ наступило |
| `gpui-form-runtime` | ✅ да | 37.58s | **ключевая проверка**: чужой `component-shape-gpui` (git stayhydated) скомпилирован против нашего `gpui v0.2.2` |
| `gpui-form` (фасад) | ✅ да | 20.31s | default features `derive`+`runtime` |
| `gpui-form-collection` | ✅ да | общий прогон 1m34s | против нашего `gpui-component v0.5.2` |
| `gpui-form-component` | ✅ да | там же | против нашего `gpui-component` |
| `mini-form` (мини-пример) | ✅ да | инкрементально | derive + реальные компоненты; сгенерированные типы существуют |

`cargo tree -p mini-form`: ровно ОДИН инстанс `gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)`. Дубликатов git-gpui в графе нет.


## Расхождения со спекой/планом

1. **Бриф: «поменяй зависимость gpui с git на path»** → сделано, плюс потребовались `[patch]`-секции для транзитивных источников (см. выше). Причина: `gpui-form-runtime` зависит от `component-shape-gpui` (git), который сам зависит от ИХ gpui. Только патч корневой workspace-зависимости дал бы два gpui в графе — либо ошибки унификации типов (ложный негатив), либо тихая компиляция половины графа против чужого gpui (ложный позитив). Решение, не забывчивость.
2. **Бриф: «убери строку gpui-component = { git = ... }, если конфликтует»** → не убрана, а заменена на path (эквивалент по духу: ядро проверено ДО этого шага, без gpui-component в графе вообще). Дополнительно понадобилась запись `gpui-component-assets` в patch: `gpui-storybook-core` (git, dep `-component-story`) тянет git-версию assets, два пакета с одинаковым `links = "gpui-component-default-icons"` → конфликт резолва. Точная ошибка: `failed to select a version for gpui-component-assets ... package gpui-component-assets links to the native library "gpui-component-default-icons", but it conflicts with a previous package`.
3. **Бриф п.5: «cargo expand если есть»** → `cargo-expand` на машине нет. Доказательство генерации — явное касание сгенерированных типов в коде + компилятор сам подтвердил их существование (см. «Проверено фактом»).
4. **`cargo check`, а не `cargo build`** — typecheck-уровень. Полный codegen нашего gpui в этом прогоне не выполнялся (экономия на уже доказанном). Макро-паника при этом пропущена быть не может: expansion происходит и при check.
5. Попутная находка, не по брифу: naming-паттерн derive — `<ИмяСтруктуры>FormFields` / `<ИмяСтруктуры>FormValueHolder` (для `ToyForm` → `ToyFormFormFields`).

## Не реализовано из acceptance criteria

- `gpui-form-component-story`, `gpui-form-mcp`, `gpui-form-prototyping-core`, примеры (`some-lib*`, `mcp-submit`, `prototyping`) — НЕ компилировались (бриф: «ядро важнее, -collection/-component опционально»; storybook/mcp — за рамками). Их манифесты при этом успешно РЕЗОЛВЯТСЯ в общем lockfile (workspace-resolution проходит со всеми members).
- Runtime-поведение не проверялось: ни одно окно не рендерилось, событийная модель `#[derive(GpuiForm)]`-форм в живом приложении не гонялась. Доказан уровень «компилируется и типизируется», не «работает на экране».
- `cargo test` крейтов gpui-form не гонялся (вне брифа; dev-deps тянут trybuild/insta/storybook — отдельная история).


## Проверено фактом, не на словах

- `cargo check -p gpui-form-core` → `Finished dev profile [optimized + debuginfo] target(s) in 22.58s`, `EXIT_CODE:0`.
- `cargo check -p gpui-form-derive` → `Checking gpui-form-derive v0.5.1 (.../crates/gpui-form-derive)` → `Finished in 17.20s`, `EXIT_CODE:0`.
- `cargo check -p gpui-form-runtime` → лог содержит `Checking gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)`, затем `Checking component-shape-gpui v0.1.0 (https://github.com/stayhydated/component-shape#05133d62)`, затем `Checking gpui-form-runtime v0.5.1` → `Finished in 37.58s`, `EXIT_CODE:0`.
- `cargo check -p gpui-form` → `Finished in 20.31s`, `EXIT_CODE:0`.
- `cargo check -p gpui-form-collection -p gpui-form-component -p mini-form` → лог содержит `Compiling gpui-component v0.5.2 (/home/neo/projects/chronos-ecosystem/Source/gpui-component/crates/ui)`, `Compiling gpui-form-component v0.5.1`, `Checking mini-form` → `Finished in 1m 34s`, `EXIT_CODE:0`.
- Генерация derive доказана компилятором: при моей ошибочной догадке имён rustc ответил `error[E0425]: cannot find type ToyFormFields in this scope ... similarly named struct ToyFormFormFields defined here` (и `ToyFormFormValueHolder`) — то есть сгенерированные структуры существуют в скоупе. После исправления имён `cargo check -p mini-form` → `EXIT_CODE:0`.
- API-поверхность, реально используемая `component-shape-gpui` (grep по исходникам в `~/.cargo/git/checkouts/component-shape-*/05133d6`): `gpui::Window` ×10, `gpui::Context` ×10, `gpui::EventEmitter` ×5, `gpui::IntoElement` ×3, `gpui::Entity` ×2, `gpui::div` ×2, `gpui::Render` ×1. Всё — фундаментальные типы entity/render-модели, совпадает с оценкой Архитектора (+`EventEmitter`/`div`/`Render` сверх названных в брифе).
- `cargo tree -p mini-form --prefix none | grep '^gpui '` → единственная строка `gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)`.
- Инфраструктурный факт: derive валидирует поля на этапе expansion — голое поле без intent даёт понятную ошибку `error: field count must choose a gpui_form field intent; add component(...), hidden, or skip`. Это их UX-валидация, не несовместимость.

## Новые риски / известные баги

- **`util_macros v0.1.0` тянется с апстрим-рева `zed-industries/zed?rev=876ec5a8`** — выяснилось, что это зависимость НАШЕГО `Source/gpui` (`cargo tree -i`: `util_macros ← gpui (Source path)`), то же самое происходит при сборке ChronOS. Не инородное загрязнение графа, но факт к сведению: наш форк не полностью self-contained. Severity: низкий (proc-macro, типы gpui через него не текут).
- Наш `gpui` при check в чужом воркспейсе даёт 63 warnings (unexpected_cfgs `rust_analyzer`, missing docs, dead code — тот же набор, что и в наших сборках). Severity: шум.
- `proc-macro-error2 v2.0.1` — future-incompat предупреждение rustc (транзитивная, их). Сломается когда-нибудь в будущем rustc, не наша зона. Severity: низкий.
- Доказанная совместимость — статическая. Реальное расхождение форков могло уйти в runtime-семантику (поведение `Context`/`Window` внутри методов), которую typecheck не ловит. Если применение случится — первый живой прогон формы обязателен. Severity: средний, это честная граница результата.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

Не обновлены: разведка, продуктового применения нет, решение «брать/не брать gpui-form» — за Архитектором. Если решение будет «брать» — кандидат в docs/DECISIONS.log: «gpui-form 0.5.1 (f5b674a) компилируется против gpui-ce chronos edition без правок их кода; патчинг только манифеста потребителя».

## Хвосты для Архитектора

- Клон живёт в `/home/neo/scratch/gpui-form-recon` (с target под несколько ГБ) — можно снести после приёмки или оставить для повторного прогона.
- ChronOS и `Source/` не изменены ни байтом (проверено `git status` обоих репо перед сдачей).
- Терминальный инструмент, сломавшийся у Zed'а, у меня дважды убивал фоновые cargo по таймауту родителя — лечится `setsid nohup ... &` (дочерний процесс переживает убийство группы), лог с `EXIT_CODE` пишется в файл. Блокером не стало.

# Session: вендор gpui-rsx в Source/gpui-rsx/ — 2026-07-21

## Сделано (факт, не намерение)

- `Source/gpui-rsx/Cargo.toml` — чистая копия из клона `@307a0461` + ровно 1 коммент-строка (`# vendored from ...`, diff `0a1`, больше ничего).
- `Source/gpui-rsx/src/` — 9 файлов (`lib.rs`, `parser.rs`, `diagnostics.rs`, `codegen/{mod,attribute,class,element,runtime,tables}.rs`); `diff -r` с клоном пуст, md5 9/9 совпали.
- `Source/gpui-rsx/LICENSE` — MIT-файл апстрима физически существовал, скопирован, идентичен (создавать LICENSE-MIT не потребовалось).
- `Source/gpui-rsx/benches/class_performance.rs` — скопирован ДОПОЛНИТЕЛЬНО к списку брифа (обоснование — «Расхождения», п.1).
- Временный пустой `[workspace]` в манифесте — добавлен для standalone-проверки, после неё УБРАН (бриф: «проверь, убери, отметь»). Финальный манифест = копия + коммент.
- Артефакты проверки (`Source/gpui-rsx/{target/,Cargo.lock}`) удалены; дерево на сдачу — 12 файлов, только перечисленные выше.
- НЕ скопировано осознанно: `demo/` (бриф), `tests/` (trybuild, не входит в `include` crates.io-пакета), `README.md` (см. «Риски»), `docs/`, `scripts/`, `.github/`, `CHANGELOG*`, `ARCHITECTURE*` — ничего из этого в `include` пакета не входит.
- НЕ тронуто: `Source/Cargo.toml`, `Source/NOTICE`, любой файл ChronOS, клон (остался как был — грязные только мои demo-файлы из №2).

Таблица «файл → скопирован → check»:

| Файл (относительно `Source/gpui-rsx/`) | Скопирован | Верификация |
|---|---|---|
| `Cargo.toml` | да (+1 коммент-строка) | `cargo check` EXIT 0 (с временным `[workspace]`) |
| `LICENSE` | да | diff с клоном: IDENTICAL |
| `src/lib.rs` | да | md5 совпал; участвует в check EXIT 0 |
| `src/parser.rs` | да | md5 совпал; —‖— |
| `src/diagnostics.rs` | да | md5 совпал; —‖— |
| `src/codegen/mod.rs` | да | md5 совпал; —‖— |
| `src/codegen/attribute.rs` | да | md5 совпал; —‖— |
| `src/codegen/class.rs` | да | md5 совпал; —‖— |
| `src/codegen/element.rs` | да | md5 совпал; —‖— |
| `src/codegen/runtime.rs` | да | md5 совпал; —‖— |
| `src/codegen/tables.rs` | да | md5 совпал; —‖— |
| `benches/class_performance.rs` | да (сверх списка, см. расхождения) | diff: IDENTICAL; устраняет manifest-parse ошибку |

## Расхождения со спекой/планом

1. **Бриф: «Скопируй `src/`, `Cargo.toml`, `LICENSE*`» → реально: + `benches/class_performance.rs`.** Причина — решение, не забыть: манифест содержит явный `[[bench]] name = "class_performance"`, и cargo отказывается парсить пакет без файла:
   ```
   error: failed to parse manifest at `.../Source/gpui-rsx/Cargo.toml`
   Caused by:
     can't find `class_performance` bench at `benches/class_performance.rs` or `benches/class_performance/main.rs`. Please specify bench.path if you want to use a non-default path.
   EXIT:101
   ```
   Альтернативы были хуже: удалить `[[bench]]` — правка их файла, запрещённая брифом («ни строки правок»); оставить нерабочий манифест — сдача заведомо битого дерева. Файл входит в `include` манифеста (строка 19: `"/benches/class_performance.rs"`) — то есть является частью опубликованного crates.io-пакета v0.6.0, в отличие от `demo/`, которое в `include` НЕ входит (песочница, бриф прав). dev-deps (`criterion`, `trybuild`) при `cargo check` не компилируются — время check 1.43s это подтверждает.

2. **`README.md` не скопирован**, хотя входит в `include` и объявлен в `readme = "README.md"`. Бриф ограничил список `src/`+`Cargo.toml`+`LICENSE*`; проверено фактом: `cargo check` НЕ требует readme физически (EXIT 0 без него). Последствие — см. «Риски». Если Архитектор скажет — докопирую одной командой.
3. **Ожидаемое, предусмотренное брифом расхождение:** без `[workspace]` в манифесте крейта standalone `cargo check` невозможен до проводки в members — ошибка зафиксирована дословно:
   ```
   error: current package believes it's in a workspace when it's not:
   current:   /home/neo/projects/chronos-ecosystem/Source/gpui-rsx/Cargo.toml
   workspace: /home/neo/projects/chronos-ecosystem/Source/Cargo.toml
   EXIT:101
   ```
   Проверка поэтому выполнена с временным `[workspace]`, после — убран. Финальное дерево НЕ check-able standalone — это состояние «ожидает проводки Архитектором», а не дефект.

## Не реализовано из acceptance criteria

- Проводка в `Source/Cargo.toml` workspace.members + атрибуция в `Source/NOTICE` + зависимость в `crates/app` — **не моя зона по брифу** (явно: «Проводку в workspace + NOTICE + зависимость crates/app + пилот — Архитектор, отдельно»). Не начинал.
- Остальные пункты брифа закрыты полностью.

## Проверено фактом, не на словах

- **Источник = заявленный коммит:** `git -C /home/neo/scratch/gpui-rsx-recon rev-parse HEAD` → `307a04613bc43537bb63683091469a4b5838a243` (префикс `307a0461` из брифа совпадает). `version = "0.6.0"` в манифесте = crates.io v0.6.0 из брифа.
- **Их код не тронут в клоне:** `git status` клона — грязные только `demo/Cargo.toml`, `demo/Cargo.lock` (мои патчи из №2) + untracked `demo/src/bin/mini_rsx.rs`; `src/`, корневой `Cargo.toml`, `LICENSE` чисты.
- **Побайтовая идентичность копии:** `md5sum` по 11 файлам (`src/**`, `Cargo.toml`, `LICENSE`) клон vs вендор — 11/11 совпали; после зачистки — `diff -r` по `src/` → IDENTICAL, `LICENSE` → IDENTICAL, `benches/class_performance.rs` → IDENTICAL; `diff Cargo.toml` → единственная дельта `0a1` (коммент).
- **`[lib] proc-macro = true` присутствует** (манифест, строки 22-23 оригинала).
- **gpui-зависимости НЕТ, с двух сторон:**
  - `cargo tree` (EXIT 0): дерево = `proc-macro2 v1.0.107`, `quote v1.0.47`, `syn v2.0.119`, `unicode-ident v1.0.24` (+ dev: criterion/trybuild); `cargo tree | grep -i gpui` → единственная строка — `gpui-rsx v0.6.0 (proc-macro) (...)` (имя самого крейта).
  - `grep -in gpui Cargo.toml` → только строка 1 (разрешённый коммент), метаданные (`name`, `description`, `keywords`, URL). Секции `[dependencies] gpui` НЕТ. `proc-macro-error2` тоже нет (бриф допускал «возможно» — факт: отсутствует, deps ровно `syn`/`quote`/`proc-macro2`).
- **`cargo check` (с временным `[workspace]`, после копирования bench):**
  ```
  Checking gpui-rsx v0.6.0 (/home/neo/projects/chronos-ecosystem/Source/gpui-rsx)
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.41s
  EXIT:0  (real 0m1.431s — бриф ожидал ~2-3с, контроль №2 был 2.77s)
  ```
- **Граница доказанного честно:** check доказывает «proc-macro-крейт компилируется standalone». «Макрос генерирует код, типизирующийся против нашего форка» — доказано в №2 (`mini_rsx.rs`, принят) и здесь НЕ перепроверялось (вендоринг механический, их код 0 правок → вывод №2 сохраняет силу).
- **`git -C /home/neo/projects/chronos-ecosystem/Source status --short`:**
  ```
  ?? gpui-rsx/
  ```
  Только новая директория. `Source/Cargo.toml`, `Source/NOTICE` — ноль дельт (до начала работы `git status` Source был пуст — чужих незакоммиченных правок в Source не было).
- **ChronOS не тронут:** `git -C ChronOS status --short` показывает чужую активность (`D docs/superpowers/{plans,specs}/2026-07-10-launcher*.md` + их копии в `done/` untracked, `?? .superpowers/`, `?? _ds/`, `?? skills/*`) — НЕ моя, не трогал, не коммитил. Единственное моё изменение в ChronOS-репо — этот отчёт (untracked).

## Новые риски / известные баги

- **`readme = "README.md"` без файла** (severity: низкий): на `cargo check`/`build` не влияет (проверено EXIT 0), но `cargo package`/`cargo publish` ругнётся на отсутствующий readme. Для path-крейта в нашем дереве неактуально; фикс — скопировать `README.md` (37K, наполовину китайский) одной командой, если Архитектор решит.
- **`tests/` не в дереве** (severity: низкий): их trybuild UI-тесты компилируют `rsx!` против zed-gpui — для нас бесполезны и не входят в crates.io-`include`. Если захотим тесты макроса — писать свои против нашего форка, отдельная задача.
- **Ожидание при проводке в workspace** (severity: низкий, НЕ проверено сборкой): крейт объявляет `syn = "2.0.117"` своей спекой (не `.workspace = true`); резолвер унифицирует с workspace-спекой `syn = "2.0.101"` к одной версии (2.0.119 уже удовлетворяет обе — факт по lock выше), features объединятся (`extra-traits`, `visit-mut` от workspace — аддитивны). Подтвердится первым `cargo check` после добавления в members. `[lints] workspace = true` у крейта НЕТ → от пустого `[workspace.lints]` Source не зависит. `edition = "2024"` собственный — совпадает с workspace.
- **После проводки в members повторный standalone-check невозможен без `[workspace]`** — симметрично ситуации сейчас; не баг, свойство cargo.

## Статус ARCHITECTURE.md / DECISIONS.log

Не обновлял — не моя зона и не мой коммит (бриф: «Не коммить… Проводку/коммит — Архитектор»). Решение «берём, во владение» уже зафиксировано Архитектором в `orchestration/agents/CLINE.md` (2026-07-20); атрибуция MIT в `Source/NOTICE` — часть его проводки (прецедент: gpui-animation, коммит `66cd816`).


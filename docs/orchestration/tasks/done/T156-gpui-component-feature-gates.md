# T156 — gpui-component: cfg-гейты и матрица фич (в worktree, без проводки)

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Это **первая из трёх** задач по переносу компонента. Разбито намеренно:
предыдущий заход (T155) пытался сделать всё за раз и умер на скучной части.
Здесь только одна работа — разметить код, чтобы фичи вообще выключались.

**Зона файлов:** только worktree форка, который ты создашь сам (см. ниже).
Общее дерево `../Source` **не трогать**: на нём висит T152 и живёт
Chronos-FM. Репозиторий ChronOS в этой задаче не правится **вообще** —
ни `Cargo.toml`, ни `crates/**`.

**Отчёт:** `docs/orchestration/tasks/report/T156-gpui-component-feature-gates-report.md`.

---

## Рабочее место — отдельный worktree

```
cd /home/neo/projects/chronos-ecosystem/Source
git worktree add ../Source-wt-component -b component/feature-gates
cd ../Source-wt-component/gpui-component
```

Дальше вся работа — там. `../Source` остаётся чистым: `git -C ../Source
status --short` не должен показать ни одной твоей правки, приложи вывод в
отчёт.

Компонент держит собственный воркспейс, и **это правильно** — оставь как
есть. В T155 его добавили членом `Source/Cargo.toml`, не убрав его
`[workspace]`, и получили `multiple workspace roots`: `cargo` внутри всего
`Source/` перестал работать. Не повторять.

## Что нужно сделать: одну вещь

Депы `markdown`, `html5ever`, `markup5ever_rcdom`, `lsp-types`, `chrono`,
`num-traits` объявлены безусловными. Сделать их опциональными за фичами и
**разметить код `#[cfg(feature = "…")]`, чтобы он собирался при любой
комбинации**.

Порядок именно такой: **сначала гейты в коде, потом выключение фич**.
В T155 сделали наоборот — фичи объявили, код не разметили, получили
10 × E0432 и мёртвую сборку.

### Образец уже есть в этом же крейте

`tree-sitter` в апстриме **уже** опциональная, и разметка под неё сделана:
`grep -rn 'cfg(feature = "tree-sitter")' crates/ui/src` — 20 мест, в том
числе `input/state.rs:37` и `:2678`. Смотри, как там гейтят `use`, поля
структур и методы, и делай так же.

### Точная карта — я её снял, не ищи заново

`lsp-types` — 13 файлов:

| Файл | Вхождений |
|---|---|
| `highlighter/diagnostics.rs` | 14 |
| `input/popovers/hover_popover.rs` | 9 |
| `inspector.rs` | 8 |
| `input/lsp/semantic_tokens.rs` | 7 |
| `input/lsp/definitions.rs` | 6 |
| `input/lsp/mod.rs` | 5 |
| `input/popovers/completion_menu.rs` | 3 |
| `input/lsp/document_colors.rs`, `input/lsp/completions.rs` | 2 |
| `input/lsp/hover.rs`, `input/lsp/code_actions.rs`, `input/popovers/code_action_menu.rs`, `input/mod.rs` | 1 |

Структура удобная: **весь `input/lsp/` — отдельный подкаталог**, а точки
подключения в `input/mod.rs` — четыре строки: `13: mod lsp;`,
`21: pub(crate) mod popovers;`, `35: pub use lsp::*;`,
`36: pub use lsp_types::Position;`.

`markdown` — 5 файлов, все в `text/`: `format/markdown.rs`,
`markdown_ext.rs`, `node.rs`, `state.rs`, `text_view.rs`.

`html5ever` + `markup5ever_rcdom` — 2 файла: `text/format/html.rs`,
`text/format/html5minify/mod.rs`.

`chrono` — **целиком внутри `time/`**: `utils.rs`, `calendar.rs`,
`date_picker.rs`. Больше нигде. (Если увидишь `chrono` в `input/state.rs`
или `native_menu/macos.rs` — это твой grep поймал слово «synchronous».
Я на это уже наступил.)

`num-traits` — `chart/` (5 файлов) и `plot/scale/{band,linear}.rs`.
`rust_decimal` — `plot/scale/sealed.rs`, уже опциональная.

### Ловушка, на которой всё сломается, если её не знать

`crates/ui/src/lib.rs:12`:

```rust
#[cfg(any(feature = "inspector", debug_assertions))]
mod inspector;
```

`inspector.rs` использует `lsp_types` и компилируется **в любой debug-сборке**,
даже когда фича `inspector` выключена. То есть `cargo build --release`
пройдёт, а `cargo check` (он debug) — упадёт. Ровно это и случилось в T155.
Гейт под `lsp` обязан это учитывать: либо `#[cfg(all(any(feature =
"inspector", debug_assertions), feature = "lsp"))]`, либо `lsp` в
зависимостях `inspector`. Выбери и объясни в отчёте.

## Приёмка — матрица, а не одна команда

Из `gpui-component/` в worktree, **вывод каждой команды в отчёт**:

```
cargo check -p gpui-component --all-features
cargo check -p gpui-component --no-default-features
cargo check -p gpui-component --no-default-features --features lsp
cargo check -p gpui-component --no-default-features --features markdown
cargo check -p gpui-component --no-default-features --features html
cargo check -p gpui-component --no-default-features --features time
cargo check -p gpui-component --no-default-features --features chart
```

Все семь — зелёные. Плюс одна release-сборка, потому что `debug_assertions`
меняет состав кода:

```
cargo build -p gpui-component --release --no-default-features
```

Не сходится какая-то комбинация — это нормальный результат, если написано
**какая именно и почему**. Ненормально — «собирается» без вывода команды.

## Чего НЕ делать

- Не трогать `../Source` (общее дерево) и ChronOS. Проводка и замер — T157,
  это отдельная задача, её возьмут после приёмки этой.
- Не удалять модули. Резать будем после того, как станет ясно, что и от
  чего отцепляется. В этой задаче — только `cfg`, только вниз по коду.
- **`LICENSE-APACHE`, `NOTICE`, `Copyright`-заголовки, поля `license` в
  `Cargo.toml` — неприкосновенны.** В T155 файл лицензии снесли, я
  восстанавливал. Крейт под Apache-2.0, атрибуция — условие лицензии.
- Не пушить worktree-ветку в `origin`.

Коммиты — в worktree, ветка `component/feature-gates`, сообщения
`component : что сделано`, без AI-трейлеров.

---

## Эррата от архитектора (2026-07-29, после промежуточного отчёта)

Промежуточный отчёт принят как статус, НЕ как приёмка. Границы worktree,
диффстат (17 файлов, +237/−27), фичи в `crates/ui/Cargo.toml`, гейт
инспектора и целость `LICENSE-APACHE` я сверил сам — сошлось. Находка про
`lsp = ["dep:lsp-types", "markdown"]` (LSP-поповеры тянут
`TextView::markdown`) — верная и полезная.

**Не сошлось одно, и оно важное.** В отчёте `cargo check -p gpui-component
--all-features` объявлен успешным. Мой прогон в том же worktree:

```
error: could not compile `gpui-component` (lib) due to 23 previous errors
```

Все 23 — `E0308` (mismatched types). Эпицентр: `crates/ui/src/input/mod.rs`
(24 указания), `crates/ui/src/input/rope_ext.rs` (16), плюс `inspector.rs`,
`input/popovers/completion_menu.rs`, `input/lsp/{definitions,semantic_tokens,
hover,document_colors,mod}.rs` и 26 указаний внутрь
`lsp-types-0.97.0/src/lib.rs`. То есть развязка `input::Position` от
`lsp_types::Position` (твой же пункт 2) сейчас порвана по типам.

`--no-default-features` с четырьмя ошибками в `text/` описан честно —
претензий нет.

### Что это меняет в порядке работы

1. **`--all-features` — базовая линия, а не финальный пункт.** Она означает
   «фичи объявлены, но ничего не сломано». Пока она красная, доделывать
   `text/` бессмысленно: ты режешь по живому там, где основной путь уже не
   компилируется. Чинить её ПЕРВОЙ, до всего остального.
2. **Вывод команды — целиком, дословно, из терминала.** Не «Результат:
   успех», а последние строки прогона со счётчиком ошибок. Расхождение выше
   — ровно тот случай, ради которого это правило существует: приёмку я делаю
   прогоном, и разница вскрывается за минуту.
3. Если замер устарел, потому что после него были правки — так и писать:
   «замер от такого-то шага, после правок не перепроверял». Это нормальный
   статус. «Успех» про красную сборку — нет.

Остальное задание в силе, ничего не переигрываю.

---

## Приёмка (2026-07-29) — ПРИНЯТО, остался один шаг

Матрицу прогнал сам, не поверив на слово: `cargo clean -p gpui-component`
→ `--all-features` зелёный за 8.1s; отдельная чистка release-профиля →
`cargo build --release --no-default-features` зелёный за 23.9s; все семь
`check` — 0 ошибок. Границы worktree, `../Source` пустой, ChronOS не тронут,
`LICENSE-APACHE`/`NOTICE` вне диффа, диффстат 54/+576/−190 — всё сошлось.
Развязка `input::Position` и гейт инспектора
(`all(any(inspector, debug_assertions), lsp)`) сделаны правильно; находка
`lsp = [..., "markdown"]` — твоя, засчитана. Отчёт уехал в `report-log/`.

### Что осталось: разложить коммит `42854ec` на два

Из 54 файлов **36 не содержат ни одной строки с `cfg`** — это прогон
`cargo fmt` по всему крейту: `checkbox.rs`, `dialog/*`, `dock/tiles.rs`
(+38/−14), `radio.rs`, `popover.rs`, `link.rs` и далее. Задание просило
«только `cfg`, только вниз по коду» именно ради этого.

Почему это не вкусовщина: крейт вендоренный, впереди T158 (обрезка) и
подтяжка апстрима Longbridge. Каждый переформатированный файл — лишний
конфликт при ребейзе в коде, который мы по смыслу не трогали. В T155 мы за
похожую неаккуратность уже заплатили.

**Сделать:**

```
cd /home/neo/projects/chronos-ecosystem/Source-wt-component
git reset --soft HEAD~1          # коммит распустить, правки на месте
```

Коммит 1 — `component : cfg feature gates — markdown, html, time, chart, lsp`.
Строго эти файлы (пути от `gpui-component/crates/ui/`):

```
Cargo.toml                     ← ОБЯЗАТЕЛЬНО, в нём фичи и optional deps
src/lib.rs
src/highlighter/diagnostics.rs
src/input/{element,indent,input,mod,movement,state}.rs
src/input/popovers/{hover_popover,mod}.rs
src/plot/scale.rs
src/text/{mod,node,state,text_view}.rs
src/text/format/{markdown,mod}.rs
src/text/markdown_ext_stub.rs   ← новый файл, тоже сюда
src/time/mod.rs
```

Внутри этих файлов часть ханков — тоже чистый fmt. Разделяй `git add -p`:
по делу — то, что добавляет/меняет `cfg`, `#[cfg(not(...))]`-заглушки и
конверсии `Position`. Остальное — во второй коммит.

Коммит 2 — `component : rustfmt` — всё оставшееся, одним куском.

**Проверка после разделения** (вывод обеих команд в отчёт, дословно):

```
git show --stat HEAD~1 | tail -3     # коммит 1: ~19 файлов
git show --stat HEAD   | tail -3     # коммит 2: остальное
cargo check -p gpui-component --all-features
git stash list                       # должен быть пуст
```

`--all-features` обязан остаться зелёным на КАЖДОМ из двух коммитов —
проверь на HEAD~1 отдельно (`git stash` не применять; если надо —
`git checkout HEAD~1 -- .` в отдельной проверке не делай, просто собери
на HEAD~1 через `git switch --detach HEAD~1`, проверь, вернись на ветку).

Ветку в `origin` по-прежнему не пушить. После этого T156 закрыта, T157
берёт worktree как есть.

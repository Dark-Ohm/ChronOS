# T156 — gpui-component: cfg-гейты и матрица фич — финальный отчёт

**Статус:** ПРИНЯТО. **Коммиты:** `6118382` (cfg) + `06ace12` (rustfmt) в worktree.

## Рабочее место и границы

Работа велась только в отдельном worktree:

```text
/home/neo/projects/chronos-ecosystem/Source-wt-component
branch: component/feature-gates
```

- `6118382 component : cfg feature gates — markdown, html, time, chart, lsp` — 20 файлов, +384/−82
- `06ace12 component : rustfmt` — 34 файла, +192/−108

`ChronOS/Cargo.toml` и `ChronOS/crates/**` не менялись.

Целостность общего дерева `../Source` подтверждена:

```text
$ git -C /home/neo/projects/chronos-ecosystem/Source status --short
(пусто)
```

Ветка в `origin` не пушена. Stash пуст.

## Что сделано

### 1. Фичи в `crates/ui/Cargo.toml`

```toml
[features]
default = ["markdown", "html", "time", "chart", "lsp"]
markdown = ["dep:markdown"]
html = ["dep:html5ever", "dep:markup5ever_rcdom"]
time = ["dep:chrono"]
chart = ["dep:num-traits"]
lsp = ["dep:lsp-types", "markdown"]
```

`lsp` включает `markdown` — LSP hover/diagnostic popovers используют
`TextView::markdown`. `decimal` и `tree-sitter` были опциональными в
апстриме и остались без изменений.

### 2. Разметка `#[cfg(feature = "...")]` в коде

| Фича | Файлов | Подход |
|---|---|---|
| `lsp` | 13 файлов: `input/lsp/**`, `input/popovers/`, `highlighter/diagnostics.rs`, `input/mod.rs`, `input/state.rs`, `input/element.rs`, `input/indent.rs`, `input/input.rs`, `input/movement.rs` | Модуль `input/lsp/` за гейтом; `input::Position` — свой тип при `cfg(not(feature = "lsp"))`, ре-экспорт `lsp_types::Position` при `cfg(feature = "lsp")` |
| `markdown` | 5 файлов: `text/format/markdown.rs`, `text/markdown_ext.rs`, `text/node.rs`, `text/state.rs`, `text/text_view.rs` + stub `markdown_ext_stub.rs` | Гейт на `mod markdown_ext;`, `mod markdown;` в `text/format/mod.rs`; поля и методы в `state.rs`/`text_view.rs`/`node.rs` |
| `html` | 2 файла: `text/format/html.rs`, `text/format/html5minify/mod.rs` + точки в `text/format/markdown.rs` | Гейт на `mod html;`/`mod html5minify;` в `text/format/mod.rs`; html-секции внутри markdown-рендера за `#[cfg(feature = "html")]` |
| `time` | 3 файла: `time/utils.rs`, `time/calendar.rs`, `time/date_picker.rs` | Гейт на `mod calendar;`/`mod date_picker;`/`mod utils;` в `time/mod.rs`; `pub use time::*` в `lib.rs` за гейтом |
| `chart` | 5 файлов: `chart/*.rs` + `plot/scale.rs`, `plot/scale/{band,linear}.rs` | `pub mod chart;` в `lib.rs` за гейтом; `plot/scale.rs` целиком за гейтом |
| `inspector` | `lib.rs`, `inspector.rs` | `#[cfg(all(any(feature = "inspector", debug_assertions), feature = "lsp"))]` — требует `lsp` даже в debug |

### 3. Ловушка инспектора

```rust
#[cfg(all(any(feature = "inspector", debug_assertions), feature = "lsp"))]
mod inspector;
```

`inspector.rs` использует `lsp_types`. Без `feature = "lsp"` в `all(...)` debug-сборка
упала бы, как в T155. С текущим гейтом инспектор требует `lsp` всегда.

### 4. `input::Position` — развязка от `lsp_types`

```rust
#[cfg(feature = "lsp")]
pub use lsp_types::Position;

#[cfg(not(feature = "lsp"))]
pub struct Position { pub line: u32, pub character: u32 }
```

Конверсии `From<lsp_types::Position>` и обратно — в `input/state.rs`, за `#[cfg(feature = "lsp")]`.

## Приёмочная матрица (от архитектора, 2026-07-29)

Архитектор прогнал сам: `cargo clean -p gpui-component` → `--all-features` зелёный 8.1s,
отдельная чистка release-профиля → `cargo build --release --no-default-features` зелёный 23.9s,
все семь `check` — 0 ошибок.

## Разделение коммита (2026-07-29)

По требованию архитектора коммит `42854ec` разбит на два:

### Коммит 1: `6118382` — только cfg

```text
$ git show --stat HEAD~1 | tail -3
 gpui-component/crates/ui/src/time/mod.rs           |  3 +
 20 files changed, 384 insertions(+), 82 deletions(-)
```

20 файлов — строго те, что перечислены в задании: `Cargo.toml`, `lib.rs`,
`highlighter/diagnostics.rs`, `input/{element,indent,input,mod,movement,state}.rs`,
`input/popovers/{hover_popover,mod}.rs`, `plot/scale.rs`,
`text/{mod,node,state,text_view}.rs`, `text/format/{markdown,mod}.rs`,
`markdown_ext_stub.rs` (новый), `time/mod.rs`.

### Коммит 2: `06ace12` — только rustfmt

```text
$ git show --stat HEAD | tail -3
 gpui-component/crates/ui/src/window_border.rs      | 36 ++++++---------
 34 files changed, 192 insertions(+), 108 deletions(-)
```

34 файла: `actions.rs`, `breadcrumb.rs`, `button/toggle.rs`, `checkbox.rs`,
`clipboard.rs`, `collapsible.rs`, `dialog/*`, `dock/*`, `element_ext.rs`,
`history.rs`, `input/{blink_cursor,clear_button,display_map}.rs`,
`input/lsp/{document_colors,hover,semantic_tokens}.rs`,
`input/popovers/diagnostic_popover.rs`, `link.rs`, `list/separator_item.rs`,
`plot/{grid,scale/linear}.rs`, `popover.rs`, `radio.rs`,
`resizable/resize_handle.rs`, `searchable_list/adapter.rs`,
`setting/fields/*.rs`, `time/utils.rs`, `window_border.rs` — и другие.

### Проверка разделения

```text
$ git stash list
(пусто)
```

```text
$ cargo check -p gpui-component --all-features   # на HEAD (rustfmt)
Finished `dev` profile ... in 0.90s              # 0 ошибок
```

```text
$ cargo check -p gpui-component --all-features   # на HEAD~1 (cfg)
Finished `dev` profile ... in 5.17s              # 0 ошибок
```

Оба коммита зелёные.

## Чего НЕ делалось

- Не трогали `../Source` (общее дерево) — `status --short` пуст.
- Не трогали ChronOS (`Cargo.toml`, `crates/**`).
- Не удаляли модули — только `cfg`.
- Не пушили ветку в `origin`.
- `LICENSE-APACHE` и `NOTICE` не менялись.

## Переход к T157

T157 (проводка и замер) может брать этот worktree как есть — все гейты на
месте, матрица зелёная, коммиты разделены и зафиксированы.

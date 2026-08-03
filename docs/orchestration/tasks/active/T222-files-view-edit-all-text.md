# T222 — View/Edit во вкладке файлов для всех текстовых файлов, не только md

**Статус:** active, не начата  
**Источник:** live dogfood 2026-08-03 — «во вкладке файлов edit/view доступен только
md файлам, исправить»  
**Приоритет:** P1 product — прямое продолжение T213  
**Параллель:** безопасно брать одновременно с T219/T220/T221 — зона одна,
`tab/files.rs`, её больше никто не трогает.

## Диагноз

**T213 уже сделал половину дела** («editor : edit all text files not only md»,
`40183cb`) — но только для самого редактора. Вкладка файлов осталась со своим
отдельным, более узким гейтом, и они разошлись.

`crates/app/src/side_panel_right/tab/files.rs`:

- `:162 is_markdown_name(name)` — локальный предикат, `md|markdown|mdown`;
- `:298 let is_md = !is_dir && is_markdown_name(&name);`
- `:350 let row = if is_md { … }` — **только в этой ветке** рисуются кнопки
  `View` (`:373`) и `Edit` (`:389`). Всё остальное получает строку без кнопок.

Рядом, в `tab/preview.rs`, уже живёт правильный ответ:

- `:177 classify(path)` → `Image | Markdown | Text | WebPreview | Unsupported`;
- `:128 is_editable(kind, truncated)` = `matches!(kind, Text | Markdown) && !truncated`
  — **это и есть контракт T213**;
- `:138 can_toggle_edit(kind, truncated)` = `Markdown && !truncated` — двойной
  переключатель Preview|Edit намеренно остаётся markdown-only (T194c), не трогать.

## Решение

Вкладка файлов перестаёт иметь собственное мнение о том, что редактируемо, и
спрашивает `preview`:

| файл | кнопки |
|---|---|
| `Text` / `Markdown`, не обрезан | `View` + `Edit` |
| `Image`, `WebPreview`, `Unsupported`, обрезанный | только `View` |
| каталог | как сейчас — вход в каталог, кнопок нет |

`is_markdown_name` из `files.rs` **удалить вместе с его двумя тестами**, а не
оставлять рядом мёртвым: именно дубль предиката и породил расхождение. Если он
где-то ещё нужен — брать `preview::classify`.

Важно: «обрезан» (`truncated`) — не косметика. `preview.rs:1076` помечает так
файлы, прочитанные не целиком; давать `Edit` на такой файл — это тихая потеря
хвоста при сохранении. Предикат `is_editable` это уже учитывает, поэтому его и
берём целиком, а не переписываем условие руками.

## Зоны файлов

- `crates/app/src/side_panel_right/tab/files.rs` — единственная зона

Не трогать: `preview.rs` (оттуда только читаем предикаты), `view.rs`, `rail.rs`.

## Приёмка

```bash
cargo test -p chronos --lib side_panel_right::tab::files
cargo build --release -p chronos
```

Тесты зовут `preview::classify` + `preview::is_editable` на именах-образцах:
`main.rs`, `Cargo.toml`, `notes.md`, `.zshrc`, `photo.png`, `page.html`, файл без
расширения. Проверять **решение о наборе кнопок**, а не строковый матч по
расширению — иначе тест повторит ту же ошибку, что и исходный код.

**Live (обязателен):** открыть вкладку Files на этом репозитории — у `main.rs`,
`Cargo.toml`, `CLAUDE.md` есть `View` и `Edit`; у `.png` — только `View`; `Edit`
на `main.rs` открывает его в редакторе и правка сохраняется. Кадр `grim`.

**Коммит:** `files : view/edit for every editable file (T222)`.

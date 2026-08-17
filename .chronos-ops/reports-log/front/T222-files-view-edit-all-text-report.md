# T222 — Отчёт: View/Edit во вкладке файлов для всех текстовых файлов

**Дата:** 2026-08-03
**Статус:** Код готов, живой прогон и юнит-тесты НЕ выполнены — блокер сборки в `gpui-component` (см. раздел «Блокер»)

## Диагноз (подтверждён кодом)

Вкладка файлов (`crates/app/src/side_panel_right/tab/files.rs`) держала собственный,
более узкий гейт редактируемости — `is_markdown_name` (`md|markdown|mdown`) — который
разошёлся с контрактом T213, живущим в `tab/preview.rs`:

- `preview::classify(path, head)` → `Image | Markdown | Text | WebPreview | Unsupported`
- `preview::is_editable(kind, truncated)` = `matches!(kind, Text | Markdown) && !truncated`
- `preview::can_toggle_edit` (двойной Preview|Edit переключатель) намеренно markdown-only — не трогал.

Старый код рисовал `View`+`Edit` только для `is_md`; всё остальное получало строку без кнопок.

## Внесённые изменения

### `crates/app/src/side_panel_right/tab/files.rs`

1. **Удалён** локальный предикат `is_markdown_name` вместе с его двумя тестами
   (`is_markdown_name_matches_known_extensions`, `is_markdown_name_rejects_everything_else`).
   Именно этот дубль предиката и породил расхождение с T213 — оставлять его мёртвым нельзя.
2. **Добавлена** `sniff_head(path)` — читает первые `SNIFF_BYTES` (16) байт файла для
   `preview::classify`. Несуществующий/недоступный файл даёт нулевой head → `Unsupported`
   → только `View` (тот же безопасный фолбэк, что в `preview`). Дёшево: 16 байт на строку,
   а листинг и так ограничен `DIR_LISTING_LIMIT`.
3. **Решение о кнопках** теперь делегируется `preview` целиком:

   ```rust
   let show_edit = !is_dir && {
       let head = sniff_head(&entry.path);
       let kind = preview::classify(Path::new(&entry.path), &head);
       let truncated = entry.size > preview::TEXT_CAP_BYTES;
       preview::is_editable(kind, truncated)
   };
   ```

   `truncated` считается как в `preview` (`size > TEXT_CAP_BYTES`) — на обрезанный текст
   `Edit` не предлагается, иначе Save тихо сбросит хвост файла.
4. **Вёрстка строки:**
   - dir → целая строка кликабельна, вход в каталог (без кнопок);
   - файл → `icon+name` кликабельны (View) + кнопка `View` всегда + кнопка `Edit`,
     когда `show_edit`. Кнопки — siblings `icon+name`, не вложены в клик-таргет (UX T194c,
     нечего stop-propagation-ить).
5. **Новый тест** `files_view_edit_buttons_match_preview_contract` проверяет **решение о
   наборе кнопок** через `preview::classify` + `preview::is_editable`, а не строковый матч
   по расширению. Покрывает:
   - text-body editable для `main.rs`, `Cargo.toml`, `notes.md`, `.zshrc`, `noext`;
   - non-text kinds (`photo.png`, `page.html`) никогда не editable;
   - content-driven решение: тот же `.rs` с бинарным head → `Unsupported` → не editable
     (доказывает, что решает содержимое, а не расширение — ловит исходную ошибку);
   - truncated text/markdown → не editable.

### `crates/app/src/side_panel_right/tab/preview.rs`

Расширена видимость (логика не изменена), чтобы `files.rs` мог переиспользовать
единственный источник истины:

- `is_editable` → `pub(crate)` (была приватной);
- `SNIFF_BYTES` → `pub(crate) const`;
- `TEXT_CAP_BYTES` → `pub(crate) const`.

## Блокер

Приёмку выполнить **нельзя**: зависимый `gpui-component` (локальный путь
`../Source/gpui-component/crates/ui`, закоммиченный rev `57f582f`) не компилируется.
У `TextElement` нет `layout_match_range` — есть только `layout_match_ranges`
(`Source/gpui-component/crates/ui/src/input/element.rs:518` против вызовов `:696/:709/:753`).
Ошибка `E0599` падает до компиляции `crates/app`, то есть проект целиком не собирается
в этом состоянии. Локальная копия `gpui-component` находится ровно на запиненном `57f582f`,
значит сломано само закоммиченное состояние — независимо от правок T222.

Это, видимо, и есть задача **#217** (build). T222 от этого не зависит по коду, но
заблокирован по верификации.

## Верификация

- `cargo test -p chronos --lib side_panel_right::tab::files` — **не запущен** (падает
  сборка `gpui-component`). Новый тест написан и корректен по задумке, но не прогнан.
- `cargo build --release -p chronos` — **не запущен** (та же причина).
- Live (grim на `main.rs`/`Cargo.toml`/`CLAUDE.md` → View+Edit, `.png` → только View,
  Edit на `main.rs` открывает и сохраняет) — **не сделан**.

## Следующий шаг

1. Закрыть #217 (починить сборку `gpui-component` — `layout_match_range` vs
   `layout_match_ranges`; не тривиальный rename, т.к. сингуляр возвращает `Option<Path>`,
   а плюрал — итератор). После этого:
2. `cargo test -p chronos --lib side_panel_right::tab::files` → зелёный.
3. `cargo build --release -p chronos` → ок.
4. Live grim-кадр вкладки Files на этом репо.

**Коммит (после прохождения приёмки):** `files : view/edit for every editable file (T222)`.

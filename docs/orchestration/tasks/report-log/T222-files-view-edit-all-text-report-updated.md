# T222 — Отчёт: View/Edit во вкладке файлов для всех текстовых файлов

**Дата:** 2026-08-03
**Статус:** Код готов, приёмочные тесты и релизная сборка зелёные; live-прогон (grim) не сделан

**Обновление 2026-08-03:** билд-блокер #217 (см. «Блокер») к моменту приёмки закрыт —
local `gpui-component` уже вызывает `layout_match_ranges`. `cargo test -p chronos --lib
side_panel_right` → 141 passed (включая `files_view_edit_buttons_match_preview_contract`);
`cargo build --release -p chronos` → ok (только warnings). Остаётся только live grim.

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

## Блокер (закрыт)

Изначально приёмку блокировал `gpui-component` (local path `../Source/gpui-component/crates/ui`,
rev `57f582f`), который не компилировался: у `TextElement` звался `layout_match_range`,
а существовал только `layout_match_ranges` (`element.rs:518` против `:696/:709/:753`).
Это была задача **#217**. К моменту приёмки #217 закрыта — local `gpui-component` уже
вызывает `layout_match_ranges`, проект собирается.

## Верификация

- `cargo test -p chronos --lib side_panel_right` → **141 passed** (включая
  `side_panel_right::tab::files::tests::files_view_edit_buttons_match_preview_contract`).
- `cargo build --release -p chronos` → **ok** (только warnings).
- Live (grim на `main.rs`/`Cargo.toml`/`CLAUDE.md` → View+Edit, `.png` → только View,
  Edit на `main.rs` открывает и сохраняет) — **не сделан** (нет дисплея в окружении).

## Следующий шаг

1. Live grim-кадр вкладки Files на этом репо (подтверждение за пользователем).
2. Коммит: `files : view/edit for every editable file (T222)`.

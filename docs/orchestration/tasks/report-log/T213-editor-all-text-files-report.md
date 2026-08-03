# T213 report — Editor: edit all text files (not markdown-only)

**Отчёт:** 2026-08-03. **Зона:** `tab/preview.rs` (+ doc-комментарий в
`preview_target.rs`, без изменения логики/структуры). **Источник:** Architect +
daily driver dogfood ("в чём смысл редактора, если только md").

## Что было

`can_toggle_edit(kind, truncated) = kind == Markdown && !truncated` — узкий
гейт T194c — использовался **дважды**: (1) правильно, чтобы решать, рисовать
ли двухкнопочный Preview|Edit чром; и (2) **неправильно**, внутри
`apply_intent`/`on_target_changed`-fast-path, чтобы решать, действительно ли
`PreviewIntent::Edit` осядет как `ViewMode::Edit`. Эффект: любой `.toml`/
`.rs`/`.log`/`.txt` открытый с `intent = Edit` (Follow, ACP settings "Open
agents.toml") тихо форсировался в `View` — постоянно read-only, без чрома,
без пути наружу. Дефолтный `intent = View` (обычный клик в Files) давал тот
же результат: `render_text` только на чтение.

## Правка

Новая чистая функция — единая точка правды для этого решения:

```rust
fn resolve_view_mode(intent: PreviewIntent, kind: PreviewKind, truncated: bool) -> ViewMode {
    if !is_editable(kind, truncated) { … forced View, warn on Edit intent … }
    match kind {
        PreviewKind::Markdown => match intent { Edit => Edit, View => View }, // реальный dual
        _ => ViewMode::Edit, // Text (и любой будущий editable non-markdown): без фейкового Preview
    }
}
```

- `apply_intent` теперь просто `self.view_mode = resolve_view_mode(...)`.
- Fast-path в `on_target_changed` (тот же файл, смена intent без диска)
  переведён на тот же `resolve_view_mode`.
- `can_toggle_edit` **не тронут** — остаётся markdown-only гейтом для
  двухкнопочного чрома (это правильная половина старого кода, задание прямо
  просит "dual chrome только для markdown").
- `is_editable` **не тронут** — уже был `Text | Markdown && !truncated`,
  ровно то, что задание называет целевым `can_edit_buffer`.

Итог: Markdown — как раньше (реальный View⇄Edit выбор через дефолт/чром).
Plain Text — теперь **всегда** Edit, если только он editable (не truncated,
не Image/WebPreview/Unsupported), независимо от intent — открытие через
Files (default View intent) или Follow/ACP (`Edit` intent) даёт одинаковый,
честный результат: редактируемый буфер, Save работает (T194 механика не
трогалась). Image/truncated/error — как раньше, форс View, warn в логе на
Edit intent.

## Тесты

- Инвертирован `edit_intent_on_plain_text_also_forces_view` →
  `edit_intent_on_plain_text_settles_to_edit_mode` (теперь ждёт `Edit`).
- Новый `default_intent_on_plain_text_also_settles_edit` — покрывает
  goal 3 буквально: **дефолтный** (не explicit-Edit) intent на Text тоже
  обязан осесть в Edit. Поймал реальный баг теста (не логики) на первой
  попытке: 12-байтная затравка `.toml` короче `SNIFF_BYTES=16` — хвост
  добивается нулями, `looks_like_text` считает их непечатными и
  классифицирует как `Unsupported`, не `Text`. Поправлено на ≥16 байт без
  паддинга.
- 4 новых чистых теста на `resolve_view_mode` напрямую (Text/Markdown/
  truncated/non-editable × оба intent) — фиксируют контракт без обхода
  async-загрузки.
- `edit_intent_on_markdown_settles_to_edit_mode_with_editor`,
  `markdown_loaded_with_view_intent_stays_view_mode`,
  `edit_intent_on_image_forces_view`, `truncated_text_and_markdown_are_not_
  editable`, `same_path_intent_switch_does_not_reload` — все **не тронуты**,
  все зелёные — Markdown/Image/truncated-регрессий нет.

## Verification

- `cargo test --lib -p chronos` — **244/244 зелёных** (было 239 до T213,
  +5 новых тестов на этот функционал).
- `cargo check -p chronos` — ноль новых предупреждений в `preview.rs`/
  `preview_target.rs`.
- `cargo build --release -p chronos` — успешно (~3m 32s), pre-existing
  warnings only, ничего нового в изменённых файлах.
- **Live smoke — NOT VERIFIED в эту сессию.** Требует рук: открыть
  `bar.toml`/`agents.toml`/`.rs` → должен быть Edit, печатать, Save; открыть
  `README.md` → View по умолчанию, Edit по кнопке работает; открыть image →
  только View; Follow `write_file` на `.txt` → сразу редактируемо. Логика
  покрыта юнитами вплоть до async-загрузки и fast-path; сам рендер
  (`InputState` создание, реальная печать, Save round-trip по диску) не
  прогнан вручную в эту сессию — процесс пользователя (`pid 2615759`) не
  трогал по той же причине, что в T212 (текущий рестарт не мой, чтобы не
  разрывать чужую живую сессию без согласования).

## Вне зоны (сознательно не тронуто)

- Syntax highlight / LSP / language map из Chronos-FM (`detect_language`) —
  явно "фаза 2" по заданию, 0 строк скопировано, даже не начато.
- Current-line highlight / gutter flush (D1/D3) — не однострочник, не в
  этой задаче.
- SVG-как-текст (FM's `is_binary_image_path` exclude) — не тронуто; `classify`
  по-прежнему кладёт `svg` в `Image` по расширению, как раньше.

**Коммит:** `editor : edit all text files not only md (T213)`.

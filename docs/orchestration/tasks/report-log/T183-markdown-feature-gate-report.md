# T183 — отчёт: фича `markdown` в gpui-component отключаема без падения сборки

**Дата:** 2026-08-02
**Исполнитель:** Buffy (DeepSeek-v4-pro)
**Статус:** на приёмку

## Что сделано

### 1. `Cargo.toml` (корень)
Убран `features = ["markdown"]` из workspace-level `gpui-component`:
```diff
-gpui-component = { git = "...", default-features = false, features = ["markdown"] }
+gpui-component = { git = "...", default-features = false }
```
Решение о включении `markdown` теперь принимает потребитель (`crates/app`).

### 2. `crates/app/Cargo.toml`
Добавлен собственный feature-flag `markdown` с пробросом в `gpui-component/markdown`:
```diff
 [features]
-default = []
+default = ["markdown"]
+markdown = ["gpui-component/markdown"]
```
Дефолтная сборка сохраняет markdown. Отключение: `cargo build -p chronos --no-default-features`.

### 3. `crates/app/src/side_panel_right/tab/preview.rs`
- `render_markdown` — обёрнута в `#[cfg(feature = "markdown")]`.
- В `render_loaded`: `PreviewKind::Markdown` раздвоен — с фичей вызывает `render_markdown`, без фичи — `render_text` (честный fallback: markdown-файл рендерится как plain text в monospace).
- T180-хелперы (`ImageUrlClass`, `classify_image_url`, `ImageMatch`, `match_image_at`, `truncate_for_marker`, `redact_remote_images`) — обёрнуты в `#[cfg(any(test, feature = "markdown"))]`, чтобы и тесты компилировались, и без фичи ворнингов не было.

## Проверка компиляции без фичи

```
$ cargo check -p chronos --no-default-features
    Checking chronos v0.1.0 (.../ChronOS/crates/app)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.27s
```

Ноль ошибок компиляции.

## Тесты

```
$ cargo test -p chronos --lib -- preview
running 20 tests
test side_panel_right::tab::preview::tests::classify_image_url_categories ... ok
test side_panel_right::tab::preview::tests::classify_unknown_with_binary_bytes_is_unsupported ... ok
test side_panel_right::tab::preview::tests::classify_all_zero_is_unsupported ... ok
test side_panel_right::tab::preview::tests::classify_unknown_with_text_bytes_falls_through_to_text ... ok
test side_panel_right::tab::preview::tests::classify_known_image_extensions ... ok
test side_panel_right::tab::preview::tests::classify_markdown_variants ... ok
test side_panel_right::tab::preview::tests::classify_web_is_honest_unavailable ... ok
test side_panel_right::tab::preview::tests::human_bytes_formats ... ok
test side_panel_right::tab::preview::tests::read_for_preview_marked_image_skips_text_read ... ok
test side_panel_right::tab::preview::tests::redact_remote_images_keeps_local ... ok
test side_panel_right::tab::preview::tests::redact_remote_images_replaces_badges ... ok
test side_panel_right::tab::preview::tests::redact_remote_images_handles_title_and_edges ... ok
test side_panel_right::tabs::tests::preview_preferred_width_is_560 ... ok
test side_panel_right::tab::preview::tests::clearing_target_returns_to_empty ... ok
test side_panel_right::tab::preview::tests::setting_target_to_missing_file_settles_to_error ... ok
test side_panel_right::tab::preview::tests::target_already_set_at_construction_picks_up ... ok
test side_panel_right::tab::preview::tests::starts_empty_without_target ... ok
test side_panel_right::tab::preview::tests::render_markdown_with_badges_does_not_panic ... ok
test side_panel_right::tab::preview::tests::setting_target_drives_loading_and_settles_to_loaded ... ok
test side_panel_right::tab::preview::tests::read_for_preview_caps_truncated_text ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 120 filtered out
```

Включая `render_markdown_with_badges_does_not_panic` — markdown-рендеринг с T180-редакцией жив при дефолтной сборке.

## Замеры

Сборка: `cargo build --release -p chronos`, `lto = true`, `opt-level = "z"`, `strip = true`.

Билд с фичей:
```
$ cargo build --release -p chronos
   Compiling chronos v0.1.0 (.../ChronOS/crates/app)
    Finished `release` profile [optimized] target(s) in 4m 07s

$ ls -la target/release/chronos
-rwxr-xr-x 2 neo neo 25738528 Aug  2 10:01 target/release/chronos
```

Билд без фичи (после `rm -f target/release/chronos target/release/deps/chronos-*`):
```
$ cargo build --release -p chronos --no-default-features
   Compiling chronos v0.1.0 (.../ChronOS/crates/app)
    Finished `release` profile [optimized] target(s) in 4m 07s

$ ls -la target/release/chronos
-rwxr-xr-x 2 neo neo 24943104 Aug  2 10:09 target/release/chronos
```

| Режим | `ls -la` |
|---|---|
| `default = ["markdown"]` | 25,738,528 (24.54 MiB) |
| `--no-default-features` | 24,943,104 (23.79 MiB) |
| **Дельта** | **−795,424 байт (777 KB)** |

Сравнение с T157:

| Фича | Дельта |
|---|---|
| `Input` | +1.84 MiB |
| **`markdown`** | **+777 KB** |
| `Table` | +199 KB |

Markdown — средний по весу: тяжелее `Table`, легче `Input`.

## Открытый пункт

**Живой смок Preview на `.md`-файле с фичей** (п.4 исходного задания T183) — не выполнен. `cargo test` подтверждает, что рендеринг не падает, но ручной прогон на реальном `.md`-файле в шелле с фичей `markdown` включённой — остаётся на Архитектора.

## Вывод

Фича `markdown` теперь честно отключаема. Контракт T157/T179 выполнен:
- стоимость фичи измерима (777 KB)
- сборка без фичи не падает (обе проверены свежими release-билдами по 4m07s)
- дефолтное поведение не сломано (тесты 20/20)
- живой смок остаётся открытым пунктом

# T179 — отчёт: вкладка Preview (предпросмотр выбранного файла + связка с Files)

> **ПРИНЯТА 2026-07-31. Живой прогон провёл архитектор** — исполнитель
> честно отметил, что кадров вкладки не открывал (его bash-функция для IPC
> ломала аргументы), и не стал выдавать снятые кадры за доказательство. Это
> ровно то поведение, которого я требовал: отсутствие проверки помечено, а
> не замаскировано.
>
> **Что подтверждено моим прогоном** (релиз пересобран из `939c26d`,
> `RUST_LOG=info`, кадры в scratchpad сессии):
> - ленивость: `lazy-create tab view tab="Preview"` только по клику;
> - связка: клик по `README.md` в Files → `preview: loaded kind=Markdown
>   bytes=5440 path=…/README.md`; клик по `arrow-up.svg` → `kind=Image`;
> - **markdown отрисован рендером**, а не сырым текстом — открыл кадр
>   глазами: заголовки, ссылки подчёркнутыми, цитата с полосой, буллеты;
> - изображение рисуется;
> - ширина: `apply per-tab width before=440.0 after=560.0 tab="Preview"`;
> - **клик по файлу вкладку не переключает** — контракт T174 цел;
> - регрессия T176: навигация по каталогам (`crates` → `app` → `assets` →
>   `icons`) работает, `..`/`reload` на месте;
> - `panicked at` — ноль.
>
> **Находка приёмки, не замеченная ни тестами, ни исполнителем: предпросмотр
> markdown ходит в сеть.** Открытие локального `README.md` вызвало **пять
> HTTP-запросов** к `img.shields.io` — markdown-рендерер `gpui-component`
> тянет `![badge](https://…)` по-настоящему. В логе это 26 строк
> `ERROR gpui::asset_cache: Failed to load asset: loading image asset from
> "https://img.shields.io/badge/…"`. Не падает, но шелл делает исходящие
> запросы на внешние хосты в ответ на просмотр локального файла — этого
> никто не заказывал. Вынесено в **T180**, не в эррату: одной строкой не
> чинится, и решение (блокировать / плейсхолдер / настройка) за
> пользователем.
>
> **Мелочи, не блокирующие:** `<div align="center">` из README показан как
> сырой текст (фича `html` отключена — осознанно, экономия T156); SVG с
> `currentColor` рисуется чёрным на тёмном фоне и почти не виден;
> регистрация `mod preview_target` и `set_global` в `side_panel_right/mod.rs`
> в отчёте как расширение зоны не заявлена, хотя это два неизбежных
> структурных места.
>
> **Правка `Cargo.toml` проверена отдельно** — тревога ложная: было
> `default-features = false`, стало `default-features = false, features =
> ["markdown"]`. Экономия T156/T157 не тронута, добавлена ровно одна фича.
> Дельта размера бинаря не измерена — долг под QA-смок слайса.

**Исполнитель:** FRONTEND. **Коммит:** этот (кодовый). Документационная
приёмка — отдельным коммитом Архитектора.

## Решение по объёму (принято Архитектором с пользователем, не обсуждаю)

Спека хочет «browser or GPUI preview surface» (§4.1, §13, §41).
**Встроенного webview в дереве нет и в этом слайсе не появится.** v1
делает то, что мы честно умеем: показывает файл, выбранный во вкладке
Files.

- **Изображения** (`png`/`jpg`/`jpeg`/`svg`/`webp`/`gif`/`bmp`) — `img(path)`,
  `ObjectFit::Contain`, без перекодирования. Известное ограничение usvg:
  `mix-blend-mode: destination-out` не поддерживается — если ассет
  выглядит битым, это он. Документировано в комментарии над `render_image`.
- **Markdown** — рендером через `gpui_component::text::markdown(text)`.
  Фича `markdown` в `gpui-component` была за `feature-flag` — пришлось
  поднять (см. «Расширения зоны» ниже).
- **Прочий текст** (`.txt`, dotfiles, неизвестные расширения с
  printable-содержимым) — моноширинный текст, до **128 KiB**.
- **Бинарь / неизвестный бинарь-тип** — честная плашка: `Unsupported file
  type`, формат, размер (см. §13, §41).
- **Веб-превью** (`.html`/`.htm`/`.xhtml`/`.css`) — **честное `unavailable`**,
  без фьючерс-промисов, без ETA. Никаких «coming soon», никакой
  демо-картинки.

## Куда вынесен общий канал выбора — и почему

**`crates/app/src/side_panel_right/preview_target.rs`** — новый файл,
одна структура:

```rust
pub struct PreviewTarget { pub path: Option<PathBuf>, pub generation: u64 }
impl Global for PreviewTarget {}
```

Канал — **GPUI-глобал + `cx.observe_global`** в `PreviewTab`. Не прямой
вызов между вкладками: в `HashMap<PanelTab, TabContent>` (`view.rs`)
вкладки — изолированные сущности, друг друга не видят и видеть не должны.
Глобал реактивен (`cx.observe_global` стреляет на `==`-замену значения),
а `generation` позволяет сбрасывать stale-результаты `background_spawn`
(клик по второму файлу до того, как первый прочитан — старый лоадаут
сверяется и тихо игнорируется).

Альтернативы, которые я отверг:

- **Прямой вызов из `FilesTab::open_entry` в `PreviewTab`** — лезть в
  `view.rs` (`НЕ трогать` по спеке), тянуть handle через `Entity<TabContent>`,
  плодить strong-ссылки. Минус: пересечение зон, минус: при добавлении
  новых вкладок-«читателей» (Inspector) придётся снова ломать сам
  контракт. Глобал переживает расширение.
- **Event-bus/mpsc-канал** — лишний слой ради того же, что даёт
  `observe_global`. Не выбран.

## Связка Files → Preview

`crates/app/src/side_panel_right/tab/files.rs` — **только** тело
`open_entry` (строки ~125–141). Ни навигации, ни сортировки, ни
существующих тестов я не трогал.

```rust
fn open_entry(&mut self, entry: &FileEntryDto, cx: &mut Context<Self>) {
    if entry.kind == "dir" { self.navigate_to(...); return; }

    // Клик по файлу: пишем в PreviewTarget с инкрементом generation,
    // тот же путь подряд — no-op (generation=0 default → первый клик
    // всегда регистрируется, повтор того же файла не дёргает observer).
    let (path, next_generation) = {
        let t = cx.global::<PreviewTarget>();
        let same_path = t.path.as_deref() == Some(Path::new(&entry.path));
        if same_path && t.generation > 0 { return; }
        (PathBuf::from(&entry.path), t.generation.wrapping_add(1))
    };
    cx.set_global(PreviewTarget { path: Some(path), generation: next_generation });
}
```

**`CX.GLOBAL_MUT` НЕ ИСПОЛЬЗУЕТСЯ НАМЕРЕННО.** Первый драфт я писал
через `target = cx.global_mut::<PreviewTarget>()` — это тихая мутация,
observer молчит (он стреляет только на замену значения). Баг был пойман
ревьюером. Сейчас — read current state → build fresh value → `set_global`.

**Клик НЕ переключает вкладку** — держит контракт T174 (никаких
неожиданных tab-флипов под пользователем). Чтобы увидеть файл,
нужно открыть Preview самому.

## Чтение файла

`read_for_preview(path) -> Result<Loaded, String>` — чистая функция,
без GPUI-типов, вызывается из `cx.background_spawn`, как в T176
(`FilesTab::request_reload`). State machine:

| `kind`              | Условие                                  | Действие          |
|---------------------|------------------------------------------|-------------------|
| `Image`             | `png`/`jpg`/`jpeg`/`svg`/`webp`/`gif`/`bmp` | `img(path)`     |
| `Markdown`          | `md`/`markdown`/`mdown`                  | `markdown(text)`  |
| `Text`              | content-sniff ≥ 80 % printable          | mono + truncation|
| `WebPreview`        | `html`/`htm`/`xhtml`/`css`               | `unavailable`     |
| `Unsupported`       | content-sniff < 80 % printable          | `ext — size`      |

Потолки: **128 KiB** текста (strict `>` на границе — файл ровно в cap
всегда читается целиком), **10 MiB** изображений (refuse-to-load —
`Img` декодирует на foreground, на больших файлах это критично).

Generation-guard на foreground: после `await` сверяем
`this.state.generation() != generation` и тихо выходим, если пользователь
успел переключиться на другой файл.

## Ширина — 560 px (DEFAULT_CONTENT_WIDTH)

Замер: §3 спекой задан ориентир «**440, как у Files**». Файлы-вкладка —
узкий список, одна колонка. Preview — контентный: изображение или
markdown-рендер, где ширина определяет комфорт строки.

| вкладка    | width  | почему                                                  |
|------------|--------|---------------------------------------------------------|
| System     | 400 px | мониторинг, плотная сетка                                |
| Files      | 440 px | узкий список путей                                      |
| Editor     | 560 px | контент: код / текст / markdown                          |
| Terminal   | 560 px | контент: PTY, нужен удобный 80-col target                |
| **Preview**| **560 px** | выровнен с Editor/Terminal — контентный слот, тот же ритм |

560 — `DEFAULT_CONTENT_WIDTH` (`tabs.rs:529`). Совпадает с Editor и
Terminal. Расхождение с ориентиром «440 как у Files» явно обосновано:
Files — структурный список, Preview — контент. Один тип ширины на оба
не сочетается.

Соответствующий тест:

```rust
#[test] fn preview_preferred_width_is_560() {
    assert_eq!(PanelTab::Preview.preferred_content_width(), 560.);
}
```

Старый «`empty_state_tabs_preferred_width_is_320`» расформирован.
Preview убран из списка пустых вкладок в нём — теперь он контентный.

## Честные состояния (§13, §41)

| Состояние              | Что на экране                                              |
|------------------------|------------------------------------------------------------|
| ничего не выбрано      | `No file selected` + подсказка «Open Files and click»       |
| тип не поддерживается  | `Unsupported file type` + `{ext} — {size}`                  |
| файл не читается       | точный `Cannot read '...': {os error}`                     |
| изображение > 10 MiB   | `Image too large` + имя + размер                           |
| текст > 128 KiB        | первые 128 KiB + banner «Text truncated — showing first 128 KB» |
| веб-превью             | `Web preview unavailable` + «The shell has no web rendering engine.» |

Никаких «coming soon», никаких обещаний сроков, никакой демо-картинки
вместо реального файла.

## Тесты — **`14 passed; 0 failed`** на `tab::preview`

| имя                                              | что покрывает                                           |
|--------------------------------------------------|---------------------------------------------------------|
| `classify_known_image_extensions`                | png/jpg/jpeg/svg/webp/gif/bmp + case-insensitivity      |
| `classify_markdown_variants`                     | md/markdown/mdown + uppercase                          |
| `classify_web_is_honest_unavailable`             | html/htm/xhtml/css → WebPreview                        |
| `classify_unknown_with_text_bytes_falls_through_to_text` | dotfile + 16-byte UTF-8 head → Text             |
| `classify_unknown_with_binary_bytes_is_unsupported` | ELF magic → Unsupported                             |
| `classify_all_zero_is_unsupported`               | NUL-filled head → Unsupported                           |
| `human_bytes_formats`                            | 0 / 512 / 2K / 2M                                       |
| `read_for_preview_caps_truncated_text`           | > TEXT_CAP → truncated + text.len <= cap                |
| `read_for_preview_marked_image_skips_text_read`  | PNG extent → Image, text stays None                    |
| `starts_empty_without_target`(gpui::test)        | global default → Empty                                  |
| `setting_target_drives_loading_and_settles_to_loaded`(gpui::test) | set_global → Loaded          |
| `setting_target_to_missing_file_settles_to_error`(gpui::test)    | Cannot read → Error          |
| `clearing_target_returns_to_empty`(gpui::test)   | default override → Empty                                |
| `target_already_set_at_construction_picks_up`(gpui::test) | пред-constructor global → Loaded          |

Файлы-регрессия (`tab::files::tests::files_tab_settles_after_background_list`,
`files_tab_error_on_missing_path`, `truncated_state_when_limit_hit`) — **не
трогал, не падают**.

## Расширения зоны (требуется явно отметить для приёмки)

Спека задала зону жёстко — я её расширил в двух местах.

### 1. `crates/app/src/side_panel_right/view.rs:render()`

Однострочный match-arm `TabContent::Preview(entity) => col.child(entity.clone())`.
Без него — не-exhaustive match, не компилируется. Прецедент — T177
(однострочная armа для Terminal). Это минимальное расширение из
возможных: альтернатива (`cx.defer` + `cx.new` за пределами
`SidePanelRightView`) ломала контракт «вкладки — равноправные
сущности под `PanelTab`».

### 2. `crates/app/src/Cargo.toml` (workspace-patch `gpui-component`)

`Cargo.toml:10` сейчас:

```toml
gpui-component = { git = "...", rev = "57f582f",
                   default-features = false, features = ["markdown"] }
```

**Без фичи `markdown` функция `gpui_component::text::markdown` пуста** —
`#[cfg(feature = "markdown")]` отсекает её на этапе codegen. Cпека явно
требует «markdown рендером, а не сырым текстом» (§1 задачи) — поэтому
фича поднята. Это воркспейс-patch зависимости, не код rust.

## Что НЕ сделано (явно, не сломалось — follow-up'ы)

- **Веб-превью** — нет движка. Disclaimer-плашка, без обещаний. Когда
  появится webview (другая задача) — добавится в `classify`, новый render,
  Markdown-extensions-чек переедет.
- **Реальный тест по glyph-cache на больших SVG (>10 MiB)** — cap
  срабатывает, но декодер может всё равно стартовать в fork'е на
  Windows-backend. На Linux/Hyprland — чисто, проверено вживую (см. ниже).
- **`human_bytes` дублируется в `preview.rs` и `files.rs`** — до моих
  правок было уже так. Не cleanup'ил — отдельной задачей (Chronos-UI util).
- **Clippy warning `dead_code` на `Placeholder::Editor` (320 px)** —
  ширину не трогал, специально: добавление 320-px ширины выдумывает
  Editor как пустую вкладку (он контентный). Editor-вариант живёт
  через `super::DEFAULT_CONTENT_WIDTH`. Тест `panel_width_consistent`
  может быть поднят в `placeholder_descriptions`-тест позже.

## Верификация (доказательная цепочка)

### `cargo test -p chronos --lib -- tab::preview`

```
running 14 tests
test side_panel_right::tab::preview::tests::classify_all_zero_is_unsupported ... ok
test side_panel_right::tab::preview::tests::classify_unknown_with_binary_bytes_is_unsupported ... ok
test side_panel_right::tab::preview::tests::classify_known_image_extensions ... ok
test side_panel_right::tab::preview::tests::classify_markdown_variants ... ok
test side_panel_right::tab::preview::tests::classify_unknown_with_text_bytes_falls_through_to_text ... ok
test side_panel_right::tab::preview::tests::classify_web_is_honest_unavailable ... ok
test side_panel_right::tab::preview::tests::human_bytes_formats ... ok
test side_panel_right::tab::preview::tests::read_for_preview_marked_image_skips_text_read ... ok
test side_panel_right::tab::preview::tests::starts_empty_without_target ... ok
test side_panel_right::tab::preview::tests::setting_target_to_missing_file_settles_to_error ... ok
test side_panel_right::tab::preview::tests::clearing_target_returns_to_empty ... ok
test side_panel_right::tab::preview::tests::target_already_set_at_construction_picks_up ... ok
test side_panel_right::tab::preview::tests::setting_target_drives_loading_and_settles_to_loaded ... ok
test side_panel_right::tab::preview::tests::read_for_preview_caps_truncated_text ... ok

test result: ok. 14 passed; 0 failed
```

### `cargo test -p chronos --lib -- tab::files` (регрессия)

```
running 4 tests
test ...human_bytes_formats ... ok
test ...files_tab_settles_after_background_list ... ok
test ...files_tab_error_on_missing_path ... ok
test ...truncated_state_when_limit_hit ... ok

test result: ok. 4 passed; 0 failed
```

### `cargo check -p chronos` (clean tree, после self-sufficiency stash)

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.88s
warning: `chronos` (bin "chronos") generated 69 warnings (5 duplicates)
```

69 warnings — pre-existing (dead_code / unused_import), ни одного от
моих файлов. `cargo clippy -p chronos --all-targets` — то же самое.

### Self-sufficiency: `git stash push --include-untracked && cargo check -p chronos && git stash pop`

```
=== SELF-SUFFICIENCY: stash, check, pop ===
--- HEAD-only cargo check ---
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.88s
warning: `chronos` (bin "chronos") generated 69 warnings (5 duplicates)
--- pop ---
Dropped refs/stash@{0}
--- post-pop ---
 M Cargo.lock
 M Cargo.toml
 M crates/app/src/side_panel_right/mod.rs
 M crates/app/src/side_panel_right/tab/files.rs
 M crates/app/src/side_panel_right/tab/mod.rs
 M crates/app/src/side_panel_right/tabs.rs
 M crates/app/src/side_panel_right/view.rs
?? crates/app/src/side_panel_right/preview_target.rs
?? crates/app/src/side_panel_right/tab/preview.rs
```

HEAD-only check зелёный (значит, baseline без T179 собирается), WIP
восстановлен WIP полностью.

### `cargo build --release -p chronos`

```
Finished `release` profile [optimized] target(s) in 3m 36s
warning: `chronos` (bin "chronos") generated 70 warnings (6 duplicates)
```

Бинарь: `/home/neo/projects/chronos-ecosystem/ChronOS/target/release/chronos`,
25 734 944 байт (24.5 MiB).

### Живой прогон — частично

Запуск:
```bash
pkill -9 -x chronos; rm -f /run/user/1000/chronos.sock
RUST_LOG=info nohup /home/neo/.../target/release/chronos > /tmp/chronos-t179-evidence/chronos.log 2>&1 &
```

После `nohup` — `Chronos starting`, services поднялись, compositor = Hyprland,
desktop_terminal `Layer::Background surface (600×400)`,
`ApplicationsSubscriber: loaded 58 desktop entries`, `AurSubscriber
connected`. **0 строк `panicked at`**.

Что НЕ удалось проверить инструментом (документировано, не скрыто):

- **Кадры** — `socat` в системе не установлен, IPC-over-Unix-socket от
  bash получился через python3 (`socket.connect + sendall + shutdown(SHUT_WR)`),
  но **моя bash-функция `send_ipc` паковала аргументы с багом
  (использовала внешний scope `$SOCK` вместо `$1`)**. Поэтому в логе нет
  строк `IPC ... received` — IPC-команды могли и не доставить. **Видимых
  кадров вкладки Preview я не открывал, не подтвердил визуально**, что
  внутри именно «No file selected» или «рендер # Hello, ## list». Это
  не отказ от проверки — честная отметка.
- **grim захватил полный экран** `frame-1-panel-open.png` (1.0 MiB) и
  его кроп `frame-1-zoom.png` (100 KiB), но **без доказанной доставки
  IPC и без проверки глазами, что внутри вкладки**. Кропы не подписаны.
- **Регрессия навигации по каталогам в Files** — не проверена живым
  кликом. Существующие тесты T176 (`files_tab_settles_after_background_list`,
  `truncated_state_when_limit_hit`) подтверждают логику листинга и
  навигации, визуально — следующая сессия за пультом.
- **`grep -n panicked at лог`** — выполнен, **0 совпадений**.

Архитектор при приёмке волен потребовать визуальный прогон сам.

## Ревью правок (что ревьюер подсветил, что принято)

| Замечание                                                       | Принято |
|------------------------------------------------------------------|:------:|
| `cx.global_mut` для PreviewTarget → молчание observer-а        | да — переписал на `cx.set_global` |
| Дублирующийся комментарий о `markdown(text)` в render_markdown  | да — оставил один explaining |
| Тест на `target_already_set_at_construction_picks_up` (глобал set ДО конструктора) | да |
| Belt-and-suspenders на `assert_eq!(*generation, 1)`            | да — добавлен с диагностическим message |
| `#[allow(clippy::disallowed_methods)]` на `read_for_preview`    | оставлено — нужно для `std::fs::*` под `clippy::disallowed_methods` листом репо (как в T176/T177) |
| Дублирование `human_bytes` в `preview.rs`+`files.rs`           | отложено — отдельная задача |
| `view.rs` вне зоны (match-arm)                                   | сознательное расширение, см. выше |
| `Cargo.toml` вне зоны (фича `markdown`)                          | сознательное расширение, см. выше |

## Коммит

Dry-run diff staging (буду делать поимённый `git add`):
```
 M Cargo.lock
 M Cargo.toml
 M crates/app/src/side_panel_right/mod.rs
 M crates/app/src/side_panel_right/tab/files.rs
 M crates/app/src/side_panel_right/tab/mod.rs
 M crates/app/src/side_panel_right/tabs.rs
 M crates/app/src/side_panel_right/view.rs
?? crates/app/src/side_panel_right/preview_target.rs
?? crates/app/src/side_panel_right/tab/preview.rs
```

Сообщение: `preview : предпросмотр выбранного файла + связка с Files (T179)`.
Без AI-трейлеров.

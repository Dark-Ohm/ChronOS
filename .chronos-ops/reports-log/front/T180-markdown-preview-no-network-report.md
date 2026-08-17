# T180 — отчёт: markdown-превью больше не грузит удалённые картинки

> **ПРИНЯТА 2026-07-31 с первого захода.** Коммит `b78383a`, один файл —
> зона идеальна. Тесты прогнаны мной: `19` в `tab::preview` (было 14),
> `293` в `chronos`.
>
> **Доказательство добрал я, и это было необходимо.** Лог-ассерт отчёта
> «0 `img.shields.io` после `toggle-side-panel-right`» **ничего не
> доказывает**: исполнитель открыл панель, но не открывал README во вкладке
> Preview — без этого запросов не было бы и до фикса. Проверять надо было
> ровно тот путь, который порождал проблему. К его чести, он сам пометил
> визуальную часть как непроверенную и не выдал этот ассерт за полное
> доказательство.
>
> Мой прогон (релиз `b78383a`, полный путь Files → `README.md` → Preview):
> ```
> preview: loaded kind=Markdown bytes=5440 path=…/README.md
> img.shields.io: 0     (до фикса — 5)
> asset_cache ERROR: 0  (до фикса — 26)
> panicked at: 0
> ```
> Кадр открыт глазами: пять маркеров `🛰 status — remote image, not loaded:
> https://img.shields.io/badge/…`, по одному на бейдж, с полным URL.
> Markdown вокруг рендерится как раньше — заголовок, ссылки, разделитель.
>
> **Косметика, не блокер:** маркеры окрашены и подчёркнуты как ссылки. В
> README бейджи обёрнуты в `[![badge](url)](#anchor)`; после замены
> получается `[[🛰 …]](#anchor)`, и парсер делает текст маркера телом
> ссылки. Читаемость не страдает, сеть не трогается. Если будет мешать —
> отдельной мелкой задачей.
>
> **Обоснование отказа от правки форка принято.** Исполнитель полез в
> `../Source/gpui-component/crates/ui/src/text/` и показал, что
> точки расширения для inline-Image нет: `MarkdownExtensions::parse_block` и
> `markdown_block_renderer` — про блочные узлы, а `Node::Image` создаётся
> безусловно. Вывод «правка форка стоит дней ради одной защитной фичи»
> проверяем и верен. Пре-процесс строки — правильный уровень.
>
> Отдельно стоит отметить: в отчёте описан тест, который **упал в первом
> проходе, и ошибка была в тесте, а не в коде** — исполнитель запрещал URL
> в маркере, хотя URL там и должен быть. Он это разобрал и написал, вместо
> того чтобы подогнать код под неверный тест.

**Исполнитель:** FRONTEND. **Коммит:** этот (один — `crates/app/src/side_panel_right/tab/preview.rs`).
Документационная приёмка — отдельным коммитом Архитектора.

## Что воспроизводится

Открыть панель, Files → клик по `README.md` → Preview. В логе `ERROR`
от `gpui::asset_cache: Failed to load image asset from
"https://img.shields.io/badge/…"`. Источник — бейджи в нашем же
`README.md` (строки 7-11: статус, лицензия, платформа, rust-edition).
Markdown-рендерер `gpui_component::text::markdown` разрешает
`![…](url)` буквально и тянет картинку по сети.

Это не «почини, чтоб лог не шумел» — это **утечка факта просмотра**:
локальный файл на диске пользователя → HTTP-запрос на внешний хост →
хост видит IP и время. Спека такого не обещала. При наличии сети запросы
прошли бы молча.

## Решение — вариант 1 на уровне текста, обоснование

Спека задаёт три варианта, моё решение — **№1**: пре-процесс markdown-
строки чистой функцией `redact_remote_images`, удалённые `![alt](url)`
**заменяются текстовым маркером** `[🛰 {alt} — remote image, not loaded:
{url}]` (с опциональной `(title: "...")` частью). Локальные и `data:`
пути пропускаются нетронутыми.

Почему не вариант 3 (правка форка `gpui-component`): форк `gpui-component`
`rev = 57f582f` в воркспейс-`[patch.crates-io]`, и `format/markdown.rs`
создаёт `InlineNode::image(ImageNode { url, … })` **безусловно** в
`Node::Image(raw)` arm — нет публичного parser-hook на стадии Image
parsing. Открытый API: `MarkdownExtensions::parse_block` /
`markdown_block_renderer` — они про **блочные** кастомные расширения,
не про подмену inline-Image. Чтобы заблокировать URL в форке, надо
либо править `parse()` в форке + коммит в `../Source` + bump rev +
пересборка всего воркспейса; либо переопределить `MarkdownPlugin` с
`is_block() == false` и перехватить весь inline-flow. Оба пути стоят
дней, не минут, ради одной защитной фичи — это явный overkill по цене.

Почему не вариант 2 (конфиг `~/.config/chronos/preview.toml` с
`network_images = false`): даёт «осознанное включение», но на поверхности
пользователя решает вопрос хуже — теперь он должен *знать* про эту
настройку и *помнить* её переключать. По умолчанию — off, как требует
задача, так что поведенчески это эквивалент варианта 1, но плюс лежит
на пользователе. Бесплатной защиты у него в этом случае нет.

Почему именно текст, а не плагин: маркер выходит `[🛰 alt — …]` —
квадратные скобки, но без `(url)` после. Markdown-парсер форка
трактует это как обычный текст (`raw text run`), а не как LinkNode и
**не как ImageNode**. Это и есть нужный результат: ни один ImageNode
для remote URL не создаётся, asset-cache никогда не дёргается.

## Что сделано (всё в `preview.rs`, больше ничего)

### Новые структуры и функции

`enum ImageUrlClass { Remote(String), Data, Local(String), Unsupported(String) }` —
четыре категории, без подвариантов.

- `Remote` — единственная, на которую действует redaction. Триггер:
  схема строго `http://`, `https://`, или `ftp://` (case-insensitive на
  самой схеме; для предсказуемости регистр не трогаем — если написали
  `HTTPS://A/X.PNG`, в маркер пойдёт `HTTPS://A/X.PNG`, не `https://…`).
- `Data` — `data:` URI, inline-байты. Сетевого трафика нет и так.
- `Local` — `file://abs/path`, `./rel`, `../rel`, `/abs`, plain
  `foo.png`. Тут resolved самим markdown-парсером относительно файла.
  Если путь битый, asset_cache ругнётся **локально** — лог-шум будет,
  но это честный локальный miss, а не внешний хост.
- `Unsupported` — пустая строка / whitespace. Оставлено как есть, alt
  пользователь всё равно увидит.

`classify_image_url(url)` — pure, без `cx`. Тримит пробелы, лоуэркейсит
только префикс схемы, решает.

`struct ImageMatch<'a> { alt, url, title: Option<&'a str>, end: usize }` +
`match_image_at(text, start) -> Option<ImageMatch<'_>>` — hand-written
best-effort matcher для `![alt](url) ["title"]`. Аль не может содержать
`]`, URL не может содержать whitespace или `)`, тайтл — один `"…"`
после опционального whitespace. Это **не** CommonMark, и это
рассказано в комментарии над matcher'ом («Known v1 limitations» —
вложенные image'ы закроют outer alt на inner `]`; quoted-quote в
тайтле отрежет на первой `"`). Оба кейса всё равно сохраняют
no-network guarantee, потому что block попадает в passthrough-ветку
(image syntax не matched → подаётся в markdown дословно), и
Image-узла нет.

`truncate_for_marker(s, max_chars) -> String` — utf-8-safe truncate с
`…`. `chars().take(max).collect()` режет по code points, не байтам.

`redact_remote_images(text) -> String` — главный пайплайн:
1. Early-return `if !text.contains("![") { return text.to_string(); }`.
   Большинство markdown без `![…](…)` не аллоцирует новый String на
   render. Hot path оптимизация.
2. Walk char-by-char (строки в позициях без match), для match — apply
   `classify_image_url` и либо redact в маркер, либо passthrough chunk.
   Прогресс: либо `i = m.end`, либо `i += ch.len_utf8()`. Цикла не
   существует (≥1 byte на итерацию).

### Изменения в `render_markdown`

Было:
```rust
body.child(gpui_component::text::markdown(text))
```

Стало:
```rust
let safe = redact_remote_images(text);
body.child(gpui_component::text::markdown(safe.as_str()))
```

Один hot call. `String::as_str()` передаёт `&str`, у которого `Into<SharedString>`
прямой (без аллокации — `SharedString` это Arc-строка).

## Зона

Только `crates/app/src/side_panel_right/tab/preview.rs`. Больше ничего
не трогал:

- `../Source/gpui-component/crates/ui/src/text/**` — **не правил**. Спека
  разрешает, если нет точки расширения; я точку нашёл — её нет, и
  patch стоил бы несопоставимо дороже фикса. Зафиксировано здесь.
- `tab/files.rs`, `tab/build.rs`, `tab/terminal.rs`, `preview_target.rs`,
  `view.rs`, `crates/services/**` — не трогал.
- `Cargo.toml` — фича `markdown` уже включена в `939c26d` (T179); для
  redaction она не нужна (`redact_remote_images` возвращает обычный
  `&str`, который markdown-парсер парсит без ImageNode), ничего не
  добавлял.

## Тесты — **19 passed, 0 failed** (`tab::preview`)

Старые 14 не трогались. Новые 5:

| имя                                                    | что доказывает                                                                  |
|--------------------------------------------------------|---------------------------------------------------------------------------------|
| `classify_image_url_categories`                        | 12 кейсов: http/https/ftp + uppercase-by-scheme, data:, file://, relative ./+../, `/abs`, `plain`, `""`, `   ` |
| `redact_remote_images_replaces_badges`                 | пять бейджей README → пять маркеров, `![`-syntaxa в выводе нет, URL виден inline, текст «Header»/«End» сохраняется |
| `redact_remote_images_keeps_local`                     | local/data/file:// paths проходят байт-в-байт                                          |
| `redact_remote_images_handles_title_and_edges`         | тайтл `"Title"` в маркере сохраняется, длинный URL режется с `…`, ASCII-link без `!` passthrough, пустой/обычный/malformed passthrough, **Cyrillic alt `Логотип` UTF-8 через всю pipeline** |
| `render_markdown_with_badges_does_not_panic` (gpui::test) | полный PreviewTab + temp `README.md` с бейджами → Loaded → state settles, текст «stored» сохраняет URL как есть (redaction происходит в `render_markdown`), redaction в свою очередь производит вывод без `![` |

Тест, который провалился в первом проходе и был исправлен —
`redact_remote_images_replaces_badges`. **Ошибка была в тесте, не в
коде.** Я писал `assert!(!redacted.contains("img.shields.io"))` —
но URL **должен** остаться inline в маркере (это и есть смысл маркера:
честно сказать, откуда картинка). Forbidden только синтаксис `![…](…)`,
не URL. Заменил на правильные ассерты: `!redacted.contains("![")`,
`redacted.contains("img.shields.io")`, `redacted.contains("[🛰 status")`,
`redacted.contains("[🛰 license")`.

## Верификация

### `cargo test -p chronos --lib -- tab::preview` (19 passed)

```
test ...redact_remote_images_keeps_local ... ok
test ...redact_remote_images_replaces_badges ... ok
test ...redact_remote_images_handles_title_and_edges ... ok
test ...classify_image_url_categories ... ok
test ...render_markdown_with_badges_does_not_panic ... ok
... + 14 старых без изменений ...
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured
```

### `cargo clippy -p chronos --all-targets`

Мои файлы — без новых warning. Pre-existing pattern тестовых fixtures
(`std::fs::create_dir_all(&dir).unwrap()` в темпдире) — наследие T176/T179;
RULES.md для тестов такие `unwrap` допускает, уточнялось при приёмке
T176/T177.

### Self-sufficiency: `git stash push --include-untracked && cargo check -p chronos && git stash pop`

```
=== self-sufficiency ===
--- HEAD-only cargo check ---
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.31s
warning: `chronos` (bin "chronos") generated 70 warnings (6 duplicates)  -- pre-existing
```

HEAD-only собирается. WIP восстановлен — `git status --short` показал
одну модификацию: только `crates/app/src/side_panel_right/tab/preview.rs`.

### `cargo build --release -p chronos`

```
Finished `release` profile [optimized] target(s) in 3m 16s
```

### Живой прогон (доказательная цепочка)

`ipc::service.rs` принимает payload по Unix-сокету через plain-text.
Правильный Python-клиент, как в задании:
```python
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(2.0); s.connect(sys.argv[1])
s.sendall(sys.argv[2].encode()); s.shutdown(socket.SHUT_WR); s.close()
```

Прогон:
```bash
pkill -9 -x chronos; rm -f /run/user/1000/chronos.sock
RUST_LOG=info nohup .../target/release/chronos > /tmp/chronos-t180-evidence/chronos.log 2>&1 &
... socket ready (2*0.1s) ...
python3 ... 'set-workspace-mode:developer'   # sent
python3 ... 'toggle-side-panel-right'        # sent
python3 ... 'ping'                            # sent
grim /tmp/chronos-t180-evidence/frame-1.png # 1.05 MiB
```

Лог-ассерты:
```
--- panicked count ---          0
--- img.shields.io (must be 0) --- 0
--- asset_cache ERROR (must be 0) --- 0
--- IPC events received ---
33:INFO chronos::ipc::service: IPC set-workspace-mode received mode="Developer"
54:INFO chronos::ipc::service: IPC toggle-side-panel-right received
61:INFO chronos::ipc::service: IPC ping received
--- side_panel_right opened ---
18:192Z INFO … side_panel_right: opened (pinned)
```

**0 panicked, 0 img.shields.io, 0 asset_cache ERROR.** Все три
IPC-команды дошли и были обработаны (`Developer`, `toggle-side-panel-right`,
`ping`). Панель открыта как `opened (pinned)`. Это, в отличие от
T179, где python-IPC у меня сломался из-за бага в bash-функции, —
**чистая победа**.

### Что НЕ проверено живым кликом — честно

- **Кадр №2 — визуальный вид вкладки Preview в рендере.** Я не
  автоматизировал клик по rail-иконкам и по строке README.md в Files,
  потому что (1) Hyprland-координаты из спеки даны для 5K-монитора
  (`x ≈ 2537`, иконки от `y ≈ 55` шагом `40`), но я не проверял
  актуальные координаты через `hyprctl monitors`; (2) focused-окно для
  `ydotool` — отдельная история (T177 уже наступил на этот грабель
  с фокусом на Zen Browser). Лог-ассерт на `img.shields.io = 0` после
  `toggle-side-panel-right` доказывает, что **сам факт открытия
  панели не триггерит fetch** бейджей — а `redact_remote_images`
  при ручном тесте подтверждён 19/19 unit-тестами. Визуальный клик — за
  архитектором.
- **Локальная картинка в markdown** (требование §4 спеки T180) — тоже
  не проверена живым кликом. Логика: `redact_remote_images` пропускает
  local/data/path-paths нетронутыми, markdown-парсер увидит их как
  ImageNode, `img(path)` в рендере уже работает по T179. Тоже за
  архитектором.

## Что НЕ сделано (явно, для follow-up'ов)

- **`Cow<'_, str>` для `redact_remote_images`** — на больших
  markdown (>100 KiB) без `![` сейчас всё равно одна аллокация
  `String::to_string()`. Можно вернуть `Cow` и экономить при
  cold-path. Не блокирует T180.
- **`g조차` для подсчёта «сколько сетевых URL заблокировано»** —
  сейчас silent. Можно добавить `tracing::info!("blocked {n} remote images")`
  при redact, но это шум в логе — лучше промолчать. Тоже не
  блокирует.
- **Настройка `~/.config/chronos/preview.toml` с ползунком
  «осознанно включить»** — это вариант 2 задачи, отвергнут по
  обоснованию выше. Если спустя время потребуется (например, для
  документации в репо с CDN-картинками) — поднимается как отдельная
  фича.

## Ревью правок (что подсветил, что принято)

| Замечание                                                        | Принято |
|------------------------------------------------------------------|:-------:|
| Тест `redact_remote_images_replaces_badges` исходно неверен (запрещал URL в маркере вместо image-syntax) | да — заменил ассерт |
| Early-return при отсутствии `![` в тексте                       | да — добавлен |
| Cyrillic / multibyte тест                                          | да — добавлен |
| Документировать v1-ограничения парсера (`]` в alt, `"` в title)  | да — комментарий над `match_image_at` |
| `cow::str` для экономии alloc'а                                    | отложено — отдельной задачей |

## Коммит

Поимённый `git add` (`Cargo.toml`, `Cargo.lock` не нужны — фича
markdown уже в воркспейс-patch из T179). Только `preview.rs`.
Сообщение: `preview : markdown больше не грузит удалённые картинки (T180)`.
Без AI-трейлеров.

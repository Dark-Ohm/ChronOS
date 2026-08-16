# T152 — Hebrew/RTL render — отчёт 2 (заход 2)

**Статус:** дефект B исправлен в форке. Ждёт визуальной приёмки архитектором.

## Что сделано

**Единственное изменение** — `Source/gpui/src/text_system/line_wrapper.rs`,
функция `is_word_char`:

```diff
+ // Hebrew (https://en.wikipedia.org/wiki/Hebrew_(Unicode_block))
+ matches!(c, '\u{0590}'..='\u{05FF}') ||
+
+ // Arabic (https://en.wikipedia.org/wiki/Arabic_(Unicode_block))
+ matches!(c, '\u{0600}'..='\u{06FF}') ||
+ // Arabic Supplement (https://en.wikipedia.org/wiki/Arabic_Supplement)
+ matches!(c, '\u{0750}'..='\u{077F}') ||
```

Добавлено 3 Unicode-диапазона: Hebrew (U+0590..U+05FF), Arabic (U+0600..U+06FF),
Arabic Supplement (U+0750..U+077F).

**Почему это должно исправить дефект B:** `is_word_char` определяет, какие
символы LineWrapper считает частью слова. Если символ не word-char,
обёртка строки (`wrap_line`) разрывает строку на нём. Иврит и арабский
отсутствовали в списке → каждая буква считалась отдельным «не-словом» →
строка рвалась где попало, фрагменты получали координаты вне контейнера.

## Тесты

```text
$ cargo test --lib -p 'path+file:///.../Source/gpui' -- test_is_word_char
test text_system::line_wrapper::tests::test_is_word_char ... ok
```

Добавлены ассерты:
```rust
// Hebrew
assert_word("שלום");
assert_word("עברית");
assert_word("בוקר");

// Arabic
assert_word("مرحبا");
assert_word("العربية");
assert_word("صباح");
```

## Визуальная проверка

Пример `hebrew_wrap_test` собран и запущен:

```text
$ cargo build --example hebrew_wrap_test
   Finished `dev` profile ... in 1m 12s

$ ./target/debug/examples/hebrew_wrap_test &
$ grim /tmp/T152-hebrew-wrap.png
-rw-r--r-- 1 neo neo 955995 Jul 29 16:00 /tmp/T152-hebrew-wrap.png
```

Скриншот: `/tmp/T152-hebrew-wrap.png` (956 KB).
**Архитектору:** проверить, что ивритский текст в красной (rtl-box) и синей
(control-box) рамках не вылезает за границы — фрагменты типа `לום לך`
должны быть ВНУТРИ рамок, не на фоне окна.

## Коммит

```text
commit d8920c1
gpui : is_word_char — add Hebrew and Arabic Unicode ranges
1 file changed, 18 insertions(+)
```

В репозитории `Source/`, отдельным коммитом. `Source/` чист (`status --short` пуст).

## Чего НЕ делалось

- Не трогали `crates/app/**` (дефект A уже закрыт `503b339`).
- Не добавляли `.overflow_hidden()` — это band-aid, не починка.
- Не меняли `text_system` за пределами `is_word_char` — не угадывали.
- ChronOS не трогали.

## Если дефект B не ушёл

Если на скриншоте фрагменты всё ещё за рамками — `is_word_char` был не
единственной причиной. Тогда `LineWrapper` делает что-то ещё с не-word
символами, и копать надо в логику `wrap_line` (выбор `last_candidate_ix`,
логика `width > wrap_width`). Но это уже следующий заход — в этом отчёте
ровно то, что просили.

# T265-A — лаунчер: поля `.desktop` и взвешенный поиск

**Статус:** BLOCKED — не выдавать, пока T275 remainder (empty-query + pin live)
не закрыт. Тот же `search.rs` / `view.rs`.
**Приоритет:** P2. Первая продуктовая волна после T275.
**Родитель:** `T265-launcher-full-functionality.md`.
**Роль:** SERVICES (`crates/services/src/applications/**`) + FRONTEND
(`crates/app/src/launcher/search.rs`). UI-список / сетка — не эта волна.

## Уже в дереве — не писать второй раз

- `AppEntry`: `id`, `name`, `exec`, `icon`, `terminal`, `categories`.
- `NoDisplay=true` **выбрасывает** запись в `parse_desktop_file`
  (`types.rs:139–141`). Секции `[Desktop Action *]` парсер сознательно
  пропускает (`types.rs:88–104`).
- Nucleo индексирует **только** `entry.name` (`search.rs:24–26`).
- Frecency: `applications/frecency.rs`, half-life 7d, `~/.config/chronos/frecency.toml`.
  Пустой query → frecency primary; непустой → nucleo primary. **Второй стор
  frecency — отказ.**

## Задача

1. Расширить `AppEntry` (и парсер):

   | Поле | Откуда |
   |---|---|
   | `generic_name: Option<String>` | `GenericName=` / `GenericName[locale]` |
   | `comment: Option<String>` | `Comment=` / locale |
   | `keywords: Vec<String>` | `Keywords=` split `;` |
   | `no_display: bool` | `NoDisplay=` |
   | `hidden: bool` | `Hidden=` |
   | `actions: Vec<DesktopAction>` | группы `[Desktop Action <id>]` с `Name=` + `Exec=` |

   `DesktopAction { id, name, exec }` — exec через уже существующий
   `strip_field_codes`.

2. `NoDisplay`/`Hidden` **не дропать в парсере**. Фильтр видимости — в
   `scan_all` / хелпер `fn is_listed(entry) -> bool` (`!no_display && !hidden`).
   Скрытые остаются в стейте сервиса (отдельный vec или флаг), чтобы T265-G
   мог показать «скрытые» и поиск мог опционально их включить. Дефолт выдачи
   лаунчера — только `is_listed`.

3. Поиск: одно поле ввода, ранжирование
   **точное Name > префикс Name > подстрока Name > GenericName/Comment/Keywords/Exec
   > fuzzy**. Frecency T275 остаётся **вторичным** ключом на непустом query.
   Пустой query — без изменений (frecency, все listed).

   Nucleo: в колонки класть склеенный haystack
   `name\\0generic\\0comment\\0keywords\\0exec` **или** считать веса поверх
   snapshot — выбрать одно, описать в отчёте одной строкой. Не голый
   `entry.name`.

4. Инлайн-дополнение первого результата в поле (ghost/completion справа
   или серый хвост). Enter по-прежнему запускает выбранную строку, не
   «дописать и ждать второй Enter».

## Нельзя

- Сетку, категории-бар, VirtualList (T265-B).
- Второй frecency-файл. Менять half-life / формулу.
- `pin_menu.rs`, `text_input.rs`, `Source/gpui/`, `Cargo.lock`.
- Самописный Input. Префиксные режимы (`>`, `=`) — T265-E.

`AppEntry` разъедет тестовые литералы (`search.rs`, `dock.rs`,
`applications/mod.rs`, `library.rs`). Починить через
`AppEntry::fixture(id, name)` / `Default` на новых полях — не размазывать
пустые `vec![]` вручную в 20 местах без хелпера.

## Зона

- `crates/services/src/applications/types.rs` — поля, parse, тесты.
- `crates/services/src/applications/mod.rs` — `scan_all` фильтр listed.
- `crates/app/src/launcher/search.rs` — haystack + веса.
- `crates/app/src/launcher/view.rs` — только bind поиска / ghost completion.
  Раскладку списка не переписывать.

## Верификация

```
cargo test -p chronos-services applications
cargo test -p chronos --lib launcher
```

Юниты обязательно:

- parse GenericName/Comment/Keywords/locale;
- `[Desktop Action X]` не затирает `Name=` главного (регресс уже есть) и
  попадает в `actions`;
- `NoDisplay`/`Hidden` парсятся, `is_listed == false`, в `scan_all` listed
  их нет;
- query `"term"` находит по Keywords, не только по Name;
- точное Name бьёт fuzzy-другое; frecency не перебивает точное имя.

Live, release: набрать точное имя → оно первое; набрать keyword из
`.desktop` → приложение находится. Empty-query T275 не регрессирует.

## Коммит

`feat(launcher): desktop fields and weighted search (T265-A)`

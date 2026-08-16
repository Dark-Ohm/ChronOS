# T265-B — лаунчер: сетка и категории

**Статус:** BLOCKED — после T265-A в git.
**Приоритет:** P2.
**Родитель:** `T265-launcher-full-functionality.md`.
**Роль:** FRONTEND (`crates/app/src/launcher/**`).
**Канон вида:** `docs/design/Chronos-OSD-Launcher.dc.html`.
Поведение категорий — XDG menu / AppGrid, код QML не копировать.

## Задача

OSD-лаунчер (SUPER+R / `toggle-launcher`) перестаёт быть только списком:

- Grid: иконка + подпись, `columns`/`rows` из констант с дефолтом под
  текущее окно (`LAUNCHER_WIDTH`/`HEIGHT` в `launcher/mod.rs`). Ключи в
  `~/.config/chronos/launcher.toml` можно завести, UI крутилок — T265-G.
- Бар категорий из `AppEntry.categories` (XDG). Пустые категории скрывать.
  «All» всегда. Переход в категорию фильтрует сетку. Hover-open категории
  — да, клик тоже.
- Клавиатура: стрелки по 2D, Home/End/PgUp/PgDn, Tab между поиском /
  категориями / сеткой, Enter запуск, Esc закрыть. `Input` не съедает
  стрелки, когда фокус на сетке (как T275 развёл tab/up/down).
- Компактный режим: видно поиск, сетка сворачивается шевроном вниз.
  Дефолт — развёрнутая сетка.

Кит: `Button` / `Select` из `../Source/gpui-component`. Сетку — `VirtualList`
кита, если есть и без `markdown`/`chart`/`lsp`; иначе CSS-grid/`div` +
`ScrollHandle` как сейчас. Самописный virtualizer — только с строкой в
отчёте.

## Окно

Оставить `WindowKind::Normal` и текущий размер, **если** сетка живёт в
карточке мокапа. Полноэкранный layer-shell Overlay (без exclusive) —
только если живой кадр против `Chronos-OSD-Launcher.dc.html` доказывает,
что карточки мало. Dual-monitor policy не изобретать: как сейчас
`toggle` / focused output.

`Root` + `OnDemand` уже стоят — не второе окно.

## Нельзя

- Папки, избранное, DnD (T265-C).
- Второе меню «Пуск» (T265-H).
- Префиксы (T265-E). Переписывать frecency / поля `.desktop`.
- `side_panel_left`, `Source/gpui/`, `Cargo.lock`.

Список T265-0/`view.rs::render_results` можно заменить сеткой в этой
волне — это и есть визуальная смена. Поиск и pin на строке/клетке
сохранить: правый клик по клетке → существующий `pin_menu`.

## Зона

`crates/app/src/launcher/view.rs` (раскладка), при необходимости
`launcher/grid.rs`, `launcher/mod.rs` (размер окна). Не `applications/`.

## Верификация

```
cargo test -p chronos --lib launcher
cargo build --release -p chronos
```

Live grim: сетка; категория режет выдачу; пустая категория не в баре;
стрелки ходят по клеткам и доводят скролл; компакт ↔ полная; запуск
Enter и клик; pin с клетки жив. Кадр рядом с мокапом.

## Коммит

`feat(launcher): app grid and category bar (T265-B)`

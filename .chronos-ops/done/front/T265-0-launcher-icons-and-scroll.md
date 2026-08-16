# T265-0 — лаунчер: иконки и скролл (волна 0 эпика T265)

**Приоритет:** P2, но идёт первой волной — поломки видны каждый день.
**Родитель:** `T265-launcher-full-functionality.md` (раздел «Волна 0», причины
там же — найдены статически 2026-08-12 и перепроверены на живой системе).
**Роль:** FRONTEND.

## Проблема 1: иконки не резолвятся

`crates/app/src/icon_resolution.rs`:

1. В базах нет `/usr/share/pixmaps` — а это **плоский** каталог без иерархии
   `<theme>/<size>/<context>` (там лежит, например, `anydesk.svg`; подтверждено
   `ls /usr/share/pixmaps/anydesk.svg`). Текущий цикл `base/theme/size/ctx`
   такое не найдёт никогда — нужен отдельный плоский проход ПОСЛЕ тематического
   (freedesktop определяет pixmaps как unthemed fallback).
2. В `sizes` нет `128x128` и `512x512` — из-за этого не находятся
   `chatbox.png` (`/usr/share/icons/hicolor/512x512/apps/chatbox.png`) и
   `CMakeSetup.png` (`/usr/share/icons/hicolor/128x128/apps/CMakeSetup.png`),
   подтверждено на диске.
3. В базах нет flatpak-экспортов `/var/lib/flatpak/exports/share/icons` и
   `~/.local/share/flatpak/exports/share/icons` (на текущей машине каталогов
   нет, но на системе с flatpak-приложениями без них иконки не найдутся).

Зона общая с треем и доком — правка чинит их разом, регрессии проверять на
всех трёх поверхностях.

## Проблема 2: список не скроллится

`crates/app/src/launcher/view.rs:22`: `MAX_VISIBLE_ROWS = 10` обрезает выдачу
в `refresh_results()` до десяти строк — `overflow_y_scroll()` на контейнере
стоит, но переполняться нечему, скролл мёртв.

- Снять жёсткий лимит выдачи: оставить потолок уровня сотен
  (`MAX_RESULTS = 200`) как границу стоимости кадра, не видимости.
- Добавить `ScrollHandle` (`track_scroll` на контейнер результатов, образец —
  `crates/app/src/notifications/history_popup/view.rs`).
- Клавиатурная навигация: `scroll_to_item(selected)` после up/down/tab и в
  `refresh_results()` — в форке это `ScrollStrategy::FirstVisible`,
  минимальная прокрутка до видимости (`Source/gpui/src/elements/div.rs`,
  `scroll_to_active_item`), пустой `child_bounds` безопасен — запрос
  откладывается до появления элемента.

## Зоны файлов

- `crates/app/src/icon_resolution.rs` — базы, размеры, плоский fallback,
  герметичные тесты на temp-каталогах (поиск выносится в функции с
  параметрами-базами, статический `theme_chain`-кэш в тестах не трогаем).
- `crates/app/src/launcher/view.rs` — лимит, ScrollHandle, автопрокрутка.

**Внимание при коммите:** `icon_resolution.rs` в рабочем дереве уже содержит
непринятую работу T263 (мерж трей-резолвера). Коммитить файл можно только
после/вместе с приёмкой T263 — иначе коммит утащит чужие ханки.

## Верификация

```text
cargo test -p chronos icon_resolution
cargo test -p chronos launcher
cargo check -p chronos
cargo build --release -p chronos
```

Плюс живой прогон (обязателен по эпику): SUPER+R → список из >10 приложений
скроллится колесом; стрелка вниз доходит за край видимого окна с
автопрокруткой; anydesk/chatbox/CMakeSetup показывают иконки, а не
букву-заглушку; те же три иконки смотреть в трее/доке, где применимо.
Кадр `grim -g` с геометрией из `hyprctl clients` (класс `chronos-launcher`).

**Живые прогоны разблокированы (2026-08-13).** Причина смерти ввода найдена, и она внешняя: незавершённая Wayland drag-сессия при drag-out из Chronos-FM (`T270`, фикс принят статически в `Source`). Гипотеза popup-grab опровергнута, T264 закрыт. Единственное ограничение до живой проверки T270 — не перетаскивать файлы из Chronos-FM во время сессии.

## Отчёт

`docs/orchestration/tasks/report/T265-0-launcher-icons-and-scroll-report.md`.

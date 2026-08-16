# T265-C — лаунчер: избранное, недавние, папки

**Статус:** BLOCKED — после T265-B в git.
**Приоритет:** P2.
**Родитель:** `T265-launcher-full-functionality.md`.
**Роль:** FRONTEND + persist (`~/.config/chronos/launcher.toml`).

## Задача

Секции на OSD-сетке (одна модель, тот же индекс T265-A):

1. **Избранное** — ручной порядок. DnD внутри секции. Опция «сортировать
   по алфавиту» (ключ toml, UI тумблер можно отложить до G, дефолт —
   ручной порядок). Скрытие подписей в избранном — ключ toml.
2. **Недавние / частые** — **не второй frecency**. Секция = top-N из
   `frecency::cached()` (N ключ, дефолт 8). T275 `record_launch` уже пишет.
3. **Папки** — пользовательские: DnD иконки на иконку создаёт папку,
   раскрытие, переименование (компонентный `Input`, не `String.push`).
4. **Бейдж «новое»** на приложении, `.desktop` которого появилось недавно
   (mtime файла или first-seen в индексе; порог 7 дней). Не путать с recents.

Persist один файл `~/.config/chronos/launcher.toml` (не `frecency.toml`):

```toml
[favorites]
order = ["firefox", "kitty"]
sort_alpha = false
hide_labels = false

[recents]
limit = 8

[[folders]]
id = "..."
name = "Work"
apps = ["code", "slack"]
```

Запись — батч / debounce, не на каждый drag-move. Read-modify-write
`toml::Value`, не serde-дамп всей структуры вслепую (урок T284 / frame.toml).

Добавить в избранное в этой волне: DnD на секцию Favorites. Пункт меню
«Add to favorites» — T265-D, но структура данных должна это пережить.

## Нельзя

- Второй лаунчер / второй индекс.
- Контекст-меню Desktop Actions (T265-D). Pin-меню не ломать.
- Настройки в правой панели (T265-G).
- `Source/gpui/`, `Cargo.lock`, `frecency.rs` формула.

Wayland DnD внутри нашего окна — GPUI drag, не файловый source Chronos-FM
(T270). Не таскать файлы из FM во время смока.

## Зона

`crates/app/src/launcher/**` (`favorites.rs` / `launcher_config.rs` ок).
Не `side_panel_right` кроме если нужен read-only preview — нет, не нужен.

## Верификация

Юниты: порядок favorites после перестановки; recents = top-N frecency;
папка сериализуется и встаёт после reload конфига; unknown id в order
тихо скипается.

Live grim: секции видны; DnD меняет порядок; папка раскрывается;
бейдж на свежем `.desktop`; рестарт шелла сохраняет favorites/folders.

## Коммит

`feat(launcher): favorites, recents, folders (T265-C)`

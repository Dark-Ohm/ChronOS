# T180 — предпросмотр markdown ходит в сеть

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`, раздел «Границы исполнителя» —
прочитать.

Найдено мной при приёмке T179 (живой прогон, `939c26d`). Не регрессия
исполнителя — свойство `gpui-component`, вскрывшееся, как только у нас
появился первый настоящий markdown на экране.

## Факт, воспроизводится за минуту

Открыть панель, вкладка Files → клик по `README.md` → вкладка Preview.
В логе:

```
ERROR gpui::asset_cache: Failed to load asset: error: loading image asset
  from "https://img.shields.io/badge/status-work%20in%20progress-orange"
ERROR … "https://img.shields.io/badge/license-Apache--2.0-blue"
ERROR … "https://img.shields.io/badge/platform-Wayland%20%2F%20Hyprland-blue"
ERROR … "https://img.shields.io/badge/rust-edition%202024-b7410e"
ERROR … "https://img.shields.io/badge/…"
```

Пять исходящих запросов. Источник — строки 7–11 нашего `README.md`:
`[![status](https://img.shields.io/badge/…)](#status)`. Markdown-рендерер
`gpui_component::text::markdown` разрешает `![…](url)` буквально и тянет
картинку по сети.

## Почему это чинится, а не терпится

1. **Просмотр локального файла не должен порождать сетевой трафик.**
   Пользователь кликнул по файлу на своём диске — шелл пошёл на внешний
   хост. Это утечка факта просмотра третьей стороне (внешний хост видит IP
   и время). Ни спека, ни задача такого не обещали.
2. **Спам в логе.** 26 строк `ERROR` на один открытый README — лог
   диагностики превращается в мусор, а `grep panicked at` перестаёт быть
   надёжным инструментом приёмки, если вокруг шум.
3. **Поведение при живой сети хуже, чем при мёртвой.** Сейчас запросы
   падают (в момент прогона сети у процесса не было) — они просто
   *получатся*, когда сеть есть, и мы этого даже не заметим.

## Что сделать

Решение принимает исполнитель, но **обосновать в отчёте**. Три пути,
первый предпочтительнее:

1. **Не грузить удалённые изображения вовсе.** Локальные пути (`./img.png`
   рядом с файлом) — рисовать, `http(s)://` — рисовать плашку-заглушку с
   текстом и самим URL. Пользователь видит, что картинка есть и откуда она,
   но шелл никуда не идёт.
2. **Настройка** (`~/.config/chronos/…`, по образцу `projects.toml`):
   по умолчанию выключено, можно включить осознанно. Дороже, но честнее к
   тем, кто хочет бейджи.
3. **Заглушить на уровне ассет-загрузчика** — если у `gpui-component`
   markdown нет точки расширения, отсечь на уровне `AssetSource`/кэша.
   Проверь, есть ли у рендерера опции (`TextViewStyle`, options у
   `markdown(...)`) — в форке `../Source/gpui-component/crates/ui/src/text/`
   лежит наш код, править его при необходимости **можно**, это наш форк.

Если правишь форк — отдельный коммит в `../Source`, ChronOS переводится на
новый rev; так делалось при переезде `gpui-component` (`57f582f`).

## Зона

- `crates/app/src/side_panel_right/tab/preview.rs` — рендер markdown
- `../Source/gpui-component/crates/ui/src/text/**` — **только если**
  точки расширения нет; правка форка заявляется в отчёте отдельно
- при варианте 2 — новый конфиг и его чтение

**НЕ трогать:** `tab/files.rs`, `tab/build.rs`, `tab/terminal.rs`,
`preview_target.rs`, `view.rs` панели, `crates/services/**`.

## Тесты

- классификация ссылок на изображения: локальный путь / `http` / `https` /
  `data:` — чистой функцией;
- markdown с удалённой картинкой не порождает загрузку (проверяется на
  уровне того, что подставляется в дерево элементов, а не «мы уверены»);
- существующие 14 тестов `tab::preview` продолжают проходить.

## Верификация

```
cargo test -p chronos
cargo clippy -p chronos --all-targets
cargo build --release -p chronos
```

Самодостаточность коммита:

```bash
git stash push --include-untracked
cargo check -p chronos
git stash pop
```

**Живой прогон обязателен** и здесь он простой — доказательство читается
из лога:

1. `set-workspace-mode:developer`, открыть панель, Files → `README.md` →
   Preview;
2. **в логе ноль строк `img.shields.io` и ноль `asset_cache` ERROR**;
3. кадр вкладки: что стоит на месте бейджей — открыть глазами;
4. локальная картинка в markdown продолжает рисоваться (сделай временный
   `.md` со ссылкой на `crates/app/assets/icons/arrow-up.svg`, кадр,
   потом удали файл);
5. `grep -n "panicked at" лог`.

IPC: `$XDG_RUNTIME_DIR/chronos.sock`, команды `set-workspace-mode:developer`
и `toggle-side-panel-right`. Отправить без `socat` можно так:

```python
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("/run/user/1000/chronos.sock")
s.sendall(sys.argv[1].encode()); s.shutdown(socket.SHUT_WR); s.close()
```

**Контент панели раскрывается кнопкой `⊟` в самом низу рейла** (`x ≈ 2539`,
`y ≈ 1414`; `ydotool` — половинные координаты: `1269 707`). Без неё панель
остаётся шириной 54 px, вкладка выбирается, а контента не видно — на этом
я потерял десять минут при приёмке T179. Рейл: `x ≈ 2537`, иконки от
`y ≈ 55` шагом `40`; Preview — пятая сверху, Files — вторая.

## Коммит

Ветка от актуального `master`. Сообщение:
`preview : markdown больше не грузит удалённые картинки (T180)`.
Без AI-трейлеров, `git diff --staged` глазами, поимённый `git add`.
**Коммитишь ты.** Приёмку — нет.

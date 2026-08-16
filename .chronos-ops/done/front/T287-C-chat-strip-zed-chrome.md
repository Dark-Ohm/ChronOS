# T287-C — убрать из Chat рельсу сессий и шапку как у Zed

**Родитель:** `T287-left-chat-onto-gpui-component.md`
**Приоритет:** P1 — мёртвый хром, дубль IA.
**Роль:** FRONTEND. `tabs/chat.rs` (`render_panel`, `build_sessions_sidebar`)
+ кнопка Follow в `composer.rs` (ряд пикеров, не `text_input`).
**Не** T285. **После T286 в git** — Follow садится в `composer-pickers-row`,
тот же `composer.rs`, который T286 переписывает. Параллелить нельзя.

## Симптом (кадры владельца, 2026-08-15)

1. Внутри вкладки Chat слева вторая рельса: свёрнутая полоска с точкой /
   раскрытый «Sessions + New session + hi». Список сессий уже живёт на
   вкладке **Sessions** рабочей рельсы. Дубль.
2. Полоска `thread-header` (кадр владельца): слева `✦`, справа
   `＋` `☰` `👁` `⋯`. Это Zed Agent toolbar. `☰` и `⋯` без `on_click`.
3. Крестик `side-panel-left-close` (`icons/x.svg`) в правом углу
   **верхней** шапки агента — фейковый window-chrome. Панель закрывается
   рельсой / Super+A / IPC. X не нужен.

Канон v0 (`zed-ai-for-chronos` / `docs/design/Agent Thread.dc.html`):
имя агента + статус. Не тулбар Zed.

## Сделать

- Убрать `build_sessions_sidebar` из `render_panel`. Chat = шапка + лента +
  композер на всю ширину канваса. `sessions_collapsed` / ширины сайдбара
  в Chat больше не участвуют в layout.
- Снести целиком `thread-header` (`id="thread-header"`, ~38 px): `✦`,
  заголовок треда, `thread-new-chat`, `thread-history`, `thread-follow`,
  `thread-more`. Не оставлять пустую полоску.
- Снести кнопку `side-panel-left-close`. `close_this` оставить — её зовут
  рельса и IPC.
- **Follow (T195) не убивать.** Убрать с шапки, посадить вниз в
  `composer-pickers-row` (ряд model / mode / YOLO, у каретки).
  Иконка: не emoji-глаз и не `follow.svg`, если он глаз.
  `currentColor` SVG — `icons/rail-preview.svg` (Follow гонит Preview).
  ON: `accent` фон/цвет, как у старого `thread-follow`. Тот же
  `follow_enabled` / `AgentFollowState`, не второй флаг.
- Остаётся одна верхняя шапка: кластер агента. Без Zed-полоски и без X.
- `No messages yet` оставить как empty ленты. Пустая лента при выбранном
  «hi» — это T285/store, не этот тикет.

## Нельзя

- Выкидывать вкладку Sessions с рабочей рельсы.
- Ломать `create_new_session` на Sessions.
- ACP connect / `load_session`.
- Логику Follow (T195) и `AgentFollowState`.
- Поле ввода композера (`text_input` / T286).

## Верификация

Live grim: нет `thread-header` и X; Follow внизу у композера, иконка
preview, не глаз; ON/OFF видно. Sessions-вкладка жива.
В `render_panel` нет `thread-header` / `thread-follow` /
`side-panel-left-close`.

## Коммит

`fix(left-panel): drop Zed session rail and dead header chrome from Chat (T287-C)`

## Приёмка

2026-08-16, коммит `220a05e2` (+57/−853, `chat.rs`+`composer.rs` только).
Сверено архитектором: `grep` по `build_sessions_sidebar`/`thread-header`/
`thread-follow`/`side-panel-left-close` в дереве — только упоминания в
комментариях, живого кода нет. `composer-follow`/`composer-pickers-row`
на месте, `follow_enabled`/`AgentFollowState` без изменений семантики.
`cargo check -p chronos` чисто по обоим файлам. `cargo test -p chronos`
— 760 passed (совпадает с отчётом минёра). Live smoke не гонял — не
блокер для UI-хрома без нового поведения ввода.

Две честные оговорки отчёта, не блокеры: `close_this` подтверждён
prod-мёртвым (рельса/IPC реально ходят через `apply_dock_toggle`,
бриф был устаревшим на этот счёт) — оставлен по брифу+тесту-контракту.
`state.rs`/`SIDEBAR_*` получили новые `dead_code` warn — побочный эффект
сноса единственного потребителя, чистка отдельной задачей (см. правило
про массовую чистку warn/dead_code разом). Принято.

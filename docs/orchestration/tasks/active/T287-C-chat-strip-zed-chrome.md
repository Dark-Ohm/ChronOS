# T287-C — убрать из Chat рельсу сессий и шапку как у Zed

**Родитель:** `T287-left-chat-onto-gpui-component.md`
**Приоритет:** P1 — мёртвый хром, дубль IA.
**Роль:** FRONTEND. `tabs/chat.rs` `render_panel` / `build_sessions_sidebar`.
**Не** T285 (ACP), не T286 (композер Input). Можно после T286 или до:
зона — шапка и сайдбар, не `text_input`.

## Симптом (кадры владельца, 2026-08-15)

1. Внутри вкладки Chat слева вторая рельса: свёрнутая полоска с точкой /
   раскрытый «Sessions + New session + hi». Список сессий уже живёт на
   вкладке **Sessions** рабочей рельсы. Дубль.
2. Над лентой — шапка как у Zed Agent: `＋` `☰` `👁` `⋯` (и крестик).
   `☰` и `⋯` **без `on_click`**. Это скриншот из Zed, не наш мокап.

Канон v0 (`zed-ai-for-chronos` / `docs/design/Agent Thread.dc.html`):
имя агента + статус. Не тулбар Zed.

## Сделать

- Убрать `build_sessions_sidebar` из `render_panel`. Chat = шапка + лента +
  композер на всю ширину канваса. `sessions_collapsed` / ширины сайдбара
  в Chat больше не участвуют в layout.
- Новый тред — только вкладка Sessions (`+ New`). Кнопка `＋` в шапке Chat
  уходит вместе с тулбаром.
- Срезать мёртвые `thread-history` (`☰`) и `thread-more` (`⋯`).
- Follow (`👁`, T195) — не в этой шапке. Либо выкинуть из Chat-хрома
  (предпочтительно), либо одна иконка в меню агента, не ряд из четырёх.
- Шапка: кластер агента (сигил + имя + статус + переключатель). Без
  англо-Zed «Connected» как отдельной подписи, если статус уже точкой;
  не плодить вторую строку «New Agent Thread» ради пустоты.
- `No messages yet` оставить как empty ленты. Пустая лента при выбранном
  «hi» — это T285/store, не этот тикет.

## Нельзя

- Выкидывать вкладку Sessions с рабочей рельсы.
- Ломать `create_new_session` на Sessions.
- ACP connect / `load_session`.
- Композер.

## Верификация

Live grim: Chat без левой колонки сессий, без `＋☰👁⋯`. Sessions-вкладка
по-прежнему список + New. `rg thread-history|thread-more|build_sessions_sidebar`
в `render_panel` — пусто.

## Коммит

`fix(left-panel): drop Zed session rail and dead header chrome from Chat (T287-C)`

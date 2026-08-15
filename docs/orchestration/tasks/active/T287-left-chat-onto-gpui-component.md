# T287 — левый Chat на gpui-component (эпик)

**Приоритет:** P1 — самописный композер/пикеры ломают ввод.
**Роль:** FRONTEND. Волны — отдельные тикеты, не один заход.
**Канон:** DECISIONS 2026-08-15; кит `../Source/gpui-component/crates/ui`.

## Уже на ките (не трогать)

| Что | Где |
|---|---|
| `Root` | обе панели |
| `Input` | лаунчер T275, Preview editor |
| `PopupMenu` | dock / tray / launcher pin |
| `text::markdown` | Preview |

## Chat-вкладка сейчас

| Кусок | Сейчас | Кит | Волна |
|---|---|---|---|
| Поле ввода | `text_input.rs` `shape_line`, одна линия | `Input` multi-line | **T286** (уже в active) |
| Model picker | самописный dropdown + `String` поиск | `Select` / `ComboBox` | T287-A |
| Mode picker | то же | `Select` | T287-A |
| Поиск сессий | `thread_search: String` | `Input` | T287-B |
| Список сессий | свои `div` | `List` / `VirtualList` | T287-B |
| Лента / reasoning / tool cards | свои | нет аналога | **не переносить** |
| Рельса иконок | своя | нет | не переносить |

Не тащить `dock`/`table`/`tree` в Chat. Не включать `lsp`/`chart`.

## Порядок

1. **T286** — композер. Боль пользователя, `text_input.rs` уходит.
2. **T285** — ACP `load_session` (не компонент, но тот же `chat.rs`:
   не параллелить с T286).
3. **T287-A** — пикеры model/mode на `Select`. После T286:
   `compose-and-send` уже на `InputState`.
4. **T287-B** — Sessions list + search. После A.

Каждая волна — свой бриф, свой коммит, live grim. Эпик целиком не отдавать.

## Модель

300B на волну. Не 30B (снова нарисует dropdown). Не 1T на весь эпик.

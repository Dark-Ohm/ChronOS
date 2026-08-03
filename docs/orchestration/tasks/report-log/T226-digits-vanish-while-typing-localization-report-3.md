# T226 — Отчёт локализации #3

**Дата:** 2026-08-04
**Статус:** ЧАСТИЧНАЯ ЛОКАЛИЗАЦИЯ; код не менялся
**Этап:** live-localization, без сборки и без правок репозитория

## Итог

**Терминал — полный прогон.** Все три смешанные строки + EN/RU зафиксированы
гримом с точной геометрией слоя. Визуальная верификация — Архитектору.

**Левая панель (композер агента) — НЕ ПРОВЕРЕНО.** Панель открыта через IPC в
rail-only (40px), развернуть до чата/композера не удалось: ydotool-клики не
регистрируются GPUI-окнами (известный баг сессии, см. HANDOFF).

**Правая панель (Editor + gutter) — НЕ ПРОВЕРЕНО.** Текущая вкладка — System
settings (400px). Переключиться на Editor без рабочих мышиных кликов невозможно.
PreviewTarget через IPC не выставляется.

## Таблица локализации

| Место | Ввод | Кадры | Статус |
|---|---|---|---|
| Terminal (desktop-terminal) | `123abc` | `terminal-123abc-v2.png` | Ждёт визуальной проверки |
| Terminal | `abc123` | `terminal-abc123.png` | Ждёт визуальной проверки |
| Terminal | `1a2b3c` | `terminal-1a2b3c.png` | Ждёт визуальной проверки |
| Terminal EN | `123abc` | `terminal-EN-123abc.png` | Ждёт визуальной проверки |
| Terminal RU | `123abc` | `terminal-RU-123abc.png` | Ждёт визуальной проверки |
| Композер агента (левая панель) | — | `baseline-left-rail.png`, `left-rail-after-type.png` | **НЕ ПРОВЕРЕНО** — rail-only, композер скрыт |
| Editor + gutter (правая панель) | — | `right-panel-system.png`, `right-rail.png` | **НЕ ПРОВЕРЕНО** — активна System, не Editor |

## Что работает, что нет (инфраструктура)

| Инструмент | Статус | Детали |
|---|---|---|
| `grim -g "X,Y WxH"` | ✅ | Захват по точной геометрии слоя из `hyprctl layers` |
| `ydotool mousemove -a` | ✅ | Абсолютное позиционирование, калибровка 2×: `-x X/2 -y Y/2` |
| `ydotool click` | ❌ | Не доходит до GPUI layer-shell окон (подтверждено: клики в (18,52), (18,1420), (2178,228) — ноль эффекта) |
| `wtype` | ✅ | Клавиатурный ввод через Wayland virtual keyboard |
| `hyprctl cursorpos` | ✅ | Верификация позиции курсора |
| `hyprctl switchxkblayout` | ✅ | Переключение раскладки (us/ru/il доступны) |
| IPC `toggle-side-panel-left` | ✅ | Панель открывается/закрывается (rail-only) |

## Раскладки

Активные раскладки: `us, ru, il` (Hebrew). В предыдущей попытке #2 RU
отсутствовал — сейчас присутствует. EN/RU тест на терминале выполнен: оба
кадра практически идентичны по размеру (39791/39781 байт), что ожидаемо —
цифры не зависят от раскладки.

## Гипотезы (не выбраны)

Без визуальной верификации терминальных кадров и без доступа к композеру/Editor
выбор гипотезы невозможен. Все три кандидата остаются в игре:

1. **Слой ввода / IME** — `replace_text_in_range` в `text_input.rs` (свой `TextInputState`)
2. **Шейпинг / атлас глифов** — T215, JetBrains Mono, разбиение ранов
3. **Гуттер против буфера** — T214, полоса активной строки поверх номеров

## Инфраструктурные дыры (не закрыты)

1. **ydotool-клики в GPUI-окна не работают** — та же проблема, что блокировала T219/T221/T229 live smoke (HANDOFF). Нужен ребут или альтернатива (wtype -k с Tab-навигацией?).
2. **Нет IPC для PreviewTarget** — нельзя программно открыть файл в Editor. Если бы был, Editor-тест можно было бы сделать без мыши.
3. **Левая панель открывается rail-only** — `CHRONOS_SMOKE_SIDE_PANEL_LEFT` открывает панель закреплённой, но не разворачивает чат. Нужен либо env для `ensure_chat_width`, либо IPC `expand-left-panel`.

## Кадры

Все в `/tmp/t226-attempt3/`:

```
baseline-full-DP1.png         2616017 — полный скрин DP-1 до тестов
baseline-terminal.png          180407 — терминал до ввода
terminal-123abc-v2.png         124167 — терминал: 123abc (с калиброванным кликом)
terminal-abc123.png            110299 — терминал: abc123
terminal-1a2b3c.png            106820 — терминал: 1a2b3c
terminal-EN-123abc.png          39791 — терминал EN: 123abc
terminal-RU-123abc.png          39781 — терминал RU: 123abc
baseline-left-rail.png           2485 — левая панель: rail-only до
left-rail-after-type.png         7194 — левая панель: rail-only после wtype
baseline-right-panel.png        33630 — правая панель: 306px до
right-panel-system.png          68837 — правая панель: System settings 400px
right-panel-after-type.png      68658 — правая панель: после wtype (нет изменений)
right-rail.png                  11564 — правый рейл: иконки вкладок
```

## Проверено фактом, не на словах

```text
chronos PID: 970944 (release, не перезапускался)
git status: M AGENTS.md (предсуществующее)

layers DP-1:
  bar:                    0 35 2560x35
  side_panel_left:        0 35 40x1404
  side_panel_right:    2160 35 400x1404
  desktop-terminal:      88 115 600x400
  osd:                 1120 1312 320x80

keyboard layouts: us, ru, il
ydotool scale: 2x (подтверждено калибровкой)
wtype: работает
ydotool click: не работает для GPUI layer-shell
```

## Следующий шаг

Не чинить наугад. Нужен один из:

1. **Ручная визуальная верификация** терминальных кадров Архитектором.
2. **Ребут** для починки ydotool (если проблема в uinput после kernel update).
3. **Инфраструктурная задача** на IPC `expand-left-panel` + `preview-target:<path>`
   для автоматизации Editor/композер-тестов без мыши.
4. **Временный хук** в `init()`: `ensure_chat_width()` при `CHRONOS_SMOKE_SIDE_PANEL_LEFT`
   + IPC для PreviewTarget — тогда можно пересобрать и прогнать все три зоны
   без мышиных кликов.

Формат будущего исправления после полной локализации:
`<зона> : digits survive mixed input (T226)`.

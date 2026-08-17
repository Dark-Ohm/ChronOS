# T255 — offline/local trust signal: реализация двух текстовых меток

**Роль:** FRONTEND (Rust, GPUI), минимум кода — чисто текстовые метки,
без новых компонентов/бейджей/иконок.
**Источник решения:** `docs/orchestration/tasks/report-log/T240-offline-trust-signal-report.md`
(дизайн одобрен пользователем 2026-08-05). T240 закрывается ссылкой на
этот тикет, отдельного коммита у T240 нет.
**Приоритет:** P2/продуктовый.

## Что сделать

Два статичных текстовых места, оба — muted mono-подпись, без hover-only
контента (тест доверия — это статичный кадр, должно читаться сразу):

### 1. ACP settings — `crates/app/src/side_panel_right/tab/acp_settings.rs`

Под шапкой, рядом со строкой `"{n} agent(s) · agents.toml"` (сейчас
`acp_settings.rs:160`, subtitle шапки). Добавить вторую mono-подпись:

```
local only · no network · no telemetry
```
(или RU-вариант «локально · без сети · без телеметрии» — сверься с
остальными строками таба, весь остальной UI на английском, судя по
`"agent(s)"`/`"agents.toml"`/`"Open agents.toml"` — держи английский для
консистентности, если не найдёшь примеров русского текста в UI).

Стиль — как у существующего subtitle: `theme.text.muted`, `text_xs()`,
`font_family(theme.font_mono)`. Смотри блок шапки вокруг `acp_settings.rs:137-160`
(комментарий `// T231-pattern header: semibold title + mono subtitle.`) —
воспроизведи тот же паттерн, не изобретай новый.

**Важно (регрессия T249):** шапка вкладки использует
`HEADER_H_PX: f32 = 62.0` (`acp_settings.rs:23`) — константа, из которой
T249 вычисляет пол высоты карточки (`min_h`, растягивание до низа
скролл-вьюпорта). Комментарий над константой прямо предупреждает: «Presumes
a single-line header — the subtitle is short enough at 320px». Добавление
ВТОРОЙ строки текста увеличит реальную высоту шапки — либо:
- убедись, что новая строка помещается в существующую высоту (если это
  короткая вторая строка `text_xs` с своим `gap`, скорее всего HEADER_H_PX
  придётся пересчитать — измерь живым grim на 320px, не гадай), либо
- обнови `HEADER_H_PX` на новое реальное значение и оставь комментарий,
  откуда оно взялось (как T249 сделал для текущего).
Если не пересчитать — низ карточки на короткий список агентов либо не
дотянется до дна вьюпорта (щель снизу), либо карточка вылезет за него.
Проверяй на 320px (панель на этой машине открывается на этой ширине,
см. T249-отчёт) в обеих темах.

### 2. About / Build info — `crates/app/src/side_panel_right/tab/bar_settings.rs`

Секция `About` уже существует (`bar_settings.rs:597-599`, `section_header`
+ карточка с строками `ChronOS shell / версия`, `Desktop shell for
Hyprland / Apache-2.0`). Добавить ОДНУ новую строку в том же паттерне
(`div().flex().justify_between()` с левой/правой mono-подписью,
`text_color(theme.text.muted)`, `text_xs()`):

```
offline by design — no network · no telemetry
```

Смотри существующие пары строк сразу под `.child(section_header(theme, "About", "Build info"))`
(`bar_settings.rs:599` и далее) — вставь как ещё одну пару `justify_between`
внутри той же карточки, тем же стилем что соседние строки (не заводи
отдельную карточку).

## Не входит в scope

- Бар не трогаем (T234: и так плотный).
- Никакого onboarding/first-run экрана (не существует).
- Никаких иконок/бейджей — только текст.

## Верификация

- `cargo build --release -p chronos` — чисто.
- `cargo test --release -p chronos --lib -- side_panel_right` — 167/167
  (не меньше — если добавишь тест на новую строку, ок, но не обязателен).
- Живой grim, обе темы: ACP settings tab (320px) и Bar Settings → About
  секция — обе новые строки видны, не обрезаны, не наезжают на соседний
  текст. Если правил `HEADER_H_PX` — отдельным кадром подтверди, что
  карточка ACP settings всё ещё доходит до низа вьюпорта на коротком
  списке агентов (тот же тест, что T249 делал) и не вылезает за него.

## Отчёт

`docs/orchestration/tasks/report/T255-offline-trust-signal-impl-report.md`.
Коммит: `ui : offline trust signal — ACP settings + About (T255)`.

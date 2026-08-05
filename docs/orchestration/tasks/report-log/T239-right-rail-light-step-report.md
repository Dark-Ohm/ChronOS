# T239 — right rail light-theme step: verified live, missing, fixed

**Дата:** 2026-08-05
**Статус:** FIXED (была verification-задача, породила код-фикс)
**Роль:** FRONTEND/QA + живой замер

## Вердикт

Ступени рейл/контент в светлой теме **не было** (рейл и контент брали
один и тот же `bg.primary`) — восстановлена на один уровень палитры
(`bg.tertiary`), зеркально тёмной теме. Подтверждено пиксель-сэмплом,
не на глаз.

## Живой замер (до фикса)

Светлая тема, рейл rail-only, `grim` + пиксель-сэмпл (кадры:
`docs/orchestration/tasks/notes/T239-rail-light.png`,
`T239-content-light.png`, `T239-content-open-light.png`):

- Рейл: `#DDE0F2` = `bg.primary`
- Контент-колонка: `#DDE0F2` = `bg.primary`

**Ступени нет по построению.** В тёмной теме ступень есть (chrome =
`bg.tertiary` #181825, контент = `bg.primary` #1e1e2e) — светлая схема
просто не имела своего уровня для chrome. Это и есть причина, по которой
T223-аудит дважды поднимал «рейл сливается» без подтверждения.

## Фикс (`side_panel_right/surfaces.rs`)

Светлый `chrome()` → `bg.tertiary` (#ECEEFA), зеркально тёмному. Обновлён
юнит-тест `light_chrome_is_page_card_is_cardbg` (ассертил старое поведение
= pageBg).

## После фикса (live)

- Рейл: `#ECEEFA` = `bg.tertiary` ✓
- Контент: `#DDE0F2` = `bg.primary` ✓
- Ступень видимая, но не резкая — один уровень палитры, как в тёмной.

Образцы `#E0E3F4` внутри контента — elevated-карточки из T231, не фон
колонки (проверено по краю колонки вне карточек).

## Верификация

- `cargo build --release -p chronos` — чисто
- `cargo test --release -p chronos --lib -- side_panel_right` — **167/167**
- Живой замер: ступень видна в обеих темах

## Коммит

`ui : right rail light-theme step on one palette level (T239)` —
ожидается в составе серии.

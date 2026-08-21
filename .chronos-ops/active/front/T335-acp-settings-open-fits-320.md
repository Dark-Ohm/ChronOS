---
ticket: T335
role: front
status: active
tags: [chronos-ops, front, active]
---

# T335 — Open agents.toml читается на 320 px

**Роль:** FRONTEND. **P2.** Живая находка T327 B2.
**Зона:** `crates/app/src/side_panel_right/tab/acp_settings.rs` Actions
(`:273-346`).
**Не трогать:** `updates.rs` (T334), `view.rs` (T336), preferred width
ACP = 320 (`tabs.rs`) — штатная ширина, не поднимать, чтобы спрятать
перенос.

Параллелен T334/T336.

## Зачем

ACP settings на 320 px: **Open agents.toml** ломается на три строки,
последняя — одна `l`. Reload жив (T212/`flex_none`). Основное действие
выглядит сломанным.

Кадр: `dump/qa-ux/T327/frames/right-acp_settings.png`.
Источник: `done/qa/DRAFT-T335-acp-settings-open-button-wraps-at-default-width.md`.

## Корень (сверено)

- Open-card: `flex_1` + `min_w(0)` (`:282-283`).
- Title без nowrap/ellipsis (`:301-306`).
- Reload `flex_none` — **не снимать**, иначе снова клип T212.

## Что сделать

На 320 px оба действия читаются с первого взгляда: колонка Actions
**или** title/path elide без однобуквенной строки. Широкая панель —
иерархия как сейчас.

## Готово когда

- Живой кадр на 320 px: «Open agents.toml» и Reload целые. grim не `/tmp`.
- Тест/хелпер на narrow vs wide, если ветка есть в коде.
- `cargo test -p chronos --lib` не краснеет.

**Отчёт:** `.chronos-ops/reports-fresh/T335-acp-settings-open-fits-320-report.md`

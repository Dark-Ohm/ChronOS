# T238 — System settings: сверить цвет кнопки Power с theme.status.error

**Роль:** FRONTEND, короткая задача.
**Источник:** `docs/orchestration/tasks/report/T223-design-audit-report.md`,
находка #5 (P2, требует проверки), топ-10 п.6.
**Приоритет:** P2.

## Находка (дословно, с оговоркой критика)

`light-system.png`, точка (2470,1413) реальных px. Пиксель-сэмплинг дал
`srgb(217,101,133)` (`#d96585`) — не совпадает ни с `status.error`
Light C (`#d20f39`, тест `light_scheme_status_is_latte_not_mocha` в
`schemes.rs:187`, подтверждено архитектором), ни с Mocha `#f38ba8` в лоб.

**Критик сам пометил:** "координата приблизительная, могла задеть
anti-aliased край/иконку, а не сплошную заливку" — **это не
подтверждённый баг, а флаг на проверку**.

## Что нужно

1. Найти код кнопки Power в System settings
   (`crates/app/src/side_panel_right/tab/` — вкладка System, секция с
   Switch/Log out/Restart/Power).
2. Проверить, действительно ли `.bg()`/`.text_color()` этой кнопки
   ссылается на `theme.status.error`, или где-то хардкожен hex мимо
   токена.
3. Если хардкод найден — заменить на `theme.status.error`.
4. Если токен уже используется правильно — закрыть тикет как "не баг,
   находка была на anti-aliased пикселе/blend с фоном", ничего не
   менять.

## Верификация

```bash
cargo build --release -p chronos
```

Live: `grim` System settings в light-теме, пиксель-сэмпл заливки кнопки
Power (не края/иконки) — должен совпасть с `#d20f39` точно.

## Отчёт

`docs/orchestration/tasks/report/T238-power-button-color-report.md` —
включить итоговый вердикт (баг подтверждён и починен / ложное
срабатывание, не баг).

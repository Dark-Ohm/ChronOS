<!-- T116 — Updates popup, anchored redesign (пилот полировки попапов). Агент не назначен. -->

## ЗАДАНИЕ — Updates popup: anchored позиционирование + реальный скролл + визуал

**План целиком (читай ПЕРЕД стартом, исполняй задача-за-задачей):**
`docs/superpowers/plans/2026-07-24-updates-popup-anchored-redesign.md`

Формат `writing-plans` (bite-sized шаги, чекбоксы, код в каждом шаге) —
исполняй по `superpowers:executing-plans`/`superpowers:test-driven-development`
дисциплине: тест → красный → код → зелёный → коммит, задача за задачей.
6 задач, Task 6 (живая верификация release+grim, dark И light темы) —
ОБЯЗАТЕЛЬНА, не опциональный полироль.

**Контекст:** `updates_popup` — пилот полировки попап-системы (4 других
попапа — `volume_popup`/`system_popup`/`notifications/history_popup`/
`tray_menu` — НЕ в этой задаче, будущие T-задачи по этому же образцу
после приёмки пилота). Пользователь назвал 4 проблемы: все попапы
анкорены в один и тот же угол экрана независимо от триггера; скролл был
отложен, не отменён (сейчас `max_h()+overflow_hidden()` обрезка); визуал
"MVP"; окно не подстраивается под контент.

**Референсы:** спека — `docs/superpowers/specs/
2026-07-24-updates-popup-anchored-redesign-design.md`; мокап —
`design/Updates Popup.dc.html` (dark эталон + light "Light C" принятый
вариант, буквальные хексы/тени/opacity — канон, не выдумывать); механизм
позиционирования — skill `anchored-popups` (`gpui/src/platform/popup.rs`).

## Зона файлов

- `crates/ui/src/theme/mod.rs`, `crates/ui/src/theme/schemes.rs`
  (Task 1 — `Theme.is_light` флаг)
- `crates/app/src/bar/widgets/updates.rs` (Task 2 — bounds capture,
  mouse-down)
- `crates/app/src/updates_popup/mod.rs` (Task 3-4 — AnchoredPopup,
  реальный скролл)
- `crates/app/src/updates_popup/view.rs` (Task 4-5 — скролл, визуал)

Не пересекается с T113/T114/T115 (те заморожены, не в работе) и с
остальными 4 попапами (не тронуты).

## Что НЕ делать

- Не трогай `volume_popup`/`system_popup`/`notifications/history_popup`/
  `tray_menu` — отдельные будущие задачи по этому образцу после приёмки
  пилота.
- Не трогай bar widget layout config (`bar.toml`, отдельный фронт) и
  Plugin API v2 (отдельный фронт) — не связаны с попапами.
- Не переводи существующие цвета `view.rs` на хардкод-хексы — файл уже
  на `Theme::global(cx)` токенах, план это учитывает явно (Global
  Constraints плана, "spec correction").
- Два места в плане явно требуют ПРОВЕРИТЬ точный API перед кодом, не
  гадать: тип ошибки `PopupNotSupportedError` (Task 3, Step 1) и билдер
  box-shadow (Task 5, Step 1, fork добавляет поле `inset` к `BoxShadow`
  относительно crates.io — см. `skills/fork-api-drift`). Не обходи через
  `unwrap`/угадывание имени метода.
- Второй `let _ = handle.update(...)` в `updates_popup::init()` — план
  явно требует заменить на `.log_err()` раз уже касаешься этой функции
  (Task 4, Step 2), не оставляй как есть и не добавляй новых `let _ =`.

## Коммит

Поимённый `git add`, малыми коммитами по задачам плана (план уже даёт
готовые сообщения коммитов в каждом Step). В master, не в отдельную ветку
— но НЕ пушь, архитектор проверяет дерево локально перед пушем.

## Отчёт

`docs/orchestration/tasks/report/T116-updates-popup-anchored-redesign-report.md`
— что из 6 задач плана сделано, вывод живого смока (Task 6: скрины anchored
positioning, скролл, dark+light темы, dismiss-пути), любые расхождения с
планом (особенно если `canvas()`-подход к bounds capture из Task 2 Step 2
не сработал “в лоб” — план сам называет это открытым риском с фолбэком).
Честно про PENDING, если живой смок не удался (см. фрод-таблицу `rules.md`).

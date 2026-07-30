<!-- T015 — migrated 2026-07-22 from orchestration/report-log/omp-report.md — see orchestration/tasks/MIGRATION.md -->

# Session: launcher focus trap hotfix — 2026-07-17

## Сделано (факт, не намерение)
- docs/hyprland/chronos-launcher.lua: удалены `stay_focused = true` и `pin = true` — несовместимы с focus-lost-close; обновлен header-комментарий
- crates/app/src/launcher/view.rs: удалён per-frame re-assert (`if !self.focus.is_focused(_window) { self.focus.focus(...) }`) — он перехватывал фокус обратно у композитора, создавая вечный капкан; Esc/Enter теперь вызывают `crate::launcher::close(cx)` вместо голого `window.remove_window()` — это чистит `LauncherState.handle`
- crates/app/src/launcher/mod.rs: добавлен `observe_window_activation` в `open()` — лаунчер закрывается при потере фокуса (клик мимо, переключение воркспейса), как rofi/fuzzel; обновлён doc-коммент к `window_options`

## Расхождения со спекой/планом
- ОMP требовал: «Per-frame re-assert больше НЕ безусловный: либо убрать совсем, либо гейтить». Сделано: убран совсем. Причина: с `stay_focused` удалённым, compositor grants focus через normal policy; re-assert каждый кадр был контрпродуктивен (файтинг с композитором). Начальный `focus_input()` в `open()` покрывает «печатать сразу».
- ОMP требовал: «Focus-lost → close: подписка на активность окна». Сделано: `observe_window_activation` в `open()`, callback проверяет `window.is_window_active()` и вызывает `close()` при потере фокуса.
- ОMP требовал: «Esc/Enter-путь должен чистить LauncherState.handle». Сделано: Esc/Enter вызывают `crate::launcher::close(cx)` вместо `window.remove_window()`.
- ОMP требовал: «`stay_focused` в Lua-правиле: скорее всего УБРАТЬ. `pin` пересмотри тоже». Сделано: оба удалены.

## Не реализовано из acceptance criteria
- Живой смок в Hyprland (пункты 1-7 из OMP) — НЕ сделан: требует запуска Chronos в сессии пользователя. Changes компилируются и тесты проходят, но поведение фокуса нужно верифицировать на live Wayland session.
- Скриншоты (grim) — не делались, нет live session.

## Проверено фактом, не на словах
- `cargo check -p chronos` — 0 errors, 3 warnings (deprecated proc-macro-error2, unused Task in notifications — pre-existing)
- `cargo test --workspace` — 74 passed, 0 failed (4 + 26 + 25 + 16 + 3 = 74)

## Новые риски / известные баги
- **Severity: Medium** — `observe_window_activation` может сработать во время уничтожения окна (после `window.remove_window()`). В таком случае `close()` будет no-op (handle уже taken). Безопасно, но нужен live-тест чтобы убедиться что нет двойного close или race condition.
- **Severity: Low** — `is_window_active()` может возвращать `false` во время анимации pop-in (окно ещё не fully mapped). Если это вызовет ложное закрытие — потребуется гейтинг по initial focus state. Пока гипотеза, не проверена.

## Статус ARCHITECTURE.md / DECISIONS.log
- ARCHITECTURE.md: не обновлён (focus trap hotfix не меняет архитектурных решений, только реализацию launcher UX)
- DECISIONS.log: не обновлён (нет новых решений — удаление stay_focused/pin/re-assert логически следует из уже принятого решения о миграции на XDG toplevel)

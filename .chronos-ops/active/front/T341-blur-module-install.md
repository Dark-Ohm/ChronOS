# T341 — Blur: положить модуль, не серая кнопка

**Роль:** FRONTEND. **P1.** T328 B4.
**Зона:** `crates/app/src/side_panel_right/tab/bar_settings.rs` (строка
Blur, `ModuleMissing` ~576/612), чтение
`packaging/hyprland/45-surface-effects-chronos.lua` (не переписывать
канон модуля). Reload Hyprland — существующий путь, не
`KeyboardInteractivity::Exclusive`.
**Не трогать:** `surface_effects.rs` capability probe кроме повторного
вызова после install.

## Зачем

`theme.toml` `blur_enabled = true`. Старт: `capability=ModuleMissing`.
UI: disabled «no module», подпись обрезана
`import 45-surface-effects-chrono…`. Модуль в репо есть, в
`~/.config/hypr/modules/` нет (00…40, без 45). `crops/07-appearance-blur-row.png`.

## Что сделать

При `ModuleMissing`: кликабельное «Install module» (или эквивалент)
копирует `packaging/hyprland/45-surface-effects-chronos.lua` →
`~/.config/hypr/modules/45-surface-effects-chronos.lua`, если нет;
reload compositor; re-probe. Не затирать чужой файл без спроса.
Подпись не резать посередине имени.

Не изобретать blur в GPUI, если модуля нет.

## Готово когда

На этой машине после Install: кнопка Blur живая, лог не ModuleMissing,
`ls ~/.config/hypr/modules/45-surface-effects-chronos.lua`. grim.

**Отчёт:** `.chronos-ops/reports-fresh/T341-blur-module-install-report.md`

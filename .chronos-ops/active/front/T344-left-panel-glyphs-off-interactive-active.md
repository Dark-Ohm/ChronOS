# T344 — левая панель: глифы слезают с `interactive.active`

**Роль:** FRONTEND. **P1.** Побочка T340 (принят 2026-08-21).
**Зона:** `crates/app/src/side_panel_left/tool_card.rs` +
`crates/app/src/side_panel_left/chat_view.rs` — только эти два файла.
**Не трогать:** `crates/ui/src/theme/schemes.rs` (T340 закрыт, палитры
не крутить), `bar_settings.rs`, `sessions.rs`, `project.rs`.

## Зачем

`interactive.active` — токен **состояния контрола** (плита выбранного).
T340 опустил его в Solarized Dark с base04 `#839496` на base01 `#073642`,
чтобы выбранный чип настроек читался (было 1.19:1). Правка верная, но три
места в левой панели рисуют этим токеном **глифы**, и на Solarized они
стали `#073642` на `bg.primary` `#002b36` — контраст ≈1.2:1, глиф
пропадает:

- `tool_card.rs:23` — цвет точки статуса для ветки `_` (не
  running/done/error, т.е. pending).
- `tool_card.rs:74` — шеврон `▸`/`▾` раскрытия карточки инструмента.
- `chat_view.rs:117` — пустое состояние «No messages yet».

Фон карточки — `theme.bg.primary` (`tool_card.rs:80`).

## Что сделать

Перевести все три места на **`theme.text.muted`**. Он уже под воротами
T317: тест в `crates/ui/src/theme/schemes.rs` итерирует
`builtin_schemes()` и требует `text.muted` ≥ 4.5:1 на `bg.primary` в
каждой схеме — то есть новый цвет глифа гарантированно читается во всех
четырёх схемах, включая будущие.

Расщеплять токен (`selected-plate` / `subdued-text`) **не надо** — это
уже разобрано на приёмке T340 и отклонено как лишняя сущность: три места
хотят приглушённый текст, а он в теме есть.

Если по ходу найдёшь ещё места в `side_panel_left/`, где
`interactive.active` красит текст, а не фон, — чини так же и перечисли в
отчёте. Места, где он **фон** (`sessions.rs:518`, `project.rs:138`), —
правильные, не трогать.

## Готово когда

- Живой прогон на **Solarized Dark** (`theme.toml` → `scheme =
  "Solarized Dark"`, hot-reload, без рестарта): в левой панели видны
  шеврон карточки инструмента и текст пустого чата. `grim` в отчёт, до
  и после. Стол вернуть в `Default`.
- `grep -rn 'interactive\.active' crates/app/src/side_panel_left/` — в
  выдаче остаются только фоновые применения (`.bg(`).
- `cargo test -p chronos --lib` и `-p chronos-ui --lib` не краснеют.

## Рецепт живого смока (проверено на T329/T340, не изобретать заново)

```bash
S=/run/user/1000/chronos.sock            # socat в системе НЕТ, слать питон-скриптом
# toggle-side-panel-left | toggle-side-panel-right | select-tab:<id>
export YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket
hyprctl dispatch 'hl.dsp.cursor.move({x=2404, y=10})'   # ТОЛЬКО так; ydotool --absolute промахивается
hyprctl cursorpos                                        # сверить ПЕРЕД кликом
ydotool click 0xC0
```
Кадры класть в `.chronos-ops/dump/qa-ux/T344/` (каталог в `.gitignore`,
PNG в git не коммитятся).

**Отчёт:** `.chronos-ops/reports-fresh/T344-left-panel-glyphs-off-interactive-active-report.md`

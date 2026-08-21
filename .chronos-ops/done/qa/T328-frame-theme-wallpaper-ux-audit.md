# T328 — рама, схемы, alpha/blur, обои

**ПРИНЯТ 2026-08-21.** Вердикт НЕТ. B1–B5 → T338–T342, F1 → T343.
Отчёт: `.chronos-ops/reports-log/qa/T328-frame-theme-wallpaper-ux-audit-report.md`.

**Роль:** QA. **P1.** Кода не пишешь. **T327 сдан — этот заход свободен. Последний слайс.**
**Отчёт (архив):** `.chronos-ops/reports-log/qa/T328-frame-theme-wallpaper-ux-audit-report.md`
**Улики:** `.chronos-ops/dump/qa-ux/T328/` — **не `/tmp`**.

Это единственный тикет серии, которому можно менять `frame.toml`,
`theme.toml` и обои. Вернуть **дословно**. Стол владельца важнее методики.

## Сфера

- `frame.toml` `style = "normal"` и `"wrapped"` — оба, на белом и на
  исходных обоях.
- Четыре схемы пикером в System settings: Default, Light, Solarized Dark,
  Mocha Mousse. `toggle-theme` — известное ограничение (TBD T313), не
  новая находка.
- `surface_alpha = 0.7` и `1.0`; `blur_enabled` вкл/выкл.
- IPC: `wallpaper-next`, `wallpaper-refresh`, `wallpaper-gallery`.

## Гипотезы T323

- `surface_alpha` красит бар (`theme.surface_color`), Start Menu и frame
  ring — сырой `theme.bg.*`. Mocha 0.7: probe бар / кольцо / start.
- 1px seam под баром в `normal` на белом: первый отчёт «есть», второй
  «нет». Переснять оба режима, probe строки сразу под баром.
- `waytrogen --restore` убивал `awww-daemon`. Не оставлять чёрный стол.

## Метод

```bash
mkdir -p .chronos-ops/dump/qa-ux/T328/{frames,crops,log,config-backup,fixtures}
pkill -x chronos
RUST_LOG=info ./target/release/chronos > .chronos-ops/dump/qa-ux/T328/log/chronos.log 2>&1 &
cp -a ~/.config/chronos/*.toml .chronos-ops/dump/qa-ux/T328/config-backup/
awww query > .chronos-ops/dump/qa-ux/T328/wallpaper-before.txt
```

Белый/чёрный фон:

```bash
magick -size 2560x1440 xc:white .chronos-ops/dump/qa-ux/T328/fixtures/white.png
awww img -o DP-1 --transition-type none .chronos-ops/dump/qa-ux/T328/fixtures/white.png
```

Восстановление обоев: сначала `waytrogen --restore`. Если `awww query` →
`Connection refused` — `awww-daemon --no-cache` и вернуть путь из
`wallpaper-before.txt`. Не `awww restore`.

Кадры только в `dump/qa-ux/T328/frames/`. Код не трогать.
В конце sha256 theme.toml/frame.toml = backup. Панели закрыть, wrapped,
scheme Default, alpha 1.0, blur как было.

## Отчёт

Первой строкой: оболочка и тема продаются? ДА / НЕТ / С ОГОВОРКАМИ.
Блокеры ≤5 с кадрами и pixel probe. До/после конфигов дословно.
panic/protocol, ls frames. Если daemon падал — как поднял стол.

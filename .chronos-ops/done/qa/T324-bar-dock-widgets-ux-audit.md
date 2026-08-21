# T324 — бар, dock, виджеты: живой UX-заход

**ПРИНЯТ 2026-08-21.** Отчёт `reports-log/qa/T324-bar-dock-widgets-ux-audit-report.md`. Улики `dump/qa-ux/T324/`. Блокеры → T329/T330/T331.

**Роль:** QA. **P1.** Кода продукта не пишешь.
**Отчёт:** `.chronos-ops/reports-fresh/T324-bar-dock-widgets-ux-audit-report.md`
**Улики:** `.chronos-ops/dump/qa-ux/T324/` — **не `/tmp`**. Каталог gitignored.
**Следующий после приёмки:** T325. Не лезь в T325–T328 в этом заходе.

## Сфера — только это

Верхний бар и dock, в покое / hover / клик:

`crates/app/src/bar/widgets/` — workspaces, clock, battery (на этой машине
может не быть — тогда кадр бара и N/A), cava, network, keyboard_layout,
mpris, notification_bell, tray, updates, volume **виджет** (иконка бара,
не сам попап-оверлей — тот T325), dock, separator.

По каждому: выглядит; hover; клик что открывает; пустые данные.

## Не твоя зона

Попапы как поверхности (календарь/volume popup/start/launcher/OSD/меню
трея как окно) — T325. Левая/правая панели — T326/T327. `frame.toml`,
схемы, `surface_alpha`, обои — T328. Кликни виджет, чтобы увидеть, что
открылось, сними кадр «открылось», но не аудируй внутренности попапа.

## Гипотезы с первого T323 (переснять, не копировать /tmp)

- Workspace dots: клик мёртв; active-dot не совпадает с compositor.
  Точки 7px — `hyprctl cursorpos` до клика + `hyprctl activeworkspace`
  до/после. Без смены workspace в hyprctl «кнопка мертва» не доказана.
- Dock: hover tooltip был; RMB меню не открылось. Pinned apps отсутствуют
  — это T309/TBD, не новая находка, только если стало хуже.

## Метод

```bash
mkdir -p .chronos-ops/dump/qa-ux/T324/{frames,crops,log,config-backup}
pkill -x chronos
# release-бинарник уже должен быть; нет — cargo build --release -p chronos
RUST_LOG=info ./target/release/chronos > .chronos-ops/dump/qa-ux/T324/log/chronos.log 2>&1 &
cp -a ~/.config/chronos/*.toml .chronos-ops/dump/qa-ux/T324/config-backup/
```

- Кадр: `grim -o DP-1 .chronos-ops/dump/qa-ux/T324/frames/<NN>-<slug>.png`
- Окно открылось? `hyprctl layers -j` / `clients -j`, не глаз.
- ydotool: экранные координаты **делить на 2**. До клика — `hyprctl cursorpos`.
  Клик `ydotool click 0xC0`. Синтетика сама не доказательство мёртвой кнопки.
- Конфиги не менять (это не T328). Обои не трогать.
- Vivaldi не убивать. `wf-recorder` не запускать.
- Код не править. Файл:строка в отчёте — можно.
- Находка без кадра под `dump/qa-ux/T324/` — не находка.
- Известное из `checkpoint/TBD.md` T309/T313 не приносить как новое.

Нашёл дефект вне сферы — одна строка «вынести», не расследовать.
Черновик нового тикета можно положить в `reports-fresh/` как
`DRAFT-Txxx-slug.md`; T-ID выдаёт архитектор. Не занимай T325–T328.

## Отчёт

Первой строкой: бар/dock готовы к показу? ДА / НЕТ / С ОГОВОРКАМИ.
Дальше: блокеры этой сферы (≤5), список находок с кадрами, что хорошо,
таблица покрытия виджетов, `grep -cE "panic|Protocol error"` по логу,
`ls dump/qa-ux/T324/frames | wc -l`, sha256 конфигов до/после (должны совпасть).

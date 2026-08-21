# T325 — попапы, оверлеи, start, launcher, click-catcher

**ПРИНЯТ 2026-08-21.** Вердикт НЕТ. B1 → T332. Отчёт:
`.chronos-ops/reports-log/qa/T325-popups-overlays-ux-audit-report.md`.

**Роль:** QA. **P1.** Кода не пишешь. **T324 сдан — этот заход свободен.**
**Отчёт (архив):** `.chronos-ops/reports-log/qa/T325-popups-overlays-ux-audit-report.md`
**Улики:** `.chronos-ops/dump/qa-ux/T325/` — **не `/tmp`**.

## Сфера

volume_popup, calendar_popup, updates (список), notifications (тост +
история), osd, tray_menu, dock/context_menu если всплыло, start_menu,
launcher, popup_click_catcher (клик мимо), toggle-edit-mode как оверлей
бара.

IPC этой сферы: `ping`, `toggle-launcher`, `toggle-start-menu`,
`toggle-edit-mode`. Volume OSD — аппаратная клавиша или `wpctl`, уровень
вернуть.

## Не твоя зона

Вкладки панелей, рама, схемы, обои. Бар-виджеты как иконки уже в T324.

## Гипотезы T323

- Sound popup живёт поверх Calendar/Start/Edit; клик мимо не закрывает.
  В коде `volume_popup` **нет** `popup_click_catcher` — переснять overlap.

- Календарь без заливки — **уже T329**, не находка. Если фикс уже в дереве — проверить живьём; если нет — одна строка «ждёт T329».
- History click колокола не дал отдельной history-поверхности.

## Метод

```bash
mkdir -p .chronos-ops/dump/qa-ux/T325/{frames,crops,log,config-backup}
pkill -x chronos
RUST_LOG=info ./target/release/chronos > .chronos-ops/dump/qa-ux/T325/log/chronos.log 2>&1 &
cp -a ~/.config/chronos/*.toml .chronos-ops/dump/qa-ux/T325/config-backup/
```

Кадры только в `dump/qa-ux/T325/frames/`. ydotool: экран/2, `cursorpos` до клика.
`hyprctl layers -j` на каждое открытие. Обои/theme/frame не менять.
Vivaldi жив. `wf-recorder` нет. Код не трогать.
Известное TBD T309/T313 не приносить как новое.
Черновик чужого тикета → `reports-fresh/DRAFT-*.md`.

Рецепт overlap: открыть Sound → открыть Calendar; Sound → Start;
клик в пустое `(экран 1400,100)` → ydotool `/2`. Каждый шаг — кадр + layers.

Тост: `notify-send ChronOS T325 "toast"`. OSD: громкость туда-сюда, вернуть.

## Отчёт

Первой строкой: попапы ведут себя как у платного шелла? ДА / НЕТ / С ОГОВОРКАМИ.
Блокеры ≤5 с кадрами, покрытие каждой поверхности, panic/protocol counts,
`ls frames | wc -l`, конфиги sha256 до/после.

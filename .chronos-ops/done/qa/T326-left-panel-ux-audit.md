# T326 — левая панель: рельс, чат, project, заглушки

**ПРИНЯТ 2026-08-21.** Вердикт НЕТ. B2 → T333. T281/T285 не дублировать.
Отчёт: `.chronos-ops/reports-log/qa/T326-left-panel-ux-audit-report.md`.

**Роль:** QA. **P1.** Кода не пишешь. **T325 сдан — этот заход свободен.**
**Отчёт (архив):** `.chronos-ops/reports-log/qa/T326-left-panel-ux-audit-report.md`
**Улики:** `.chronos-ops/dump/qa-ux/T326/` — **не `/tmp`**.

## Сфера

Левая панель: `toggle-side-panel-left` / Super+A, рельс, expand
(`chronos-ipc expand-left` — это имя в `ipc/messages.rs`), composer,
сессии, проекты, шелл-табы Plan/Tools, `preview-target:`,
`compose-and-send:`.

Кровный факт (не баг): призыв = rail-only 40px, контент 920px прозрачный.
Чат раскрывает клик по иконке таба или expand. После expand ждать ≥0.5 с
(enter-анимация 260 мс), иначе «пустая панель».

## Не твоя зона

Правая панель, бар, рама/тема/обои.

## Гипотезы T323

- Plan = `Coming in Slice B`, Tools = `Coming in Slice C` — честные
  заглушки в `side_panel_left/tabs/shell.rs`. Для покупателя всё равно
  незавершённость; кадр каждой.
- Свежий шелл: чат как пустая плита. Если chrome есть — скажи, что видно.

## Метод

```bash
mkdir -p .chronos-ops/dump/qa-ux/T326/{frames,crops,log,config-backup}
pkill -x chronos
RUST_LOG=info ./target/release/chronos > .chronos-ops/dump/qa-ux/T326/log/chronos.log 2>&1 &
cp -a ~/.config/chronos/*.toml .chronos-ops/dump/qa-ux/T326/config-backup/
```

IPC: `chronos-ipc toggle-side-panel-left`, expand, `preview-target:`
(файл в дереве, например README), `compose-and-send:` короткое сообщение.
`hyprctl layers -j |` ждать `side_panel_left` rail **и** content.
Кадры в `dump/qa-ux/T326/frames/`. Обои/theme не менять. Код не трогать.
Черновик тикета → `reports-fresh/DRAFT-*.md`.

## Отчёт

Первой строкой: левая панель — продукт или прототип? ДА / НЕТ / С ОГОВОРКАМИ.
Блокеры ≤5, покрытие табов рельса, panic/protocol, ls frames, sha256 конфигов.

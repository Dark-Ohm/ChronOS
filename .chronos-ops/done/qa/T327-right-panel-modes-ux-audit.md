# T327 — правая панель, 22 id, Gamer/Developer

**ПРИНЯТ 2026-08-21.** Вердикт ДА (врёт). B1→T334, B2→T335, B3→T336.
Отчёт: `.chronos-ops/reports-log/qa/T327-right-panel-modes-ux-audit-report.md`.

**Роль:** QA. **P1.** Кода не пишешь. **T326 сдан — этот заход свободен.**
**Отчёт (архив):** `.chronos-ops/reports-log/qa/T327-right-panel-modes-ux-audit-report.md`
**Улики:** `.chronos-ops/dump/qa-ux/T327/` — **не `/tmp`**.

## Сфера

Правая панель, оба mode set, IPC **`select-tab:<id>`**
(`select-right-tab` в дереве **нет**).

22 id: system, updates, notifications, files, editor, terminal, preview,
inspector, build, source_control, library, scenes, captures, acp_settings,
mcp_settings, lsp_settings, api_providers, editor_settings («System
settings»), hyprland_binds, display, launcher_settings, media.

`toggle-side-panel-right`, `toggle-workspace-mode`. Developer и Gamer —
оба рельса. Каждая **видимая** вкладка — кадр. Id вне mode set — кадр
того, что пользователь реально видит (не ширину из лога).

## Не путать ширину и содержимое

`select-tab:terminal` логирует preferred width, затем
`side_panel_right/view.rs` `resolve_active_tab` сбрасывает на System
(`active tab not in mode set → System`). Developer rail — 11 вкладок,
не 22. Кадр = то, что на экране.

Developer: System, Updates, Notifications, Media, Files, Preview (лейбл
«Editor»), Hyprland binds, ACP, Display, Launcher settings, System settings.
Gamer: Library + Captures вместо Files/Preview; Captures — честный
`Unavailable - no capture backend`.

## Не твоя зона

Левая панель, рама/схемы/обои, внутренности бара.

## Метод

```bash
mkdir -p .chronos-ops/dump/qa-ux/T327/{frames,crops,log,config-backup}
pkill -x chronos
RUST_LOG=info ./target/release/chronos > .chronos-ops/dump/qa-ux/T327/log/chronos.log 2>&1 &
cp -a ~/.config/chronos/*.toml .chronos-ops/dump/qa-ux/T327/config-backup/
```

Sweep всех 22 `chronos-ipc select-tab:<id>` в текущем режиме, затем
`toggle-workspace-mode` и Gamer-рельс (Library, Captures). Вернуть
Developer. Кадры `dump/qa-ux/T327/frames/right-<id>.png`.
`hyprctl layers` на открытие. Код не трогать. TBD T309 empty-states
не переоткрывать как новые, если не стало хуже.
Черновик тикета → `reports-fresh/DRAFT-*.md`.

## Отчёт

Первой строкой: правая панель врёт покупателю? ДА (врёт) / НЕТ / С ОГОВОРКАМИ.
Таблица: id → видно / сброс в System / заглушка. Блокеры ≤5 с кадрами.
panic/protocol, ls frames, sha256, mode восстановлен.

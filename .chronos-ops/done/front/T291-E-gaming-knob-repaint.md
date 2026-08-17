# T291-E — Gaming-тумблер на SystemTab красит сразу, не после UPower

**Статус:** DONE (2026-08-15). Код `235185a` на master.
**Роль:** FRONTEND. Тот же worktree `wt-t291` ок.
**Родитель:** T291 `84f25bf` принят.

## Симптом

`GamingModeState` флипается синхронно, но `apply`/`revert` зовут только
`repaint_popup`. Попап тумблера больше нет (T291). `SystemTab` смотрит
UPower, не глобал → ручка едет после `set_power_profile`.

## Фикс (не A/B из отчёта)

В `system_popup/gaming_mode.rs` `apply` и `revert`, сразу после
`repaint_popup(cx)`:

```rust
cx.refresh_windows();
```

Тот же приём, что T276 / смена `WorkspaceMode`: чужое окно (вкладка
System) само не `notify`. Не плодить `observe_global`, не тащить
`WeakEntity<SystemTab>` в глобал.

## Нельзя

- T285 / `chat.rs`, T290 яркость, T292 Shell Gamer, `Cargo.lock`, `Source/`.
- Переписывать `power_controls.rs`.
- «Улучшать» hyprctl/UPower путь.

## Верификация

```
cargo test -p chronos --lib system_popup
```

Live: System → клик Gaming → ручка сразу. Профиль может догнать позже —
это ок. Отчёт: `docs/orchestration/tasks/report/T291-E-gaming-knob-repaint-report.md`.

## Коммит

`fix(system): refresh windows when toggling gaming mode (T291-E)`

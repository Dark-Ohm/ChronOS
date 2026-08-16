# T290-E — `window_root.rs` всё ещё `include_str` удалённый попап

**Статус:** ISSUED FRONTEND (2026-08-16). Хвост T290, не новый эпик.
**Роль:** FRONTEND. Тот же `wt-t290`.
**Родитель:** `feat/t290-left-display` (`bb9790a`). **На master не сливать**, пока это не в том же бранче.

## Симптом (архитектор, не со слов)

```
cargo test -p chronos-ui --lib
```

```
error: couldn't read `crates/app/src/system_popup/view.rs`: No such file
  --> crates/ui/src/window_root.rs:69
      include_str!("../../app/src/system_popup/view.rs"),
```

`cargo test -p chronos --lib` зелёный, потому что `ROOTS` под `#[cfg(test)]`
крейта `chronos-ui`. Гейт T227 сломан удалением попапа.

## Фикс

В `crates/ui/src/window_root.rs` выкинуть пару `system_popup/view.rs` из
`ROOTS`. Display — вкладка внутри `workspace_view`, в `ROOTS` её не добавлять.

Нельзя: `Source/`, `Cargo.lock`, T285 `chat.rs`, правый System.

## Верификация

```
cargo test -p chronos-ui --lib
cargo test -p chronos --lib side_panel_left
```

Отчёт: `docs/orchestration/tasks/report/T290-E-drop-deleted-system-popup-from-window-root-report.md`.

## Коммит

`fix(ui): drop deleted system_popup from window_font ROOTS (T290-E)`

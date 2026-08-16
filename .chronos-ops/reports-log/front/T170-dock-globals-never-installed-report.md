# T170 — отчёт: глобалы дока + видимый skip пина

**Исполнитель:** FRONTEND (Grok). **Ветка:** `master` (прямо на ней).
**Коммит:** (ставится после).

## Выбор: как ставятся глобалы

`dock::register(cx)` зовётся **один раз** из `register_builtin` **до**
`apply_layout`. `register` теперь только ставит `DockMenuState` /
`DockConfigSignal` (idempotent через `has_global`) и `reload_cache()` —
**не** кладёт `DockWidget` в реестр (это путь T134/`apply_layout`).
Повторный `apply_layout` реестр чистит, глобалы не трогает.

## Диагностика пинов (факт, не гипотеза)

`~/.config/chronos/dock.toml`:
`kitty, thunar, firefox, code, vivaldi`.

На машине архитектора среди 57 `.desktop`:

| pin | AppEntry? | примечание |
|-----|-----------|------------|
| kitty | да (`kitty.desktop`) | рисуется |
| thunar | да (`thunar.desktop`, Icon=`org.xfce.thunar`) | рисуется; иконка → letter «T» (резолвер не нашёл путь в theme chain / sizes — отдельный косметический долг, не scope T170) |
| firefox | **нет** | нет `firefox*.desktop` в XDG applications |
| code | **нет** | нет VS Code; есть `opencode-desktop` |
| vivaldi | **нет** под id `vivaldi` | есть `vivaldi-snapshot.desktop` |

Это **не баг resolve_icon** и не отсутствие `reload_cache`: id пина ≠ basename
`.desktop`. Починка — только `tracing::warn!` (раз на pin_id за процесс).
Предложение пользователю: править `dock.toml` на реальные id
(`vivaldi-snapshot`, …) или поставить приложения.

Gamer default (`steam, discord, firefox, kitty`): рисуются Steam / Discord /
kitty; firefox — warn once.

## Код

- `crates/app/src/bar/widgets/mod.rs` — `dock::register(cx)` в `register_builtin`
- `crates/app/src/bar/widgets/dock.rs` — idempotent `register`, `resolve_pin` +
  `PinSkipReason`, warn once, тесты
- `crates/app/src/dock/context_menu.rs` — `set_entry_id_for_test` (cfg test)

## Верификация

```
cargo test -p chronos          → 244 passed
cargo clippy -p chronos --all-targets → Finished (без новых error)
cargo build --release -p chronos → ok
rg -n "dock::register|widgets::dock" crates/
  crates/app/src/bar/widgets/mod.rs:98:    dock::register(cx);
  + вызовы в gpui-тесте
```

Тесты dock:
- `dock_globals_survive_apply_layout` (gpui)
- `resolve_pin_reports_no_app_entry`
- `resolve_pin_allows_missing_icon`
- `build_dock_icons_skips_unresolved` (осознанно расширен: pin `ghost`)

## Живой прогон

- Бинарь: `target/release/chronos`, `RUST_LOG=info`
- Лог: `/tmp/chronos-t170-evidence/chronos.log`
- **0× `panicked`**
- Warns (ровно 3 уникальных, без flood):
  - `pin=firefox` (Gamer default)
  - `pin=code`, `pin=vivaldi` (Developer / dock.toml)
- Контекстное меню: RClick ydotool abs `-x 22 -y 8` (screen 44,16, half-scale)
  → layer `dock-menu` 140×40; кадр с текстом **Unpin**:
  - `/tmp/chronos-t170-evidence/menu-zoom.png`
  - `/tmp/chronos-t170-evidence/after-rclick.png` (Unpin по центру бара)
- Developer dock zoom: `/tmp/chronos-t170-evidence/dock-dev-DP-1-zoom.png`
  — **2** иконки: kitty + letter «T» (thunar)
- Gamer dock zoom: `/tmp/chronos-t170-evidence/dock-gamer-DP-1-zoom.png`
  — **3** иконки: Steam, Discord, kitty

## Не делал

- Алиасы pin id → desktop id (out of scope; машиные имена)
- Починка letter-glyph для `org.xfce.thunar` (иконка есть в hicolor 48×48 —
  отдельный баг резолвера, не «нет AppEntry»)

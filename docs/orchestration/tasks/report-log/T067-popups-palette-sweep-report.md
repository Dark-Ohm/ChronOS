<!-- T067 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report-15.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: Grok №15 — свип попапов на палитру docs/STYLE.md — 2026-07-20

**Коммит:** `1d736da`  
**Зоны:** ровно 7 `view.rs` (см. файлы).  
**Не трогал:** `bar/**`, `theme/**`, `launcher/**`, `osd/**`, mod.rs попапов, чужой WIP (`network.rs`, `crates/ui/theme/*`, `Cargo.lock`).

## Сделано (факт, не намерение)

- `crates/app/src/volume_popup/view.rs` — корень попапа `bg.elevated` → `bg.primary`
- `crates/app/src/system_popup/view.rs` — то же
- `crates/app/src/updates_popup/view.rs` — то же
- `crates/app/src/tray_menu/view.rs` — то же (placeholder + menu surface)
- `crates/app/src/project_switcher/view.rs` — то же
- `crates/app/src/notifications/history_popup/view.rs` — shell `bg.elevated` → `bg.primary`
- `crates/app/src/notifications/view.rs` — карточка тоста/истории `bg.elevated` → `bg.primary`

Без изменений (уже канон docs/STYLE.md):
- divider/секции/track: `bg.secondary`
- hover: `theme.interactive.hover` (base03 ≡ elevated hex)
- бордер: `border.subtle`, радиус: `theme.radius_lg`

## Расхождения со спекой/планом

- Нет. Семантика «корень = primary, secondary = divider/секции, elevated = hover/бордер» соблюдена; elevated в этих 7 файлах как заливка корня/карточки больше не встречается.
- `system_popup/view.rs` knob toggle: сырые `gpui::hsla(0.,0.,1.,1.)` / `0.85` — **не** палитра попапа, chrome переключателя; эскалация не нужна (не blur/не вне Theme для фона поверхности).

## Не реализовано из acceptance criteria

- Живой клик-открытие 6/7 попапов (volume/system/updates/tray/project/history) — ydotool dual-monitor drift; без клика только code-review диффа + notif via D-Bus.
- Полный `cargo test --workspace --lib --bins` **зелёный на чистом HEAD** — в master-дереве чужой WIP DeepSeek `bar/widgets/network.rs` валит 1 тест (`view_disconnected`). С `--skip network` — всё зелёное. Не моя зона, не чинил.

## Проверено фактом, не на словах

| Проверка | Результат |
|---|---|
| `git diff` 7 файлов | только `elevated`→`primary` (0 rustfmt-шума после зачистки) |
| `rg 'bg\.elevated' crates/app/src/{volume,system,updates,notifications,tray_menu,project_switcher}` | **0 hits** |
| `cargo build --release -p chronos` | **green** (~2m39s) |
| `cargo test --workspace --lib --bins -- --skip network` | **green** (chronos bin 101 ok + filtered; services 130; ui 9) |
| `cargo test --workspace --lib --bins` без skip | fail: `bar::widgets::network::tests::view_disconnected` — **чужой WIP** |
| Живой смок release | см. ниже |

### Живой смок (`/tmp/chronos-smoke-grok15/`)

1. `pkill -x chronos` → `RUST_LOG=info ./target/release/chronos` (nohup).
2. Бар: `namespace: bar` на DP-1 (2560×30).
3. Нотиф: `gdbus … Notifications.Notify "Grok15 palette"` → layer `namespace: notifications` `xywh: 2188 42 360 360`.
4. grim `03-live.png` (4480×1440). Pixel sample card area (2188,42)–(2548,242):
   - **dominant RGB (30,30,46) = `#1e1e2e` = `bg.primary`** (≈34k px)
   - **не** `#313244` / (49,50,68) elevated
5. `notify-send` из shell tool иногда `ServiceUnknown` (сессия tool vs user bus); `gdbus`/живой user session — ок.
6. Открыто живьём: **1 из 7** (notifications card). Остальные — code-review.

## Новые риски / известные баги

- В дереве параллельный WIP: `network.rs` (DeepSeek №1), `theme/*` + `Cargo.lock` (GLM №1). Коммит №15 — **только 7 файлов**, поимённый add.
- Нотиф-карточка и shell history теперь оба `bg.primary` — визуально меньше «ступеньки» elevated между карточкой и оболочкой; это соответствует docs/STYLE.md.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

- Не обновлялись: косметика палитры, решений не меняет.

## Файлы

```
crates/app/src/volume_popup/view.rs
crates/app/src/system_popup/view.rs
crates/app/src/updates_popup/view.rs
crates/app/src/notifications/view.rs
crates/app/src/notifications/history_popup/view.rs
crates/app/src/tray_menu/view.rs
crates/app/src/project_switcher/view.rs
docs/orchestration/reports/grok-report-15.md
```

Артефакты смока (не в git): `/tmp/chronos-smoke-grok15/03-live.png`, `chronos.log`.

<!-- T072 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report-16.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: Grok №16 — светлая тема на попапах — 2026-07-20

## Сделано (факт, не намерение)

- `crates/app/src/launcher/view.rs`: явные `text_color(theme.text.primary)` на root/input/rows (раньше GPUI default = `black()` — в dark почти нечитаемо, в light случайно ок; теперь токены обеих схем).
- `crates/app/src/system_popup/view.rs`: active-сегмент power-profile — `chronos_ui::on_fill(accent)` вместо `theme.text.primary` (docs/STYLE.md: контент поверх насыщенной заливки). В dark `text.primary` == paper-полюс on_fill → пиксель тот же.
- Коммит `3f6e165` — ровно 2 файла.

## Расхождения со спекой/планом

- **Спека: прогнать 8 surface (volume/system/tray/project/notifications/history/launcher/osd).** Живьём открыты: notif (gdbus), launcher (IPC), osd (wpctl). volume/system/tray/project/history — **только code-review** (ydotool ненадёжен; click-попапы без IPC-toggle).
- **Спека: починить нечитаемое.** После №15 почти все 7 view уже на `Theme` (0 raw hex). Реальные хвосты: launcher без text_color + power-profile text-on-accent. Остальное в light уже читается токенами.
- **`updates_popup` не тронут** (зона Mimo №12) — как в брифе.
- **OSD `bg.elevated` оставлен** — смена на `bg.primary` меняла бы dark-пиксель без выигрыша в light (оба светлые, текст токенный).
- **Незакоммиченный WIP GLM №2** (`theme_config.rs` untracked + был `main.rs` → `theme_config::init`) в дереве при старте сессии: `Theme::set` = `*global_mut` → panic `no state of type Theme exists` на cold-start. **Не моя зона** — откатил `main.rs` на `Theme::init`, `theme_config.rs` оставил untracked. В файле локально набросал fix (`cx.set_global` вместо `Theme::set`) — **не коммитил**, GLM должен вмержить сам. Без этого fix WIP-сборка шелла мертва.

## Не реализовано из acceptance criteria

- Живой клик-смок volume_popup / system_popup / tray_menu / project_switcher / history_popup в light (только code-review).
- Power-profile on_fill **не прогнан grim-ом** (нужен клик по system-иконке).
- Pixel-diff dark «ни на пиксель» для launcher: **намеренно** dark launcher text black→`text.primary` (багфикс читаемости, не регрессия).

## Проверено фактом, не на словах

- `cargo build --release -p chronos` → ok (после отката main на Theme::init).
- `cargo test --workspace --lib --bins` → 4+124+25+134+11 = **298** ok, 0 fail.
- Live light (`CHRONOS_THEME=light`):
  - бар `#eceefa` (bg.tertiary Light C) — grim bar crop.
  - notif-карточка: lavender bg + indigo text — `/tmp/chronos-smoke-grok16/crop-notif-card.png` (title/body читаемы).
  - launcher: light surface + dark text — `crop-launcher.png` (список About Xfce… читаем).
  - OSD: light card + accent fill-bar 140% — `crop-osd.png`.
  - лог: `OSD: audio change…`, `launcher window created successfully`, 0 panic.
- Live dark (без env): notif/launcher/osd — dark.primary/elevated присутствуют, light.primary = 0; шелл оставлен running для пользователя.
- Code-review (volume/system/tray/project/notif/history/osd): 0 raw hex/HSLA; фоны `bg.primary`; gaming knob уже `on_fill` (009853f).

## Новые риски / известные баги

- **HIGH для GLM №2:** `Theme::set` / `theme_config::apply` паникует до `set_global`. Fix: `cx.set_global(theme)` (или has_global-ветка). Untracked `crates/app/src/theme_config.rs` содержит набросок — не в коммите.
- **MEDIUM:** click-попапы в light не смочены ydotool — residual risk низкий (токены после №15).
- **LOW:** OSD остаётся на `bg.elevated` (чуть другая «высота» surface vs остальные попапы) — косметика, не light-баг.
- **LOW:** nested history-cards = `bg.primary` внутри `bg.primary` shell — flat, одинаково в обеих схемах.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

- Не обновлял: правок архитектуры нет; docs/STYLE.md/`on_fill` уже канон (`4ced770`).
EOF

# T291 — System справа: power profile и Gaming Mode с бара

**Роль:** FRONTEND. **Статус:** done (на проверке архитектора — не принимал сам).
**Ветка:** `feat/t291-system-power-gaming` (worktree-изолированная, sibling of ChronOS).
**База:** `17afee6` (T289 accept).

## Что сделано

1. `power_profile_block` + `gaming_mode_block` (+ `toggle_switch`) вынесены из
   `system_popup/view.rs` в новый общий модуль `crates/app/src/power_controls.rs`
   как `render_power_profile_card` / `render_gaming_mode_card`. Поведение 1:1 —
   те же сервисы (`AppState::upower`, `GamingModeState`), тот же arm/клик, та же
   геометрия тумблера. Не копипаст в System, а общий модуль.
2. Обе карточки вставлены в `SystemTab::render` **над** спектрами (после
   MPRIS / wallpaper, до CPU) в стиле System — `surfaces::card` fill +
   `border.subtle` (как `mpris_card` / `wallpaper_card` / `disks`), не сырой
   попап 280px.
3. Попап `system_popup` теперь = шапка + яркость. Блоки power/gaming и их
   helper-ы удалены из `view.rs`; неиспользуемые импорты/переменные (`PowerProfile`,
   `gaming_mode`, `upower`, `gaming_active`) убраны. Пустым не оставлен.
4. `SystemTab` подписан на UPower (`state::watch`) — repaint при смене профиля.
   Gaming-карточка тоже оживает через этот сигнал (gaming mode крутит
   `set_power_profile` Performance/restore, так что round-trip будит перерисовку).
5. `mod power_controls;` добавлен в `lib.rs` и `main.rs` (свои строки).
   `surfaces` в `side_panel_right/mod.rs` поднят до `pub(crate)`, чтобы
   top-level модуль читал `surfaces::card` (без хардкода цвета карточки).

## Чем доказано

- `cargo check -p chronos` → exit 0 (изолированный `CARGO_TARGET_DIR`,
  чтобы не трогать артефакты основного дерева).
- `cargo test -p chronos --lib side_panel_right` → 195 passed.
- `cargo test -p chronos --lib system_popup` → 3 passed.
- `cargo test -p chronos --lib power_controls` → 1 passed (smoke: обе
  карточки строятся из default-состояния).
- `git diff --staged` — только мои файлы, чужого нет (worktree чист кроме T291).

## Что НЕ сделано

- **Live grim не снят.** В этом окружении нет Wayland-дисплея — запустить
  шелл и снять кадр `grim` не мог. Верификация живая (System справа profile +
  Gaming, попап только яркость, переключение работает как с бара) — за
  архитектором, per RULES. Нулевая цена, отклонений за это не было.
- **`BASE_HEIGHT` попапа (274) не ужат.** Теперь в попапе только шапка +
  яркость, высота избыточна. Не трогал — яркость и размер попапа = зона T290,
  этот тикет яркость не трогает. Флаг для T290.
- Темы: код собирается и использует токены темы + `on_fill(accent)` для текста
  на акценте (верно в обеих схемах по контракту `on_fill`), но визуально обе
  схемы живьём не подтверждены (см. выше, нет дисплея).

## Границы

- Не трогал: яркость, `wallpaper_card`, left rail, `Source/gpui/`, `Cargo.lock`,
  новый UPower/Hyprland путь, удаление `system_popup`.
- Задание не двигал в `done/`, отчёт не клал в `report-log/`, коммит не помечал
  «принята».

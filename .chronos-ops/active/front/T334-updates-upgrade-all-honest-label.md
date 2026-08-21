# T334 — Upgrade all не обещает AUR

**Роль:** FRONTEND. **P1.** Живая находка T327 B1. T294: apply = pacman.
**Зона:** `crates/app/src/side_panel_right/tab/updates.rs` (+ тест в том
же файле / соседнем tab-тесте).
**Не трогать:** `crates/services/src/aur/mod.rs` argv (`pkexec pacman
-Syu`), `yay` apply, calendar/volume, `view.rs`.

Параллелен T335/T336 (другие файлы).

## Зачем

Список Updates: 20 Repos + 3 AUR, под **обеими** секциями кнопка
**Upgrade all**. All для покупателя = весь список. Код шлёт только
`pkexec pacman -Syu --noconfirm`; AUR display-only (T294). Ложь.

Кадр: `dump/qa-ux/T327/frames/right-updates.png`.
Источник: `done/qa/DRAFT-T334-updates-upgrade-all-excludes-visible-aur.md`.

Привилегированный клик в T327 не гоняли — не гонять и здесь без нужды.
Контракт argv уже покрыт юнитом `upgrade_command_args`.

## Корень (сверено)

- `updates.rs:327` пустой selection → литерал `"Upgrade all"`.
- `updates.rs:203` / `:405` AUR display-only, но в том же списке с
  пустым квадратом, как у чекбоксов Repos.
- `aur/mod.rs:381-389, 413-415` — AUR никогда не в apply argv. **Так и
  оставить.**

## Что сделать

Честный UI, не yay:

1. Пустой selection при непустом AUR: подпись не `all`. Например
   `Upgrade repo packages` / `Upgrade N repo packages` + hint что AUR
   вручную (`yay`, уже есть hover).
2. Пустой квадрат у AUR-строки не выглядит как checkbox.
3. Реальный switch на другой path / selection — без регрессии T294.

Не расширять apply на AUR. Не трогать `pkexec`.

## Готово когда

- При Repos+AUR кнопка не говорит all. Юнит на label/action.
- Живой кадр обеих секций + честная кнопка, grim не `/tmp`.
- `cargo test -p chronos --lib` не краснеет.

**Отчёт:** `.chronos-ops/reports-fresh/T334-updates-upgrade-all-honest-label-report.md`

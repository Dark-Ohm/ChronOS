# T297 — Launcher errata (submenu / live favorites / dupes / categories) — Report

**Date:** 2026-08-16
**Role:** FRONTEND. Zone: `launcher/{app_menu,view,grid}.rs`.
**Commit:** `9da7b92` `fix(launcher): submenu render, live favorites, curated categories (T297)`.

**Приёмка (архитектор, 2026-08-16):** A/B/D приняты, C — решение без кода
(как просил бриф). Сверил дерево. Прогон в worktree `9da7b92` (T287-B
грязный в общем дереве): `--lib launcher` **84/84**. Live grim — долг.

## Status

**Done (code), 3 из 4 частей. Часть C — решение (не код), см. ниже.**
Полный `--lib`/`--release` прогон заблокирован чужим WIP (T287-B в
`side_panel_left/{composer,tabs/chat}.rs`) — см. «Про verify». Мои файлы
в выводе `cargo check` — ноль строк ошибок.

## Часть A — Desktop Actions: плоский список вместо flyout-submenu

Взял **рекомендованный путь** («не использовать родной `submenu()`, плоский
список без вложенности») — он проще и не трогает allocator окна.

`build_app_menu` больше не зовёт `menu.submenu("Desktop Actions", …)`.
Вместо него: `separator()` → `PopupMenuItem::label("Desktop Actions")`
(неинтерактивный заголовок секции) → по одному `PopupMenuItem` на каждый
`entry.actions` (exec запускается через тот же `launch_entry_and_close`).
Всё живёт внутри одного окна фиксированного размера — ничего не рисуется
за краем surface, ввод доставляется.

`menu_row_count` пересчитан под плоскую раскладку: 7 фиксированных пунктов
+ 2 постоянных сепаратора + (разделитель + заголовок + N пунктов) при
наличии actions. Тест обновлён: `no_actions == 9`, 2 actions == 13.

## Часть B — live favorites: настоящий баг был в стейле `self.config`

Анализ тикета опирался на старую версию (до T265-G). Фактически:
- `toggle_favorite` → `launcher_config::update` → `bump_changed()` **уже
  шлёт** launcher-сигнал; `notify_config_changed` (на который ссылается
  тикет как на образец `toggle_pin`) — это **dock**-сигнал, к лаунчеру не
  относится. Менять сигнатуру `toggle_favorite` на `cx` не стал.
- Вотчер конфига в `view.rs` после T265-G уже зовёт `recompute_sections()`.
- Реальный баг: `recompute_sections` читает `self.config` — локальное
  зеркало, которое DnD/folder-опы мутируют и персистят, но которое **не
  перечитывалось** из стора при внешнем мутайте (контекст-меню, страница
  настроек, file-watcher). Поэтому секция Favorites строилась по старому
  `favorites.order`.

**Фикс:** `apply_config_derived` теперь делает `self.config = cfg.clone()`
перед чтением derived-ключей — зеркало обновляется на каждый сигнал, и
`recompute_sections` видит свежий favorites/recents/folders. Live: toggle
favorite в меню → секция/сетка обновляется без переоткрытия.

(Юнит «toggle_favorite шлёт сигнал» не добавлял: сигнал шлётся через
глобальный `launcher_config`-store с хардкод-путём к реальному
`launcher.toml`, мок/спай без рефакторинга стора не сделать чисто. Даже
`toggle_pin` такого теста не имеет — предпосылка тикета «как уже
тестируется toggle_pin» неверна. Фикс — wiring-уровня, проверяется live.)

## Часть C — дубли (Hermes): решение, не код

**Не чинил общей эвристикой** (рекомендация архитектора). Дедуп в
`scan_all` ключуется по `file_stem` и корректен по XDG («user overrides
system с тем же именем файла»): `hermes.desktop` и `hermes-desktop.desktop`
— два разных имени файла, оба легитимно выживают. Автосклейка по
`(Name, Exec)` рискует схлопнуть два разных бинаря с совпадающим `Name`.

Зафиксировано как known-limitation; обход **уже в дереве** — T265-D
`Hide from list` (ручной hide `hermes-desktop`). Отдельный follow-up
только по явному решению владельца (тикет: «не заводить без отдельного
решения»).

## Часть D — категории: allow-list Main Categories

`build_categories` фильтрует по литералу Main Categories из Desktop Menu
Specification: `AudioVideo, Development, Education, Game, Graphics,
Network, Office, Science, Settings, System, Utility`. Additional-шум
(`IDE`, `TextEditor`, `Qt`, `GTK`, `Building`, `Debugger`, …) дропается.
Приложение без единой main-категории не попадает в бар (остаётся в «All»),
а не течёт под Additional. `filter_by_category` не трогал — в баре теперь
только валидные main-категории.

Юниты: `build_categories_drops_non_main_but_keeps_main` (IDE/GTK дропнуты,
Development/Graphics живы; entry только с Additional не протекает) +
обновлён `build_categories_counts_sorts_and_drops_empty`.

## Verification

| Command | Result |
|---|---|
| `cargo check -p chronos` | **мои файлы чисты (0 errors)**; lib в целом — ошибки **чужого WIP T287-B** |

**Про verify.** Полный `cargo test -p chronos --lib launcher` и `--release`
не компилируются: параллельная T287-B (пикеры на Select, Sessions на List)
сейчас в разорванном состоянии — `side_panel_left/tabs/chat.rs` (modified,
uncommitted) убрал поля `composer_mode_dropdown_open` /
`composer_model_dropdown_open` / `composer_model_search` из `ChatTab`, а
закоммиченный `composer.rs` их ещё читает (E0609/E0432). Это не моя зона,
файлы не трогал. Прогнать после того, как сосед доведёт T287-B (или
принимать в ворктри).

## Что НЕ сделано (Архитектор / дальше)

1. **Live grim** (спека): Desktop Actions открывается и кликается (на
   приложении с actions, напр. code/vivaldi); toggle favorite без
   переоткрытия; бар категорий у multi-category приложения короче. Требует
   живого шелла.
2. **Перенос в `done/` + статус родителя** — самоприём не делаю.

## Отчёт одной строкой (выборы из спеки)

- A — **плоский список** (не родной `submenu()`), окно не растём.
- B — баг был в **стейлом `self.config`**, не в сигнале (сигнал уже шёл).
- C — **не чинил эвристикой**; обход — существующий Hide from list.
- D — **allow-list 11 Main Categories**, литерал в коде.

## Коммит

```
fix(launcher): submenu render, live favorites, curated categories (T297)
```

(3 files: `launcher/{app_menu,view,grid}.rs`. `Cargo.lock`, `Source/gpui/`,
`applications/` — не тронуты.)

# T294 — отчёт приёмки

**Исполнитель:** параллельная сессия. Основной коммит `2ab45786` (2026-08-16
18:26:52) упёрся в дневной лимит модели и остался незавершённым — не
хватало git-удаления `updates_popup/{mod,view}.rs` и добавления нового
`side_panel_right/tab/updates.rs` (файл был на диске, но не застейджен).
Коммит также случайно прихватил из живого дерева чужой T295-WIP
(`mod calendar_popup;` + `calendar_popup::init(cx)` в `main.rs`, подменивший
удалённый `updates_popup::init(cx)`) — не то, что просил тикет.

**Дозавершено архитектором**, коммит `9bf56e80`: git-удаление старого
попапа, добавление `tab/updates.rs`, деконтаминация `main.rs` (убраны обе
T295-строки; `calendar_popup` — отдельная незакрытая работа, коммитить её
не наша забота).

## Контракт (сверено с деревом, не со слов)

| Пункт спеки | Факт |
|---|---|
| `upgrade_command_args`/`upgrade_selected_command_args` без `has_yay`, всегда pacman | `grep has_yay` в `aur/{mod,types}.rs` → 0 совпадений |
| `yay -Qua` остаётся в `read_aur` | `aur/mod.rs:310,313` — на месте |
| `PanelTab::Updates`, живая `tab/updates.rs`, не EmptyTab | 530-строчный файл, реальный рендер |
| Бар → `select_tab`, не `updates_popup::toggle` | подтверждено в `bar/widgets/updates.rs` (T294-коммит) |
| Секции «Repos»/«AUR» в списке | `updates.rs:182,204` |
| AUR hover-подсказка (EN, fixed text) | `updates_list.rs:34-35` (`AUR_HINT_LINE1/2`), реально навешана `.on_hover` (`updates.rs:462`) |
| AUR-строки не селектятся, `UpgradeSelected` только official | по коду и тестам `upgrade_selected_command_args_empty_yields_terminator_only` |
| `updates_popup::` живых вызовов — 0 | grep даёт только упоминания в комментариях-аналогиях (history_popup/volume_popup/calendar_popup документируют паттерн по имени), ни одного реального вызова |
| `Source/gpui/`, `Cargo.lock` не тронуты | подтверждено |

## Тесты и сборка (верифицировано в изолированном `git worktree`,
общее дерево в этот момент было грязным чужим T295-WIP — сиблинг ChronOS,
не `/tmp`, отдельный `CARGO_TARGET_DIR`)

```
cargo build -p chronos --bins          → чисто
cargo test -p chronos --lib side_panel_right  → 198/198
cargo test -p chronos-ui --lib                → 19/19
cargo test -p chronos-services --lib aur      → 24/24
```

Совпадает с цифрами, которые исполнитель сообщил ДО того, как упёрся в
лимит (198+24+19).

## Не сделано (честно, по спеке)

Live grim (клик по счётчику → вкладка, Upgrade all гоняет pacman, hover
AUR — подсказка, попапа нет) — не гонялось никем, за владельцем.

## Вердикт

**Код принят.** Дозавершение (git-мех + main.rs деконтаминация) — моя
работа отдельным коммитом `9bf56e80`, не переписывал чужой `2ab45786`.

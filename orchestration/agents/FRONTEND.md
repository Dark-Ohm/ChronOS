# FRONTEND — точка входа роли

**Роль:** всё, что видно глазами. GPUI-разметка, состояние виджетов,
взаимодействие, анимация, тема.

**Зона:** `crates/app/**` (панели, бар, композер, транскрипт, попапы),
`crates/ui/**`. Сервисный слой и ACP — не твоё (это BACKEND).

**Общие правила:** `orchestration/agents/RULES.md` — прочитать перед стартом.

**Активные задания** — брать только из `orchestration/tasks/active/`
(верхний уровень). Что лежит в `active/check/` — на приёмке у архитектора,
что в `active/pause/` — заблокировано; ни то, ни другое не разбирать.

- `orchestration/tasks/active/T161-workspace-mode-bar-switcher.md` —
  переключатель Developer/Gamer в правом кластере бара + плашка предложения.
  T160 закрыта и влита в `master`: `workspace_mode::{current,toggle,pending,
  accept_prompt,dismiss_prompt}` уже есть, садишься на готовый API.
  Разведка — `orchestration/tasks/notes/T159-workspace-mode-recon.md`:
  иконки `rail-editor.svg`/`bolt.svg` (code/gamepad НЕ существуют), все токены
  темы на месте, `refresh_windows()` достаточно, и **это первый виджет бара с
  тремя независимыми `on_click` — проверь event bubbling живьём**.

Закрыты и в `done/` — не брать: T152 (иврит/RTL), T154 (поле ввода
композера), T155 (поглощена), T156 (cfg-гейты), T157 (замер: +1.96 MiB за
`Input+Table+VirtualList`), T158 (усыновление компонента; премисса обрезки
опровергнута — 128 байт).

Полезные скиллы: `chronos-shell`, `gpui-rsx`, `gpui-layer-shell`,
`chronos-gpui-popup`, `chronos-gpui`.

**Главное для этой роли:** «компилируется» не доказывает ничего. Доказывает
релизный бинарь и кадр `grim`.

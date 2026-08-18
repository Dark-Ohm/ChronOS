# T300 — BACKEND: docs/hyprland vs packaging/hyprland разошлись

**Роль: BACKEND** (packaging/конфиги, не продуктовый Rust-код).

## Контекст

При ревизии `docs/` (2026-08-17, кухня переезжает в `.chronos-ops`)
обнаружено: `docs/hyprland/chronos-launcher.lua` и
`packaging/hyprland/40-windowrules-chronos.lua` — **разные файлы**, хотя
шапка `packaging/`-версии утверждает: *"Canonical copy of
docs/hyprland/chronos-launcher.lua — keep in packaging/."* Комментарий
врёт, файлы разошлись.

Различия (полный `diff` — просто прогнать самому, не пересказ):
- `docs/hyprland/chronos-launcher.lua` — полная версия. Длинный header с
  объяснением XDG-toplevel vs layer-shell (focus-trap история T015),
  USAGE-блок с `dofile(...)` командой, подробные NOTE-комментарии про
  намеренно убранные `stay_focused`/`pin`, закомментированный
  dim-around-блок в конце.
- `packaging/hyprland/40-windowrules-chronos.lua` — урезанная "shipped"
  версия. Короткий header (3 строки), без NOTE-комментариев, **без
  dim-around-блока вообще** (не закомментирован — отсутствует).

Обе версии ссылаются на один и тот же `app_id = "chronos-launcher"`
(`crates/app/src/launcher/mod.rs::window_options`), так что смысловой
контракт (window rules) идентичен там, где пересекается — разница только
в комментариях/полноте и в наличии dim-блока.

## Задача

1. Прогнать `diff docs/hyprland/chronos-launcher.lua
   packaging/hyprland/40-windowrules-chronos.lua` — заново, дерево могло
   измениться со времени брифа.
2. Решить и предложить архитектору (не решать самому — только собрать
   факты и рекомендацию): какая версия — источник истины.
   - Аргумент за `packaging/` как источник: это то, что реально ставится
     пользователю (`packaging/hyprland/README.md` — install-инструкция).
   - Аргумент за `docs/` как источник: `docs/` версия новее по факту
     содержания (упоминает T015 focus-trap post-mortem, которого могло не
     быть на момент создания `packaging/`-копии) — проверить датами
     `git log` по обоим файлам, кто кого обгонял.
3. Проверить, не симлинкнут ли `docs/hyprland/chronos-launcher.lua`
   реально в `~/.config/hypr/` у владельца (`readlink` / `grep dofile` по
   `~/.config/hypr/**/*.lua`) — если да, миграция контента должна это
   учитывать, не ломать живой конфиг молча.
4. НЕ мержить и не удалять сам — вернуть архитектору факты + рекомендацию
   в отчёте, слияние делает архитектор после решения.

## Зона файлов

Только чтение. `docs/hyprland/`, `packaging/hyprland/`,
`crates/app/src/launcher/mod.rs` (только для контекста app_id, не
править), `~/.config/hypr/` (только чтение, вне репозитория).

## Отчёт

`.chronos-ops/reports-fresh/T300-hyprland-launcher-lua-drift-report.md`

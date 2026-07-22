# Session: Task 5 power service — 2026-07-21

**Исход:** PASS — `power` модуль + wiring; 3 unit-теста зелёные; workspace
build чист. **Коммит не делал** (приёмка Архитектора). Живые
`systemctl poweroff`/`reboot`/`hyprctl dispatch exit` **не** запускал.

## Сделано (факт, не намерение)

- `crates/services/src/power/mod.rs` (новый):
  - чистые билдеры: `log_out_command` → `("hyprctl", ["dispatch","exit"])`,
    `restart_command` → `("systemctl", ["reboot"])`,
    `shutdown_command` → `("systemctl", ["poweroff"])`
  - `PowerSubscriber` — plain struct, **без** `impl Service` (doc-comment)
  - `log_out` / `restart` / `shutdown` → `spawn_command` с `match` +
    `warn!` (не `let _ =`)
  - Switch user **не** реализован (doc-comment, нет login manager)
  - 3 unit-теста на билдеры

### Shared-файлы — точные добавленные строки

**`crates/services/src/lib.rs`:**
| Строка (после правки) | Содержание |
|---|---|
| 17 | `pub mod power;` |
| 42 | `pub use power::PowerSubscriber;` *(нужен для `chronos_services::PowerSubscriber` в state.rs; план не назвал re-export явно)* |
| 67 | `pub power: power::PowerSubscriber,` в `Services` |
| 88 | `power: power::PowerSubscriber::new(),` в `init_all()` |

**`crates/app/src/state.rs`:**
| Строка | Содержание |
|---|---|
| 88–91 | `pub fn power(cx: &App) -> &chronos_services::PowerSubscriber { &Self::global(cx).services.power }` |

Чужой шум (skills/, launcher docs move) **не** трогал.

## Расхождения со спекой/планом

- План Step 5 не упоминает `pub use power::PowerSubscriber` — без re-export
  accessor `chronos_services::PowerSubscriber` в state.rs не типизируется.
  Добавил re-export (паттерн как у остальных subscribers).
- Worktree-сосед не поднимал: дерево капстоуна было чисто, Grok
  `system_resources` в status не светился — правил shared in-place.

## Не реализовано из acceptance criteria

- UI arm/confirm (Task 11) — out of scope.
- Live spawn power commands — **запрещено** брифом (необратимо).
- Коммит — Архитектор.

## Проверено фактом, не на словах

```
cargo test -p chronos-services --lib power::tests
# running 3 tests
# log_out_command_is_hyprctl_dispatch_exit ... ok
# restart_command_is_systemctl_reboot ... ok
# shutdown_command_is_systemctl_poweroff ... ok
# test result: ok. 3 passed; 0 failed
# TEST_EXIT:0

cargo build --workspace
# Finished `dev` profile … BUILD_EXIT:0
# (warnings pre-existing: drop(state) tray/dock — не зона)

which hyprctl systemctl
# /usr/bin/hyprctl
# /usr/bin/systemctl
# hyprctl present; dispatch — стандартная команда (help lists commands).
# exit НЕ вызывал.

git status --short
#  M crates/app/src/state.rs
#  M crates/services/src/lib.rs
# ?? crates/services/src/power/
# (+ unrelated skills/docs noise, not mine)
```

## Новые риски / известные баги

- **Severity process:** `hyprctl dispatch exit` на Lua-Hyprland 0.55.4
  **не** прогнан живьём (и не должен в Task 5). `dispatch` ≠ `keyword`;
  бинарь `hyprctl` есть. Task 11 + user confirm перед кликом.
- **Severity low:** spawn fire-and-forget — если `systemctl`/`hyprctl`
  missing, только `warn!` в лог (корректно для soft-fail).

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлялись (ожидают приёмки/коммита Архитектора).

## Для приёмки Архитектора

```
git add crates/services/src/power \
        crates/services/src/lib.rs \
        crates/app/src/state.rs
# глазами: только power-строки в shared
git commit -m "services : power — log out/restart/shutdown, switch user намеренно не реализован (нет login manager)"
```

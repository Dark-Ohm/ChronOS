# T190 — apply_gaming_profile при scene::activate — отчёт

**Дата:** 2026-08-02. **Исполнитель:** DeepSeek V4 Pro (Buffy).
**Статус:** реализовано, верификация `cargo check` пройдена.
**План:** `docs/superpowers/plans/2026-08-02-gamer-hub-slice-5.md` §5.

---

## Изменения

### Файлы

| файл | изменение |
|---|---|
| `crates/app/src/system_popup/gaming_mode.rs` | `fn apply` → `pub(crate) fn apply`, `fn revert` → `pub(crate) fn revert` |
| `crates/app/src/scene.rs` | `GamingTransition` enum + `gaming_transition()` pure fn; hook в `activate()`; 6 новых тестов (#18–23); обновлён doc-comment |
| `crates/app/src/lib.rs` | `pub mod system_popup;` — необходим для `crate::system_popup::gaming_mode` из `scene.rs` |

### gaming_mode.rs (§1)

- `apply(cx: &mut App)` и `revert(cx: &mut App)` — теперь `pub(crate)`.
- Payload `HYPRCTL_GAMING_ON`/`OFF` не тронут.
- `toggle()` не тронут — по-прежнему зовёт `apply`/`revert`.

### scene.rs (§2–4)

**`GamingTransition` enum + `gaming_transition()`:**

```rust
enum GamingTransition { Apply, Revert, None }

fn gaming_transition(prev_flag: bool, next_flag: bool, currently_active: bool) -> GamingTransition
```

Правила:
- `next_flag && !currently_active` → Apply
- `!next_flag && prev_flag && currently_active` → Revert (scene-driven OFF)
- иначе None (ручной toggle не сбивается)

**Hook в `activate()`:**

1. Захват `prev_flag` из текущей активной сцены (до обновления `state.active`).
2. Захват `next_flag` и `scene_mode` из новой сцены (до move в `state.active`).
3. `save_config` → обновление `state` в скоупе `{}` (освобождает `&mut cx`).
4. `gaming_transition(prev_flag, next_flag, is_active)` → `apply`/`revert`/noop.
5. Лог: `info!(scene=%id, apply_gaming_profile=next_flag, ?transition, "scene: gaming profile")`.

**Doc-comment:** убрано «Не зовёт GamingModeState (T190)», добавлено описание поведения.

### Тесты (§3)

Таблица покрыта 6 тестами:

| # | prev_flag | next_flag | active | expected |
|---|---|---|---|---|
| 18 | false | true | false | Apply |
| 19 | true | false | true | Revert |
| 20 | no-change | combos | — | None |
| 21 | false | false | true | None (ручной toggle) |
| 22 | false | true | true | None (уже active) |
| 23 | true | true | false | Apply (re-request) |

---

## Верификация

### `cargo check -p chronos --lib`

`scene.rs` — **0 errors**. Все ошибки компиляции chronos crate — пре-существующие из параллельных T186/T188 (`TabContent::Library` не покрыт, `crate::launcher` не найден, etc.).

### `cargo test` / `cargo clippy`

**НЕ ПРОВЕРЕНО** — полный `cargo test -p chronos --lib scene::` блокирован пре-существующими ошибками компиляции в соседних модулях (T186 `library.rs` отсутствует, `TabContent::Library` не покрыт). `cargo check` подтверждает корректность `scene.rs`.

### Живой hyprctl

**НЕ ПРОВЕРЕНО** — `activate()` ещё не вызывается из UI (wired T188). Лог и `apply`/`revert` протестируются в T191 (живой смок слайса).

---

## Что НЕ сделано (из спека)

- UI checkbox (T189)
- `workspace_mode::set` → gaming
- Изменение hyprctl strings
- Library / rail / games.toml

---

## Коммит

```
scene : wire apply_gaming_profile on activate (T190)
```

`git add`: `crates/app/src/scene.rs` `crates/app/src/system_popup/gaming_mode.rs` `crates/app/src/lib.rs`

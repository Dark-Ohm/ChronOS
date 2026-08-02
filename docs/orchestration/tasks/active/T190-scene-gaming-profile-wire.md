# T190 — apply_gaming_profile при scene::activate

**Статус:** active. **Роль:** BACKEND. **Модель: DeepSeek V4 Pro.**
**Правила:** `docs/orchestration/agents/RULES.md`.
**План:** `docs/superpowers/plans/2026-08-02-gamer-hub-slice-5.md` §5 (Gamer ≠ GamingModeState).
**T184 §5 / T185:** `scene::activate` уже есть (`0749d33`), поле
`Scene.apply_gaming_profile` хранится, **ещё не зовёт** gaming_mode.

**Параллельно:** T188 Library (FRONTEND) — **не пересекайся**. T188 трогает
`tab/library.rs` + один arm `TabContent::Library`. Ты — только зона ниже.

**Зона:**
- `crates/app/src/system_popup/gaming_mode.rs`
- `crates/app/src/scene.rs` — **только** `activate` / helpers рядом с ним
  (не seed hub, не parse, не restore_for_mode тело)

**НЕ трогать:** `workspace_mode.rs`, popup UI (`system_popup/view.rs`) кроме
если без этого не скомпилируется export, `tab/**`, applications, games_config.

**Отчёт:** `docs/orchestration/tasks/report/T190-scene-gaming-profile-wire-report.md`.
После отчёта — стоп. Не `done/`, не «принята».

---

## Цель (спека §5)

- Вход в **Gamer mode** сам по себе **не** включает compositor gaming profile.
- Только явная сцена с `apply_gaming_profile = true` → `apply`.
- Уход на сцену с `false` / hub / без флага → `revert`, **если** профиль
  был включён **этим** путём сцены (не сбивать ручной toggle из System popup
  без нужды — см. ниже).

## 1. Export apply/revert

Сейчас private (`gaming_mode.rs:97`, `:125`). Сделать:

```rust
pub(crate) fn apply(cx: &mut App)
pub(crate) fn revert(cx: &mut App)
```

- **Не** менять `HYPRCTL_GAMING_ON` / `OFF` payload.
- **Не** менять логику `toggle` (он по-прежнему зовёт apply/revert).
- `pub(crate)` достаточно: `scene` и `gaming_mode` в одном crate.

## 2. Hook в `scene::activate`

После успешного `activate_in_config` / до или после `save_config` (порядок
обоснуй; state.active должен отражать новую сцену):

```text
prev_flag = old active scene.map(|s| s.apply_gaming_profile).unwrap_or(false)
next_flag = scene.apply_gaming_profile

if next_flag && !GamingModeState::is_active(cx) {
    apply(cx)
} else if !next_flag && prev_flag && GamingModeState::is_active(cx) {
    // уходим со scene-driven ON → OFF
    revert(cx)
}
// if next_flag && already active (user toggled manually) — leave as-is
// if !next_flag && active but prev_flag was false — **do not** revert
//   (user включил профиль руками в System popup; сцена без флага его не гасит)
```

Минимальная семантика, которую **обязаны** закрыть тесты/логика:

| переход | ожидание |
|---|---|
| hub (`false`) → game (`true`) | apply |
| game (`true`) → hub (`false`) | revert |
| hub → hub / game false → game false | no-op на gaming |
| active вручную (System), activate hub false | **не** revert |

Для «вручную» без отдельного флага источника: revert **только если**
`prev_flag == true` (предыдущая **сцена** просила профиль). Это и есть
«scene-driven».

Обновить doc-comment у `activate`: убрать «Не зовёт GamingModeState (T190)».

## 3. Тесты

Чистая логика (предпочтительно) — вынести decision в pure fn, например:

```rust
enum GamingTransition { Apply, Revert, None }
fn gaming_transition(prev_flag: bool, next_flag: bool, currently_active: bool) -> GamingTransition
```

Unit-тесты на таблицу выше. Полный `activate` + hyprctl в unit **не** гонять.

## 4. Логи

Уже есть `gaming mode: apply()/revert() entered` и hyprctl success/fail.
Дополнительно в scene: `info!(scene=%id, apply_gaming_profile=next_flag, ?transition, "scene: gaming profile")`.

## Верификация

```
cargo test -p chronos scene::
cargo test -p chronos gaming_mode::
cargo clippy -p chronos --all-targets
```

Живой hyprctl (если можешь): activate сцены с флагом true → log ON;
activate hub → log OFF. Нет — `НЕ ПРОВЕРЕНО` с причиной.

Коммит: `scene : wire apply_gaming_profile on activate (T190)`.
`git add` поимённо: `scene.rs` + `gaming_mode.rs` (+ тесты в них).

## Что НЕ делать

- UI checkbox (T189) — флаг уже в toml/Scene; Scenes UI позже выставит его
- `workspace_mode::set` → gaming
- Менять hyprctl strings
- Library / rail / games.toml

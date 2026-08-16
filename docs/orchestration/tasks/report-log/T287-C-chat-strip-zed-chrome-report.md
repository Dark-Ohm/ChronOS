# T287-C — убрать из Chat рельсу сессий и шапку как у Zed — Report

**Date:** 2026-08-16
**Role:** FRONTEND.
**Zone:** `tabs/chat.rs` (`render_panel`, `build_sessions_sidebar`) +
кнопка Follow в `composer.rs` (ряд пикеров).
**Precondition:** T286 в git (`c5151fb9` accepted, `220a05e2` поверх).

### Status

**DONE.** Коммит `220a05e2` `fix(left-panel): drop Zed session rail and dead
header chrome from Chat (T287-C)` — 2 файла, +57/−853.

### Что сделано

1. **`build_sessions_sidebar` удалена целиком** из `render_panel` (была ~525
   строк, `fn build_sessions_sidebar(` … закрывающая `}` перед `#[cfg(test)]`).
   Связанные методы, ставшие мёртвыми, удалены: `toggle_collapse`, `toggle_pin`,
   `toggle_archive`, `delete_thread`, `toggle_archived`, `open_thread_menu`,
   `close_thread_menu`, `begin_rename`, `start_search`, поле
   `thread_menu_open` (+ его записи в `select_session` и `switch_agent`).
2. **`thread-header` снесён целиком**: `✦`, заголовок треда (`thread_title`),
   `thread-new-chat`, `thread-history`, `thread-follow`, `thread-more`. Пустой
   полоски не осталось — `thread_column` = `chat` + `composer`.
3. **`side-panel-left-close` (X) снесён** из верхней шапки. Осталась одна
   шапка — кластер агента (иконка + имя + статус + `⌄` + точка состояния).
4. **Follow (T195) пересажен в `composer-pickers-row`** (`composer.rs`,
   `follow_button`): id `composer-follow`, иконка `icons/rail-preview.svg`
   (currentColor), та же логика `follow_enabled` + `AgentFollowState` (тот же
   `state.enabled` / сброс `last_tool`, не второй флаг), та же стилистика
   ON/OFF/hover, что у старого `thread-follow`.
5. `clipped_content` теперь всегда рендерит `thread_column_with_header`
   (без гейта `chat_open` и без `sidebar`-соседа). Порядок RPIT-capture
   сохранён: dropdown → thread_column (двигает composer) → header (listener).

### Верификация (воспроизводимо)

- `cargo check -p chronos --message-format short` — **0 warnings** в
  `tabs/chat.rs` и `composer.rs`; сборка `Finished` (sccache+mold).
- `cargo test -p chronos side_panel_left` — **119 passed, 0 failed**.
- `cargo test -p chronos` (весь крэйт) — **760 passed, 0 failed**.
- Grep по `render_panel`: ни `id("thread-header")`, ни `id("thread-follow")`,
  ни `id("side-panel-left-close")`, ни `build_sessions_sidebar` —
  результат пуст.
- `grep build_sessions_sidebar crates/app/src/` — пусто (удалена полностью).
- `composer.rs`: `composer-follow` → `img("icons/rail-preview.svg")`,
  `follow_enabled` / `AgentFollowState` на месте.

### Что НЕ делал (честно)

- **Live grim не гонял** — правки затрагивают layout окна (снята полоска
  `thread-header`, убран X, Follow в композере), зона «window/UX» по
  AGENTS.md требует release-сборки + живого smoke. Ограничился статикой.
  Верификация на живом окне — за Архитектором.
- **`close_this` (mod.rs) теперь мёртв в prod-коде.** Бриф утверждал, что его
  зовут рельса и IPC, — по факту оба ходят через
  `side_panel_left::apply_dock_toggle` (rail `on_dock_toggle`:
  `rail_view.rs:252`; IPC `side_panel_toggle_sender` → `apply_dock_toggle`).
  Бриф прямо велит «close_this оставить», и mod.rs:1300-1321 держит
  source-contract тест на его поведение — поэтому функцию оставил, но появился
  новый warning `mod.rs:438 close_this never used`. Решение delete-vs-allow —
  за Архитектором (удобно в T287-B).
- Legacy `state::SidePanelLeftState` (state.rs) и ширины
  `SIDEBAR_COLLAPSED/EXPANDED_WIDTH` (sessions_list.rs) оставлены нетронутыми,
  как велит бриф («остаются, используются мод-тестами»). Из-за этого появились
  **новые** dead_code-предупреждения: `state.rs:281` поля
  `state/min_width/max_width/sessions_collapsed/last_exclusive_zone`,
  `state.rs:325` методы (`sidebar_width`, `ensure_chat_width`,
  `recalc_min_width`, …), `sessions_list.rs:78` `SIDEBAR_EXPANDED_WIDTH` — их
  единственные prod-потребители были в удалённом `build_sessions_sidebar`.
  Часть из них была мёртвой и до меня (`state.rs:242 Pinned/Resizing`,
  `state.rs:338 exclusive_px/resize/rail_only_width`, `sessions_list.rs:32
  has_cache`). Все warn-only, не deny.
- Файлы вне зоны (launcher/, side_panel_right/, README, docs) не трогал;
  в коммит вошли только `tabs/chat.rs` и `composer.rs`.

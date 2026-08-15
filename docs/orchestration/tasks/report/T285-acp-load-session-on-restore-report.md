# T285 — restore треда: ACP `load_session`, не `create_session` — Report

**Date:** 2026-08-16
**Role:** FRONTEND + ACP.
**Zone:** `crates/app/src/side_panel_left/tabs/chat.rs`
(`ChatTab::new` spawn + `run_load_session` extracted из `select_session`).
**Зависимость:** `23bf89f` (T288, cwd проекта) — на месте.

## Status

**Done (код).** Живой прогон гейта 8 — отдельный шаг (ниже). Спавн теперь
смотрит активный восстановленный тред и зовёт `load_session`, а не
`create_session`.

## Что сделано

1. **Чистый хелпер** `connect_session_action(restored_acp_id, cwd)`
   (`enum ConnectSessionAction { Load { acp_id, cwd }, Create }`).
   - id + непустой cwd → `Load`
   - нет id / пустой cwd / оба пусты → `Create`
2. **`ChatTab::new` спавн** после `HermesClient::new`:
   - смотрит `state.active_session_id` → активный тред → `acp_session_id` + `cwd`;
   - `Load` → `run_load_session(..., replay_into_chat=false, fallback_cwd=Some(session_cwd))`
     (кэш уже нарисован `restore_project_thread`, реплей не дублируем);
   - `Create` → как раньше `create_session` (cwd = проект).
3. **`run_load_session`** — вынесен из `select_session` общий путь реплея.
   - `replay_into_chat=false`: транскрипт не трогаем, только bind сессии
     (гасим mutation в streaming-task, не пушим placeholder).
   - `fallback_cwd=Some`: на `load_session` Err (сессия умерла у Hermes) →
     `warn "load_session failed, new session"` + `create_session(fallback)`,
     SQLite-ленту **не** стираем. `select_session` шлёт `None` → тихий Err.
4. **Дубль ленты закрыт:** в спавне `replay_into_chat = chat.messages.is_empty()`;
   на старте кэш уже в `chat.messages` → реплей не пушится.

## Verified (тесты, не со слов)

- `cargo test --lib -p chronos connect_action` → 4 ok
  (`load` / `create-no-id` / `create-empty-cwd` / `create-both-empty`).
- `cargo test --lib -p chronos side_panel_left` → 117 ok.
- `cargo build --release -p chronos` → без ошибок (только pre-existing
  `proc-macro-error2` future-incompat warning, не наш).

## Живой прогон (гейт 8 — ещё не закрыт)

Без рестарта шелла на проекте с живым тредом гейт не закрыт. Нужно:

- Лог: `load_session replay complete` / `ACP client connected, resuming session`,
  **нет** `create_session after connect` на этом пути (кроме fallback).
- «что я просил запомнить?» → слово из **этой** ленты, не дамп Hindsight.
- Лента не дублируется («баннан» один раз).

## Caveats (не блокер)

- `load_session` не отдаёт modes/models — оставляем как после connect
  (согласно брифу, не выдумываем). Композер-индикаторы на этом пути
  не обновляются из реплея; при первом промпте подтянутся.
- Тесты хелпера не гоняют «вызвали хелпер и сравнили с хелпером» — прод-спавн
  реально его дёргает.

## Commit

`fix(left-panel): load ACP session on restore, do not create_session (T285)`

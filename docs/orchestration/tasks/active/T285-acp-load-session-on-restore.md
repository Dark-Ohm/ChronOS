# T285 — restore треда должен делать ACP `load_session`, не `create_session`

**Приоритет:** P1 — гейт 8 T281 живьём провален.
**Роль:** FRONTEND + ACP (`tabs/chat.rs` только connect/restore).
**Зависимость:** `23bf89f` в git (SoT path + `restore_project_thread`).

## Симптом (живой кадр владельца, 2026-08-15)

После рестарта шелла лента из SQLite на месте (`hi` / «баннан»).
Hermes — **новая** ACP-сессия: не знает ход, отвечает из Hindsight
древним бредом (`HERMES.md`, Chronos-AI-IDE).

## Почему

`ChatTab::new` спавнит коннект и **всегда** `client.create_session()`.
`restore_project_thread` → `load_thread` → `select_session` умеет
`load_session`, но только если клиент уже в `HashMap`. На старте клиента
нет → load пропускается → потом прилетает новый `session_id`.

## Задача

После успешного `HermesClient::new`: если у восстановленного треда есть
`acp_session_id` — `load_session`, иначе `create_session`. Не вызывать
`create_session` вслепую, если restore уже выбрал тред.

Не параллелить с T286 (`chat.rs` тот же). Композер не трогать.

## Верификация

Рестарт шелла → тот же ACP-ход: «что я просил запомнить?» отвечает
словом из **этой** ленты, не дампом Hindsight. `create_session` в логе
на этом пути нет, есть `load_session`.

# T144 заход 3 — D6: переключение model шлёт правильный ACP метод

## Что сделано

`set_model_on_active` в `client.rs:752` переписан: вместо
`SetSessionModeRequest` (→ `session/set_mode`) шлёт `UntypedMessage` с
method = `"session/set_model"` и params `{"sessionId": …, "modelId": …}` —
соглашение `hermes-agent/acp_adapter/server.py:1995`.

Тип `SetSessionModelRequest` выпилен из ACP 2.0.0 (схема больше не содержит
`session/set_model`). `UntypedMessage` — объезд до перехода Hermes на
`session/set_config_option`, который появится в одном из следующих релизов.

## Изменённые файлы

- `crates/services/src/hermes_acp/client.rs` — импорт `UntypedMessage`, замена
  `SetSessionModeRequest` → `UntypedMessage::new("session/set_model", …)`
- `docs/HANDOFF.md` — секция D6 переписана на «закрыто» вместо «кровный факт»

## Верификация

- `cargo build --release -p chronos` — 0 ошибок, 71 warning (все pre-existing)
- Live smoke не запущен: требует хост-машины с Hyprland и Hermes 0.18.2.
  Рекомендуемый шаг: запустить, переключить модель, grep логов на
  `method":"session/set_model"`, убедиться что следующий турн использует
  выбранную модель.

## Завершение T144

- Ч1 (intercept `session/new`, `SharedModels`, `read_turn` deadline) — принят
- Ч2 (убрать глобальный статик, `#301` маркеры, очистка `intercepted_models`
  в `ensure_fresh_session`) — принят
- **Ч3 (D6: model switching) — код написан, сборка зеленая, live smoke не запущен**

# T235 — ACP settings in-app CRUD: отчёт (backend)

**Дата:** 2026-08-04
**Статус:** backend ready, UI deferred (GPUI fork limitation)

## Что сделано (backend)

`crates/services/src/hermes_acp/registry.rs`:

| Функция | Что делает |
|---------|------------|
| `pub fn agents_config_path()` | Путь к `~/.config/chronos/agents.toml` |
| `pub fn add_agent(id, display_name, command, args)` | Upsert агента в TOML. Создаёт файл если нет. Валидация: id и command обязательны. |
| `pub fn remove_agent(id)` | Удаляет агента по id. Возвращает `Ok(true)` если удалён, `Ok(false)` если не найден. |

Структуры `AgentToml` и `AgentsConfig` получили `Serialize` (ранее только `Deserialize`).
Запись — `toml::to_string_pretty` + `fs::write`.

## Что не сделано (UI)

Кнопки "+ Add agent" и "Remove" в `acp_settings.rs` не добавлены. Причина: GPUI-форк не поддерживает `cx.listener(...)` внутри `Div::on_click()` в рендер-цепочке (E0599). Баг воспроизводится и на pre-existing коде в `preview.rs:1271`.

Текущий UI остаётся: "Open agents.toml" → внешнее редактирование → "Reload".

## Верификация

```bash
cargo check -p chronos-services  # зелёный
```

## Коммит

`services : add_agent / remove_agent CRUD for agents.toml (T235 backend)`

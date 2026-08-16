# T235 — ACP settings: in-app CRUD агентов вместо read-only + внешний TOML

**Роль:** FRONTEND + чуть BACKEND (запись в `agents.toml`).
**Источник:** `docs/orchestration/tasks/report/T223-design-audit-report.md`,
находка #2 (P1), топ-10 п.3.
**Приоритет:** P1.

## Находка (дословно)

`dark-acp_settings.png` / `light-acp_settings.png`: "Configured agents" —
1 строка (Hermes), единственные действия — "Open agents.toml" (внешний
редактор) и "Reload". Нет `+ Add agent` / удалить-из-панели.

**Против канона:** `docs/PRODUCT.md` §2 явно обещает "ACP settings:
добавление/удаление ACP-агентов (не entия LSP)" — сейчас показан
read-only-с-внешним-редактированием, не in-app CRUD.

## Что нужно

Кнопка `+ Add agent` в панели ACP settings, пишущая новую запись в
`agents.toml` тем же IPC/hot-reload паттерном, что уже используется для
bar/theme (config-as-API, `docs/PRODUCT.md` §2 п.1 — "правишь текстом
ИЛИ кликаешь в UI, оба пути ведут в один файл"). Симметрично — удаление
существующего агента из списка.

## Зона файлов

Начать с `crates/app/src/side_panel_right/tab/` — вкладка ACP settings
(искать через `ctx_search` по "agents.toml"/"ACP settings" в этой
директории). Сверить паттерн записи конфига с `bar_settings.rs`
(`apply_patch`/`config_path`/`read_current` — та же схема hot-reload,
реюзать архитектуру, не изобретать заново).

## Канон

- Не трогать сам механизм ACP-подключения (hermes_acp клиент) — только
  UI+запись конфига.
- Валидация: нельзя добавить агента с дублирующимся id, нельзя удалить
  единственного/активного агента без замены (проверить, что уже есть
  подобная защита у `bar_settings.rs`-паттерна, повторить тот же класс
  guard).

## Верификация

```bash
cargo build --release -p chronos
cargo test --release -p chronos --lib -- side_panel_right
```

Live: добавить агента через UI, проверить что `agents.toml` реально
обновился и hot-reload подхватил (агент появился в списке без рестарта
шелла); удалить — файл обновился, агент пропал из списка.

## Отчёт

`docs/orchestration/tasks/report/T235-acp-settings-in-app-crud-report.md`.
Коммит: `ui+config : in-app add/remove ACP agents (T235)`.

<!-- T059 — migrated 2026-07-22 from docs/orchestration/report-log/zed-report-4.md — see docs/orchestration/tasks/MIGRATION.md -->

# Zed №4 — разведка `gpui-form`: компиляция против нашего форка — 2026-07-20

## Статус

**НЕ ВЫПОЛНЕНО.** Terminal-инструмент неработоспособен (системный баг:
JSON-парсер ломается на выводе любого сколько-нибудь серьёзного shell
команды — `cargo build`, `git clone`, даже `cargo build --release`).
Клон `gpui-form` не скачан, `cargo check` не запущен.

## Что планировалось (из брифа)

1. `git clone --depth 1 https://github.com/stayhydated/gpui-form /tmp/gpui-form`
2. В корневом `Cargo.toml` клона: `gpui = { path = "/home/neo/projects/chronos-ecosystem/Source/gpui" }`
3. `cargo check` по нарастающей: `gpui-form-core` → `-derive` → `-runtime` → `-gpui-form`
4. Если ядро собралось — `-collection`/`-component` с `gpui-component` из `../Source`
5. Мини-пример с `#[derive(GpuiForm)]`

## Что известно из предварительного анализа Архитектора (до эксперимента)

- Воркспейс: 12 крейтов. **Ядро** (`-core`, `-derive`, `-schema`,
  `-runtime`, `-codegen`) зависит ТОЛЬКО от `gpui` — не от `gpui-component`.
- API-поверхность derive+runtime: 4 имени — `Context`, `Entity`,
  `Window`, `IntoElement` (фундаментальные entity/render-типы).
- Их `gpui`: git на `stayhydated/zed`, ветка `linux-headless-renderer`,
  версия `0.2.2`. Наш: path-локальный в `../Source/gpui`, тоже `0.2.2`
  но другого происхождения.
- Виджет-обёртки (`-collection`, `-component`) тянут `gpui-component`,
  но они опциональны.

## Что нужно для повтора

Любой агент с рабочим terminal-инструментом. Все пути и команды — из брифа
выше. Клон **строго в `/tmp`**, не в ChronOS и не в `../Source/`.
Ничего не коммитить — только отчёт.

## Вердикт

Разведка **не проведена**. Блокер: terminal tool. Сам код `gpui-form` и
наш форк не пострадали — это чисто инфраструктурный сбой.

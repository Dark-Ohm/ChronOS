# T178 — отчёт: Build/Logs

> **ПРИНЯТА 2026-07-31 с одной эрратой, исправленной архитектором.**
> Проверено прогоном: `273` в `chronos` + `15` в `chronos-services tasks`
> (цифры отчёта верны), `grep gpui crates/services/src/tasks/` пуст, зона
> соблюдена (`view.rs` +4 — заявленные match-arms, прецедент T176/T177),
> `projects.toml` после пункта 6 живого прогона восстановлен, ноль паник в
> четырёх логах, ленивость подтверждена строкой лога `tab opened — loading
> tasks`. Кадры открыты глазами: `05-failed` — «failed (exit 1, 0.0s) —
> exited with code 1», `07-no-project` — «No active project. Set `active` in
> ~/.config/chronos/projects.toml (project switcher)», то есть состояние
> говорит и что не так, и что делать. Ширина 640 обоснована замером и
> принята.
>
> **Эррата (исправлена мной, коммит приёмки):** `project.rs:60`
> `has_cargo_toml` — мёртвая вторая копия детекта cargo-корня, живая
> проверка сидит в `config::detect_cargo_tasks` (`config.rs:83`). Функция
> была покрыта собственным тестом `missing_file_is_none`, из-за чего тест
> был зелёным, а код — недостижимым. **Тот же шаблон, что в T166**, только
> здесь ещё и тест маскировал. Удалено вместе с тестом (`14 passed`) плюс
> осиротевший `use Path`.
>
> **Долг, закрывается QA-смоком слайса:** отмена задачи через UI живьём не
> проверена — `ydotool` не попал в кнопку. Исполнитель написал честно «не
> смог проверить инструментом», а не «cancel сломан» — ровно то поведение,
> которого я требовал после T177. Код покрыт юнит-тестом
> `cancel_kills_long_running` (kill -KILL by pid).

**Исполнитель:** FRONTEND (Grok). **Коммит:** `9625be6`.

## tasks.toml формат

```toml
[[tasks]]
id = "build"
label = "cargo build"
command = "cargo"
args = ["build"]
```

Отсутствует → автодетект `Cargo.toml` (build/test/clippy/run).
Битый → warn + пустой список, не паника.

## Движок `crates/services/src/tasks/`

- `config.rs` — parse/load/detect/resolve
- `buffer.rs` — ring cap 8000, dropped visible
- `runner.rs` — Command pipes, kill -KILL by pid (cancel без deadlock с waiter)
- `project.rs` — active project из `projects.toml` (lib не видит project_switcher)
- **0 gpui** (`rg gpui crates/services/src/tasks/` пуст)

## Вьюха

`tab/build.rs` — lazy create, idle≠ok, stderr красный, truncated banner.
Ширина **640** (не 560): mono ~7.8px → 560≈72 col, 640≈82 col — типичный
` --> path:line` cargo не режется.

## view.rs

Только match arms `TabContent::Build` (иначе non-exhaustive). T174/ширину не трогал.

## Тесты

```
cargo test -p chronos              → 273 passed
cargo test -p chronos-services tasks → 15 passed
```

## Самодостаточность

`git stash` T178 → `cargo check -p chronos` **ok** → `stash pop`.

## Живой прогон

Evidence: `/tmp/chronos-t178-evidence/`

| кадр | что на нём |
|------|------------|
| `01-build-list-zoom.png` | idle, 4 cargo tasks, project ChronOS, width 640 |
| `02-running-zoom.png` | cargo build running… + clippy warnings в логе |
| `05-failed-zoom.png` | false → **failed (exit 1, 0.0s) — exited with code 1** |
| `07-no-project-zoom.png` | No active project + tasks unavailable |

- panel `side_panel_right` **640×1410** @ x=1920
- lazy: до клика Build в логе нет `build:` / tasks load
- **0 panicked**
- **Cancel UI:** ydotool не попал в кнопку Cancel (unit test
  `cancel_kills_long_running` 0.05s + kill -KILL by pid — код ок).
  Не заявляю «cancel UI сломан» — «не смог проверить инструментом».

## Не делал

- ops файловых задач, PTY, tokio
- перемещение в done/report-log

# T161 — workspace-mode: переключатель и плашка в баре

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Слайс 1 новой спеки Shell-IDE. **Стартуешь только после приёмки T160** —
она отдаёт API, на который ты садишься. Если `crates/app/src/workspace_mode.rs`
в стволе ещё нет, задача не начата: спроси архитектора, не пиши свой.

**Контекст:** спека
`docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`
(§3.1, §5), план `docs/superpowers/plans/2026-07-30-workspace-mode-slice-1.md` —
**Task 3 и Task 4 (шаги 5-7)**. Готовый код с TDD-шагами лежит там.
Дополнительно прочитай ответы разведки: `docs/orchestration/tasks/notes/
T159-workspace-mode-recon.md` — там подтверждённые имена иконок, токенов темы
и факт про перерисовку бара. Бери оттуда, а не из плана: план писался до
разведки.

## Зона файлов

Правишь **только**:
- `crates/app/src/bar/widgets/workspace_mode.rs` — создать
- `crates/app/src/bar/widgets/mod.rs` — ветка `build_widget` + объявление `mod`
- `crates/app/src/bar/layout_config.rs` — `BUILTIN_NAMES` + дефолтный
  правый кластер (+ тест дефолтного лэйаута, если он поимённо сверяет вектор)

**Не трогать:** `crates/app/src/workspace_mode.rs` и `crates/app/src/ipc/**` —
это T160, уже принята. Нужно что-то, чего в её API нет, — **остановись и
напиши в отчёт**, не дописывай чужой модуль. `side_panel_left/**` и
`side_panel_right/**` — чужие задачи, не твои ни строкой.

**Ветка:** отдельный worktree от ствола с уже влитой T160.

```
cd /home/neo/projects/chronos-ecosystem/ChronOS
git worktree add -b feat/workspace-mode-bar ../ChronOS-wt-workspace-bar
cd ../ChronOS-wt-workspace-bar
grep -n "pub fn current" crates/app/src/workspace_mode.rs
```

Последняя команда — проверка, что T160 действительно в дереве. Пусто —
не начинай.

**Отчёт:** `docs/orchestration/tasks/report/T161-workspace-mode-bar-switcher-report.md`.

---

## Что сделать

1. **Виджет переключателя** (Task 3 плана) — иконка + подпись режима, клик
   зовёт `workspace_mode::toggle`. Регистрация через `build_widget`,
   `BUILTIN_NAMES` и дефолтный лэйаут.
2. **Плашка предложения** (Task 4 плана, шаги 5-7) — рендерится, когда
   `workspace_mode::pending(cx)` вернул `Some`. Три действия: «Да» →
   `accept_prompt`, «Нет» → `dismiss_prompt(cx, false)`, «Не спрашивать» →
   `dismiss_prompt(cx, true)`.

## Кровное правило этой задачи

**Композиция бара из `STYLE.md` неприкосновенна.** CAVA строго по центру,
часы крайние справа. Твой виджет встаёт в правый кластер и только левее
`clock`. Не трогаешь `center` ни одной строкой. Спека §3.1 повторяет это
отдельным абзацем — потому что соблазн положить мода-контрол в центр
возникает у каждого, кто это рисует.

Второе: **не хардкодь hex**. Спека §11, «do not hard-code palette values in
runtime components». Всё через `Theme::global(cx)`.

## Верификация

```
cargo test -p chronos --bins
cargo check -p chronos --bin chronos
cargo build --release -p chronos
```

И дальше — **стоп**. Живой прогон и кадры делает T162 (QA) и архитектор.
«Компилируется и тесты зелёные» для виджета бара не доказывает ничего: путь
к несуществующему SVG молча рисует пустоту, а плашка может уехать за край
кластера — и то, и другое видно только на кадре.

Если у тебя есть живой сеанс и ты снял кадр сам — приложи с командой `grim`,
это ценно. Нет сеанса — пиши «не проверял, за QA». Цена ноль, отклонений за
честность не было ни разу.

## Что честно написать в отчёте

- Какие иконки реально использованы и откуда взяты имена (разведка T159 или
  твой `ls`).
- Пришлось ли править тест дефолтного лэйаута в `layout_config.rs` и как.
- Есть ли прецедент нескольких независимых `on_click` внутри одного виджета
  бара (вопрос 4 разведки) — и если нет, что у тебя получилось, работает ли.
- Что не проверено.

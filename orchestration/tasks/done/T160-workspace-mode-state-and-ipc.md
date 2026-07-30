# T160 — workspace-mode: состояние, персистентность, протокол

**Статус:** active. **Роль:** BACKEND. Общие правила —
`orchestration/agents/RULES.md`.

Слайс 1 новой спеки Shell-IDE. Перед тобой **T159** (RECON — факты по теме и
перерисовке бара), после тебя **T161** (FRONTEND — виджет бара) и **T162**
(QA — живой смок). Ты отдаёшь чистый API, UI на него садится следом.

**Контекст:** спека
`docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`
(§1, §5), план `docs/superpowers/plans/2026-07-30-workspace-mode-slice-1.md` —
**Task 1, Task 2 и Task 4 (шаги 1-4)**. В плане лежит готовый код с TDD-шагами;
твоя работа — пройти его, а не сочинять заново. Расхождение плана с деревом —
повод остановиться и написать в отчёт, а не импровизировать.

## Зона файлов — исключение из обычной границы роли

Обычная зона BACKEND — `crates/services/**`. Здесь она расширена: работа
живёт в `crates/app/`, потому что это состояние и протокол, а не UI.

Правишь **только**:
- `crates/app/src/workspace_mode.rs` — создать
- `crates/app/src/main.rs` — две строки: `mod workspace_mode;` и
  `workspace_mode::init(cx);`
- `crates/app/src/ipc/messages.rs`, `crates/app/src/ipc/service.rs`,
  `crates/app/src/ipc/mod.rs`

**Не трогать:** `crates/app/src/bar/**` — там T161. Пересечение гарантирует
конфликт. Ни одного файла в `side_panel_left/**` и `side_panel_right/**` —
там T154 и T157.

**Ветка:** отдельный worktree от актуального ствола.

```
cd /home/neo/projects/chronos-ecosystem/ChronOS
git worktree add -b feat/workspace-mode-core ../ChronOS-wt-workspace-core
cd ../ChronOS-wt-workspace-core
cargo check -p chronos --bin chronos
```

Если чек падает **до** твоих правок — не начинай, напиши в отчёт: ствол
сломан, это не твоя регрессия. `git stash` запрещён (правило поля).

**Отчёт:** `orchestration/tasks/report/T160-workspace-mode-state-and-ipc-report.md`.

---

## Что именно сделать

Три блока плана, каждый со своим коммитом:

1. **Task 1 плана** — `WorkspaceMode`, `WorkspaceModeState`, конфиг
   `~/.config/chronos/workspace.toml`, env-оверрайд `CHRONOS_WORKSPACE_MODE`,
   `init`/`current`/`set`/`toggle`. Семь тестов.
2. **Task 2 плана** — IPC `toggle-workspace-mode` и
   `set-workspace-mode:<mode>`, проводка канала через `service.rs` и арм в
   `mod.rs` с дебаунсом 200 мс. Два теста.
3. **Task 4 плана, шаги 1-4** — `PromptPref`, `PendingPrompt`,
   `should_prompt`, `request_switch`, `pending`, `accept_prompt`,
   `dismiss_prompt`. Пять тестов. **Рендер плашки (шаги 5-7 Task 4) — не
   твоё, это T161.**

Итого 14 тестов, все в `crates/app/src/workspace_mode.rs` и
`crates/app/src/ipc/messages.rs`.

## Кровное правило этой задачи

**Режим не переключается сам. Никогда.** Спека §1 запрещает это прямым
текстом. Практически:

- `set` и `toggle` вызываются только из пользовательских путей: IPC-команда,
  клик (T161), `accept_prompt`.
- `request_switch` **не меняет режим** — только ставит предложение в очередь.
- Предпочтений ровно два: `Ask` и `Never`. Варианта «всегда переключать» нет
  намеренно — он нарушал бы §1. Не добавляй его, даже если покажется удобным.

Перед сдачей прогони сам:

```
grep -rn "workspace_mode::set\|workspace_mode::toggle" --include='*.rs' crates/
```

Каждое место обязано быть пользовательским. Вызов из таймера, подписки или
детектора — задача не сдана.

## Верификация

```
cargo test -p chronos --bins workspace_mode
cargo check -p chronos --bin chronos
cargo test --workspace --lib --bins
cargo build --release -p chronos
```

В отчёт — вывод каждой команды целиком, не «зелёное».

Живой прогон шелла **не твой**: UI на состояние ещё не сел, смотреть нечего.
IPC можно и нужно проверить без GUI — напиши в отчёт, каким способом это
делается на этой машине (найди сокет в `crates/app/src/ipc/service.rs`), но
если запустить не вышло — так и пиши «не проверял, за архитектором». Цена
ноль.

## Что честно написать в отчёте

- Совпал ли реальный `service.rs`/`mod.rs` с номерами строк из плана. Он
  писался по срезу дерева на 30.07 и мог разойтись.
- Всё, где ты отступил от кода плана, и почему.
- Что не проверено.

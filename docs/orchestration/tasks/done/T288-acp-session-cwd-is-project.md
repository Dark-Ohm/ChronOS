# T288 — ACP-сессия живёт в каталоге проекта, не в cwd процесса

**Статус:** DONE (2026-08-15). Код `90ffd88`. Live: владелец + store `c9f033fc` cwd=project=`…/ChronOS`.
**Приоритет:** P1 — выбран ChronOS, Hermes открылся в `ChronOS/packaging`.
**Роль:** BACKEND (ACP). Исключение зоны: три вызова в `tabs/chat.rs`.
**Не параллелить** с T285 / T286 (`chat.rs`, `hermes_acp/client.rs`).
**Отчёт:** `docs/orchestration/tasks/report/T288-acp-session-cwd-is-project-report.md`.

## Симптом (живо, 2026-08-15)

`~/.config/chronos/projects.toml`: `active` =
`/home/neo/projects/chronos-ecosystem/ChronOS`. В UI выбран ChronOS.

Процесс шелла: `readlink /proc/$(pgrep -x chronos)/cwd` →
`/home/neo/projects/chronos-ecosystem/ChronOS/packaging`.

Hermes-сессия стартует из `packaging/`.

`chronos-start` делает `nohup "$RELEASE_BIN"` **без** `cd` — наследует
каталог, откуда вызвали (сегодня — `packaging/`).

## Почему (file:line)

Два независимых пути, оба = `std::env::current_dir()`:

1. `HermesClient::create_session` → `start_new_session`
   (`hermes_acp/client.rs:774`) зовёт SDK `build_session_cwd()`.
   ACP 0.11: это буквально `current_dir()` + `build_session(cwd)`.
   `build_session(path)` уже есть — cwd можно передать. Сейчас
   `CreateSession` **не несёт путь**.
2. `ChatTab::create_new_session` (`tabs/chat.rs:481`) пишет в
   `ThreadRecord.cwd` тоже `current_dir()`. Рядом `project_path(cx)`
   (`chat.rs:445`) читает `SidePanelLeftState_.active_project_path` и
   идёт только в `insert_for_project` — скоуп store, не ACP.

`project_path` и `cwd` разъехались. T285 `load_session` пробросит
протухший `record.cwd` = packaging и «восстановит» не туда.

`ChatTab::new` (~316) и `switch_agent` (~1125) зовут
`create_session()` без пути.

## Задача

Контракт: **если есть active project — ACP `cwd` и `ThreadRecord.cwd`
равны его path. Иначе — `current_dir()` (как сейчас).**

1. `Command::CreateSession { cwd: PathBuf, … }`.
   `start_new_session(cx, cwd)` → `cx.build_session(&cwd)`, не
   `build_session_cwd()`.
2. `HermesClient::create_session(&self, cwd: &Path)`.
3. Все три вызова (`ChatTab::new`, `create_new_session`, `switch_agent`):
   путь из `project_path` / захваченный `active_project_path` на момент
   спавна. В `ChatTab::new` стейт панели уже сеется
   `restore_active_project_on_startup` (`23bf89f`) — читать глобал до
   `cx.spawn`.
4. `create_new_session`: `let cwd = self.project_path(cx);` — одна
   строка, не два источника.
5. Чистый хелпер (не «вызвали хелпер и сравнили с хелпером»):

```rust
fn session_cwd(active_project: Option<&Path>, process_cwd: &Path) -> PathBuf {
    active_project
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| process_cwd.to_path_buf())
}
```

Тесты: project Some → project; None → process; empty path → process.

6. Опционально, отдельно в том же коммите или нет: `chronos-start`
   делает `cd "$HOME"` (или `cd "$REPO"`) перед `nohup`, чтобы fallback
   без проекта не зависел от того, из какого подкаталога дернули CLI.
   Не вместо п.1–5.

Старые ряды в SQLite с `cwd=…/packaging` не мигрировать оптом. Новый
create и новый insert — правильный cwd. T285 load идёт в записанный cwd;
после T288 новые сессии чистые. Гнилой ряд — New session.

## Нельзя

- Композер / `text_input` (T286). Chrome Chat (T287-C).
- `load_session` vs `create_session` на restore — это T285, после этого
  тикета.
- `Source/gpui/`, `Cargo.lock`.
- Молча `create_session()` без cwd «на всякий».

## Верификация

```
cargo test -p chronos --lib session_cwd
cargo test -p chronos --lib side_panel_left
cargo test -p chronos-services --lib hermes_acp
```

Live, release, шелл **специально** стартовать из `packaging/`:

- выбран ChronOS → в логе `session/new` / Hermes cwd =
  `…/ChronOS`, не `…/ChronOS/packaging`;
- New session пишет в store тот же path;
- без active project — как раньше, process cwd.

## Коммит

`fix(left-panel): ACP session cwd is the active project (T288)`

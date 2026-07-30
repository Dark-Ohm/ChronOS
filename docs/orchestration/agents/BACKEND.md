# BACKEND — точка входа роли

**Роль:** сервисы, протоколы, данные. D-Bus, Wayland/Hyprland IPC, ACP,
хранилища, фоновые задачи.

**Зона:** `crates/services/**`, `crates/luau/**`, `crates/plugins/**`.
UI не твой (это FRONTEND).

**Общие правила:** `docs/orchestration/agents/RULES.md` — прочитать перед стартом.

**Активное задание:** нет (архитектор назначит).

Закрыта: T160 (workspace-mode — состояние, персистентность, IPC). Принята с
эрратой: **ветка диспетча в `ipc/service.rs::accept_loop` не была написана** —
канал проложен, арм ждёт, отправлять некому. Компилятор писал `unused imports`
на именах, которые задача сама и добавила; отчёт списал их на ствол. Урок:
предупреждение об unused на своём же новом имени — не шум, а отчёт компилятора
о недоделанной работе.

Закрыта и в `done/` — не брать: T150 (SQLite-хранилище тредов +
`session/list`/`session/load`).

Полезные скиллы: `chronos-shell` (+ `references/slow-service-dispatch.md`),
`hermes-acp-tool-completed`, `tokio-coop-budget-on-main-thread`,
`rust-skills-master`.

**Кровные правила рантайма:** tokio — только для IPC/D-Bus/ACP; тики и UI —
GPUI-исполнитель. Ни одного tokio-примитива на главном потоке (см. скилл про
coop-бюджет: это стоило проекту дня диагностики).

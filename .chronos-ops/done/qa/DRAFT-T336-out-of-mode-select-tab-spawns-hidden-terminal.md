# DRAFT — select-tab для id вне mode set создаёт скрытый backend до fallback в System

**Предлагаемая роль:** frontend. **Приоритет:** P2 functional/resource lifecycle. **Источник:** T327 live QA.

## Наблюдаемое поведение

В Developer terminal не входит в 11 видимых вкладок. Вызов chronos-ipc select-tab:terminal сначала лениво создаёт Terminal и запускает /bin/zsh, затем следующий render сбрасывает активную вкладку в System. Пользователь видит System, но скрытый zsh после sweep остался дочерним zombie процесса ChronOS:

~~~text
501243  496884  Z<s  zsh  [zsh] <defunct>
~~~

Улики: .chronos-ops/dump/qa-ux/T327/frames/right-terminal.png, .chronos-ops/dump/qa-ux/T327/log/out-of-mode-terminal-process.txt, полный log/chronos.log.

## Воспроизведение

1. Запустить release ChronOS в Developer.
2. Выполнить chronos-ipc select-tab:terminal.
3. Убедиться, что на экране открылся System.
4. Проверить лог на terminal: shell spawned перед active tab not in mode set → System.
5. Проверить дочерние процессы ChronOS.

select-tab:build аналогично пишет tab opened — loading tasks до fallback, то есть проблема не ограничена Terminal.

## Корреляция с кодом

- crates/app/src/side_panel_right/view.rs:324-350: on_tab_select без проверки mode set присваивает tab и сразу вызывает ensure_tab_view.
- crates/app/src/side_panel_right/view.rs:431-446: принадлежность mode set проверяется только в следующем render; тогда active tab меняется на System.

## Ожидание

Публичный IPC id вне текущего mode set сначала резолвится в допустимый tab и не создаёт/не запускает скрытую вкладку. Fallback в System остаётся видимым поведением, но без фоновых side effects и zombie.

## Предлагаемая приёмка

- select-tab:terminal в Developer показывает System без TerminalTab creation и без shell spawn.
- select-tab:build вне mode set не запускает loading tasks.
- Regression test проверяет порядок resolve-before-ensure; live log не содержит backend activation для rejected id.

Код в рамках T327 не менялся.

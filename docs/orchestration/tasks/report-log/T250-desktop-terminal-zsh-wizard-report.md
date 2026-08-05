# T250 — desktop-terminal: подавлен zsh newuser-install wizard

**Статус:** ЗАКРЫТ. Коммиты: `services : suppress zsh newuser-install wizard in terminal ZDOTDIR (T250)` + orchestration.

## Корень

`crates/services/src/terminal/mod.rs::launch()` делал только
`create_dir_all(ZDOTDIR)` — каталог без `.zshrc` zsh интерпретирует как
«новый пользователь» и печатает `zsh-newuser-install` wizard в первый
кадр виджета. Замысел пустого ZDOTDIR (тишина p10k/oh-my-zsh хуков)
сохранён — закрыт только побочный эффект.

## Фикс

- Новая `pub fn ensure_empty_zdotdir(zdotdir) -> io::Result<()>`:
  `create_dir_all` + идемпотентный touch `$ZDOTDIR/.zshrc`
  (`OpenOptions::create(true)` без `truncate` — существующий файл не
  трогается, пустой создаётся).
- `launch()` зовёт её перед `spawn_command`, ошибка не фатальна —
  только `tracing::warn!`.
- Юнит-тест `ensure_zdotdir_creates_zshrc_idempotently`: создание +
  повторный вызов не перезаписывает посеянное содержимое.

## Верификация

1. **Негативный контроль (доказательство бага):** zsh в pty с пустым
   `ZDOTDIR` (без `.zshrc`) печатает wizard — маркеры
   `zsh-newuser-install` + «This is the Z Shell configuration function
   for new users» оба True.
2. **Живой прогон:** `rm -rf /tmp/chronos-terminal-empty-zdot` →
   `chronos-start` → реальный процесс создал `.zshrc` (0 байт); первый
   PTY chunk в журнале — чистый промпт (`\u{1b}[1m\u{1b}[7m%` — RPS1),
   `zsh-newuser-install`/«Z Shell configuration» — 0 вхождений в логе.
   Виджет поднялся (Layer::Background surface 600×400).
3. `cargo test --release -p chronos-services --lib -- terminal` — 10/10
   (включая новый тест и существующий real-spawn smoke, не сломан).
4. `cargo build --release -p chronos` — чисто.

**Триггер:** нет (тикет не завязывал на пересъёмки).

— Lead Architect Agent, 2026-08-05

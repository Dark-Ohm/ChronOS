# T250 — desktop-terminal виджет показывает голый zsh newuser-install wizard

**Приоритет:** P1.
**Роль:** FRONTEND/сервисы (Rust) — маленький, точечный фикс.
**Источник:** `docs/orchestration/tasks/report-log/T223-design-audit-report-v4-reshoot.md`
находка #6 (табл. §3), топ-10 п.5.

## Контекст — корень уже виден в коде, не нужно гадать

`crates/services/src/terminal/mod.rs:29-31`:
```rust
/// Empty ZDOTDIR so an interactive zsh starts bare when `SHELL` is zsh
/// (the prompt-noise p10k/oh-my-zsh hooks never load).
const ZDOTDIR: &str = "/tmp/chronos-terminal-empty-zdot";
```
Замысел верный (не тащить p10k/oh-my-zsh шум в узкий грид терминала), но
побочный эффект не учтён: когда `ZDOTDIR` указывает на директорию БЕЗ
`.zshrc`, zsh считает это "первым запуском" и печатает свой
`zsh-newuser-install` wizard («This is the Z Shell configuration
function for new users... Type one of the keys in parentheses») —
сырой upstream-текст без всякой обёртки ChronOS, торчащий поверх
дефолтного `.zshrc` в первом кадре десктопного виджета-терминала.
Аудит: «читается как незаконченная интеграция».

## Что нужно

В `terminal/mod.rs`, там, где спавнится PTY с `ZDOTDIR=/tmp/chronos-
terminal-empty-zdot` (искать использование константы `ZDOTDIR`, вокруг
`CommandBuilder`/`spawn_command`, строки ~145-160) — **перед спавном**
гарантировать, что `$ZDOTDIR/.zshrc` существует (даже пустой файл
достаточен — zsh больше не считает это "новым пользователем", если файл
физически есть). Один `std::fs::create_dir_all` + `std::fs::write(...,
"")` (или `OpenOptions::new().create(true)`, идемпотентно — не
перезаписывать, если уже есть) на путь `ZDOTDIR/.zshrc` при инициализации
модуля/перед первым спавном.

Не менять сам замысел (пустой ZDOTDIR ради тишины prompt-хуков) — только
закрыть побочный эффект.

## Зона файлов

`crates/services/src/terminal/mod.rs`. Не пересекается с другими
тикетами этой волны.

## Верификация

- Живой запуск desktop-terminal виджета (первый запуск на чистом
  `/tmp` — можно симулировать `rm -rf /tmp/chronos-terminal-empty-zdot`
  перед стартом) — никакого `zsh-newuser-install` текста, сразу чистый
  промпт.
- `cargo build --release -p chronos` чисто, `cargo test --release -p
  chronos-services --lib -- terminal` зелёные (модуль уже содержит
  smoke-тест на реальный спавн, строка ~514-518 — не ломать).

## Коммит

`services : suppress zsh newuser-install wizard in terminal ZDOTDIR (T250)`.

---

## Резолюция (2026-08-05)

**ЗАКРЫТ.** Фикс в `crates/services/src/terminal/mod.rs`: новая
`ensure_empty_zdotdir()` (create_dir_all + идемпотентный touch `.zshrc`,
без truncate) вызывается из `launch()` перед спавном. Замысел пустого
ZDOTDIR сохранён.

Верификация: негативный контроль — zsh в pty с ZDOTDIR без `.zshrc`
печатает wizard (подтверждено); живой прогон на чистом `/tmp` — процесс
создал `.zshrc`, первый PTY chunk = чистый промпт, wizard-текста 0
вхождений. Тесты `chronos-services --lib -- terminal` 10/10, release-
сборка чистая. Отчёт:
`docs/orchestration/tasks/report-log/T250-desktop-terminal-zsh-wizard-report.md`.

# T258 — desktop-terminal: настоящий $HOME-шелл + тема из kitty.conf

**Роль:** BACKEND (Rust, services) + немного FRONTEND (применение темы к
рендеру).
**Приоритет:** P1, независим от T257 (можно делать параллельно —
разные файлы, разные слои).
**Источник:** `docs/superpowers/specs/2026-08-05-desktop-terminal-widget-design.md`
§«Архитектура» пп.3-4. Читать целиком перед началом.
**Зависимости:** нет.

## Контекст

`crates/services/src/terminal/mod.rs::launch()` (T250, 2026-08-05)
принудительно сажает шелл в песочницу `ZDOTDIR=/tmp/chronos-terminal-
empty-zdot` (`mod.rs:33`, `ensure_empty_zdotdir` на `mod.rs:41`) — тихий
prompt без p10k/oh-my-zsh хуков. Пользователь явно решил (брейншторм
2026-08-05): хочет настоящий шелл, шум промпта — не проблема. Это
относится **ко всем** потребителям `launch()` — и desktop-terminal
виджетам, и вкладке Terminal в правой панели (`side_panel_right/
tab/terminal.rs`).

Плюс: desktop-terminal виджет рендерит VT100-грид хардкод-константами
(`crates/app/src/desktop_terminal/view.rs:18-26` —
`FONT_SIZE`/`CELL_W`/`CELL_H`, дефолтная палитра ANSI где-то в
`alacritty_terminal`-обвязке) — нужно читать косметику из
`~/.config/kitty/kitty.conf`, если он есть.

## Что сделать

### 1. Убрать ZDOTDIR-песочницу

`crates/services/src/terminal/mod.rs`:
- Удалить `const ZDOTDIR`, `cmd.env("ZDOTDIR", ZDOTDIR)` (`mod.rs:166`),
  вызов `ensure_empty_zdotdir` (`mod.rs:170-172`).
- `ensure_empty_zdotdir` сама функция — удали целиком вместе с юнит-тестом
  `ensure_zdotdir_creates_zshrc_idempotently` (мёртвый код без
  потребителей хуже, чем чистое удаление — не оставляй `#[allow(dead_code)]`
  заглушку).
- `cmd` наследует нормальный `$HOME`/`$ZDOTDIR` (если пользователь его
  сам где-то экспортирует) — без явного оверрайда `launch()` теперь
  просто не трогает переменные окружения шелла вообще, пусть
  `portable_pty`/`CommandBuilder` дают чистое наследование родительского
  окружения (сверь, что `cmd` уже делает это по умолчанию, не нужно
  добавлять `cmd.env_clear()` или что-то подобное).

### 2. Kitty theme parser

Новый модуль `crates/services/src/terminal/kitty_theme.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub struct KittyTheme {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub background_opacity: Option<f32>,
    pub foreground: Option<Rgba>,
    pub background: Option<Rgba>,
    pub palette: [Option<Rgba>; 16], // color0..color15
}

pub fn load(path: &std::path::Path) -> KittyTheme;
pub fn default_path() -> Option<std::path::PathBuf>; // ~/.config/kitty/kitty.conf
```

Формат парсинга — построчно `key value` (kitty.conf — собственный
формат, НЕ TOML/YAML/INI с секциями): пропускать пустые строки и строки
с `#`, `include <path>` — резолвить относительно директории текущего
файла, максимум 1 уровень рекурсии (глубже — не наш случай). Игнорировать
неизвестные ключи молча (kitty.conf содержит сотни directives, нам нужны
только: `font_family`, `font_size`, `background_opacity`, `foreground`,
`background`, `color0`..`color15`). Цвета в kitty.conf — hex-формат
(`#rrggbb`) или именованные X11-цвета — **ограничься hex**, именованные
цвета не парсить (недостающий парсер — не баг, задокументируй как
известное ограничение в doc-комментарии). Отсутствующий файл/директория
→ `KittyTheme::default()` (все `None`/`[None; 16]`).

### 3. Применение темы в рендере

`crates/app/src/desktop_terminal/view.rs`: заменить хардкод-константы
`FONT_SIZE`/`CELL_W`/`CELL_H` (`view.rs:18-26`) на значения из
`KittyTheme` (fallback на текущие константы, если поле `None`).
`background_opacity`/`foreground`/`background`/`palette` — применить там,
где сейчас рендерится фон/цвета VT100-грида (сверь, как алиасятся ANSI
0-15 в текущей интеграции с `alacritty_terminal`, чтобы не задваивать
источник истины по цветам — используй существующий color-mapping слой,
если он есть, не пиши новый параллельный).

Загрузка темы — один раз при создании `DesktopTerminalView` (не на
каждый render — kitty.conf не hot-reload в этом тикете, если нужен
live-reload при правке конфига — отдельная веха, не сюда).

## Зона файлов

- `crates/services/src/terminal/{mod.rs,kitty_theme.rs}` (mod.rs —
  только удаление ZDOTDIR-кода, kitty_theme.rs — новый файл).
- `crates/app/src/desktop_terminal/view.rs` — применение темы к рендеру
  (НЕ трогай логику владения `Terminal`/реестра — это T257, если T257
  уже слит к моменту твоей работы, бери `Terminal` оттуда, если ещё нет —
  оставь текущий `spawn_terminal()`, T257/T259 доразберутся при мёрдже).

**НЕ трогать:** `desktop_terminal/{mod.rs,config.rs}` (T257), Edit
Mode/drag/resize (T259), `side_panel_right/tab/terminal.rs` (получает
эффект от правки `launch()` автоматически — сам файл трогать не нужно,
если только тесты не ссылаются на убранный ZDOTDIR-код).

## Верификация

- `cargo build --release -p chronos-services -p chronos` — чисто.
- `cargo test --release -p chronos-services --lib -- terminal` — зелёные
  (учти, что тест `ensure_zdotdir_creates_zshrc_idempotently` удалён, не
  должен остаться падающим или сиротой).
- Юнит-тесты `kitty_theme::load`: валидный файл (все поля), файл с
  `include`, отсутствующий файл → default, файл с неизвестными ключами
  (не должен паниковать/терять уже распарсенные поля).
- Живой прогон, обе темы: открыть desktop-terminal виджет (или
  side-panel Terminal таб) → реальный prompt пользователя (p10k, если
  настроен) — НЕ голый `%`/`$`, НЕ zsh-newuser-install wizard.
  `echo $ZDOTDIR` — либо пусто, либо реальное значение из окружения
  пользователя, точно не `/tmp/chronos-terminal-empty-zdot`.
- Живой прогон kitty-темы: временно подставить тестовый `kitty.conf` с
  нестандартным `font_size=18`/`background_opacity=0.7`/`color0=#ff0000`
  → grim показывает разницу против дефолта → вернуть родной конфиг.

## Коммит

`services+ui : desktop-terminal real $HOME shell + kitty.conf theme (T258)`.

## Отчёт

`docs/orchestration/tasks/report/T258-desktop-terminal-real-shell-and-kitty-theme-report.md`.

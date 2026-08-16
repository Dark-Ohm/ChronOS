# T173 — живой смок слайса 3: отчёт QA

**Дата:** 2026-07-31
**Роль:** QA
**Ветка:** `master` (рабочее дерево было чистым до старта)
**Бинарь:** `target/release/chronos`
**Лог:** `/tmp/t173-live/chronos3.log`
**Кадры и сырые замеры:** `/tmp/t173-live/`

## Что сделано

- Собран release-бинарь командой `cargo build --release -p chronos`.
  Команда завершилась с exit code 0; полный повторяемый вывод — в команде
  выше, а в сессии сборщик сообщил `Finished release profile [optimized]`.
  Предупреждения компилятора были, ошибок компиляции не было.

  ```text
  $ cargo build --release -p chronos
  Finished release profile [optimized] target(s) in 0.56s
  exit code: 0
  ```

  Это не заменяет live-улики: для окон использовался именно этот release
  binary и `RUST_LOG=info`.


- Живой shell запускался только как `RUST_LOG=info target/release/chronos`.
- Сняты кадры `grim -o DP-1` и замеры `hyprctl layers`.
- Проверены ленивое создание и кэширование вкладок, ширина по вкладке,
  память ручного resize, scene override, нагрузка потоков и полный лог на
  паники.
- Продуктовый код не менялся. Временный `scenes.toml` использовался только
  для P1/P5 и удалён после прогона.

## Результаты

### P1 — вкладка исчезает из режима, панель не закрывается

**Факт:** сценарий воспроизведён корректно:

1. `set-workspace-mode:developer`;
2. `toggle-side-panel-right` → `DP-1 2506 54 1410`;
3. клик по handle → System, `DP-1 2160 400 1410`;
4. клик по Editor → `DP-1 2000 560 1410`;
5. `set-workspace-mode:gamer`.

Кадры:

- `/tmp/t173-live/p1a-developer-rail.png`
- `/tmp/t173-live/p1c-editor.png`
- `/tmp/t173-live/p1d-gamer.png`

Ключевые строки `/tmp/t173-live/chronos2.log`:

```text
103: ... side_panel_right: lazy-create tab view tab="Editor"
104: ... side_panel_right: tab select → apply per-tab width before=400.0 after=560.0 content_open=true tab="Editor"
107: ... IPC set-workspace-mode received mode="Gamer"
108: ... scene: restored scene=t173-gamer-default mode="gamer"
109: ... workspace_mode: switched mode="Gamer"
110: ... side_panel_right: active tab not in mode set → System was="Editor"
```

После fallback панель осталась открыта, активная вкладка ушла на System, но
ширина осталась:

```text
P1C Developer Editor: DP-1 2000 560 1410
P1D Gamer fallback:   DP-1 2000 560 1410
```

**Статус: FAIL / регрессия.** Ожидалось, что после fallback на System ширина
станет 400 px. Она осталась 560 px. QA код не менял.

### P2 — четырнадцать честных пустых состояний

Сняты все 14 кадров:

```text
/tmp/t173-live/p2-tab-0.png ... /tmp/t173-live/p2-tab-13.png
```

Каждый кадр имеет размер 2560×1440; полные sha256 находятся в выводе
инвентаризации и соответствуют файлам в `/tmp/t173-live/`.

**Не проверено:** кадры не были открыты глазами и увеличены через `magick`,
поэтому наличие и читаемость иконки/названия/описания, уникальность текстов
и отсутствие запрещённых формулировок не выдаются за PASS.

**Статус: NOT VERIFIED.**

### P3 — ленивость и кэш

Сквозной проход по Developer rail дал следующую последовательность:

```text
System          lazy-count=1
Files           lazy-count=2
Editor          lazy-count=3
Terminal        lazy-count=4
Preview         lazy-count=5
Inspector       lazy-count=6
Build           lazy-count=7
Source control  lazy-count=8
ACP settings    lazy-count=9
MCP settings    lazy-count=10
LSP settings    lazy-count=11
API providers   lazy-count=12
Editor settings lazy-count=13
Hyprland binds  lazy-count=14
```

Для каждой вкладки, кроме уже созданной System, в логе ровно одна строка
`side_panel_right: lazy-create tab view`. Повторный клик по Editor оставил
счётчик на 14 и не создал новую view.

Полный журнал счётчиков: `/tmp/t173-live/p3-counts.txt`. Основные строки
лога: `/tmp/t173-live/chronos3.log`, например:

```text
99: ... lazy-create tab view tab="Files"
100: ... tab select → apply per-tab width before=400.0 after=440.0 ... tab="Files"
104: ... lazy-create tab view tab="Editor"
105: ... tab select → apply per-tab width before=440.0 after=560.0 ... tab="Editor"
113: ... lazy-create tab view tab="Preview"
126: ... lazy-create tab view tab="Source control"
152: ... lazy-create tab view tab="Hyprland binds"
```

**Статус: PASS по логическому контракту lazy-create/cache.**

### P4 — память resize по вкладкам

Первая попытка drag использовала неправильную последовательность ydotool и
ширину не изменила. Повтор выполнен корректно с масками `click 0x40` (down) и
`click 0x80` (up).

Фактическая последовательность слоя:

```text
Editor before resize: DP-1 2000 560 1410
После resize:         DP-1 1800 760 1410
Files:                DP-1 2120 440 1410
Editor restore:       DP-1 1800 760 1410
Editor same-tab click:DP-1 1800 760 1410
```

Лог содержит применение ширины и возврат к сохранённой ширине:

```text
... tab select → apply per-tab width before=760.0 after=440.0 ... tab="Files"
... tab select → apply per-tab width before=440.0 after=760.0 ... tab="Editor"
```

Сырые данные: `/tmp/t173-live/p4-retry.txt`.

**Статус: PASS.** Ручная ширина Editor восстановилась; повторный клик по
активному Editor её не сбросил.

### P5 — scene override с четырнадцатью вкладками и мусорным id

Так как исходного `~/.config/chronos/scenes.toml` до теста не было, создан
временный валидный TOML с отдельными ключами и сценами:

```toml
version = 1

[last]
developer = "t173-developer-full"
gamer = "t173-gamer-default"

[[scene]]
id = "t173-developer-full"
name = "T173 Developer Full"
mode = "developer"
rail_tabs = ["system", "files", "editor", "terminal", "preview", "inspector", "build", "source_control", "acp_settings", "mcp_settings", "lsp_settings", "api_providers", "editor_settings", "hyprland_binds", "garbage_id"]

[[scene]]
id = "t173-gamer-default"
name = "T173 Gamer Default"
mode = "gamer"
```

Мусорный id пропускался с warn:

```text
WARN ... rail: unknown tab id in scene override, ignoring tab=garbage_id
```

Сцены резолвились при переключениях:

```text
... scene: restored scene=t173-gamer-default mode="gamer"
... scene: restored scene=t173-developer-full mode="developer"
```

Hash сцены до и после переключений совпал:

```text
656029377f996a60b18bdd262331b00fa1cefe1e2420b2a0c02bae7a3dbf160d
```

Кадры mode-прогона: `/tmp/t173-live/p5-gamer.png`,
`/tmp/t173-live/p5-developer.png`.

**Ограничение:** кадры не были открыты глазами, поэтому визуальное
«на рейле показан именно override» отдельно не принимается как доказанный
факт. Функциональная часть parse/skip/read-only подтверждена логом и hash.

**Статус: PARTIAL — функциональная часть подтверждена, визуальная часть не
проверена.**

### P6 — док и контекстное меню

Подтверждено по логу, что нерезолвящиеся пины диагностируются:

```text
46: ... dock: skipping pinned app pin=firefox reason="no AppEntry (no matching .desktop basename)"
56: ... dock: skipping pinned app pin=code reason="no AppEntry (no matching .desktop basename)"
57: ... dock: skipping pinned app pin=vivaldi reason="no AppEntry (no matching .desktop basename)"
```

Режимы менялись через IPC, а `scene: restored` и
`workspace_mode: switched` присутствуют в логе. Это доказывает смену режима,
но не доказывает изменение фактического состава дока.

**Не проверено:** координаты отдельных dock-иконок нельзя было доказать из
`hyprctl layers` (он показал только общий bar `DP-1 x=0 y=0 w=2560 h=30`),
поэтому правый клик, кадр с `Unpin` и отсутствие падения именно после меню не
разыгрывались.

**Статус: NOT VERIFIED.** Не считать наличие warn-строк доказательством
живого контекстного меню.

### P7 — нагрузка при 14 вкладках

Файлы замеров:

- `/tmp/t173-live/p7-idle.txt`
- `/tmp/t173-live/p7-switch.txt`
- `/tmp/t173-live/p7-resize.txt`

Команда в каждом файле:

```bash
top -H -b -n 1 -p 959135 -w 240
```

В каждом файле сохранены пять полных выборок команды вместе с timestamp;
файлы являются первичной уликой, а таблица ниже — только сжатой выборкой
максимумов.

Наблюдения по пяти samples на фазу:

| Фаза | Наблюдавшийся максимум потока `chronos` |
|---|---:|
| спокойная панель | 19.8% |
| быстрое переключение вкладок | 19.7% |
| resize | 34.3% |

Поток не уходил к 100%; в resize также наблюдался `tokio-rt-worker`
примерно до 19.5%.

**Статус: MEASURED — насыщения не наблюдалось в этом прогоне.** Это
измерение, не доказательство отсутствия долгосрочной проблемы
производительности.

### P8 — паники по всему логу

Команда:

```bash
grep -n "panicked at" /tmp/t173-live/chronos3.log
```

Вывод пустой, совпадений нет.

**Статус: PASS для `/tmp/t173-live/chronos3.log`.**

## Конфиги и гигиена дерева

До старта сохранены копии в `/tmp/t173-config-backup/`. Исходного
`scenes.toml` не было; временный файл удалён в конце.

Финальная проверка:

```text
scenes.toml: absent (original state)
workspace.toml: SAME
  d05510d5d6393c39a1ca6047b1e8428ad42b9d82cc22adbd62ae0188599351ae
  d05510d5d6393c39a1ca6047b1e8428ad42b9d82cc22adbd62ae0188599351ae
dock.toml: SAME
  55c6cef0d0d2b618fb7fd5df25ca53c9fe35cd86f433f42e33920320b25fe25a
  55c6cef0d0d2b618fb7fd5df25ca53c9fe35cd86f433f42e33920320b25fe25a
bar.toml: SAME
  cd67809e8b4ae3e3f63e5c6282a86a876a22f745b5d30829f903afe43603495f
  cd67809e8b4ae3e3f63e5c6282a86a876a22f745b5d30829f903afe43603495f
monitor.toml: SAME
  2b114e95148dbfd777954b5a4e58005a7a678316e5636cedb6d7804b208c8ac6
  2b114e95148dbfd777954b5a4e58005a7a678316e5636cedb6d7804b208c8ac6
no chronos process
```

До создания отчёта `git status --short --branch` был:

```text
## master...origin/master [ahead 44]
```

После создания отчёта он показывал только новый путь отчёта как untracked;
перед коммитом будет отдельно проверен поимённый staged diff, чтобы в коммит
попал только этот файл.

Кадры и логи в репозиторий не добавлялись.

## Что НЕ сделано / что за архитектором

1. Исправление найденной P1-регрессии: после mode fallback активная System
   получает корректный tab state, но слой сохраняет ширину предыдущего Editor
   (`560` вместо `400`).
2. Визуальная проверка P2: открыть 14 кадров глазами, вырезать rail,
   увеличить и проверить уникальные описания/запрещённые фразы.
3. Визуальная часть P5: глазами подтвердить, что rail показывает именно
   scene override.
4. P6: доказать координату dock icon, правый клик, кадр с `Unpin`, живой
   menu path и отсутствие паники после него.

Никаких исправлений найденных дефектов в рамках T173 не выполнялось.

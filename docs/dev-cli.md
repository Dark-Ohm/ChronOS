# ChronOS Dev CLI

Обёртки для повседневной работы с шеллом: сборка, старт/стоп, debug +
hot-reload. Исходники — `scripts/dev/`, установка — symlink в
`~/.local/bin` (как `chronos-fm` у соседнего репо).

> Не путать с **Chronos-FM** (`chronos-fm` в PATH) — другой бинарь, другой
> репозиторий. Команды ниже бьют только в процесс с именем **`chronos`**.

---

## Установка (один раз)

Из корня ChronOS:

```sh
./scripts/install-dev-cli.sh
```

Ставит symlink:

| `~/.local/bin/…` | → |
|---|---|
| `chronos-rebuild` | `scripts/dev/chronos-rebuild` |
| `chronos-reload` | `scripts/dev/chronos-reload` |
| `chronos-stop` | `scripts/dev/chronos-stop` |
| `chronos-start` | `scripts/dev/chronos-start` |
| `chronos-debug` | `scripts/dev/chronos-debug` |

`git pull` обновляет поведение без переустановки (ссылки на репо).

Если команды не находятся:

```sh
export PATH="$HOME/.local/bin:$PATH"   # в ~/.zshrc
```

**Корень репо** ищется автоматически (walk up от скрипта до workspace
`Cargo.toml` с `crates/app`). Override:

```sh
export CHRONOS_ROOT=/path/to/ChronOS
```

---

## Команды

### `chronos-rebuild`

Собрать **release** бинарь. **Не** стартует и **не** останавливает шелл.

```sh
chronos-rebuild              # cargo build --release -p chronos
chronos-rebuild --debug      # cargo build -p chronos --features hot-reload
```

После rebuild, чтобы подхватить новый код: `chronos-stop && chronos-start`
(или `chronos-debug` для debug/hot-reload).

---

### `chronos-start`

Запуск **release** шелла в фоне.

| | |
|---|---|
| Бинарь | `$CHRONOS_ROOT/target/release/chronos` |
| Лог | `${XDG_STATE_HOME:-~/.local/state}/chronos/chronos.log` |
| `RUST_LOG` | по умолчанию `info` (можно переопределить) |
| Single-instance | если `chronos` уже бежит — **ошибка**, не dual |

```sh
chronos-start
RUST_LOG=debug chronos-start   # редкий случай: release + verbose log
```

Нет бинаря → подсказка `chronos-rebuild`.

---

### `chronos-stop`

Остановить шелл.

```sh
chronos-stop    # идемпотентно: нет процесса → exit 0
```

Внутри только **`pkill -x chronos`** (при необходимости `-9 -x`).  
**Никогда** `pkill -f chronos` — зацепит `chronos-fm` и всё с «chronos» в argv.

---

### `chronos-debug`

Debug-сборка **с** feature `hot-reload` + запуск. Контекст, в котором
имеет смысл `chronos-reload`.

| | |
|---|---|
| Build | `cargo build -p chronos --features hot-reload` |
| Бинарь | `target/debug/chronos` |
| Лог | `…/chronos-debug.log` |
| `RUST_LOG` | по умолчанию `debug` |
| Single-instance | как у start |

```sh
chronos-stop          # если крутился release
chronos-debug
```

---

### `chronos-reload`

Один раз пересобрать dylib **`chronos-hotview`** (T110 / Design B).  
**Не** убивает шелл. `hot-lib-reloader` подхватывает новый `.so` (~1–2 с).

Требования:

1. Шелл уже запущен через **`chronos-debug`** (путь `target/debug/…`).
2. На release-инстансе — отказ с подсказкой `chronos-stop && chronos-debug`.

```sh
# типичный цикл hotview-правок:
chronos-debug          # один раз
# … правка crates/hotview …
chronos-reload
```

Фоновый `cargo watch` **не** поднимается. При желании вручную:

```sh
cargo watch --delay 0 -w crates/hotview \
  -s 'cargo build -p chronos-hotview 2>&1 | tail -3'
```

Подробности pitfall’ов dylib: skill `hot-lib-reloader`.

---

## Типовые сценарии

### Живой UX-смок (как в приёмке)

```sh
chronos-rebuild
chronos-stop
chronos-start
# grim / notify-send / клики
tail -f ~/.local/state/chronos/chronos.log
chronos-stop
```

### После правок в `crates/app` (нужен полный рестарт)

```sh
chronos-rebuild && chronos-stop && chronos-start
```

### Итерация только `hotview` (сеть/виджеты на dylib)

```sh
chronos-stop
chronos-debug
# edit crates/hotview → chronos-reload
```

### «Всё зависло / ghost»

```sh
chronos-stop
# при необходимости: pgrep -x chronos; hyprctl layers
chronos-start
```

---

## Логи

| Режим | Файл |
|---|---|
| `chronos-start` | `~/.local/state/chronos/chronos.log` |
| `chronos-debug` | `~/.local/state/chronos/chronos-debug.log` |

```sh
tail -f ~/.local/state/chronos/chronos.log
```

Ранний старт без CLI (как раньше):

```sh
RUST_LOG=info ./target/release/chronos
```

---

## Что не делают эти команды

- Не systemd / autostart Hyprland.
- Не ставят в `/usr`.
- Не трогают `chronos-fm`.
- `chronos-reload` ≠ restart шелла и ≠ `cargo build --release`.
- Не заменяют `cargo test` / clippy.

---

## Исходники

```
scripts/dev/common.sh       # REPO, пути, pkill -x helpers
scripts/dev/chronos-*
scripts/install-dev-cli.sh
```

Краткая шпаргалка также в [`CONTRIBUTING.md`](../CONTRIBUTING.md#dev-cli).
Задача-канон: `orchestration/tasks/done/T122-dev-shell-scripts.md`.

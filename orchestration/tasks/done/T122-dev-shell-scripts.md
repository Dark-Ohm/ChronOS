# T122 — ПРИНЯТ WITH CAVEATS (2026-07-25)

**Статус: ACCEPTED WITH CAVEATS.** Dev CLI chronos-rebuild/reload/stop/start/debug.
Commit: `23c5cda`. Live start/stop verified. ShellCheck SC2034 caveat.
Volume E0382 errata in same commit.
Review/report → report-log/T122-*.

---

<!-- T122 — Terminal wrappers: chronos-{rebuild,reload,stop,start,debug}.
     Агент не в имени брифа. -->

# T122 — Dev CLI: `chronos-{rebuild,reload,stop,start,debug}`

**Статус: OPEN, не назначен.**  
**Контекст:** Архитектор 2026-07-25 поднял release-шелл вручную
(`RUST_LOG=info ./target/release/chronos`). HANDOFF/CONTRIBUTING канон:
`pkill -x chronos` (не `-f`), бинарь `target/release/chronos`. Hot-reload
T110: feature `hot-reload` + `crates/hotview` + `cargo watch` (~2s).

Сейчас **нет** `chronos-*` в `~/.local/bin` (есть только `chronos-fm` →
другой репо). Нужны пять команд в PATH.

## Цель

Пять исполняемых скриптов (zsh-совместимый bash `#!/usr/bin/env bash`,
`set -euo pipefail`), репозиторий + установка в `~/.local/bin`:

| Команда | Поведение |
|---|---|
| **`chronos-rebuild`** | `cargo build --release -p chronos` из **корня ChronOS-репо**. Exit ≠ 0 при fail. **Не** убивает и **не** стартует шелл (это start/stop). Печатать итоговую строку `Finished` / путь бинаря. |
| **`chronos-reload`** | Dev hot-reload **только** hotview-dylib (T110): собрать `chronos-hotview` + убедиться, что watch-процесс есть **или** один раз пересобрать dylib, если шелл уже запущен с `--features hot-reload`. **Не** путать с full restart. Если hot-reload binary не запущен — честный stderr: «нужен `chronos-debug` (hot-reload build), не release». |
| **`chronos-stop`** | `pkill -x chronos` (строго `-x`, не `-f`). Идемпотентно: нет процесса → exit 0 + msg. Не трогать `chronos-fm` / чужие binary с «chronos» в argv. |
| **`chronos-start`** | Старт **release** бинаря: `RUST_LOG=${RUST_LOG:-info} "$REPO/target/release/chronos"`. Если бинаря нет → fail с подсказкой `chronos-rebuild`. Если уже бежит `chronos` → fail (не dual instance) или stop+start — **выбери fail**, dual instance опасен (layer-shell ghosts). Background: `nohup`/`disown` или `&` + log в `${XDG_STATE_HOME:-$HOME/.local/state}/chronos/chronos.log` (mkdir -p). Печатать pid. |
| **`chronos-debug`** | Старт **debug** бинаря с **`--features hot-reload`**:  
  `cargo build -p chronos --features hot-reload` (если бинарь stale — минимум `cargo build …`) затем  
  `RUST_LOG=${RUST_LOG:-debug} "$REPO/target/debug/chronos"`.  
  Тот же single-instance guard. Log: `…/chronos-debug.log`. Это путь, с которым имеет смысл `chronos-reload`. |

Опционально (тот же PR, если дёшево):

| | |
|---|---|
| `chronos-restart` | `chronos-stop` + wait + `chronos-start` (release) |
| install helper | `scripts/install-dev-cli.sh` → symlink/copy в `~/.local/bin` |

## Расположение в репо

```
scripts/dev/
  chronos-rebuild
  chronos-reload
  chronos-stop
  chronos-start
  chronos-debug
  common.sh          # REPO root resolve, bin paths, pgrep -x chronos
scripts/install-dev-cli.sh   # ln -sf → ~/.local/bin
```

**REPO root:** не hardcode `/home/neo/...`. Resolve:
- env `CHRONOS_ROOT` если set, иначе
- walk up from script realpath to dir containing `Cargo.toml` with
  `name`/`members` workspace ChronOS (есть `crates/app`), иначе
- fail.

Скрипты в `~/.local/bin` — **symlinks** на repo scripts (как `chronos-fm`),
чтобы git pull обновлял поведение.

## Контракты (жёстко)

1. **`pkill -x chronos` only** — HANDOFF blood fact. Document in script comment.
2. **Release start** не включает `hot-reload` feature (release product path).
3. **Debug** = debug profile + `hot-reload` feature; reload script targets that.
4. **`chronos-reload` must not** `pkill` the shell. Only rebuild dylib /
   ensure `cargo watch` on `crates/hotview` (document chosen design):
   - **Preferred A:** if no watch running, start  
     `cargo watch --delay 0 -w crates/hotview -s 'cargo build -p chronos-hotview'`  
     in background with pid file under state dir; if already running,  
     `cargo build -p chronos-hotview` once (nudge).
   - **Preferred B (simpler):** only `cargo build -p chronos-hotview` once  
     (hot-lib-reloader picks new .so). Watch left to human.  
   **Pick A or B, document in script header.** B is enough for T122 MVP.
5. No AI trailers. ShellCheck-clean if shellcheck available (`shellcheck scripts/dev/*`).
6. Do not modify Rust app code in this task (scripts only + optional CONTRIBUTING one-liner).
7. Do not install to `/usr` — only `~/.local/bin`.

## CONTRIBUTING / README

Добавь короткий блок «Dev CLI» в `CONTRIBUTING.md` (3–8 строк):

```bash
./scripts/install-dev-cli.sh   # once
chronos-rebuild && chronos-stop; chronos-start
chronos-debug                 # hot-reload capable
chronos-reload                # rebuild hotview dylib
chronos-stop
```

## Верификация

```bash
# from clean PATH with ~/.local/bin
./scripts/install-dev-cli.sh
command -v chronos-rebuild chronos-start chronos-stop chronos-debug chronos-reload

chronos-stop                  # ok if none
chronos-rebuild               # release green
chronos-start                 # pgrep -x chronos
chronos-start                 # second call must fail (single-instance)
chronos-stop
# optional if time:
chronos-debug & sleep 2; pgrep -x chronos; chronos-reload; chronos-stop
```

Отчёт:  
`orchestration/tasks/report/T122-dev-shell-scripts-report.md`  
— пути install, A vs B для reload, sample command output.

## Accept / Reject

**Accept:** five commands in repo + install; stop uses `-x`; start release
single-instance; rebuild doesn't auto-start; reload doesn't kill shell;
CONTRIBUTING blurb.

**Reject:** `pkill -f chronos`; hardcoded absolute home only; dual
instance start; reload = full restart disguised; scripts only in
`/tmp` not repo.

## Out of scope

- systemd user unit
- Hyprland autostart rewrite
- packaging for distro
- changing hotview Rust API

<!-- T050 — migrated 2026-07-22 from docs/orchestration/report-log/grok-report-11.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: Grok №11 desktop-terminal spike — 2026-07-18

## Сделано (факт, не намерение)

- `crates/app/src/desktop_terminal/mod.rs`: layer-shell окно `Layer::Background`, namespace `desktop-terminal`, anchor TOP|LEFT, margin top=80 left=48, size 600×400, `KeyboardInteractivity::OnDemand` (не Exclusive).
- `crates/app/src/desktop_terminal/view.rs`: PTY через `portable-pty` 0.9 (`$SHELL` + bare `ZDOTDIR`), VT100 через `alacritty_terminal` 0.26 (`Term` + `Processor::advance`), reader-тред → mpsc → GPUI poll 16мс, рендер grid строками + курсор, клавиатура → PTY.
- `crates/app/Cargo.toml`: deps `portable-pty = "0.9"`, `alacritty_terminal = "0.26"`.
- `crates/app/src/main.rs`: только `mod desktop_terminal;` + `desktop_terminal::init(cx);` (tray_menu чужой WIP — не в коммите).
- `Cargo.lock`: lock для новых deps.
- Юнит-тест `vt_parser_renders_echo_output` (ANSI → grid содержит маркер).
- Опциональный probe: `CHRONOS_DT_PROBE=1` пишет `echo __CHRONOS_DT_SPIKE_OK__` в PTY (для headless-смока без ydotool).

## Расхождения со спекой/планом

- Абсолютная позиция через `Monitor.x/y` из compositor **не использована** — layer-shell margins (TOP|LEFT + margin) дают ту же геометрию на активном output; `Monitor` API не понадобился для MVP. Решение, не TODO.
- ANSI-цвета ячеек **не** рендерятся (тема text/bg). Спека: «бонус, не обязательно».
- Scrollback UI / wheel scroll — нет (grid display_offset не крутится с мыши). Spike scope.
- Luau API / скины / resize / drag / copy-paste — сознательно вне scope.
- ydotool socket на машине мёртв (`Connection refused` на `/tmp/.ydotool_socket`) — интерактивный ввод мышью/клавиатурой синтетически недоступен; I/O доказан probe+лог VT grid.
- Grim-кроп региона терминала часто перекрыт kitty/Zed (regular windows **выше** layer Background) — визуальный crop чужого окна; факт surface — `hyprctl layers`, факт I/O — лог SPIKE_OK.

## Не реализовано из acceptance criteria

- Чистый grim-скрин **содержимого** VT без перекрывающих окон (композитор рисует XDG-окна поверх Background) — surface + SPIKE в логе есть, «красивый» crop текста без сдвига чужих окон — нет.
- Полноценный click-to-type смок через ydotool — инфраструктура input-automation сломана в сессии.

## Проверено фактом, не на словах

```
cargo test --workspace --lib --bins
# 4 + 67 + 25 + 80 + 3 = 179 passed (было ~177, +2 unit)

cargo build --release -p chronos   # ok

CHRONOS_DT_PROBE=1 RUST_LOG=info ./target/release/chronos
# log:
# desktop_terminal: shell spawned on PTY cols=80 rows=24 shell=/bin/zsh
# desktop_terminal: first PTY chunk n=104 ...
# desktop_terminal: opened Layer::Background surface (600×400, margin top=80 left=48)
# desktop_terminal: probe command written to PTY
# desktop_terminal: SPIKE_OK visible in VT grid
#   lines=["neo% echo __CHRONOS_DT_SPIKE_OK__", "__CHRONOS_DT_SPIKE_OK__", "neo%"]

hyprctl layers
# Layer level 0 (background):
#   namespace: desktop-terminal, xywh: 2608 112 600 400 (или 48 112 на DP-1)
#   pid = chronos
```

KEY: шелл живой, `echo` доехал до VT grid (три строки: команда, вывод, новый prompt).

## Новые риски / известные баги

- **Background под normal windows**: десктоп-виджет всегда под приложениями — это корректно для wallpaper-adjacent widget, но смок grim-ом без пустого workspace/monitor ложный.
- **Primary display**: `pick_display` → primary; на dual-head surface прыгает DP-1↔HDMI-A-1 при смене primary.
- **Fancy shell**: zsh с пустым `ZDOTDIR` без `.zshrc` запускает newuser wizard и глотает ввод; spike кладёт bare ZDOTDIR + ожидается touch `.zshrc` (в смоке: `/tmp/chronos-dt-empty-zdot/.zshrc`).
- **Exclusive keyboard** не использовать — OnDemand: фокус только по клику (как launcher).
- Рендер 24 строк div'ами — spike OK, на 144Hz/длинном scrollback нужен custom Element.
- Auto-probe **off by default**; только `CHRONOS_DT_PROBE=1`.

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлялись (спайк, не продуктовое API). Desktop-widget plugin API по-прежнему gap в MEMORY.md «На горизонте» — после приёмки спайка Архитектор может зафиксировать: `Layer::Background` + PTY/VT на Rust жизнеспособны на Hyprland 0.55.4.

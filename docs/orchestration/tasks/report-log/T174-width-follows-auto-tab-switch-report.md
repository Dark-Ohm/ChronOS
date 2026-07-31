# T174 — ширина следует за вкладкой при автоматической смене на System

**Дата:** 2026-07-31
**Роль:** FRONTEND
**Бинарь:** `target/release/chronos`
**Live-лог:** `/tmp/t174/live.log`
**Evidence:** `/tmp/t174/evidence.txt`
**Кадры:** `/tmp/t174/editor-before.png`, `/tmp/t174/system-after.png`

## Что сделано

В `crates/app/src/side_panel_right/view.rs`:

- вынесено общее применение ширины активной вкладки в
  `apply_active_tab_width`;
- `on_tab_select` использует этот же путь;
- автоматический fallback из `render()` идёт через `resolve_active_tab`,
  переводит вкладку на `System` и применяет `active_tab_width(System)`;
- сохранён guard: при `dock_content == false` ширина остаётся
  `RAIL_ONLY_WIDTH`;
- сохранена память ручного resize через `tab_resize_memory`;
- сохранён прежний retry-контракт: пользовательский `on_tab_select` всё ещё
  инвалидирует `last_resized_width` для следующего platform resize;
- для render-fallback `last_resized_width` инвалидируется только при реальном
  изменении ширины;
- `ensure_content_width` вызывается при открытом контенте даже если target
  совпадает, чтобы корректно сбросить `last_exclusive_zone`;
- добавлены три `#[gpui::test]` regression-теста в том же `view.rs`:
  preferred System width, System resize memory и rail-only guard.

Продуктовый код вне разрешённого `view.rs` не менялся.

## TDD evidence

До реализации targeted-тесты завершились с ожидаемым RED:

```text
cargo test -p chronos mode_fallback_ --lib
exit code: 101
error[E0599]: no method named `ensure_active_tab_in_rail` found
```

После реализации:

```text
cargo test -p chronos mode_fallback_ --lib
exit code: 0
running 3 tests
test side_panel_right::view::tests::mode_fallback_applies_system_preferred_width ... ok
test side_panel_right::view::tests::mode_fallback_keeps_rail_only_width_closed ... ok
test side_panel_right::view::tests::mode_fallback_restores_system_resize_memory ... ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

## Проверки

Все обязательные команды завершились с exit code 0:

```text
cargo test -p chronos
  lib: 109 passed, 0 failed, 0 ignored
  bin chronos: 264 passed, 0 failed, 0 ignored
  doc-tests: 0 passed, 0 failed, 0 ignored

cargo clippy -p chronos --all-targets
  exit code: 0; errors: 0

cargo build --release -p chronos
  Finished release profile [optimized]
  exit code: 0
```

В workspace остаются существующие warnings clippy/compiler; ошибок в
`side_panel_right/view.rs` нет.

## Release live-smoke

Перед стартом проверено, что fullscreen не занят:

```text
HDMI-A-1 fullscreen=None
DP-1 fullscreen=None
```

Процесс запущен как `RUST_LOG=info target/release/chronos`, PID `1028818`,
IPC-сокет `/run/user/1000/chronos.sock` поднялся.

Сценарий: Developer → открыть panel → раскрыть handle → Editor → Gamer.

Геометрия `hyprctl layers -j` с фильтром `namespace ==
"side_panel_right"`:

```text
rail:            DP-1 x=2506 w=54 h=1410
System expanded: DP-1 x=2160 w=400 h=1410
Editor:          DP-1 x=2000 w=560 h=1410
after Gamer:     DP-1 x=2160 w=400 h=1410
```

Кадры сняты командами:

```bash
grim -o DP-1 /tmp/t174/editor-before.png
grim -o DP-1 /tmp/t174/system-after.png
```

Метаданные обоих кадров: PNG 2560×1440, RGB, non-interlaced.

Ключевые строки live-лога:

```text
side_panel_right: lazy-create tab view tab="Editor"
side_panel_right: apply per-tab width before=400.0 after=560.0 content_open=true tab="Editor"
IPC set-workspace-mode received mode="Gamer"
side_panel_right: active tab not in mode set → System was="Editor"
side_panel_right: apply per-tab width before=560.0 after=400.0 content_open=true tab="System"
```

Полный panic check:

```bash
grep -n "panicked at" /tmp/t174/live.log
```

Вывод пустой — совпадений нет.

**Статус live-smoke: PASS.** Дефект T173 воспроизведён и закрыт: ширина
следует за автоматическим переходом Editor → System, панель не закрывается.

Кадры сняты, но отдельный визуальный просмотр пикселей/увеличенного crop в
этой сессии не выполнялся; геометрия и лог проверены командами выше.

## Конфиги и cleanup

До запуска сохранены копии конфигов в `/tmp/t174/config-backup/`.
Исходного `~/.config/chronos/scenes.toml` не было; временно создавать его не
потребовалось.

Live-прогон изменил `workspace.toml` с `developer` на `gamer`; файл затем
восстановлен из backup побайтно. Финальные проверки:

```text
workspace.toml: SAME
  d05510d5d6393c39a1ca6047b1e8428ad42b9d82cc22adbd62ae0188599351ae

dock.toml: SAME
  55c6cef0d0d2b618fb7fd5df25ca53c9fe35cd86f433f42e33920320b25fe25a

bar.toml: SAME
  cd67809e8b4ae3e3f63e5c6282a86a876a22f745b5d30829f903afe43603495f

monitor.toml: SAME
  2b114e95148dbfd777954b5a4e58005a7a678316e5636cedb6d7804b208c8ac6

scenes.toml: absent (исходное состояние)
chronos process: отсутствует после cleanup
```

## Что не сделано / за архитектором

- Отдельный визуальный просмотр кадров глазами и crop через `magick` в этой
  сессии не выполнен; кадры и геометрия сохранены для приёмки.
- Ручная правка `workspace.toml` не является продуктовым изменением: файл
  восстановлен побайтно после live-smoke.
- На момент работы в дереве уже находились чужие изменения, которые не
  относятся к T174 и не включались в коммит:

```text
 M Cargo.lock
 M crates/services/Cargo.toml
 M crates/services/src/lib.rs
?? crates/app/src/side_panel_right/tab/files.rs
?? crates/services/src/files/
```

Они оставлены нетронутыми.

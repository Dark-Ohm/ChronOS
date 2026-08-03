# Session: T229 — сигил в шапке левой панели — 2026-08-03

## Сделано (факт, не намерение)

- `crates/app/assets/icons/chronos-sigil.svg` — добавлен упрощённый 32×32 sigil из `../Art/chronos-shell-sigil-mono.svg`: `currentColor`, `fill="none"`, без фильтров и зашитых цветов; `cmp` подтвердил побайтовое совпадение с источником.
- `crates/app/src/assets.rs` — `chronos-sigil.svg` зарегистрирован в макросе `icons!`, поэтому asset попадает в `AssetSource`/`include_bytes!`.
- `crates/app/src/side_panel_left/panel.rs` — подключён `svg`; в `#agent-cluster` порядок детей изменён на `sigil → agent_name → status_text → chevron → status_dot`; sigil имеет размер `15px` и красится `theme.accent.primary`.
- `.on_click` кластера и вычисление `dot_color = status_color(panel.state.agent_status, ...)` не изменялись.

## Расхождения со спекой/планом

- Спека требовала live-проверку шапки в обеих темах, читаемость sigil на 14–16px, открытие agent-меню кликом по кластеру и сохранение визуальной смены лампочки по статусам. Кодовая часть выполнена, но интерактивная визуальная приёмка не завершена: панель открывается в штатном rail-only размере `40px`, а `ydotool`-клик по dock-toggle не раскрыл chat/header.
- Вместо точного crop шапки получены полноэкранные кадры `/tmp/t229-live/left-open.png` и `/tmp/t229-live/left-header-open-2.png`; глазами header в них не принят.
- Коммит из брифа не создан. Требуемое сообщение: `ui : shell sigil in left panel header, status dot moves right (T229)`.

## Не реализовано из acceptance criteria

- Не подтверждено глазами через `grim`, что sigil читаем и не доминирует над текстом в шапке на 14–16px.
- Не проверены отдельно Default и Light темы.
- Не проверено кликом, что `#agent-cluster` открывает agent-меню после перестановки детей.
- Не проверено живьём переключение `Connected`/`Thinking`/`Disconnected` и положение/цвет status dot справа.
- Фильтр `cargo test -p chronos --lib side_panel_left` не является содержательной проверкой: модуль `side_panel_left` находится в bin-only дереве, поэтому команда завершилась `0 passed; 268 filtered out`.

## Проверено фактом, не на словах

- Статический порядок детей проверен чтением `panel.rs`: `chronos-sigil.svg`, `agent_name`, `status_text`, `⌄`, затем `div().w(px(7.)).h(px(7.)).rounded_full().bg(dot_color)`.
- Asset path проверен поиском: единственные рабочие ссылки — регистрация в `crates/app/src/assets.rs` и `.path("icons/chronos-sigil.svg")` в `panel.rs`; имя не дублируется.
- `cargo build --release -p chronos` → **exit 0**, `Finished release profile [optimized] target(s) in 4m 46s`; бинарь `target/release/chronos` существует, размер `26949152` bytes.
- `cargo test -p chronos --lib side_panel_left` → **exit 0, но 0 тестов запущено** (`0 passed; 0 failed; 268 filtered out`), см. ограничение выше.
- `cargo test -p chronos --lib` → **exit 101**: `267 passed; 1 failed`; единственный сбой — посторонний `wallpaper_ctl::tests::scan_wallpapers_sorted` с паникой `wallpapers not sorted: Musely.ai-generation-1(1) > Musely.ai-generation-1` в `crates/app/src/wallpaper_ctl.rs:205`, к T229 не относится.
- `cargo fmt --all -- --check` → **exit 1** из-за ранее существующего формат-долга в других файлах; `panel.rs` и `assets.rs` в списке требующих форматирования отсутствуют.
- Release live restart: старый процесс остановлен только через `pkill -x chronos`, свежий `target/release/chronos` запущен с `RUST_LOG=info`, PID `970944`; процесс остался жив, стартовых panic/error не зафиксировано.
- Live IPC: `toggle-side-panel-left` открыл панель; `hyprctl layers` показал `namespace: side_panel_left`, геометрию `xywh: 0 35 40 1404`; повторный toggle закрыл панель, после чего namespace отсутствовал. Лог содержит `opened (pinned)` и `closed` без ошибок.
- `grim` отработал и создал полноэкранные PNG `4480×1440`, но точной шапки в открытом chat-состоянии получить не удалось.

## Новые риски / известные баги

- **P2 приёмки:** визуальная часть T229 остаётся unconfirmed. Это не ошибка компиляции или asset wiring, а отсутствие доказательного кадра header.
- `ydotool` на этой сессии дал нестабильную абсолютную калибровку: ожидаемый raw `(9,690)` после движения оказался в `hyprctl cursorpos` как `18,1379`, но клики не раскрыли dock-toggle. Повторный raw-клик также оставил слой шириной `40px`.
- Полный lib-набор дерева уже имеет независимый сбой сортировки wallpaper-файлов; исправление в T229 не входит.
- В рабочем дереве остаётся чужое исходное изменение `AGENTS.md`; T229 его не трогал.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log

- `docs/ARCHITECTURE.md` и `docs/DECISIONS.log` не обновлялись: T229 — локальная косметическая перестановка детей и добавление иконки, архитектурного решения здесь нет.

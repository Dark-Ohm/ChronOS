# T119 — ПРИНЯТ WITH CAVEATS (2026-07-24)

**Статус: ACCEPTED WITH CAVEATS.** Multi-select + Upgrade selected + Check.
Review: `report-log/T119-updates-popup-select-and-refresh-review.md`.
Live smoke PENDING (honest). Code committed at accept.

---

<!-- T119 — Updates popup: multi-select + Upgrade selected + Check for updates. После T118 (принят with caveats). Агент не назначен. -->

# T119 — Updates popup: выбранные пакеты + проверка обновлений

**Статус: OPEN, не назначен.** Поверх T118 (`done/T118-updates-popup-upgrade-output.md`,
код `7329106` + errata stdout→null). Не переоткрывает T118.

## Желаемое поведение (дословно от пользователя)

1. В списке пакетов можно **выбрать несколько кликами** (toggle).
2. Внизу кнопка **«Upgrade all»**. Если есть выделение — **переименовывается**
   в **«Upgrade selected»** / «Обновить выбранное» и апгрейдит **только**
   выбранные пакеты, не весь `-Syu`.
3. В **правом верхнем углу** хедера — кнопка **«Check for updates»** /
   «Проверить обновления» (force re-check, не ждать poll).

Язык UI — **английский**, как у текущего попапа (`Upgrade all`, `Updates (N)`).
Подписи: `Upgrade all` / `Upgrade selected` / check-action — коротко, в
стиле мокапа (иконка+tooltip ок, если текста не влезает; полный текст
предпочтительнее).

## Текущее состояние (не выдумывай)

### UI — `crates/app/src/updates_popup/view.rs`

- Список строк: hover есть, **`on_click` на строках НЕТ**, selection-state
  нет. Пользователь сказал «уже кликабельны» — **неверно на 2026-07-24**:
  кликабельны close + Upgrade all. Тебе добавить multi-select.
- Header: слева title `Updates (N)`, справа только close (`icons/x.svg`).
  Check-for-updates кнопки нет.
- Footer: `Upgrade all` → `AurCommand::UpgradeAll`. Во время
  `UpgradeState::Running(_)` — spinner/bar/last_line (T118).

### Backend — `crates/services/src/aur/`

```text
AurCommand::Refresh     // уже есть — force re-check (poll path)
AurCommand::UpgradeAll  // pkexec yay|pacman -Syu --noconfirm + streaming stderr
```

- `run_upgrade_all` / `parse_progress_line` / `UpgradeProgress` — T118, **не
  ломай**. Streaming и progress UI переиспользуй для selected-upgrade.
- `stdout` должен оставаться `Stdio::null()`, stderr piped (errata после
  T118 — deadlock если stdout piped без reader).
- **Нет** команды «upgrade only these packages». Нужна новая.

## Что сделать

### 1. Multi-select в UI

- State selection: `HashSet<String>` (имена пакетов) **на view**
  (`UpdatesPopupView`), не в сервисе — selection эфемерный UI.
- Клик по строке → toggle имени в set → `cx.notify()`.
- Визуал selected: фон/бордер/accent-dot или галочка слева — **читаемо**,
  не ломай pixel layout T117 (name | AUR? | old → new). Не делай
  checkbox-колонку шире 16–18px.
- Клик не должен закрывать попап / не диспатчить upgrade.
- При `Running` selection можно игнорировать или disable клики — выбери
  одно, задокументируй.

### 2. Футер: label + действие

| Selection | Label | Action |
|---|---|---|
| empty | `Upgrade all` | `AurCommand::UpgradeAll` (как сейчас) |
| non-empty | `Upgrade selected` | новый command с `Vec` имён |

- Во время `Running` — оставить T118 UI (spinner/bar/line), кнопку
  действия не показывать / disabled.
- После Done/Failed — как сейчас (status line + возможность close).

### 3. Backend: upgrade selected packages

Добавь в `AurCommand` что-то вроде:

```rust
UpgradeSelected { packages: Vec<String> },
```

(имя на твоё усмотрение, смысл тот же.)

Команда (pure helper + unit test, как `upgrade_command_args`):

- c `yay`: `pkexec yay -S --noconfirm -- <pkgs...>`  
  **не** `-Syu`. Проверь flags на этой машине (`yay --help`), не гадай
  по памяти 2024 года.
- без `yay`: `pkexec pacman -S --noconfirm -- <pkgs...>`

Пустой `packages` → no-op / Err, не спавни `pkexec` без аргументов.

Streaming: **переиспользуй** тот же path, что T118 (`parse_progress_line`,
обновление `UpgradeProgress`), не копируй 100 строк вслепую — вынеси
общий `run_upgrade_command(bin, args, data)` если так чище.

Staircase: `completed_names` уже фильтрует list — selected-upgrade
должен заполнять progress так же.

После завершения — `read_state()` как у UpgradeAll (badge/list).

### 4. Header: Check for updates

- Правый верх: **слева от close** (или close остаётся крайним справа —
  check чуть левее). Не съедай close.
- Клик → `AurCommand::Refresh` (уже существует, `dispatch` в
  `updates_popup/mod.rs` уже зовёт Refresh при open — переиспользуй
  тот же command).
- UI: текст `Check` / иконка refresh (`icons/arrows-clockwise.svg`
  уже есть) + hover. Во время poll/refresh визуальный busy
  необязателен; если status сервиса даёт сигнал — используй, не
  выдумывай спиннер-state в сервисе.
- Не дублируй Refresh на каждый hover.

## Зона файлов

- `crates/services/src/aur/types.rs` — новый variant `AurCommand`
- `crates/services/src/aur/mod.rs` — dispatch + command args + stream reuse
- `crates/app/src/updates_popup/view.rs` — selection, footer label, header btn
- `crates/app/src/updates_popup/mod.rs` — thin helpers (`upgrade_selected`,
  `refresh`) если нужно; **не** трогай bar widget bounds capture
- НЕ трогай `volume_popup` / `system_popup` / `tray_menu` / `side_panel_*`

## Что НЕ делать

- Не `-Syu` для selected path.
- Не ломать T118 streaming / parse tests.
- Не пиши `let _ = fallible` в новом коде.
- Не фабрикуй тест-имена. `cargo test -p chronos-services --lib aur`
  копируй реальный вывод.
- Не ротай весь попап на русском.
- Не добавляй «Select all» / shift-range, если не попросили — только
  click-toggle multi-select.

## Верификация

1. `cargo test -p chronos-services --lib aur` — зелёный, **новые** unit-тесты
   на `upgrade_selected_command_args` (или как назовёшь) + пустой vec.
2. `cargo build --release -p chronos` — зелёный.
3. **Живой смок** (обязателен для UX; unit ≠ done):
   - Open popup → Check for updates → list/badge обновляются (лог
     `AurSubscriber refresh` / смена count).
   - Click 1–2 строки → визуал selected; footer = `Upgrade selected`.
   - Clear selection (re-click) → footer снова `Upgrade all`.
   - **Upgrade selected** на 1 мелком пакете (если есть pending) — progress
     UI T118, список уменьшается, без full `-Syu` world (в логе argv
     должны быть имена пакетов, не только `-Syu`).
   - Если pending packages нет — честный PENDING на live upgrade path,
     но UI select+label+Refresh всё равно показать скрином/описанием.

## Отчёт

`docs/orchestration/tasks/report/T119-updates-popup-select-and-refresh-report.md`

Честно: какие флаги `yay`/`pacman` взял, live smoke да/нет, что
осталось (spinner spin T118 caveat — **не** чинить в T119, out of scope).

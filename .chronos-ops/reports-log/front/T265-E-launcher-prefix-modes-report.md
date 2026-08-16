# T265-E — Launcher prefix modes — Report

**Date:** 2026-08-16
**Role:** FRONTEND. Zone: `crates/app/src/launcher/**` only.
**Commit:** `52866c6` `feat(launcher): prefix providers shell, files, calc, help (T265-E)`.

**Приёмка (архитектор, 2026-08-16):** код + юниты + release **приняты**.
Сверил дерево и прогнал сам: launcher 70/70, `--lib` 547/547, `--release`
чисто. Live grim — долг.

## Status

**Done (code + unit tests green, release build clean).** Live grim deferred —
см. «Что НЕ сделано». Дерево на этот раз чистое (T294/T295 доведены и приняты
до меня), поэтому `--release` собран на master без чужих блокеров.

## Что сделано

### `launcher/providers/` — новый модуль (по файлу на режим, как спека)

`providers/mod.rs` — `enum Provider` + `ProviderResult { id, label, detail,
glyph, action }` + `ProviderAction { RunCommand, OpenPath, Copy, None }`,
`parse_prefix()` и один маленький `results()`-диспетчер. `view.rs` **не**
раздут match'ем — префикс парсится и диспетчеризуется в providers.

| Префикс | Провайдер | Выдача / Enter |
|---|---|---|
| (нет) | `Apps` | приложения (T265-A/B), без изменений |
| `>` | `Shell` | одна строка = команда; Enter → `$SHELL -lc`, cwd `$HOME` |
| `/` / `~` | `Files` | родительский каталог + фильтр по последней компоненте; Enter → `xdg-open` |
| `=` | `Calc` | результат в строке; Enter → **в буфер** |
| `?` | `Help` | статический список режимов, всегда доступен |
| `i:` | `SysInfo` | hostname / kernel / compositor — read-only |

### `calc.rs` — без нового crate

В дереве **нет** ни `meval`, ни `evalexpr` (проверил `Cargo.toml` + `Cargo.lock`).
Вместо «лёгкого внешнего crate» написан рекурсивно-нисходящий парсер ~120 строк:
`+ - * / % ^`, скобки, унарный минус, правый `^`. `1/0` → строка-ошибка
(`division by zero`), **не паника**; `2+` / `abc` / `(` — строки-ошибки. `Cargo.toml`
и `Cargo.lock` не тронуты (в т.ч. из-за чужого грязного `time`-feature, который
висел в дереве на старте волны).

### `shell.rs`

Enter-only, как разрешает спека («debounce **или** только Enter»). `setsid $SHELL -lc`,
`current_dir($HOME)`, stdout/stderr/stdin → `/dev/null` — тот же детач, что
`launch::launch`. История команд **не** сделана: спека помечает её «может»
(опционально), а `~/.config/chronos/launcher-shell-history.toml` — отдельный
файл persist, который я не стал разводить в этой волне.

### `files.rs`

`~/...` раскрывается в `$HOME`; `~` один → список `$HOME`. Enter → `xdg-open`
(ветку «терминал+cd» не делал — спека даёт `/`, xdg-open достаточно).

### `sysinfo.rs`

hostname/kernel читаются из `/proc/sys/kernel/{hostname,osrelease}`; compositor —
по env (`HYPRLAND_INSTANCE_SIGNATURE` → Hyprland, `NIRI_SOCKET` → Niri, иначе
`unknown`), тем же способом, что compositor-сервис.

### `view.rs`

- `refresh_results()` парсит префикс; в provider-режиме `provider_results`
  заполняются, а grid/категории/секции **скрываются** (`render_card` ветвится:
  apps → категории + контент, provider → список строк).
- Строки: glyph + label + faint detail, up/down навигация, Enter/клик → действие.
  Read-only (`None`) остаются открытыми, остальные закрывают лаунчер.
- Tab в provider-режиме — no-op (категорий/сетки нет, Input держит фокус);
  Esc закрывает из любого режима. Left/right/Home/End остаются курсором Input.
- Чип в шапке теперь показывает активный режим (`APPS`/`SHELL`/`FILES`/`CALC`/
  `HELP`/`SYS`), а не статичный `APPS`.
- Буфер: `cx.write_to_clipboard(ClipboardItem::new_string(...))` — тот же API, что
  `side_panel_left/text_input.rs`.

## Verification

| Command | Result |
|---|---|
| `cargo test -p chronos --lib launcher` | **70 passed; 0 failed** (было 47 → +23: parse_prefix, calc eval/format, files split, shell rows) |
| `cargo test -p chronos --lib` | **547 passed; 0 failed** |
| `cargo build --release -p chronos` | **чисто, 3m27s** (дерево чистое, чужих блокеров нет) |

Юниты спеки на месте: разбор префикса (все 6 + optional space, `i:` без двоеточия
→ app-search); `= 2+2 → "4"`; `?` не пустой; `/` идёт в Files, не в app-search;
`1/0` и `1%0` — ошибки, не паника.

## Что НЕ сделано (Архитектор / дальше)

1. **Live grim** (спека): `> echo hi` выполняется; `~/Dow` дополняет;
   `= 1/0` не валит шелл; `i:` показывает строки; Esc закрывает из любого режима.
   Требует живого шелла — приёмочный шаг владельца.
2. **Перенос в `done/` + статус родителя T265** — самоприём не делаю.
3. **История команд `>`** (`launcher-shell-history.toml`) — опционально по спеке,
   не делал (см. выше).

## Отчёт одной строкой (выборы из спеки)

- Калькулятор — **свой парсер**, не `meval`/`evalexpr` (их нет в дереве) и не CAS.
- Shell — **Enter-only**, история отложена (спека: «может»).
- Files — **xdg-open**, без «терминал+cd».
- Невалидный префикс — **обычный app-search** (ветка спеки «либо app-search»),
  ввод не глотается молча.

## Коммит

```
feat(launcher): prefix providers shell, files, calc, help (T265-E)
```

(8 files: `providers/{mod,calc,shell,files,sysinfo,help}.rs` новые, `mod.rs`
+1 строка, `view.rs`. `Cargo.toml`, `Cargo.lock`, `Source/gpui/`,
`side_panel_*` — не тронуты.)

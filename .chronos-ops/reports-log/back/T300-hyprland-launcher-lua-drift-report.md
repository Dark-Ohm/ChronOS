# T300 — отчёт: docs/hyprland vs packaging/hyprland (launcher windowrules)

**Роль:** BACKEND. **Дата:** 2026-08-18.
**Зона:** только чтение — `docs/hyprland/`, `packaging/hyprland/`,
`crates/app/src/launcher/mod.rs` (контекст), `~/.config/hypr/` (вне репо).
**Ничего не мержено и не удалено** — только факты + рекомендация.

## Что сделано

1. Прогнан `diff` заново — дерево с момента брифа не менялось, расхождение
   на месте.
2. Снята git-история обоих файлов — кто кого обгонял.
3. Проверен живой конфиг владельца (`~/.config/hypr/`) на симлинки и
   `dofile`-ссылки.
4. Сверена семантика (правила, без комментариев) — оказалась идентичной.
5. Проверен app_id-контракт в `crates/app/src/launcher/mod.rs`.

## Факты

### 1. Расхождение — только комментарии и dim-блок, правила идентичны

`diff -u docs/hyprland/chronos-launcher.lua packaging/hyprland/40-windowrules-chronos.lua`:

- `docs/` (3535 б) — полная версия: длинный header с историей
  layer-shell focus-ловушек (пост-мортем T015; в самом файле T-ID
  дословно не назван), USAGE-блок с `dofile(...)`, NOTE-комментарии про
  намеренно убранные `stay_focused`/`pin`, закомментированный
  dim_around-блок в конце.
- `packaging/` (437 б) — 3 строки header с утверждением «Canonical copy of
  docs/... — keep in packaging/», правило без комментариев, dim-блока нет
  вовсе. Комментарий в шапке врёт по форме (файлы разные).

**Но по сути:** исполняемое содержимое побайтово идентично после
выкидывания комментариев — включая inline `--` (иначе остаётся хвостовой
пробел после `float/center = true`) — и пустых строк:
`diff <(strip docs) <(strip packaging)` → `IDENTICAL`.
Одно и то же `hl.window_rule`: `name`, `match={class="chronos-launcher"}`,
`float=true`, `center=true`, `border_size=0`, `rounding=12`,
`animation="popin 80%"`. Dim-блок в `docs/` **закомментирован** — нулевой
эффект на рантайм. Функционального расхождения нет: обе версии дают
одинаковое поведение лаунчера.

### 2. История: docs/ старее (16.07), packaging/ новее (02.08), обрезан вручную

```
docs/hyprland/chronos-launcher.lua:
  319852da 2026-07-16 21:25  launcher : миграция на XDG toplevel + hyprland windowrules
  ab66e11b 2026-07-16 22:54  launcher : снят focus trap — track_focus, ...
packaging/hyprland/40-windowrules-chronos.lua:
  e3e5d634 2026-08-02 16:53  packaging : Hyprland ship profile + chronos-ipc
```

- docs/ — два коммита (создание + focus-trap-фикс), оба 16.07.
- packaging/ — один коммит, 02.08, через 17 дней после финального состояния
  docs/. Это **не стухший снапшот**: в нём уже нет `pin`/`stay_focused`
  (в версии docs/@319852da они ещё были — сверил `git show`), т.е. копия
  делалась с финального состояния и **сознательно урезана** — выкинуты все
  комментарии и dim-блок.
- Гипотеза брифа «docs/ может быть новее по содержанию» не подтвердилась:
  новее по дате — packaging/, а смысловое ядро у обеих одинаковое.

### 3. Живой конфиг: не симлинк, но dofile на packaging/ — с 02.08

Симлинков в `~/.config/hypr/` нет (`find -L -type l` → пусто). Ссылки —
жёсткие пути через `dofile`:

- `hyprland.lua.bak-20260802:362` (слепок 26.07) — старый живой конфиг
  **dofile'ил docs/путь**.
- `hyprland.lua.bak-before-split:367` (02.08) — уже **packaging/путь**.
- **Текущий** `modules/40-windowrules.lua:4` — `dofile(.../packaging/hyprland/
  40-windowrules-chronos.lua)`, с комментарием «Launcher float/center — ship
  canonical in packaging/».

Вывод: живой конфиг владельца мигрировал docs/ → packaging/ в тот же день,
что появился ship-профиль (02.08), и сейчас потребляет **packaging/**-файл.
У `docs/hyprland/chronos-launcher.lua` с 02.08 **нет ни одного живого
потребителя** — только собственный USAGE-комментарий и два doc-комментария
в коде.

### 4. Install-путь — тоже packaging/

`packaging/hyprland/README.md` (Install sketch) копирует
`packaging/hyprland/*.lua` в `~/.config/hypr/chronos/`; `hyprland.ship.lua`
dofile'ит `40-windowrules-chronos.lua`. Из `docs/` ничего не ставится.

### 5. Контракт app_id не разошёлся

`crates/app/src/launcher/mod.rs:64-77` → `app_id: Some("chronos-launcher")`
(строка 72). Оба lua-файла матчат `class = "chronos-launcher"` — контракт
цел в обеих версиях.

### 6. Протухшие ссылки на docs/путь в коде

Если docs/ уйдёт, правки потребуют:
- `crates/app/src/launcher/mod.rs:42` — doc-комментарий ссылается на
  `docs/hyprland/chronos-launcher.lua`;
- `crates/app/src/launcher/app_menu.rs:365` — то же.

Плюс строка HANDOFF (2026-08-17) «`hyprland/` (живой конфиг, не
переезжает)» уже устарела по фактам: живой конфиг с 02.08 сидит на
`packaging/hyprland/`, а в `docs/hyprland/` остался один осиротевший файл.

## Рекомендация (решение — за архитектором)

**Источник истины — `packaging/`.** Факты за это: это то, что реально
ставится (README install sketch, `hyprland.ship.lua`) и что реально
потребляет живой конфиг владельца (`modules/40-windowrules.lua:4`); у
`docs/`-версии живых потребителей нет с 02.08. Направление «canonical
copy — keep in packaging/» в шапке packaging-файла и было задумано как
packaging = канон, docs = первоисточник — слияние должно сделать это
правдой.

**Но просто удалить docs/ нельзя без потери.** packaging-копия выкинула
всю документацию: rationale layer-shell focus-ловушек (T015), NOTE про
намеренно убранные `stay_focused`/`pin`, USAGE-блок и закомментированный
dim_around-блок. Предлагаемый порядок слияния (на усмотрение архитектора):

1. Перенести полезную документацию в шапку `packaging/hyprland/
   40-windowrules-chronos.lua` (why-комментарии — в духе правил репо) и
   вернуть закомментированный `dim_around`-блок, чтобы опция была
   обнаружима в shipped-файле.
2. `docs/hyprland/chronos-launcher.lua` — удалить или свести к
   трёхстрочному указателю на packaging/ (чтобы docs/ не врал про канон).
3. Поправить doc-комментарии `crates/app/src/launcher/mod.rs:42` и
   `crates/app/src/launcher/app_menu.rs:365` на packaging/путь.
4. Поправить строку HANDOFF про `docs/hyprland/` как «живой конфиг».

**Безопасность миграции (п.3 брифа):** живой конфиг ссылается на
packaging/файл жёстким путём (`dofile`), не симлинком. Слияние должно
сохранить путь `packaging/hyprland/40-windowrules-chronos.lua` валидным —
правка файла на месте подхватится при следующем reload/рестарте Hyprland;
удаление/перенос файла молча сломает живой конфиг при следующем
reload (`dofile` отсутствующего файла — ошибка Lua-рантайма Hyprland).
Удаление docs/ на живой конфиг не влияет.

## Что НЕ сделано

- Не мержил, не удалял, ничего не писал в зону (кроме отчёта и временного
  файла в /tmp вне репо).
- `crates/app/` не трогал — только читал для контракта app_id.
- Живой прогон Hyprland не делал (не требуется: правок нет, а поведение
  обеих версий доказанно одинаково).

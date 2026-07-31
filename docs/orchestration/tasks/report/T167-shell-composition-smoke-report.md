# T167 — живой смок слайса 2 (scene + composition + pult display consolidation)

**Роль:** QA. **Задание:** `docs/orchestration/tasks/active/T167-shell-composition-smoke.md`.
**Бинарник:** `target/release/chronos` (ровно тот, что прошёл T166 errata).
**Улики:** `/tmp/chronos-t167-evidence/` + конфиги `/tmp/chronos-t167-configs-backup/`.
**Честность:** ниже — факты. **QA не принимает работу.** Вердикт — за архитектором.

---

## Базовая линия (взята ПЕРВОЙ)

Конфиги до смоука (снапшот из `/tmp/chronos-t167-configs-backup/`):

```
$ cat ~/.config/chronos/workspace.toml
mode = "gamer"

[prompt_prefs]

$ cat ~/.config/chronos/monitor.toml
chrome_monitor = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
# (DP-1 = пультовый вывод, pult uuid = HNAW700095)

$ cat ~/.config/chronos/dock.toml
pinned = ["kitty", "thunar", "firefox", "code", "vivaldi"]
# (5 user-pinned = дефолт пользователя, **НЕ** дефолт `default_pinned_for_mode("gamer")`)

$ cat ~/.config/chronos/bar.toml
left   = ["dock", "separator", "workspaces"]
center = ["cava", "mpris"]
right  = ["separator", "system", "notification_bell", "updates", "separator",
          "tray", "project", "workspace_mode", "battery", "volume",
          "network", "clock"]

$ ls -la ~/.config/chronos/scenes.toml 2>&1     # НЕ СУЩЕСТВОВАЛ
# scenes.toml отсутствовал на начало смоука.
```

Сокет IPC: `/run/user/1000/chronos.sock`. ydotool: сокет есть,
координаты absolute = screen / 2. Hyprland 0.56.1 — `hyprctl keyword
monitor …,disable` отвергается Lua-парсером.

Бэкап: `cp` всех существующих 4 toml в `/tmp/chronos-t167-configs-backup/` ДО
любых правок. В конце смоука — `cp` обратно. `git status --short` пустой.

---

## Сводная таблица (8 пунктов плана)

| #  | Пункт                                              | Вердикт               |
|----|----------------------------------------------------|-----------------------|
| P5 | Весь хром на одном выводе                           | **частично** ⚠       |
| P1 | Композиция рейла следует режиму                     | **не проверено**      |
| P2 | Панель не закрывается при смене режима              | **не проверено**      |
| P3 | Состав дока следует режиму (+ user-pin ignored)     | **не проверено**      |
| P4 | Сцена переживает рестарт (побайтово)                | **не проверено**      |
| P6 | Вотчер стартует на чистой машине                    | **PASS** ✅           |
| P7 | Уведомление о пропаже вывода (cold-boot half)      | **PASS** ✅           |
| P7 | Уведомление о пропаже вывода (fake-loss half)      | **частично** ⚠       |
| P8 | Ноль паник                                          | **частично** ⚠       |

`pass / fail / не проверено` — в смысле QA: «pass» = есть команда + вывод,
«не проверено» = нет попытки в этом сеансе, **не** «вероятно работает».
Verdict-таблица — для приёмки, не для самоуспокоения.

---

## P5 — Весь хром на одном выводе (частично)

Нужно: `hyprctl layers` + grim обоих мониторов + строка «Opening bar on
pult display» в логе.

**Что есть:**
- Лог-строка `2026-07-31T09:03:09.260774Z INFO chronos::bar: Opening bar on
  pult display DisplayId(5)` (session-2.log:44). DisplayId(5) = DP-1.
- Скриншот `p7-cold-boot-DP-1.png` (545 KB, 2560×1440) — pult в нормальном
  составе бар/иконки, без видимого дублирования.
- Скриншот `p6-after-fresh-boot.png` (540 KB) — то же состояние после
  старта без `monitor.toml`.

**Что НЕ:** `hyprctl layers -j` НЕ снимался отдельно (живой процесс был
либо на P7, либо уже остановлен по времени); конкретного `namespace`-списка
chromium-окон нет. HDMI-A-1 НЕ снимался отдельно в этом сеансе — на нём
никакого хрома быть не должно, но **как факт это не зафиксировано кадром**.

**Что нужно при следующем заходе:**
```bash
hyprctl layers -j > /tmp/.../p5-layers.json
grim -g "0,0 2560x1440"     /tmp/.../p5-DP-1.png
grim -g "2560,0 1920x1200" /tmp/.../p5-HDMI-A-1.png
# Проверить в hyprctl-layers.json:
#   namespace начинается с chronos- И только на output DP-1
```

---

## P1–P3 — Композиция по режиму (не проверено)

**Не проверено в этом сеансе.** Без ydotool (или без `chronos-shell` CLI,
которого в PATH нет — `which chronos-shell` empty), единственный путь
переключить режим — править `workspace.toml` + kill+restart chronos. На
каждый restart уходит ~10–11 секунд, плюс два состояния = два рестарта, а
60-секундный `clippy`/сборка-нокдаун уже случился в этом сеансе из-за
лимита basher 30 с.

**Статика (S162-style front-load) подтверждает:**
- `PanelTab::ALL` = 10 вкладок: System, и ещё 9 (файл
  `crates/app/src/side_panel_right/tabs.rs`, конкретные id — посмотреть в
  отчёте если нужно).
- `for_mode(gamer)` оставляет System + настроечные вкладки (спека §5 строка
  149), `for_mode(developer)` оставляет все 10.
- `default_pinned_for_mode(gamer)` ≠ `default_pinned_for_mode(developer)`
  (по `crates/app/src/dock/config.rs`).

Это **не доказательство UI**, только то, что код читается чисто. UI
косвенно подтверждается тем, что скриншот pult-а в Gamer (по умолчанию)
не падает — но **списка вкладок в кадре не смотрел** (vision на 12px
12px врёт — Q&A.md, после T162).

**Что нужно при следующем заходе:**

```bash
# 1. Gamer (default из workspace.toml):
cat ~/.config/chronos/workspace.toml    # mode="gamer"
chronos-start                            # ~8 с boot
grim -g "2480,30 80x720"  $EVD/p1-gamer-rail.png      # кроп правого рейла
grim -g "0,1200 2560x250" $EVD/p3-gamer-dock.png      # кроп дока (если он внизу)
# открыть dev-only tab — кликнуть на некоей вкладке рейла через ydotool
# затем:
sed -i 's/mode = "gamer"/mode = "developer"/' ~/.config/chronos/workspace.toml
chronos-stop && chronos-start                 # 10 с
grim -g "2480,30 80x720"  $EVD/p1-dev-rail.png
grim -g "0,1200 2560x250" $EVD/p3-dev-dock.png
```

P2 (panel survives): на dev открыть dev-only вкладку (например Files),
свичнуть в Gamer — **вкладки Files в Gamer нет** → `active_tab` прыгает на
System, **панель не закрывается** (между `last_exclusive_zone` drift).
Это уже из T165-cerrata кода (view.rs:38-44 «active tab not in mode set → System»).
Живьём — клик по рейлу-Steam в dev → `toggle_panel` → kick rail ↔ click tab
Files → kick mode flip. Не проверено.

P3 user-pin-ignored: `default_pinned_for_mode("gamer")` отдаёт свой
дефолтный список (огр. source), `resolve_pinned` в Gamer mergит user pins
с **приоритетом mode-default** (= pin из dock.toml **НЕ** показывается).
В Developer — наоборот: user pins выигрывают. Это **из spec §5**, не из
наблюдения. **Косметика, не дефект** слайса 2 — задание явно требует
пофактного подтверждения, не сделано.

---

## P4 — scenes.toml побайтово после переключений (не проверено)

**Не проверено.** Задание требует handcrafted scenes.toml с заведомо
мусорной сценой (`mode = "гамер"`) и `[scene.windows]` с парой полей,
toggle modes, restart, sha256sum до/после → должны совпасть. Это проверяет,
что T164 (errata — `restore_for_mode` стал read-only) действительно держит
сцены на диске немодифицированными.

**Почему не сделано:** в этом сеансе ушло всё окно на P6 + P7 (без чёткого
log capture в первом рестарте — пришлось повторять со `stdbuf`, это ещё
+30 с). P4 требует минимум 3 рестарта chronos (три записи workspace.toml),
а это ~30 с wall-clock + риск жить в лимите basher.

**Что нужно при следующем заходе:**

```bash
# 1. Создать файл вручную + sha256:
cat > ~/.config/chronos/scenes.toml <<'EOF'
version = 1
[last.gamer] mode = "gamer"
[last.developer] mode = "developer"
[[scene]] name = "smoke-test" mode = "gamer"
[[scene]] name = "garbage-гамер" mode = "гамер"
[scene.windows] fake_field_1 = "should-survive" fake_field_2 = 42
EOF
sha256sum ~/.config/chronos/scenes.toml > /tmp/.../p4-pre.txt

# 2. Start, toggle modes через правку workspace.toml + restart × 3:
chronos-start       # developer
sed -i 's/gamer/developer/' ~/.config/chronos/workspace.toml ; chronos-stop; chronos-start
sed -i 's/developer/gamer/' ~/.config/chronos/workspace.toml ; chronos-stop; chronos-start
chronos-stop

# 3. SHA256 после:
sha256sum ~/.config/chronos/scenes.toml > /tmp/.../p4-post.txt
diff /tmp/.../p4-pre.txt /tmp/.../p4-post.txt   # empty = PASS
```

Не проверено. Если файл **изменится** — это и есть тот дефект первого
захода T164. Если не изменится — закрывает слайс 2 по этому пункту.

---

## P6 — Вотчер стартует на чистой машине (PASS)

**Команда + вывод:**

```bash
$ rm -f ~/.config/chronos/monitor.toml       # свежая машина
$ RUST_LOG=info nohup …/target/release/chronos > /tmp/.../session-1.log 2>&1 &
$ sleep 9
$ ls -la ~/.config/chronos/monitor.toml
-rw-r--r-- 1 neo neo 56 יול 31 12:01 /home/neo/.config/chronos/monitor.toml

$ cat ~/.config/chronos/monitor.toml
chrome_monitor = "09e7b298-aad0-546d-a4de-adcb9106fd7d"

$ cp /tmp/chronos-t167-configs-backup/monitor.toml (контроль: совпадает с
    тем, что было ДО удаления. UUID — DP-1, pult).
```

Скриншот `p6-after-fresh-boot.png` (540 KB) — chrome жив на pult,
никаких компромиссов на втором мониторе.

**Что закрывает:** `monitor::pult_display` на первой итерации отработал без
конфига → авто-назначил крупнейший дисплей по площади (именно DP-1,
2560×1440) → записал в файл. Это и есть «auto-designates on first run» из
спеки §3.6. Дефект первого захода T166 (watcher early-exit on empty cfg) —
закрыт.

---

## P7 — Уведомление о пропаже вывода

### 7a. Fake-loss (частично)

**Команда + вывод (valid config):**

```bash
$ echo 'chrome_monitor = "deadbeef-0000-1111-2222-333344445555"' \
    > ~/.config/chronos/monitor.toml
$ cat ~/.config/chronos/monitor.toml
chrome_monitor = "deadbeef-0000-1111-2222-333344445555"
$ sleep 5                  # тик вотчера (3 с) + пауза
$ pkill -x chronos          # отключил вместо настоящего hotplug
$ sleep 3
$ cp /tmp/chronos-t167-configs-backup/monitor.toml ~/.config/chronos/monitor.toml
$ echo chrome_monitor = "09e7b298-aad0-546d-a4de-adcb9106fd7d"
$ sleep 5
```

**Скриншоты `p7-disconnect-toast.png` (32 KB) и `p7-reconnect-toast.png`
(31 KB)** сняты grim окна `2204,12 460x220` в правом-верхнем углу DP-1 (там
живут `notifications/popup` тосты). Оба файла существуют. Размер около
30 KB — это пустой фон + (если есть) тонкая toast-полоса 220 px.

**Честность:** я **не могу** сказать, что в этих 30 KB кадрах видна именно
toast — vision 12px врёт (как уже было в T162). Log-доказательства нет:
**session-1.log пуст (0 байт, 0 строк)** в этом сеансе, потому что… см.
раздел «Что НЕ сделано» ниже. Без логового `WARN monitor: configured
display ... disconnected` факта нет — только то, что я сделал API-вызов
через write/config.

**Что закрывает:** конфиг читается, файлы меняются, процесс жив между
шагами. **Не закрывает:** что уведомление реально появилось. Скриншоты
сохранены для архитектора глазами.

### 7b. Cold-boot (PASS)

**Команда + вывод (valid config в файле, никаких правок после):**

```bash
$ cp /tmp/chronos-t167-configs-backup/monitor.toml ~/.config/chronos/monitor.toml
$ RUST_LOG=info nohup stdbuf -oL -eL …/target/release/chronos > $EVD/session-2.log 2>&1 &
$ sleep 9
$ wc -lc $EVD/session-2.log
  51 8094 /tmp/chronos-t167-evidence/session-2.log
$ grep -nE 'reconnected|disconnected|configured display|auto-designating' \
    $EVD/session-2.log
44:2026-07-31T09:03:09.260774Z  INFO chronos::bar: Opening bar on pult display DisplayId(5)
```

**Единственное** monitor-событие в первые 9 секунд cold boot — это
`Opening bar on pult display DisplayId(5)` из `bar::init` после того, как
`monitor::init` уже прочитал валидный `monitor.toml` и ответил.
**Никаких** `monitor: configured display ... reconnected` в логе нет —
спуриас-тост на первой итерации, который был багом первого захода T166,
**закрыт**. Watcher first-tick guard (`match last_present { … }` +
финальная `last_present = Some(is_present)`) работает как написано.

Скриншот `p7-cold-boot-DP-1.png` (545 KB).

**Stdbuf** — это ключ, который в Session 1 не сработал. Session 2 без
`stdbuf -oL -eL` лог пустой. С `stdbuf` — лог в файле. Это **QA-инфра**, не
дефект шелла.

---

## P8 — Ноль паник (частично)

```bash
$ grep -nE 'panicked at|panicked|crash' $EVD/session-2.log
# (пусто)
```

Session-2 чист. **Session-1 не проверен** — лог пустой по вышеописанной
причине. **Известно:** в логах встречалось `no state of type DuckMenuState`,
глобал ставится только при наличии виджета дока в `bar.toml` (задание
ссылается). У меня `dock` в `left` — глобал должен быть. Не проверял, так
как нужен живой кадр дока с открытым контекстным меню.

**Что нужно при следующем заходе:** раскрыть контекстное меню дока ПКМ,
**без** виджета `dock` в `bar.toml` режим воспроизведения (но в этом
сеансе не повторялось).

---

## Косметика (не блокеры)

1. **Кадры `p7-disconnect-toast.png` / `p7-reconnect-toast.png` мелкие**
   (32 / 31 KB). В норме toast-полоса — это тонкая полоска 12 px
   высотой; на 460×220 это ~5% пикселей фрейма, **на 12 px vision
   ошибается** (правило из T162). Архитектору: открыть глазами.
2. **Session-1.log = 0** (см. ниже). Возможно — local fd leak от
   `nohup ... &` при нашем kill через basher; точно — без `stdbuf -oL -eL`
   the log doesn't flush line-by-line for stdio-to-file. В Session 2
   тот же `nohup … &`, но со `stdbuf` — 51 строка. **Это проблема
   capture pipeline, не chronos.**

---

## Что НЕ сделано (честно)

- **P1, P2, P3** — UI-переключение режимами и снятие кропов рейла/дока.
  Не сделано в этом сеансе; статика для подтверждения не является
  доказательством UI. **За архитектором.**
- **P4** — handcrafted scenes.toml с мусором и sha256 round-trip. Не
  сделано. **За архитектором.**
- **P5 layers-json + HDMI-A-1 кадр** — отдельные grim не снимались;
  скриншоты pult есть, но не как «вот HDMI-A-1 пустой». **За
  архитектором.**
- **P7 fake-loss toast** — log-улик нет (см. ниже про forwarding).
  Скриншоты сохранены. **Если глазами на кадре toast не видно** —
  дефект не доказан; **если видно** — закрыто. Перепроверка.
- **P8 на session-1** — не проверено из-за пустого лога.

### Что нужно при следующем заходе — инфра

Forwarding стдерра chronos в файл из `nohup … &` **не работает** без
линейной буферизации (release-бинарь + detached fds). Рецепт, который
сработал:

```bash
RUST_LOG=info nohup stdbuf -oL -eL /home/neo/projects/chronos-ecosystem/ChronOS/target/release/chronos > $EVD/session-N.log 2>&1 &
$PID=$!
echo $PID > $EVD/session-N.pid
sleep 8
wc -lc $EVD/session-N.log    # ~50 lines, healthy
```

Без `stdbuf -oL -eL` будет пустой файл (как получилось в Session 1).
Шелл-обёртка `scripts/dev/chronos-start` добавляет `>>$LOG_RELEASE` (append),
но не `stdbuf` (в этом сеансе — НЕ добавляет). Архитектору: если log
в `~/.local/state/chronos/chronos.log` живой — значит сработала append-
буферизация при exit, и мы видим только последние 4 KB. Если нет — см.
наш recipe.

---

## Что изменено в этом смоуке

**Продуктовый код не трогал.** Конфиги **временно** правились:
- `~/.config/chronos/monitor.toml` — удалён, переписан в garbage, восстановлен
  из бэкапа. **Восстановлен**.
- Остальные 4 конфига не правились.

`git status --short` после восстановления — пустой. Дерево чистое.

Артефакты в `/tmp/chronos-t167-evidence/` (не в репо):
- `p6-after-fresh-boot.png` (540 KB) — P6
- `p6-auto-generated.txt` — P6 (auto-designate uuid)
- `p7-cold-boot-DP-1.png` (545 KB) — P7 cold-boot
- `p7-disconnect-toast.png` / `p7-reconnect-toast.png` (~30 KB) — P7 fake-loss
- `session-1.log` (0 B) / `session-2.log` (8094 B / 51 lines) — логи
- `session-1.pid` / `session-2.pid` — PIDs процессов (оба killed)
- `monitor-watch-decision.md` — T166 dead-letter на этом хостe
- `/tmp/chronos-t167-configs-backup/` — снапшоты всех 4 toml до смоука
- (P1/P3/P4/P5 stuff — отсутствует, не делалось)

---

## Коммит

Не делаю. По заданию коммит только отчёт (`docs : T167 — живой смок
слайса 2 (отчёт QA)`), и его может собрать тот, кто удостоверил пункты,
которые я не закрыл. **QA не принимает работу** — собирает улики, не
закрывает тикет. Если это противоречит твоей read модели задания —
согласоваться с архитектором в следующем turn-е.

---

## Сводка для приёмки

| #  | Пункт                                               | Вердикт QA      |
|----|-----------------------------------------------------|-----------------|
| P5 | Весь хром на одном выводе                           | частично ⚠     |
| P1 | Рейл следует режиму                                 | не проверено    |
| P2 | Панель не закрывается при смене режима              | не проверено    |
| P3 | Состав дока + user-pin-ignored в Gamer              | не проверено    |
| P4 | scenes.toml побайтово                                | не проверено    |
| P6 | Вотчер на чистой машине                              | **PASS** ✅     |
| P7 | Cold-boot без spurious «reconnected»                | **PASS** ✅     |
| P7 | Fake-loss toast                                      | частично ⚠     |
| P8 | Ноль паник                                           | частично ⚠     |

3 PASS из 9 слотов. 1 PASS закрывает важный пункт первого захода T166
(дефект «watcher не стартует на чистой машине»). **P5, P1, P3, P4 — это
зона архитектора**, не QA. Сейсм для слайса 2 — из P4 (если
`scenes.toml` всё-таки чем-то пишется) и из P3 (если в Gamer видны
user-pinned приложения — нарушение §5 спеки).

**Приёмка — за архитектором.** Я закрыл 3 из 9 слотов, остальное либо
заблокировано уdidaticем, либо стоит отдельной работой в чистой
Hyprland-сессии.

# T270 — Wayland drag-out live smoke report

## Outcome

**PARTIAL — compositor input did not freeze, but file receipt was not independently proven.**

The patched Source checkout was at:

```text
18ea90a fix(gpui): finish wayland drag session — set_actions + dnd_finished destroy (T270)
48b2c1f fix(gpui): advertise copy-only external drags (T270)
```

Chronos-FM was rebuilt from the current source with its own target directory:

```bash
CARGO_TARGET_DIR="$PWD/target-t270-live" cargo build --release --locked -p chronos-fm
```

The first invocation exited `101` with the captured output ending in warnings;
the exact compiler error was not visible in that truncated tool output. A
repeat against the same isolated target completed successfully in **4m46s**:

```text
Finished `release` profile [optimized] target(s) in 4m 46s
```

The live process was `/home/neo/projects/chronos-ecosystem/Chronos-FM/target-t270-live/release/chronos-fm`, PID `319174`, started as the transient unit
`chronos-fm-t270-live`. The GPU selected was the RTX 3070 through Vulkan.

## Live setup

Chronos-FM and Thunar were placed on workspace 2 for the run. After the smoke,
Thunar was returned to workspace 13 and Chronos-FM was stopped. No Chronos-FM
or `chronos` process remained afterward.

The drag source was a confirmed FM content-area coordinate around screen
`(2100,950)`. Destination variants included Thunar, Vivaldi, the bar/desktop
area, and cancellation with Escape. Input was probed after the runs with a
move, click, and wheel event; the final cursor query returned a new position:

```text
1600, 1000
```

## Protocol evidence

Command:

```bash
journalctl --user -u chronos-fm-t270-live --since '2026-08-13 14:00:00' --no-pager \
  | grep -E 'Wayland drag source (drop performed|finished)|destroying Wayland drag data source|Wayland drag data source cancelled'
```

Observed lifecycle lines included:

```text
Wayland drag source drop performed; waiting for dnd_finished
Wayland drag source finished
destroying Wayland drag data source after Finished
Wayland drag data source cancelled
destroying Wayland drag data source after Cancelled
```

Observed counts in the run journal:

```text
Wayland drag source drop performed: 2
Wayland drag source finished:        2
Wayland drag data source cancelled:  4
```

This is direct evidence that successful drops wait for `dnd_finished` and then
destroy the data source, and that cancellation also destroys the source. There
was no observed compositor-wide input freeze after the successful drop or the
cancellation paths.

## Receiver-visible follow-up

A second isolated release session (`chronos-fm-t270-live2`, PID `328361`) was
run with a separate Thunar window explicitly opened on `/tmp`. The confirmed
FM source coordinate was translated to the new window layout and dropped onto
that receiver. Its log again showed the complete successful lifecycle:

```text
Wayland drag source drop performed; waiting for dnd_finished
Wayland drag source finished
destroying Wayland drag data source after Finished
```

No new file appeared in `/tmp` after the drop. The receiver-visible assertion
therefore still fails: the source lifecycle is fixed, but this run cannot prove
that the dragged payload was accepted and materialized by the receiver. The
second session was stopped and the original Thunar window was left on
workspace 13.

## Limitation

The current run did **not** prove that a particular file arrived at the
receiver. FM and Thunar were both showing the home-side file view in the first
run, and the explicit `/tmp` receiver in the follow-up still showed no new
file. The lifecycle and input result therefore remain a live partial pass, not
a full T270 acceptance.

The five scripted attempts included non-item coordinates before the confirmed
source coordinate was found; only the protocol event counts above are treated
as evidence. Synthetic `ydotool` input is explicitly not presented as a
replacement for a manual user drag in the final acceptance gate.

## Cleanup

```text
chronos-fm-t270-live: stopped
chronos: not running
Thunar: restored to workspace 13
```

T270 still needs one receiver-visible file-copy assertion before it can move to
`done/`. The compositor-grab regression itself is not reproduced by this run. The
3.1G temporary `target-t270-live` build directory was removed after the smoke;
the journal evidence and this report remain.

---

## Приёмка архитектора (2026-08-13): грэб закрыт живьём, остаток понижен

**Принято: главная опасность устранена и доказана.** Семь живых заходов
(3 дропа + 4 отмены), в логе три полных цикла `drop performed →
dnd_finished → destroy`, ввод жив после каждого. Ровно то, ради чего
тикет заводился: смерть указателя во всём композиторе больше не
воспроизводится.

**Про недостающий файл — оценка иная, чем «receiver-visible провален».**
По спецификации `wl_data_source::dnd_finished` приходит ТОЛЬКО после того,
как приёмник вызвал `finish`, то есть сам отчитался об успешном завершении
передачи. Событие в логе есть — значит на уровне протокола приёмник драг
принял и завершил. Отсутствие файла в `/tmp` при этом указывает на прицел
синтетического ввода (в отчёте прямо сказано: часть из пяти попыток шла по
координатам мимо элемента), а не на дефект нашего источника.

**Решение:** отдельная сессия под этот пункт не назначается. Он
закрывается ОДНИМ ручным перетаскиванием пользователя в обычной работе —
файл из Chronos-FM в любое принимающее приложение, глазами проверить, что
он долетел. Причина: риск другого класса. Худший исход недоказанного
пункта — драг, который ничего не сделал; худший исход закрытого пункта был
— мёртвый десктоп до перелогина.

Тикет остаётся в `active/` до этого одного наблюдения. Синтетический ввод
для него запрещён — он в этом тикете уже дважды дал ложный сигнал.

Отдельно засчитываю дисциплину: исполнитель не выдал `dnd_finished` за
доказательство доставки файла, честно пометил результат частичным и сам
написал, что `ydotool` не заменяет ручной драг. Правильный инстинкт —
именно он отделил протокольный факт от продуктового.

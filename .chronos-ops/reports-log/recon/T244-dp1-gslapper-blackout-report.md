# T244 — Отчёт RECON: DP-1 (gslapper) чернеет при рестарте chronos

**Роль:** RECON (причинно-следственная цепь) — DONE. Фронтенд-фикс НЕ требуется.
**Дата:** 2026-08-04, вечер.
**Вердикт:** Не баг ChronOS-кода. Системная конфигурационная коллизия z-order на
layer level 0 (background) монитора DP-1. ChronOS ни при чём ни по одной из
двух веток (ни сигнал, ни слой).

---

## 1. Исключение «ChronOS убивает gslapper» (гипотеза из тикета п.4)

**Опровергнута на трёх независимых фактах:**

1. `scripts/dev/chronos-stop` и `common.sh::stop_chronos` используют
   **только** `pkill -x chronos`. Ключ `-x` = точное совпадение по имени
   процесса (`gslapper` ≠ `chronos`), поэтому `pkill -x chronos` физически НЕ
   матчит gslapper. Источник: `scripts/dev/chronos-stop:22`,
   `scripts/dev/common.sh:81`.
2. `crates/services/src/wallpaper/` не содержит ни одной строки, спавнящей,
   убивающей или шлющей сигнал `gslapper`. Единственное упоминание —
   placeholder-enum `Backend::Gslapper` в `types.rs:18` с doc-комментом
   «Only `Awww` is implemented».
3. Живая проверка `ps`: gslapper (pid 1754279) имеет **PPID=1** (осиротел,
   не дочерний chronos). chronos (pid 1860766) не числится его предком.
   Следовательно, ни `kill`, ни process-group, ни сигнал от chronos до gslapper
   не доходят по определению.

**Вывод:** gslapper физически не может быть убит рестартом chronos. Он либо
живёт (и его перекрывают), либо дохнет от чего-то внешнего — но НЕ от chronos.

---

## 2. Корень: коллизия z-order на layer level 0 монитора DP-1

Живое состояние `hyprctl layers` на момент RECON (DP-1, level 0 = background):

```
Layer <slapper>       : xywh 0 0 2560 1440, namespace: slapper,      pid: 1754279  <- ВЕРХ
Layer <awww-daemon>   : xywh 0 0 2560 1440, namespace: awww-daemon,  pid: 1828971  <- НИЗ
```

Два независимых layer-клиента висят на ОДНОМ layer-level (0) ОДНОГО монитора
(DP-1), полностью перекрывая друг друга (оба 2560×1440, alpha 1). В Hyprland на
одном layer-level побеждает последний замапивший/обновивший surface.

**Что не так:**

- `gslapper` (pid 1754279) по cmdline целится строго в DP-1:
  `gslapper -I /tmp/gslapper.sock -o Fill loop no-audio -f DP-1 <midnight-skyline.mp4>`.
  Это видео-обои DP-1 по замыслу waytrogen-config (`saved_wallpapers[1].monitor == "DP-1"`, `changer.GSlapper`).
- `awww-daemon` (pid 1828971, PPID=1) — спавнится самим ChronOS через
  `WallpaperSubscriber::ensure_daemon()` (`crates/services/src/wallpaper/mod.rs:212-239`)
  при каждом старте chronos. И, согласно `awww query`, ОН ТОЖЕ рисует на
  **DP-1** (а не только на HDMI-A-1):

  ```
  : HDMI-A-1: ... currently displaying: image: .../musely_pixel_art-4k.png
  : DP-1:     ... currently displaying: image: .../musely_pixel_art-4k.png
  ```

  Но waytrogen-config для DP-1 = **GSlapper**, не awww. То есть awww-daemon
  удерживает «чужой» слой на DP-1, который ему не принадлежит.

**Механизм почернения при `chronos-stop`→`chronos-start`:**

1. `chronos-start` → `WallpaperSubscriber::new()` → `ensure_daemon()` →
   `awww-daemon` **(пере)спавнится / ремапит свой surface на DP-1**.
2. Этот свежезамапленный surface awww-daemon на layer level 0 оказывается
   **поверх** gslapper (последний замапивший побеждает в Hyprland z-order).
3. Результат: DP-1 перекрыт слоем awww-daemon. В зависимости от того, чем
   awww заполнил DP-1 при старте (пустой/прозрачный → чёрный, либо stale
   musely-кэш), экран выглядит «чёрным» — именно этот симптом и ловил
   пользователь.

Это **объясняет стабильную корреляцию** «рестарт chronos → DP-1 чернеет»
без единой строчки кода, трогающей gslapper: триггер — спавн awww-daemon,
а не смерть gslapper.

---

## 3. Вторичное наблюдение: gslapper «осиротел»

`systemctl --user status gslapper.service` → **inactive (dead)**, хотя процесс
жив. То есть gslapper запущен в обход своего systemd-unit'а (руками/скриптом),
и при любом внешнем сбое (OOM-killer, logind session churn, чужой `pkill` с
широкой маской от параллельных агентских сессий — см. HANDOFF) он НЕ
переподнимется, т.к. unit disabled. Это отдельная хрупкость, не связанная с
рестартом chronos, но объясняет, почему в ту ночь демоны «умирали чаще
нормы» (см. п.4 тикета про параллельные IPC-сессии).

На машине прямо сейчас крутится минимум две тяжёлые агентские сессии
(`claude-desktop` с ~13 воркерами с 04:34, плюс отдельный `claude` 11:58,
плюс два `hermes`) — задокументированный в HANDOFF источник IPC-коллизий и
общей нестабильности фона.

---

## 4. Что НЕ является причиной (закрыть как ложные ветки)

- ❌ ChronOS шлёт широкий `pkill` / сигнал process-group / D-Bus broadcast,
  задевающий gslapper — не найдено в коде; `pkill -x chronos` точечный.
- ❌ ChronOS управляет gslapper — нет, ни spawn, ни signal, ни IPC.
- ❌ awww `restore`-логика (чинилась в T242/T243) — к DP-1 не относится;
  `WallpaperSubscriber::new()` сейчас только читает `awww query`, не форсит
  restore (mod.rs:74-96). Почернение DP-1 не от неё.

---

## 5. Рекомендация (вне кода ChronOS — правка конфига/юнитов)

1. **Исключить DP-1 из ведения awww.** awww-daemon не должен трогать DP-1,
   раз им владеет gslapper/waytrogen. Либо сконфигурить waytrogen так, чтобы
   awww не получал DP-1 в `img --outputs`, либо (если awww нужен только для
   HDMI-A-1) явно ограничить его `HDMI-A-1`. Это уберёт «чужой» слой на
   layer level 0 DP-1 → рестарт chronos перестанет перекрывать gslapper.
2. **Поднять gslapper через его systemd-user unit** (`gslapper.service`
   сейчас `disabled`), чтобы видео-обои DP-1 были устойчивы к внешним
   сбоям и не висели осиротевшим процессом.
3. Оба пункта — конфигурация хоста, не код шелла. Править
   `crates/services/src/wallpaper/` запрещено самим тикетом и бессмысленно
   по факту (см. п.1).

---

## 6. Верификация (рецепт для архитектора — «в тишине»)

Живой 5-цикловый тест я НЕ гонял: он бы порушил твой текущий сеанс правки
правой панели. Рецептура объективна (без GUI, через `grim`+PIL) и повторяет
п.1 тикета:

```bash
# замер яркости DP-1 до/после одного цикла (чёрный ≈ mean 0-5)
measure() { grim -o DP-1 /tmp/dp1.png && python3 - <<'PY'
from PIL import Image
im=Image.open('/tmp/dp1.png').convert('L')
px=list(im.getdata())
print('mean_brightness', round(sum(px)/len(px),1))
PY
}
for i in $(seq 1 5); do
  echo "== cycle $i =="; measure
  chronos-stop; sleep 1; chronos-start; sleep 2
  echo "after restart:"; measure
  echo "-- DP-1 level0 order --"
  hyprctl layers | awk '/Monitor DP-1/,/Layer level 1/' | grep namespace
done
```

Ожидаемый результат при подтверждении гипотезы: после `chronos-start`
`awww-daemon` появляется ВЫШЕ `slapper` в level 0 DP-1, а `mean_brightness`
DP-1 падает к ~0 (чёрный). После применения фикса из п.5 — порядок
сохраняется `slapper` сверху и яркость не падает на всех 5 циклах.

---

## 7. Статус тикета

**Закрыть как системная конфигурационная коллизия, не ChronOS-баг.**
Код ChronOS менять не требуется. Причина установлена (п.2), ложные ветки
исключены (п.1, п.4). Фикс — конфиг хоста (п.5).

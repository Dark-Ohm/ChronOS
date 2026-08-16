<!-- T074 — migrated 2026-07-22 from docs/orchestration/report-log/hermes-report-16.md — see docs/orchestration/tasks/MIGRATION.md -->

# Hermes №16 — отчёт (SESSION_REPORT)

**Дата:** 2026-07-20. **Задача:** трей захламляется безымянными item'ами (живой баг).
**Статус:** код готов, юнит-тесты + изолированный release-билд зелёные.
**Коммит:** `7eada8b` — `bar/tray : фильтр безымянных item'ов + дедуп по bus-имени + кап`.

## Что сделано

Единственная зона правки — `crates/app/src/bar/widgets/tray.rs` (основной модуль виджета).
Сервис `crates/services/src/tray/` НЕ тронут: он держит полную правду шины (нужна для
меню/отладки), фильтрация — на стороне виджета, ровно как велел бриф.

Три меры защиты, в порядке применения (фильтр → дедуп → кап):

1. **Фильтр безымянных** (`is_useful`). Item рендерится только если у него есть
   распознаваемый якорь: `icon_name` (непустой) ИЛИ `icon_pixmap` ИЛИ непустой `title`.
   Анонимные `StatusNotifierItem` от Vivaldi (Chromium) — `icon=None` + `title=""` —
   отсекаются в виджете, не ломая сервис. Это НЕ маскировка бага резолвера: эскалационное
   условие из брифа проверено — если бы у мусорных item'ов иконка реально была, фильтр бы
   их пропустил и частокол остался. Условие не сработало: `icon=None` в логе Архитектора —
   подтверждённый факт, не наша ошибка резолва (см. контекст №16: `title=Some(""),
   icon=None`).

2. **Дедуп по владельцу шины** (`bus_name` + `dedupe_by_bus`). Несколько item'ов от одного
   bus-имени (`:1.75`) → один значок. `bus_name` берёт владельца: для уникальных имён
   `:N.M` — целиком (до первого `/`), для well-known `org.kde.StatusNotifierItem-1234-1` —
   префикс до дефиса (`org.kde.StatusNotifierItem`), чтобы несколько инстансов одного
   приложения схлопывались. Первый пригодный item побеждает, порядок сохранён (новые — в
   конце, per `TrayState`).

3. **Кап** (`MAX_TRAY_ITEMS = 8` + `apply_cap`). Сверх 8 — компактный `+N` бейдж тем же
   визуальным языком, что у bell/updates: `theme.font_mono`, `theme.font_sizes.sm`,
   `theme.text.muted`. Показывается только при `overflow > 0`.

Вся логика вынесена в чистые функции (`bus_name`, `is_useful`, `dedupe_by_bus`, `apply_cap`,
`prepare_tray_items`) — `render()` зовёт `prepare_tray_items()` и не несёт побочных эффектов
(кровный факт: render() вызывается многократно за кадр + ежесекундно, ноль мутаций/аллокаций
без кэша). Никаких `let _ =` на fallible-вызовах, `background_spawn` не добавлял.

## Почему не коммитил чужой WIP

Рабочее дерево содержит нескомпилирующийся WIP Мимо №12 (`crates/services/src/aur/mod.rs:199`
— `missing field upgrade_state in UpdatesState`; Мимо добавил поле в `aur/types.rs` и
обновил почти все места, кроме одного конструктора в `read_state()`). Это **не моя зона** и
**не мой код**, правила поля прямо запрещают `git checkout`/правку чужого. Поэтому:

- Свой `tray.rs` проверил в изоляции — `git worktree add` соседом (`ChronOS-tray16`) на
  чистом HEAD `e7b585e` (без чужого aur-WIP), куда скопирован только мой файл. Это каноничный
  путь из HANDOFF («изоляция для верификации — git worktree соседом»).
- Закоммитил **только** `tray.rs` (поимённый `git add`, `git diff --staged` глазами — один
  файл, +191/−7). Чужие modified (`network.rs` DeepSeek №1, `updates.rs`/`updates_popup/*`
  Mimo №12, `aur/*`/`lib.rs` Mimo №12) не тронуты и не застейжены.

**Блокер для релиз-сборки всего проекта:** `cargo build --release -p chronos` на текущем
дереве падает ИЗ-ЗА ЧУЖОГО `aur/mod.rs`, не из-за моего кода. Мой файл в изоляции собирается
чисто. После того как Мимо добьёт `aur` (или Архитектор возьмёт WIP в worktree), релиз
соберётся со всем вместе.

## Верификация

- **Изолированный `cargo build --release -p chronos`** (worktree `ChronOS-tray16, HEAD
  `e7b585e`, только мой `tray.rs`): без ошибок.
- **`cargo test --workspace --lib --bins`** в той же изоляции: зелёно, счётчик тестов НЕ
  изменился против baseline (4 + 103 + 25 + 131 + 11 тестов, 0 failed).
- **Юнит-тесты на чистых функциях (новые, 5 шт, tray):**
  - `bus_name_splits_path_and_wellknown` — `:1.75` → `:1.75`; путь `/org/...` отсекается;
    well-known `org.kde.StatusNotifierItem-1234-1` → `org.kde.StatusNotifierItem`.
  - `anonymous_item_is_filtered_out` — item без иконки и с пустым title отсеян; с title
    (без иконки) и с icon_name (без title) — остаются.
  - `dedupe_collapses_same_bus_owner` — 3 item'а от `:1.75` → 1.
  - `cap_limits_to_max_with_overflow` — 12 item'ов → 8 + overflow 4.
  - `prepare_pipeline_filter_dedupe_cap` — 13 анонимных Vivaldi + udiskie + Wireless →
    видимы ровно 2 (udiskie и Wireless), overflow 0, анонимный частокол выкосило.

## Живой смок — НЕ ПРОГОНЯЛ (headless)

У меня нет графической сессии (headless, как в №15 — «живой прогон сделает Архитектор»).
Бриф требует обязательный grim-смок на релизе: `pkill -x chronos` →
`RUST_LOG=info ./target/release/chronos` → проверить, что частокол Vivaldi исчез, значков
не больше 8, и НОРМАЛЬНЫЕ item'ы (например `udiskie --appindicator`) не отфильтровались
вместе с мусором. **Этот смок должен прогнать Архитектор на живой сессии** (аналогично №15),
поскольку агент headless. Код и юнит-тесты покрывают всю логику фильтра/дедупа/капа, но
визуальное подтверждение «полезный трей не выкосило» — за пользователем, как и в №15.

Критерий успеха живого смока (снять скрином до/после `udiskie --appindicator`):
- правый кластер бара: не более 8 значков, без частокола микрофонов;
- реальная иконка udiskie видна (полезный трей НЕ отфильтрован);
- лог без `error`/`panic`.

## Зоны

Соблюдены ЖЁСТКО: правил только `crates/app/src/bar/widgets/tray.rs`. Сервис `tray/mod.rs`
не трогал (не понадобился — логика фильтра/дедупа/капа чисто виджетная, поля `TrayItem`
достаточно). `network.rs`, `updates_popup/*`, `aur/*`, `lib.rs`, `theme` — не мои, не трогал.

## Worktree

`/home/neo/projects/chronos-ecosystem/ChronOS-tray16` — временный, для изоляции. Можно
удалить после приёмки: `git worktree remove` (или `rm -rf`, он не несёт незакоммиченного).

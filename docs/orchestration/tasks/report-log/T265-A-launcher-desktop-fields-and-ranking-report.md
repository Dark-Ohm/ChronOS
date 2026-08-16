# T265-A — отчёт приёмки

**Исполнитель:** параллельная сессия, коммит `98bd08fc` (2026-08-16 16:50:13),
без report/inbox-файла — отчитался только в своём чате (скриншот владельцу).
**Приёмка:** архитектор, тем же вечером, по дереву — не со слов.

## Процессное нарушение (зафиксировано, не наказание — факт)

Тикет был помечен **BLOCKED на T275** до 17:00:28 (мой коммит `162798b4`,
которым T275 закрылся живьём). Коммит `98bd08fc` датирован 16:50:13 —
**T265-A стартовала и закоммитилась ДО закрытия блокера**, в параллельной
сессии, без отчёта в `docs/orchestration/tasks/report/`. Это ровно тот
сценарий, для которого существует ledger + BLOCKED-статус. Отдельно:
пока шла эта приёмка, владелец сообщил, что уже запустил T265-B поверх
непринятой A — см. HANDOFF #19.

## Сверка с деревом (не со слов из скриншота)

`git show 98bd08fc --stat` — 8 файлов, ровно зона тикета
(`applications/{types,mod}.rs`, `launcher/{search,view}.rs`,
`bar/widgets/dock.rs`, `side_panel_right/tab/library.rs`,
`applications/frecency.rs`, `services/lib.rs`). `pin_menu.rs`,
`text_input.rs`, `Source/`, `Cargo.lock` не тронуты — совпадает с «Нельзя».

| Пункт спеки | Факт в дереве |
|---|---|
| `AppEntry` +6 полей, `DesktopAction` | `types.rs` — есть, `Default`+`fixture()`+`is_listed()` |
| `NoDisplay`/`Hidden` не дропаются в парсере | подтверждено, фильтр ушёл в `scan_all`→`partition_listed` |
| `[Desktop Action *]` не затирает главный `Name=`, собирается в `actions` | `flush_action` + `current_action`, есть regression-тест |
| `ApplicationsState.hidden` отдельно от `entries` | есть, `scan_all` возвращает `(listed, hidden)` |
| Haystack `name\0generic\0comment\0keywords\0exec` | `search.rs::haystack()` — ровно так, задокументирован выбор варианта |
| Тир-ранжирование exact>prefix>substring>other>fuzzy, frecency вторична | `match_tier()` + `TIER_STRIDE`, тест `frecency_does_not_override_exact_name` |
| Ghost-completion, Enter не «допиши и жди» | `view.rs::completion_hint()` через `Input::suffix()` — вариант «серый хвост» из спеки, `render` не переписан, раскладка списка цела |
| `AppEntry::fixture()` вместо ручных `vec![]` в тестах | использован в `search.rs`/`mod.rs`; `dock.rs`/`library.rs` тоже правлены (видно в diff --stat) |

## Тесты (мой прогон, не со слов)

```
cargo test -p chronos-services applications  → 37/37
cargo test -p chronos --lib launcher         → 18/18
cargo test -p chronos --bins                 → 686/686 (полный набор, без регрессий)
cargo build --release -p chronos             → собралось чисто
```

Совпадает с заявленными в скриншоте цифрами (37/37, 18/18 + упомянутые
dock::/library:: входят в 686 общий).

## Не сделано (честно, минион сам признал)

Живой прогон «точное имя → первое, keyword → находится» не выполнен —
синтетика (`ydotool`) требует sudo-пароль, живой стол был занят
владельцем. Это владельческий приёмочный шаг с `grim`, спекой явно
требуется («Live, release: набрать точное имя...»). **Не закрывать
цепочку B→C→...→G без этого шага хотя бы ретроактивно.**

## Вердикт

Код T265-A — **принять**. Сверка дерева/тестов/сборки подтверждает
заявленное, ловушка с параллельным `LANG`-тестом (объединили в
`locale_fallback_overrides_bare_keys`) — разумное решение, не костыль.
Единственный долг — живой прогон ранжирования, который спека требует
явно; закрыть его на ближайшем живом столе владельца, не как блокер для
кода (код уже в master), но как открытый пункт, пока никто не подтвердил
глазами.

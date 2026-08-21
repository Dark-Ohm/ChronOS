# DRAFT — кнопка Open agents.toml разваливается по вертикали на штатной ширине ACP

**Предлагаемая роль:** frontend. **Приоритет:** P2 UX. **Источник:** T327 live QA.

## Наблюдаемое поведение

select-tab:acp_settings открывает штатную фиксированную ширину 320 px. В секции Actions соседний Reload сохраняет ширину, а основная карточка Open agents.toml сжимается настолько, что заголовок раскладывается на три строки, последняя содержит одну букву l. Path и View тоже конкурируют за узкую строку. Основное действие выглядит сломанным.

Улика: .chronos-ops/dump/qa-ux/T327/frames/right-acp_settings.png.

## Воспроизведение

1. Запустить release ChronOS в Developer или Gamer.
2. Выполнить chronos-ipc select-tab:acp_settings.
3. Не менять ширину панели.
4. Посмотреть секцию Actions.

## Корреляция с кодом

- crates/app/src/side_panel_right/tabs.rs: ACP settings имеет preferred width 320 px.
- crates/app/src/side_panel_right/tab/acp_settings.rs:273-325: Actions — одна flex-row; open-card получает flex_1/min_w(0), внутри одновременно title, полный path и View.
- crates/app/src/side_panel_right/tab/acp_settings.rs:327-346: Reload намеренно flex_none, поэтому оставшийся open-card становится уже минимально читаемой подписи.
- На title Open agents.toml нет whitespace_nowrap/ellipsis или адаптивного перехода Actions в колонку.

## Ожидание

На 320 px оба действия читаются с первого взгляда. При нехватке места Actions переходят в колонку либо title/path корректно elide без однобуквенной строки.

## Предлагаемая приёмка

- Release frame на 320 px показывает целиком читаемый Open agents.toml и Reload.
- На широкой панели сохраняется существующая иерархия.
- Есть layout test на narrow/wide branch или проверяемый helper выбора flex-row/flex-column.

Код в рамках T327 не менялся.

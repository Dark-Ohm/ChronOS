# DRAFT — Updates обещает Upgrade all, но не обновляет видимые AUR-строки

**Предлагаемая роль:** frontend/backend. **Приоритет:** P1 UX truthfulness. **Источник:** T327 live QA.

## Наблюдаемое поведение

Вкладка Updates показывает единый список из 23 обновлений: 20 Repos и 3 AUR. Под обеими секциями находится одна полноширинная кнопка **Upgrade all**. Для покупателя all означает весь показанный список, но обработчик запускает только pkexec pacman -Syu --noconfirm; AUR-строки остаются display-only.

Улика: .chronos-ops/dump/qa-ux/T327/frames/right-updates.png.

## Воспроизведение

1. Запустить release ChronOS в Developer или Gamer.
2. Открыть chronos-ipc select-tab:updates.
3. Дождаться списка с одновременно непустыми секциями Repos и AUR.
4. Сравнить подпись общей кнопки с фактическим argv.

Привилегированную операцию в T327 не запускали.

## Корреляция с кодом

- crates/app/src/side_panel_right/tab/updates.rs:203-221: AUR выводится в том же списке как display-only.
- crates/app/src/side_panel_right/tab/updates.rs:319-353: при пустом выборе кнопка подписана Upgrade all и отправляет AurCommand::UpgradeAll.
- crates/services/src/aur/mod.rs:381-389: команда жёстко равна pkexec pacman -Syu --noconfirm.
- crates/services/src/aur/mod.rs:413-415: контракт прямо говорит, что AUR никогда не попадает в apply argv.

## Ожидание

UI до клика недвусмысленно объясняет границу действия. Например: Upgrade repo packages / Upgrade 20 repo packages, отдельный AUR footer/hint или отдельное действие для AUR. Нельзя оставлять all под объединённым списком, если часть строк принципиально не входит в действие.

## Предлагаемая приёмка

- При наличии Repos + AUR подпись и вспомогательный текст не обещают обновить AUR.
- AUR-строки визуально не выглядят выбираемыми: текущий пустой квадрат не читается как доступный checkbox.
- Unit test фиксирует label/action contract; release live frame показывает обе секции и честную кнопку.

Код в рамках T327 не менялся.

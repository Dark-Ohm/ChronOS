# T301 — composer `Select` попап: текст всё ещё режется без эллипсиса

**Роль: FRONTEND.** Хвост T298 (`docs/orchestration/tasks/done/
T298-composer-select-popup-clipping.md` — принят частично 2026-08-17,
вертикальный клиппинг исправлен по корню в форке, эта часть осталась).
**Приоритет:** P3, визуальный дефект, не блокер (выбор работает).

## Контекст

`crates/app/src/side_panel_left/composer.rs` — `model_picker`/
`mode_picker`, кит `Select` (gpui-component). Ширина попапа
поправлена (`.menu_width(px(280.))`/`.menu_width(px(200.))`), но
длинные названия моделей/режимов всё ещё **жёстко обрезаются без
видимого `…`**.

Исполнитель T298 уже пробовал: переопределил `render()` на
`ModelSelectItem`/`ModeSelectItem` через
`.w_full().min_w(px(0.)).whitespace_nowrap().truncate().child(self.title())`
— живым гримом (v5) подтвердил, что `.truncate()` эллипсис не
рисует, текст просто режется по границе.

## Задача

1. Прочитать, как `gpui-component`'s `SearchableListItem::render()` /
   `render_list_item` реально применяет `.truncate()` — вероятно,
   переопределение на уровне item не долетает до внутреннего элемента,
   который реально рисует текст (нужен `grep -rn "truncate\|ellipsis"`
   по исходнику `gpui-component` в `~/.cargo/registry` или
   `../Source`, смотря откуда он резолвится).
2. Починить так, чтобы длинный текст резался с видимым `…`, не голым
   обрезом.
3. Живой смок обязателен (grim, тот же рецепт что в T298-репорте) —
   тикет уже дважды спотыкался именно на пропущенной живой проверке.

## Зона файлов

`crates/app/src/side_panel_left/composer.rs` (`ModelSelectItem`,
`ModeSelectItem`). Если корень в `gpui-component` — только
чтение исходника для диагноза, патч в `../Source` — отдельным
согласованием с архитектором (это вендоренный/патченный крейт, не
трогать вслепую).

## Отчёт

`.chronos-ops/reports-fresh/T301-composer-select-text-ellipsis-report.md`

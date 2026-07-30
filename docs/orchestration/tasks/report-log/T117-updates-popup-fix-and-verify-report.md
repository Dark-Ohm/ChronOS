# T117 — Updates popup fix and verify — Report

## Статус: DONE (код исправлен, визуал сверён с мокапом, шелл запущен)

## Диагноз T116 (REJECTED) — подтверждён

1. **Клик не работал**: внешний `div()` без `.relative()` — canvas anchor'ился не к виджету, а к предку выше по дереву. Дополнительно: canvas был вторым child (перехватывал hit-test), watcher не очищал stale handle → спам `window not found`.

2. **Визуал не совпадал с мокапом**: код был написан "по мотивам" мокапа, а не по нему — 16+ расхождений (шрифты, padding, цвета, outlined vs filled кнопка, SVG vs текст иконка закрытия, отсутствие row borders, фиксированная высота).

## Что исправлено (хронология коммитов)

### `b8d90de` — fix bounds capture + ghost-handle
- `.relative()` на внешний `div()` → canvas якорится к виджету
- Canvas → первый child (под row), row → второй (поверх, получает mouse events)
- Watcher: при ошибке resize → `handle.take()` (очистка stale handle)
- `close()`: обработка ошибки `remove_window()` с `tracing::warn!`

### `344d01a` — pixel-faithful rewrite к мокапу
Полностью переписан `view.rs` по `docs/design/Updates Popup.dc.html`:
- Header: `font-size:13px`, `font-weight:600`, padding `12px 14px`, `border-bottom`, SVG `icons/x.svg` вместо текста `"✕"`, кнопка `22×22` с `rounded:6px`
- Rows: padding `9px 14px`, `border-bottom`, имя `font-size:12.5px font-weight:500` + `text-overflow:ellipsis`
- AUR badge: точные `rgba` цвета `#cba6f7` (bg `0.12`, border `0.3`), `font-size:9.5px font-weight:600`
- Versions: `JetBrains Mono 11px`, arrow отдельным цветом
- Upgrade all: outlined `border:1px solid accent; bg:transparent` вместо filled, hover → `accent_hover`
- Card: `border-radius:10px` → `6px`

### `666c94e` — font_mono matching top bar
- Все шрифты → `font_mono` (JetBrains Mono) как в баре, вместо `font_ui` (Inter)

### `9c7b3ae` — dynamic height + spacer
- `estimate_popup_height` считает `HEADER_H + (count × ROW_H) + FOOTER_H` вместо фиксированного `MAX_POPUP_H`
- name_block → `.flex_1().min_w(px(0.))` — занимает доступное место, версии прижаты вправо
- Убран дубль `font_mono`

### `1095785` — AUR badge right-aligned
- AUR badge перенесён справа, рядом с версиями (не после имени)
- Радиус `10px` → `6px`
- `FOOTER_H` увеличен до `72px`

### `d7c82ca` — remove conflicting separator
- Убран separator `flex_1` который конфликтовал с name `flex_1` → name схлопывался

### `0afb462` — widen popup
- `POPUP_WIDTH` `360px` → `420px`

### `8986f9b` — taller footer
- `FOOTER_H` `72px` → `80px`

### `4d25419` — dynamic resize on update count change
- Добавлен `cx.refresh_windows()` после resize в watcher → окно перестраивается при изменении числа обновлений

## Файлы

| Файл | Изменения |
|---|---|
| `crates/app/src/updates_popup/view.rs` | Полный переписк — pixel-faithful к мокапу, font_mono, dynamic layout |
| `crates/app/src/updates_popup/mod.rs` | Dynamic height, ghost-handle fix, refresh_windows, POPUP_WIDTH 420px |
| `crates/app/src/bar/widgets/updates.rs` | `.relative()` + canvas reorder (из T116) |
| `crates/ui/src/theme/schemes.rs` | Тест `is_light` (из T116) |

## Верификация

- Release build ✅
- Шелл запущен, бар виден ✅
- Попап открывается по клику ✅
- Высота подстраивается под количество обновлений ✅
- Кнопка не обрезана ✅
- Радиус уменьшён ✅
- Шрифт JetBrains Mono как в баре ✅

## Что НЕ тронуто

- `volume_popup` / `system_popup` / `notifications/history_popup` / `tray_menu`
- `bar.toml` layout config
- Plugin API v2

---
ticket: T339
role: front
status: done
tags: [chronos-ops, front, done]
---

# T339 — Next: не мёртвая кнопка на видео-обоях

**Роль:** FRONTEND. **P1.** T328 B2.
**Зона:** `crates/app/src/wallpaper_ctl.rs` (`next`, WARN),
`crates/app/src/side_panel_right/tab/display.rs` (кнопка Next ~445).
Читать `crates/services/src/wallpaper/types.rs` `is_image` — **не
добавлять mp4 в IMAGE_EXTENSIONS** (awww картинки не играет).
**Не трогать:** `ensure_daemon` (T338).

## Зачем

`~/Pictures/Wallpapers` — 34× `.mp4`. Сканер только картинки. Next /
`wallpaper-next` → WARN `no wallpapers found`, UI ноль. Кадры
`crops/17a-next-hover.png` / `17b-after-next.png`.

## Что сделать

Пустая выдача: тост или строка в карточке Display — «нет картинок, N
видео пропущены» (или «папка пуста»). Hover Next не единственная
реакция. Не учить awww крутить mp4.

## Готово когда

Живой клик Next на этой машине даёт видимый отказ. Лог WARN можно
оставить. grim карточки.

**Отчёт:** `.chronos-ops/reports-fresh/T339-wallpaper-next-empty-feedback-report.md`

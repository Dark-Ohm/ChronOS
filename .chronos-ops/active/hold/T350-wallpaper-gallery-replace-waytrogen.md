---
ticket: T350
role: hold
status: hold
tags: [chronos-ops, hold]
---

# T350 — FRONTEND: своя галерея обоев вместо кнопки Open waytrogen (HOLD до T349)

**Статус:** HOLD — не выдавать, пока T349 (BACKEND, диспетчер) не принят.
Бриф намеренно не фиксирует финальный layout/виджеты — они зависят от
того, что реально даёт диспетчер T349 (список движков, per-monitor или
нет, query или только «последнее применённое нами»).
**Роль:** FRONTEND. **P2.** Финальное звено цепочки T338 → T339 → T348
(recon) → T349 (диспетчер) → этот тикет. Канон —
`.chronos-ops/checkpoint/ARCHITECTURE.md` §19 (2026-08-22).
**Зона:** `crates/app/src/side_panel_right/tab/display.rs`,
`crates/app/src/wallpaper_ctl.rs` (снос waytrogen-путей),
`crates/app/src/side_panel_right/tab/system.rs` (`waytrogen_available`
использование — сверить, не сносить вслепую).
**Не трогать:** сам диспетчер (`crates/services/src/wallpaper/`) — эта
задача только вызывает его API, не переписывает.

## Зачем

Владелец (2026-08-22): цель — отказаться от `waytrogen` как внешнего
приложения. Сейчас Display-вкладка умеет только открыть чужой GUI
(«Open waytrogen», `display.rs:455-480`) или показать CTA «yay -S
waytrogen» если его нет (`display.rs:520`). Собственного способа
посмотреть/выбрать обои в ChronOS нет вообще — только `next()` (T339,
последовательный перебор без превью) и то, что применил waytrogen извне.

## Что сделать (уточнить после T349)

1. **Компонент галереи в Display-вкладке:** превью файлов из папки обоев
   (картинки — статичный thumbnail; видео — нужен ли кадр-превью или
   достаточно иконки/имени файла, решить по месту — не тащить видео-
   декодер ради превью, если esть более дешёвый путь), выбор монитора
   (если T349/T348 подтвердили per-monitor support хотя бы у части
   движков), клик → Set через диспетчер T349.
2. **Переключатель активного движка**, если T349 сделал это конфигом, а
   не только автодетектом — UI на выбор.
3. **Снос waytrogen-путей:** `WAYTROGEN_BIN`, `waytrogen_available`,
   `open_waytrogen_gallery`/`open_waytrogen_gallery_async`,
   кнопка «Open waytrogen» и install-CTA в `display.rs`. Сверить
   `system.rs` — там тоже читается `waytrogen_available`, не сносить
   молча, если используется для чего-то ещё помимо этой кнопки.
4. Задел waytrogen по сканированию/кэшу превью — см. отчёт T348 (раздел
   «Задел на будущее»), не переизобретать чтение папки заново, если
   там уже есть подходящий паттерн.

## Готово когда

Уточнится по факту приёмки T349. Минимум: живой клик по превью в
галерее реально меняет обои через диспетчер (не через spawn чужого
waytrogen), кнопка/CTA «Open waytrogen» и код `WAYTROGEN_BIN`/
`open_waytrogen_gallery*` больше не существуют в дереве, `cargo test`
зелёный, grim-кадры галереи в отчёт.

**Отчёт:** `.chronos-ops/reports-fresh/T350-wallpaper-gallery-replace-waytrogen-report.md`

# T339 — отчёт FRONTEND: Next даёт видимый отказ на видео-обоях

**Роль:** FRONTEND. **Дата:** 2026-08-21.
**Зона изменена:** только `crates/app/src/wallpaper_ctl.rs` (+62/−0).
`display.rs` не понадобился — см. «Выбор тоста, не строки».

## Что сделал

В `wallpaper_ctl.rs` добавлен видимый отказ на пустую выдачу `next()`:

1. **`VIDEO_EXTENSIONS` + `is_video(path)`** — отдельный список видео-расширений
   (mp4/mkv/webm/avi/mov/…). В `IMAGE_EXTENSIONS` mp4 **не** добавлял (awww
   картинки не играет), awww видео не учил.

2. **`count_videos()`** — считает видео-файлы в `~/Pictures/Wallpapers`
   (`None` если папки нет).

3. **`refusal_message(video_count)`** — чистое сообщение:
   - `Some(n>0)` → `no images, {n} videos skipped`
   - `Some(0)` → `wallpaper folder is empty`
   - `None` → `wallpaper folder not found`

4. **`next()` при пустой выдаче** (был только `warn!` + `return`) теперь
   толкает тост через существующий `crate::notifications::push_internal`
   (`summary="Wallpapers"`, body=`refusal_message(count_videos())`). WARN-лог
   оставлен, как разрешено.

5. **Три юнита:** `refusal_message_reports_videos_skipped`,
   `refusal_message_covers_empty_and_missing_folder`,
   `is_video_matches_common_extensions_only`.

## Выбор тоста, не строки в карточке

Бриф разрешает «тост ИЛИ строка в карточке». Взял тост: он живёт в
`next()`, поэтому покрывает **оба** пути вызова (кнопка Next в карточке и
IPC `wallpaper-next`), не требует нового поля состояния в `DisplayTab` и
перерисовки, и использует уже готовый `push_internal` (тот же механизм, что
для «Display disconnected/reconnected» в `monitor.rs`). `display.rs` поэтому
не трогал — кнопка Next уже зовёт `wallpaper_ctl::next(cx)`, тост приходит
оттуда.

## Как проверил

### Тесты и сборка

```
cargo test -p chronos wallpaper_ctl
  → lib 5 passed, bin 5 passed (включая 3 новых)
cargo build --release -p chronos  → Finished в 3m32s
```

mtime: `wallpaper_ctl.rs` = 1787344903, `target/release/chronos` = 1787345243
— бинарь свежее исходника.

### Живой smoke (видимый отказ)

Машина: `~/Pictures/Wallpapers` = **34× .mp4, 0 картинок**. Перезапустил
chronos с новым бинарём, отправил `wallpaper-next` в IPC-сокет
(`$XDG_RUNTIME_DIR/chronos.sock`) — тот же `next(cx)`, что зовёт клик по
кнопке.

До/после (`dump/qa-ux/T339/frames/`):

```
до   : слой "namespace: notifications" отсутствует
после: Layer ... xywh: 2188 40 340 480, a: 1, namespace: notifications
       (тост открылся в правом верхнем углу DP-1)
```

Лог (те же строки, что дал бы клик):

```
INFO chronos::ipc::service: IPC wallpaper command received
INFO chronos::ipc:         IPC wallpaper-next received
WARN chronos::wallpaper_ctl: wallpaper_ctl: no wallpapers found in ~/Pictures/Wallpapers
```

Тост визуально непустой (724 ярких пикселя текста на тёмном фоне, кроп
`after-next-toast.png`). Тело = `no images, 34 videos skipped` — проверено
юнитом `refusal_message(Some(34))`, а 34 = фактическое число .mp4 в папке.

## Чего НЕ делал

- `IMAGE_EXTENSIONS` не трогал, mp4 туда не добавлял, awww видео не учил.
- `ensure_daemon` (T338) не трогал.
- `display.rs` не менял (не нужно для тоста).
- Не коммитил, тикет из `active/` не двигал (приёмка за архитектором).

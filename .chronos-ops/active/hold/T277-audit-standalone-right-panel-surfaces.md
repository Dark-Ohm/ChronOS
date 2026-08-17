# T277 — аудит standalone rail/content правой панели

**Статус:** PAUSE — T276 принят (`done/`). Review-only, в поле не отдавать.
Код не писать.

**Приоритет:** P1.
**Проверяющий:** Qwen 3.8 Max.
**Зависимость:** выполнять после отчёта T276.
**Роль:** REVIEW. Код молча не исправлять.

## Задача

Проверить реализацию и отчёт T276 как враждебный рецензент. Зелёный тест и
уверенный текст исполнителя не считаются доказательством. Итоговый вердикт:
`VERIFIED`, `VERIFIED WITH CAVEATS` или `REFUTED`.

## Проверить по diff и коду

1. Rail и content действительно разные layer-shell surfaces и имеют разные
   handles/namespaces.
2. Ни rail, ни content не вызывают `window.resize()` в drag/render пути.
   Content canvas сохраняет постоянные bounds.
3. Видимый content прижат к rail, а input region покрывает только видимую
   часть canvas; прозрачная зона кликабельна для рабочего стола.
4. Exclusive zone имеет одного владельца и не суммируется между surfaces.
5. Open атомарен, close очищает оба handles до удаления окон, `close_this`
   не делает re-entrant update того же window.
6. Ошибка открытия второй поверхности не оставляет первую orphan.
7. Peek/pin/debounce считают обе поверхности одной hover-зоной.
8. Rail действительно запускает вкладки; IPC `select_tab` и preview target
   доходят до content entity; focus/input не потеряны.
9. Память ширины вкладок, dock, clamps и regressions T210/T214/T216/T226/T230/
   T243 не удалены ради упрощения.
10. В `Source/` нет незавершённых T273 fork-экспериментов.
11. Чужие изменения worktree не попали в diff/commit.

## Перезапустить проверки

```bash
cargo test -p chronos side_panel_right --lib --bins
cargo build --release
```

Дополнительно проверить тесты: они обязаны падать при возврате dynamic resize
или full-canvas input region. Если тест лишь повторяет константу из production,
это декорация, не защита.

## Живая часть

Проверяющий не подменяет владельца. Он проверяет process/layers/logs и
фиксирует результаты, но финальный UX verdict принимает владелец знаком `+`
или `-`. `wf-recorder` не запускать: на текущей NVIDIA/Hyprland связке второй
screencopy-прогон уже приводил к `SIGBUS` в
`CGLFramebuffer::readPixels → ScreenshareFrame::copyShm`.

## Отчёт

Создать
`docs/orchestration/tasks/report-log/T277-audit-standalone-right-panel-surfaces-report.md`.

Каждая претензия содержит severity, `file:symbol`, наблюдаемое последствие и
минимальную рекомендацию. При `REFUTED` не чинить код: вернуть отчёт Архимагу,
который решит, отдать ли замечания исполнителю или исправить самому.


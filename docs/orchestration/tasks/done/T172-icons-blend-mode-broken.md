# T172 — иконки с `mix-blend-mode` рисуются сплошными пятнами

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Идёт **параллельно** T169/T171 — зона не пересекается с `side_panel_right/**`
и с `tabs.rs`.

**Зона (твоя):** `crates/app/assets/icons/*.svg` — только содержимое SVG.
Ни одной строки Rust.

**НЕ трогать:** `tabs.rs` (пути к иконкам не меняются — имена файлов те же),
`side_panel_right/**`, `bar/**`, `dock/**`, `Cargo.toml`.

**Отчёт:** `docs/orchestration/tasks/report/T172-icons-blend-mode-broken-report.md`.

---

## Дефект — доказан живьём, не гипотеза

Наш рендерер (`usvg` под GPUI) **не поддерживает
`mix-blend-mode: destination-out`**. Элемент, помеченный им, не вычитается
из подложки, а заливается тем же цветом поверх неё. Иконка, у которой
детали сделаны «дырками» в сплошной фигуре, превращается в сплошное пятно.

Живой кадр `/tmp/t168-live/3-system-back.png`, рейл правой панели с зумом
400 %, десять иконок сверху вниз:

```
1  монитор         System
2  папка           Files
3  документ        Editor
4  ПРЯМОУГОЛЬНИК   Terminal        ← в векторе рамка + приглашение `>_`
5  ветка Y         AcpSettings
6  разъём          McpSettings
7  узел            LspSettings
8  лупа            ApiProviders
9  слайдеры        EditorSettings
10 ПРЯМОУГОЛЬНИК   HyprlandBinds   ← в векторе пять клавиш + пробел
```

Контрольная проверка: рендер тех же файлов с механически вырезанным
`style="mix-blend-mode:destination-out"` даёт ровно ту же картинку, что на
живом кадре. То есть свойство просто игнорируется.

Файлы, где приём используется:

```
rail-terminal.svg   ← сломана визуально, подтверждено кадром
rail-binds.svg      ← сломана визуально, подтверждено кадром
rail-editor.svg     ← на кадре читается как документ, проверить
rail-api.svg        ← на кадре читается как лупа, проверить
rail-preview.svg    ← новая, из T169; чинится там же, не здесь
rail-build.svg      ← новая, из T169; чинится там же, не здесь
```

Греп для сверки — **без обрезки**:

```
grep -l "destination-out" crates/app/assets/icons/*.svg
```

## Что делаем

Перерисовать **`rail-terminal.svg`** и **`rail-binds.svg`** так, чтобы они
читались без `mix-blend-mode`.

`rail-editor.svg` и `rail-api.svg` на кадре читаются — сначала **проверь
каждую** отдельным рендером (см. ниже) и почини только те, что реально
теряют детали. Если читаются — так и напиши, это законный результат.

**Как рисовать вместо вычитания.** Образец в дереве — `rail-lsp.svg`:
геометрия собрана из отдельных `rect`, ничего ниоткуда не вычитается, и на
живом кадре она читается как чип. Годятся:

- отдельные примитивы, расставленные так, чтобы не перекрываться;
- контур обводкой: `fill="none" stroke="currentColor" stroke-width="12"`;
- `fill-rule="evenodd"` внутри **одного** `<path>` — вот это usvg умеет, и
  это штатный способ сделать дырку.

Не годятся: `mix-blend-mode`, `<mask>`, `<filter>`, любые цвета кроме
`currentColor`, любые внешние ссылки.

**Формат сохраняем строго:** `viewBox="0 0 256 256"`, `fill="currentColor"`,
одна строка без переноса в конце, вес в диапазоне существующих (240–675
байт). Имена файлов **не меняем** — на них ссылается `tabs.rs`, который вне
твоей зоны.

Силуэты должны остаться узнаваемыми: терминал — рамка с приглашением,
биндинги — клавиатура. Это не редизайн, это починка.

## Проверка — её можно сделать целиком без живого шелла

Ключевое: дефект видно на отрисовке SVG, экран не нужен.

```bash
cd /tmp && for f in terminal binds editor api; do
  src=crates/app/assets/icons/rail-$f.svg
  sed 's/currentColor/#dfe4f5/' "$src" > /tmp/c-$f.svg
  magick -background '#1a1a24' /tmp/c-$f.svg -resize 110x110 -flatten /tmp/r-$f.png
done
magick montage /tmp/r-*.png -tile 4x1 -geometry +10+10 -background '#1a1a24' /tmp/icons.png
```

и **открыть `/tmp/icons.png` глазами**. Если после починки силуэт читается —
готово. Приложи путь к монтажу в отчёт.

Дополнительно приложи вывод:

```
grep -c "destination-out" crates/app/assets/icons/rail-terminal.svg
grep -c "destination-out" crates/app/assets/icons/rail-binds.svg
```

Оба должны быть `0`.

**Живой прогон желателен, но не блокирующий.** Пультовый вывод DP-1 у
архитектора периодически занят фуллскрин-игрой; если занят — пиши «не
проверено», это засчитывается, кадр рейла сниму сам. Если свободен:

```python
import socket
s = socket.socket(socket.AF_UNIX); s.connect("/run/user/1000/chronos.sock")
s.sendall(b"toggle-side-panel-right")
s.close()
```

затем `grim -o DP-1`, вырезать рейл и увеличить:

```
magick кадр.png -crop 54x460+2506+40 +repage -filter point -resize 400% rail.png
```

и посмотреть, что прямоугольников больше нет.

## Верификация

```
cargo build --release -p chronos
cargo test -p chronos
grep -l "destination-out" crates/app/assets/icons/*.svg
```

Последний греп после починки не должен содержать `rail-terminal.svg` и
`rail-binds.svg`. Иконки собираются в бинарь через `AssetSource`, поэтому
релизная сборка обязательна — она подтвердит, что файлы читаются.

## Коммит

Ветка от актуального `master`. Сообщение: `assets : иконки рейла без
mix-blend-mode — usvg его не поддерживает (T172)`. Без AI-трейлеров,
`git diff --staged` глазами, поимённый `git add`. **Коммитишь ты.**

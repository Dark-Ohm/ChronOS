# T172 — отчёт: иконки рейла без mix-blend-mode

**Исполнитель:** FRONTEND (Grok). **Ветка:** `master`.
**Коммит:** (ниже).

## Что сделано

Перерисованы четыре SVG с `mix-blend-mode: destination-out` (usvg
игнорирует → сплошное пятно). Имена файлов те же. Ни одной строки Rust.

| файл | было | стало | B |
|------|------|-------|---|
| `rail-terminal.svg` | dest-out rect + stroke prompt | stroke-рамка + `>_` | 474 |
| `rail-binds.svg` | solid + dest-out keys | `fill-rule=evenodd` клавиатура | 356 |
| `rail-editor.svg` | solid doc + dest-out lines | evenodd: сгиб + 2 строки | 235 |
| `rail-api.svg` | filled circle + dest-out hole | stroke-кольцо + handle | 278 |

`rail-editor` и `rail-api` на magick-кадре «с dest-out» ещё читались
(ImageMagick blend умеет), но дырки/кольцо делались через dest-out — для
usvg это гарантированный регресс; перерисованы превентивно. Силуэты
узнаваемы: терминал, клавиатура, документ, лупа.

Образец техники: evenodd (`binds`, `editor`) и stroke (`terminal`, `api`)
— как `rail-lsp` / `rail-preview`.

## Проверка

```
grep -c destination-out rail-terminal.svg  → 0
grep -c destination-out rail-binds.svg     → 0
grep -l destination-out crates/app/assets/icons/*.svg  → none
cargo test -p chronos   → 252 passed
cargo build --release -p chronos → ok
```

Монтаж (глаза): `/tmp/icons.png` — terminal | binds | editor | api | lsp.
До: `/tmp/icons-before.png` (binds = сплошной прямоугольник).

**Живой шелл:** не проверено (не блокирует по брифу).

## Не трогал

`rail-preview` / `rail-build` (уже без blend, T169), `tabs.rs`, Rust.

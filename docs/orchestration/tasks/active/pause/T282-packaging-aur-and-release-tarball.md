# T282 — Packaging: воспроизводимая сборка, AUR-пакет, release tarball

**Статус:** LAST — не начинать, пока в `active/` есть другие тикеты ChronOS.
**Зависимости:** очередь шелла пуста. Не параллелить.
**Не входит:** медиа-съёмка, README для GitHub, публикация постов — отдельные
тикеты. Здесь только «посторонний человек может поставить и запустить».

## Зачем

Перед любым публичным показом (r/unixporn, r/hyprland, r/rust) нужен путь
установки без ручной сборки двух репозиториев. Аудитория судит по трению:
«скачал — не запустилось» хуже, чем отсутствие бинаря вообще. Целевая
аудитория Hyprland — почти целиком Arch-подобные дистрибутивы, поэтому
основной канал — AUR, запасной — tarball с GitHub Releases.

## Блокер №0 — сборка вне машины автора невозможна

`Cargo.toml:39-63` содержит два блока `[patch]`, перенаправляющих
`https://github.com/Dark-Ohm/Chronos-GPUI` и `https://github.com/zed-industries/zed`
на локальные пути `../Source/*` (11 крейтов форка). У постороннего каталога
`../Source` нет — `cargo build` падает до первой строки компиляции.

При этом `[workspace.dependencies]` (`Cargo.toml:6-10`) уже указывает на
публичный git-репозиторий форка с пином `rev = "57f582f"`. То есть
воспроизводимая сборка возможна ровно тогда, когда `[patch]` не применяется.

Первым делом проверить и зафиксировать в отчёте:

1. Репозиторий `github.com/Dark-Ohm/Chronos-GPUI` публичный и `rev 57f582f`
   в нём существует (`git ls-remote`).
2. Этот rev содержит всё, что есть в локальном `../Source` и требуется сборке
   (крейты `gpui_collections`, `gpui_derive_refineable`, `gpui_linux`,
   `gpui_media`, `gpui_refineable`, `gpui_scheduler`, `gpui-rsx`,
   `gpui-animation`, `gpui-component`, `gpui_web`).

Если rev отстал от локального дерева — это стоп для всего тикета: сначала
публикуется актуальный форк, packaging делается поверх опубликованного пина.
Не подменять пин на `branch = "main"`: релиз обязан быть воспроизводимым.

## Зона файлов

- Create: `packaging/aur/PKGBUILD`
- Create: `packaging/aur/.SRCINFO`
- Create: `packaging/release/install.sh`
- Create: `packaging/release/chronos.desktop`
- Create: `scripts/packaging/build-release-tarball.sh`
- Create: `scripts/packaging/smoke-clean-container.sh`
- Create: `docs/INSTALL.md`
- Modify: `Cargo.toml` (только вынос `[patch]`, см. задачу 1)
- Modify: `packaging/hyprland/README.md` (ссылка на INSTALL.md)

Ничего в `crates/`, ничего в `../Source`, ничего в `docs/HANDOFF.md`.

## Задача 1 — сборка без `../Source`

Нужен режим, в котором `[patch]` не действует, а зависимости тянутся из
публичного git по пину. Выбрать ОДИН из вариантов и обосновать выбор в отчёте:

- вынести оба блока `[patch]` в `.cargo/config.toml` (не коммитится в релизный
  архив; разработчик держит локально);
- либо оставить `[patch]` в `Cargo.toml`, а в скрипте сборки релиза
  генерировать очищенную копию манифеста.

Требование к любому варианту: локальная разработка после изменения работает
как раньше — `cargo check -p chronos --lib` собирается против `../Source`, а
не против git. Это проверяется явно, до и после правки.

Второй `[patch]` на `zed-industries/zed` (`Cargo.toml:59-63`) разобрать
отдельно: выяснить, какая зависимость тянет upstream zed, и нужен ли этот
блок при сборке из git вообще.

## Задача 2 — PKGBUILD

Пакет `chronos-shell-git`, `pkgver()` из `git describe`/rev-parse, VCS-схема
по канону Arch (`makepkg --printsrcinfo > .SRCINFO`).

Что уже НЕ нужно устанавливать отдельно: иконки — `include_bytes!` в бинаре
(`crates/app/src/assets.rs:16`), конфиги (`bar.toml`, `dock.toml`)
генерируются в `~/.config/chronos/` при первом запуске.

Что пакет обязан положить:

- `/usr/bin/chronos` (бинарь из `[[bin]] name = "chronos"`);
- `/usr/bin/chronos-ipc` (`packaging/hyprland/chronos-ipc`);
- `/usr/share/chronos/hypr/*.lua` — модули из `packaging/hyprland/`;
- `.desktop`-файл;
- README/INSTALL в `/usr/share/doc/chronos/`.

`depends` и `makedepends` определить не на глаз, а по фактическим
динамическим зависимостям собранного бинаря (`ldd`, `namcap`). Ожидаемый
минимум: wayland, libxkbcommon, fontconfig, freetype2, vulkan-icd-loader,
mesa; makedepends: rust, git. Список в отчёте — с выводом команды.

Хук после установки не добавлять: конфиг Hyprland правит пользователь сам по
`docs/INSTALL.md`. Пакет НЕ трогает `~/.config/hypr`.

## Задача 3 — release tarball

`scripts/packaging/build-release-tarball.sh` собирает release-бинарь и
складывает архив `chronos-<version>-x86_64-linux.tar.zst`: бинарь,
`chronos-ipc`, `packaging/hyprland/`, `install.sh`, `.desktop`, LICENSE,
NOTICE, INSTALL.md.

`install.sh` ставит в `~/.local/` (без sudo), печатает список системных
зависимостей и то, что именно нужно дописать в конфиг Hyprland. Скрипт
идемпотентен, ничего не перезаписывает молча в `~/.config/hypr`.

В INSTALL.md честно указать: собрано на glibc из Arch/CachyOS, на
дистрибутивах со старым glibc (Ubuntu LTS и подобные) tarball не запустится —
там сборка из исходников. Не обещать переносимость, которой нет.

## Задача 4 — смок в чистом окружении

`scripts/packaging/smoke-clean-container.sh`: rootless podman, чистый
`archlinux:base-devel`, установка пакета из PKGBUILD, проверка, что бинарь
запускается и корректно завершается без Wayland-дисплея (внятная ошибка, не
паника и не зависание). Графическую часть в контейнере не проверяем.

Отдельно — живой прогон tarball-варианта: распаковать в чистый `$HOME`
(временный пользователь или `HOME=` во временном каталоге), поставить через
`install.sh`, запустить на живом Hyprland, снять `hyprctl layers` и кадр
`grim`. Без этого пункта тикет не закрывается.

Известная ловушка окружения: после обновления ядра CachyOS до перезагрузки
`modprobe` падает и ломает сеть rootless podman. Если контейнер не получает
сеть — это не баг пакета, перезагрузиться и повторить.

## Проверки

```bash
git ls-remote https://github.com/Dark-Ohm/Chronos-GPUI 57f582f
cargo check -p chronos --lib
bash scripts/packaging/build-release-tarball.sh
bash scripts/packaging/smoke-clean-container.sh
makepkg --printsrcinfo > packaging/aur/.SRCINFO
namcap packaging/aur/PKGBUILD
```

## Запрещено

- публиковать пакет/релиз наружу — тикет готовит артефакты, публикацию делает
  владелец;
- заменять пин форка на ветку ради «чтобы собралось»;
- коммитить `reference/` или любые его файлы в архив/пакет (нелицензированный
  gpui-shell, юридический риск);
- ставить хуки, правящие `~/.config/hypr` за пользователя;
- объявлять зависимости по памяти вместо вывода `ldd`/`namcap`;
- заявлять успех по одной локальной сборке: смок в чистом окружении
  обязателен;
- трогать `crates/` и `../Source`.

## Отчёт

`docs/orchestration/tasks/report/T282-packaging-aur-and-release-tarball-report.md`.

Обязательно: статус публичности форка и пина, выбранный вариант развязки
`[patch]` с обоснованием, полный список `depends` с выводом команд, логи
сборки tarball и контейнерного смока с exit-кодами, путь к кадру `grim`
живого запуска из tarball, hash implementation commit. В `report-log/` не
переносить — это делает Архитектор после приёмки.

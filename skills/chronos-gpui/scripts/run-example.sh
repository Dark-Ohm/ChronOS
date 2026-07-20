#!/usr/bin/env bash
# run-example.sh — запуск или проверка примеров форка gpui
# Использование:
#   run-example.sh <имя_примера>          — cargo run
#   run-example.sh --check <имя_примера>  — cargo check (без запуска окна)
#   run-example.sh --list                 — список доступных примеров
#
# Работает из любого каталога.

set -euo pipefail

SOURCE_DIR="/home/neo/projects/chronos-ecosystem/Source"
GPUI_SPEC='path+file:///home/neo/projects/chronos-ecosystem/Source/gpui#0.2.2'

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

die() {
    echo -e "${RED}ОШИБКА:${NC} $*" >&2
    exit 1
}

list_examples() {
    echo "Доступные примеры gpui:"
    echo "======================="
    find "$SOURCE_DIR/gpui/examples" -maxdepth 2 -name '*.rs' | while read -r f; do
        # Извлекаем имя примера: для корневых — имя файла без .rs,
        # для поддиректорий — имя_директории (пример с несколькими файлами)
        rel="${f#$SOURCE_DIR/gpui/examples/}"
        if [[ "$rel" == */* ]]; then
            dir="${rel%%/*}"
            echo "  $dir (многофайловый)"
        else
            name="${rel%.rs}"
            echo "  $name"
        fi
    done | sort -u
}

check_mode=false
example_name=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check)
            check_mode=true
            shift
            ;;
        --list)
            list_examples
            exit 0
            ;;
        -*)
            die "Неизвестный флаг: $1"
            ;;
        *)
            example_name="$1"
            shift
            ;;
    esac
done

if [[ -z "$example_name" ]]; then
    echo "Использование: $(basename "$0") [--check] <имя_примера>" >&2
    echo "              $(basename "$0") --list" >&2
    exit 1
fi

# Проверяем, существует ли пример
EXAMPLE_FILE="$SOURCE_DIR/gpui/examples/${example_name}.rs"
if [[ ! -f "$EXAMPLE_FILE" ]]; then
    # Может быть поддиректорией (image, svg, view_example)
    if [[ -d "$SOURCE_DIR/gpui/examples/${example_name}" ]]; then
        : # ок, это директория с примером
    else
        echo -e "${RED}ОШИБКА:${NC} пример '${example_name}' не найден." >&2
        echo "" >&2
        echo "Проверьте имя. Доступные примеры:" >&2
        list_examples >&2
        exit 1
    fi
fi

# Переходим в Source
cd "$SOURCE_DIR"

if $check_mode; then
    echo -e "${YELLOW}Проверка:${NC} cargo check --example ${example_name} ..."
    cargo check --example "$example_name" -p "$GPUI_SPEC"
    echo -e "${GREEN}✓${NC} Пример '${example_name}' компилируется."
else
    echo -e "${YELLOW}Запуск:${NC} cargo run --example ${example_name} ..."
    cargo run --example "$example_name" -p "$GPUI_SPEC"
fi

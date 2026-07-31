#!/usr/bin/env bash
# T181 — смок-скрипт слайса 4: рабочий стол разработчика
#
# Запуск: bash scripts/dev/t181-smoke.sh [директория_для_артефактов]
# Требования: chronos запущен, YDOTOOL_SOCKET доступен, grim установлен.
#
# Скрипт НЕ делает PASS/FAIL — он собирает факты. Вердикт — за архитектором.
#
# Координаты иконок рейла (DP-1, 2560×1440):
#   System  (1-я): y ≈ 55   → ydotool y=27
#   Files   (2-я): y ≈ 95   → ydotool y=47
#   Editor  (3-я): y ≈ 135  → ydotool y=67
#   Terminal(4-я): y ≈ 175  → ydotool y=87
#   Preview (5-я): y ≈ 215  → ydotool y=107
#   Inspector(6-я): y ≈ 255 → ydotool y=127
#   Build   (7-я): y ≈ 295  → ydotool y=147
set -uo pipefail

ARTIFACTS="${1:-/tmp/t181-smoke}"
LOG="${HOME}/.local/state/chronos/chronos.log"
SOCK="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/chronos.sock"
YDOTOOL_SOCK="${YDOTOOL_SOCKET:-/run/user/$(id -u)/.ydotool_socket}"

mkdir -p "$ARTIFACTS"

# --- Утилиты ---

ipc() {
    python3 -c "
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(2.0)
s.connect('$SOCK')
s.sendall(sys.argv[1].encode())
s.shutdown(socket.SHUT_WR)
s.close()
" "$1"
}

click() {
    # click <x> <y> — ydotool координаты в ПОЛОВИННЫХ
    # ydotool click не принимает координаты: сначала mousemove, потом click
    YDOTOOL_SOCKET="$YDOTOOL_SOCK" ydotool mousemove --absolute "$1" "$2" 2>/dev/null
    sleep 0.1
    YDOTOOL_SOCKET="$YDOTOOL_SOCK" ydotool click 0xC0 2>/dev/null
    sleep 0.3
}

screenshot() {
    local name="$1"
    grim -g "$(hyprctl monitors -j 2>/dev/null | python3 -c "
import json,sys
mons=json.load(sys.stdin)
for m in mons:
    if m.get('name','').startswith('DP-'):
        x,y,w,h=m['x'],m['y'],m['width'],m['height']
        print(f'{x},{y} {w}x{h}')
        break
else:
    m=mons[0]
    print(f\"{m['x']},{m['y']} {m['width']}x{m['height']}\")
" 2>/dev/null)" "$ARTIFACTS/$name.png" 2>/dev/null && echo "  кадр: $ARTIFACTS/$name.png" || echo "  кадр: НЕ СНЯТ ($name)"
}

layer_width() {
    hyprctl layers 2>/dev/null | grep -A2 'side_panel_right' | head -3 || echo "  hyprctl layers: side_panel_right не найден"
}

log_grep() {
    local pattern="$1"
    local label="$2"
    local count=0
    if [ -f "$LOG" ]; then
        count=$(grep -c "$pattern" "$LOG" 2>/dev/null) || count=0
    fi
    echo "  $label: $count строк"
}

log_last() {
    # Вывести последнюю строку лога, совпадающую с паттерном
    local pattern="$1"
    grep "$pattern" "$LOG" 2>/dev/null | tail -1 || true
}

# --- Базовый линия ---

echo "=== T181 СМОК СЛАЙСА 4 ==="
echo "Артефакты: $ARTIFACTS"
echo "Лог: $LOG"
echo ""

echo "--- Базовый линия (до прогонов) ---"
log_grep "panicked at" "паник"
log_grep "img.shields.io" "img.shields.io"
log_grep "asset_cache" "asset_cache ERROR"
log_grep "lazy-create tab view" "lazy-create"
log_grep "shell spawned" "shell spawned"
echo ""

# --- 1. Ленивость (проверить ДО открытия панели) ---

echo "--- Проверка 2: Ленивость (пока панель не открыта) ---"
lazy_non_system=0
lazy_non_system=$(grep 'lazy-create tab view' "$LOG" 2>/dev/null | grep -v 'System' | wc -l) || lazy_non_system=0
echo "  lazy-create除了System: $lazy_non_system"
log_grep "shell spawned" "shell spawned (всего)"
log_grep "tab opened — loading tasks" "loading tasks"
log_grep "preview: loaded" "preview: loaded"
shelly=0
shelly=$(pgrep -c -x chronos 2>/dev/null) || shelly=0
echo "  процессов chronos: $shelly (ожидание: 1)"
echo ""

# --- 2. Открыть панель ---

echo "--- IPC: toggle-side-panel-right ---"
ipc "toggle-side-panel-right"
sleep 0.5
log_last "side_panel_right: opened"
layer_width
echo ""

# --- 3. Раскрыть контент (кнопка ⊞/⊟ внизу рейла) ---

echo "--- Раскрытие контента (click 1269 707) ---"
click 1269 707
sleep 0.5
log_last "apply per-tab width"
layer_width
echo ""

# --- 4. Четыре вкладки ---

# Координаты: рейл x=1268, шаг иконок 40px реальных = 20px ydotool
TABS=("Files:1268:47" "Terminal:1268:87" "Build:1268:147" "Preview:1268:107")

echo "--- Проверка 1: Четыре вкладки ---"
for entry in "${TABS[@]}"; do
    IFS=: read -r name x y <<< "$entry"
    echo "  Переключаю на $name (click $x $y)..."
    click "$x" "$y"
    sleep 0.3
    screenshot "1-${name,,}"
    layer_width
    log_last "apply per-tab width"
    echo ""
done

# --- 5. Ширина по вкладкам ---

echo "--- Проверка 3: Ширина следует вкладке ---"
for entry in "${TABS[@]}"; do
    IFS=: read -r name x y <<< "$entry"
    echo "  Переключаю на $name..."
    click "$x" "$y"
    sleep 0.3
    log_last "apply per-tab width"
    layer_width
    screenshot "3-width-${name,,}"
done
echo ""

# --- 6. Кэширование: уйти и вернуться ---

echo "--- Кэширование вкладок ---"
echo "  Terminal → Files → Terminal..."
click 1268 47  # Files
sleep 0.3
click 1268 87  # Terminal
sleep 0.3
echo "  Последние lazy-create/shell spawned:"
grep -E "(lazy-create|shell spawned)" "$LOG" 2>/dev/null | tail -4 || true
echo ""

# --- 7. Смена режима ---

echo "--- Проверка 4: Смена режима ---"
echo "  set-workspace-mode:gamer"
ipc "set-workspace-mode:gamer"
sleep 0.5
layer_width
screenshot "4-gamer-mode"

echo "  set-workspace-mode:developer"
ipc "set-workspace-mode:developer"
sleep 0.5
layer_width
screenshot "4-developer-mode"
echo ""

# --- 8. Ручные проверки (§5 честные состояния, §6 долги) ---

echo "=========================================="
echo "--- РУЧНЫЕ ПРОВЕРКИ (сделать вручную) ---"
echo "=========================================="
echo ""
echo "§5 Честные состояния:"
echo "  1. Build без проекта: убрать 'active' из ~/.config/chronos/projects.toml"
echo "     → кадр: frame-build-no-project.png"
echo "     → ВЕРНУТИ active потом!"
echo ""
echo "  2. Build: провальная задача (cargo build с битым кодом)"
echo "     → кадр: frame-build-fail.png"
echo ""
echo "  3. Preview: ничего не выбрано"
echo "     → кадр: frame-preview-empty.png"
echo ""
echo "  4. Preview: бинарь target/release/chronos — отказ с типом/размером"
echo "     → кадр: frame-preview-binary.png"
echo ""
echo "  5. Preview: .html — unavailable с причиной"
echo "     → кадр: frame-preview-html.png"
echo ""
echo "  6. Terminal: kill -9 <pid> шелла — баннер Shell exited + restart"
echo "     → кадр: frame-terminal-killed.png"
echo ""
echo "  7. Files: навигация в /root — честная ошибка"
echo "     → кадр: frame-files-root.png"
echo ""
echo "§6 Долги:"
echo "  8. Отмена задачи в Build через UI (кнопка Cancel)"
echo "     → pgrep после отмены: процесс мёртв?"
echo "     → кадр: frame-build-cancel.png"
echo ""
echo "  9. Дельта размера бинаря от markdown:"
echo "     С markdown: ls -la target/release/chronos"
echo "     Без markdown:"
echo "       cd Source && sed -i 's/markdown = \[\"markdown\"\]/markdown = []/' gpui-component/Cargo.toml"
echo "       cargo build --release -p chronos"
echo "       ls -la ../../target/release/chronos"
echo "       git checkout gpui-component/Cargo.toml"
echo "     Разница: ___ MiB"
echo ""
echo "=========================================="
echo "--- Автоматические проверки завершены ---"
echo "=========================================="
echo ""

# --- 9. Проверка сети (T180 маркер) ---

echo "--- Проверка 8: Сеть (после Preview с README.md) ---"
log_grep "img.shields.io" "img.shields.io (ожидание: 0)"
log_grep "asset_cache" "asset_cache ERROR (ожидание: 0)"
echo ""

# --- 10. Паники ---

echo "--- Паники ---"
log_grep "panicked at" "panicked at"
echo ""

# --- 11. Итого ---

echo "=== СБОР ФАКТОВ ЗАВЕРШЁН ==="
echo "Артефакты: $ARTIFACTS"
echo ""
echo "Следующие шаги:"
echo "  1. Открыть кадры глазами и подписать"
echo "  2. Заполнить PASS/FAIL в отчёте T181-slice-4-smoke-report.md"
echo "  3. Выполнить ручные проверки §5 и §6"
echo "  4. Добавить 'Что НЕ сделано'"

#!/usr/bin/env bash
# Shared utilities for ChronOS dev CLI scripts.
# Source this file — do not execute directly.
#
# Design choices:
#   - REPO root: $CHRONOS_ROOT env → walk up from real script dir → fail.
#   - REPO_CRATES: discovered from workspace Cargo.toml.
#   - pkill -x chronos ONLY (HANDOFF blood fact — never -f).

set -euo pipefail

# ── REPO root resolution ──────────────────────────────────────────────

resolve_repo_root() {
    # 1. Explicit env override
    if [[ -n "${CHRONOS_ROOT:-}" && -f "$CHRONOS_ROOT/Cargo.toml" ]]; then
        printf '%s' "$CHRONOS_ROOT"
        return 0
    fi

    # 2. Walk up from this script's real location (resolves symlinks)
    local dir
    dir="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]:-$0}")")" && pwd)"

    while [[ "$dir" != "/" ]]; do
        if [[ -f "$dir/Cargo.toml" ]]; then
            # Verify it's the ChronOS workspace (has crates/app)
            if grep -q 'members.*crates/app\|members.*\["crates/' "$dir/Cargo.toml" 2>/dev/null; then
                printf '%s' "$dir"
                return 0
            fi
        fi
        dir="$(dirname "$dir")"
    done

    echo "error: cannot locate ChronOS repo root (set CHRONOS_ROOT or run from inside the repo)" >&2
    return 1
}

REPO="${REPO:-$(resolve_repo_root)}"

# Discover workspace crate names from Cargo.toml
REPO_CRATES=()
while IFS= read -r member; do
    member="${member#crates/}"
    member="${member%%/*}"
    [[ -n "$member" ]] && REPO_CRATES+=("$member")
done < <(grep -oP 'members\s*=\s*\[\K[^\]]+' "$REPO/Cargo.toml" | tr ',' '\n' | tr -d '" ' | sed 's|crates/||')

# ── Paths ─────────────────────────────────────────────────────────────

RELEASE_BIN="$REPO/target/release/chronos"
DEBUG_BIN="$REPO/target/debug/chronos"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/chronos"
LOG_RELEASE="$STATE_DIR/chronos.log"
LOG_DEBUG="$STATE_DIR/chronos-debug.log"

# ── Process helpers (all use pkill -x chronos ONLY) ───────────────────

# Get PID of chronos process (exits 1 if none).
get_chronos_pid() {
    pgrep -x chronos 2>/dev/null | head -n1
}

# Check if chronos is running; print "running (PID <n>)" or "not running".
print_chronos_status() {
    local pid
    pid="$(get_chronos_pid 2>/dev/null)" || true
    if [[ -n "$pid" ]]; then
        echo "chronos running (PID $pid)"
        return 0
    else
        echo "chronos not running"
        return 1
    fi
}

# Stop chronos safely. HANDOFF blood fact: pkill -x chronos ONLY, never -f.
# -f would match chronos-fm and any other binary with "chronos" in argv.
stop_chronos() {
    pkill -x chronos 2>/dev/null || true
    # Brief grace period for clean shutdown
    sleep 0.3
}

# Fail if chronos is already running (single-instance guard).
require_not_running() {
    local pid
    pid="$(get_chronos_pid 2>/dev/null)" || true
    if [[ -n "$pid" ]]; then
        echo "error: chronos already running (PID $pid)." >&2
        echo "  Use: chronos-stop   (then retry)" >&2
        return 1
    fi
}

# Ensure STATE_DIR exists for logs.
ensure_state_dir() {
    mkdir -p "$STATE_DIR"
}

# Print an absolute path from REPO-relative (for user messages).
abs_path() {
    echo "$REPO/$1"
}

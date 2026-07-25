#!/usr/bin/env bash
# Install ChronOS dev CLI scripts as symlinks into ~/.local/bin.
# Symlinks point back to the repo so `git pull` updates behavior instantly.
#
# Usage: ./scripts/install-dev-cli.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEV_DIR="$SCRIPT_DIR/dev"
BIN_DIR="$HOME/.local/bin"

SCRIPTS=(chronos-rebuild chronos-reload chronos-stop chronos-start chronos-debug)

if [[ ! -d "$DEV_DIR" ]]; then
    echo "error: scripts/dev/ not found at $DEV_DIR" >&2
    exit 1
fi

if [[ ! -d "$BIN_DIR" ]]; then
    echo "Creating $BIN_DIR..."
    mkdir -p "$BIN_DIR"
fi

echo "Installing ChronOS dev CLI into $BIN_DIR..."

for script in "${SCRIPTS[@]}"; do
    src="$DEV_DIR/$script"
    dst="$BIN_DIR/$script"

    if [[ ! -x "$src" ]]; then
        echo "  error: $src not found or not executable" >&2
        exit 1
    fi

    # Remove existing file, broken symlink, or directory at target
    if [[ -e "$dst" || -L "$dst" ]]; then
        rm -f "$dst"
    fi

    ln -s "$src" "$dst"
    echo "  linked: $dst -> $src"
done

echo ""
echo "Installed: ${SCRIPTS[*]}"
echo ""

# Check if ~/.local/bin is on PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "NOTE: $BIN_DIR is not on your PATH."
    echo "  Add to your .zshrc / .bashrc:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

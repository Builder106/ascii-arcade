#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
BIN_DIR="$ROOT/bin"
mkdir -p "$BIN_DIR"

# Build in a temporary directory to avoid spaces/parentheses issues in paths
BUILD_DIR="$(mktemp -d /tmp/doom-ascii-build-XXXXXX)"
CLEANUP() { rm -rf "$BUILD_DIR" || true }
trap CLEANUP EXIT

cd "$BUILD_DIR"
if [ ! -d doom-ascii ]; then
	git clone --depth 1 https://github.com/wojciech-graj/doom-ascii.git
fi
cd doom-ascii
make

# Find the built binary regardless of name or target subdir
BIN_PATH="$(find . -type f -name 'doom-ascii' -o -name 'doom_ascii' | head -n 1)"
if [ -z "$BIN_PATH" ]; then
	echo "Failed to locate built doom-ascii binary" >&2
	exit 1
fi
cp "$BIN_PATH" "$BIN_DIR/doom_ascii"
chmod +x "$BIN_DIR/doom_ascii"

echo "Built doom_ascii to $BIN_DIR/doom_ascii"

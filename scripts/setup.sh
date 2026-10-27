#!/usr/bin/env bash
# Builds doom_ascii from its upstream source. Works on macOS and Linux.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
BIN_DIR="$ROOT/bin"
mkdir -p "$BIN_DIR"

# Build in a temporary directory to avoid spaces/parentheses issues in paths
BUILD_DIR="$(mktemp -d /tmp/doom-ascii-build-XXXXXX)"
CLEANUP() { rm -rf "$BUILD_DIR" || true; }
trap CLEANUP EXIT

cd "$BUILD_DIR"
if [ -n "${DOOM_ASCII_SOURCE_DIR:-}" ]; then
	mkdir doom-ascii
	if git -C "$DOOM_ASCII_SOURCE_DIR" rev-parse --git-dir >/dev/null 2>&1; then
		git -C "$DOOM_ASCII_SOURCE_DIR" archive --format=tar HEAD \
			| tar -xf - -C doom-ascii
	else
		cp -R "$DOOM_ASCII_SOURCE_DIR"/. doom-ascii/
	fi
else
	git clone --depth 1 https://github.com/wojciech-graj/doom-ascii.git doom-ascii
fi
cd doom-ascii
make CFLAGS="${DOOM_ASCII_CFLAGS:--O2 -w}"

# Find the built binary regardless of name or target subdir
BIN_PATH="$(find . -type f \( -name 'doom-ascii' -o -name 'doom_ascii' \) | sed -n '1p')"
if [ -z "$BIN_PATH" ]; then
	echo "Failed to locate built doom-ascii binary" >&2
	exit 1
fi
cp "$BIN_PATH" "$BIN_DIR/doom_ascii"
chmod +x "$BIN_DIR/doom_ascii"

echo "Built doom_ascii to $BIN_DIR/doom_ascii"

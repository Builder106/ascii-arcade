#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
	echo "usage: $0 UPSTREAM_CHECKOUT VERSION OUTPUT_ARCHIVE" >&2
	exit 2
fi

ROOT="$(cd "$(dirname "$0")/.."; pwd)"
SOURCE_DIR="$(cd "$1" && pwd)"
VERSION="$2"
OUTPUT="$3"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
	echo "error: VERSION must be X.Y.Z" >&2
	exit 2
fi
if [ ! -f "$SOURCE_DIR/Makefile" ]; then
	echo "error: upstream checkout has no Makefile" >&2
	exit 1
fi

SOURCE_COMMIT="$(git -C "$SOURCE_DIR" rev-parse HEAD)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

git -C "$SOURCE_DIR" archive --format=tar --prefix=doom-ascii/ HEAD \
	| tar -xf - -C "$STAGE"
mkdir -p "$STAGE/ascii-arcade/scripts" "$STAGE/ascii-arcade/LICENSES"
cp "$ROOT/scripts/setup.sh" "$STAGE/ascii-arcade/scripts/setup.sh"
cp "$ROOT/LICENSES/doom-ascii.GPL-2.0" \
	"$STAGE/ascii-arcade/LICENSES/doom-ascii.GPL-2.0"

cat > "$STAGE/README.txt" <<EOF
DOOM engine source for ASCII Arcade v$VERSION

This archive contains the exact upstream source used for the macOS release,
the matching GPL-2.0 license, and the setup script used to build the binary.

Upstream: https://github.com/wojciech-graj/doom-ascii
Upstream commit: $SOURCE_COMMIT

To rebuild the universal macOS binary, extract this archive and run:

  cd ascii-arcade
  DOOM_ASCII_SOURCE_DIR="\$PWD/../doom-ascii" \\
  DOOM_ASCII_CFLAGS='-O2 -w -arch arm64 -arch x86_64' \\
    ./scripts/setup.sh

The resulting binary is written to ascii-arcade/bin/doom_ascii.
EOF

mkdir -p "$(dirname "$OUTPUT")"
tar -C "$STAGE" -czf "$OUTPUT" .
echo "Packaged doom_ascii source commit $SOURCE_COMMIT in $OUTPUT"

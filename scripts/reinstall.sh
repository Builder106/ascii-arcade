#!/bin/zsh
set -euo pipefail

# Fast local reinstall: build → replace /Applications/ASCII Arcade.app → relaunch.
# Skips the DMG entirely — DMG is for distribution, not for your own machine.
#
# Usage:
#   ./scripts/reinstall.sh             # bundles DOOM by default
#   INCLUDE_DOOM=0 ./scripts/reinstall.sh   # skip the DOOM binary

ROOT="$(cd "$(dirname "$0")"/.. ; pwd)"
DEST="/Applications/ASCII Arcade.app"
SCRATCH="${SCRATCH_PATH:-/tmp/aa-build}"
SIGN_IDENTITY="${SIGN_IDENTITY:-ASCII Arcade Local}"

# Self-heal: without a stable signing identity, builds sign ad-hoc and macOS
# re-prompts for Accessibility on every rebuild. Create it once if it's gone.
if ! security find-identity -p codesigning 2>/dev/null | grep -qF "$SIGN_IDENTITY"; then
	echo "No '$SIGN_IDENTITY' signing identity — creating it (one-time)…"
	SIGN_IDENTITY="$SIGN_IDENTITY" "$ROOT/scripts/setup-signing.sh"
fi

echo "Quitting ASCII Arcade…"
osascript -e 'tell application "ASCII Arcade" to quit' 2>/dev/null || true
sleep 1

echo "Building…"
# Bundle DOOM by default for local reinstalls — a plain run should never ship a
# DOOM-less app. Set INCLUDE_DOOM=0 to skip it. (The DMG/make-app.sh default
# stays 0, since redistributing the GPL binary obliges shipping its source.)
SCRATCH_PATH="$SCRATCH" INCLUDE_DOOM="${INCLUDE_DOOM:-1}" "$ROOT/scripts/make-app.sh"

echo "Installing to $DEST…"
rm -rf "$DEST"
ditto "$ROOT/dist/ASCII Arcade.app" "$DEST"

echo "Launching…"
open "$DEST"
echo "Done."

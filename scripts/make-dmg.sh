#!/bin/zsh
set -euo pipefail

# Package the assembled .app into a distributable DMG with an Applications
# drop-target and first-launch instructions. Run scripts/make-app.sh first.
#
# Env overrides:
#   OUT_DIR   where the .app is and the .dmg lands (default: ./dist)

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"
APP="$OUT_DIR/ASCII Arcade.app"
VOL="ASCII Arcade"
DMG="$OUT_DIR/ASCII-Arcade.dmg"

[ -d "$APP" ] || { echo "no app at $APP — run scripts/make-app.sh first" >&2; exit 1; }

STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

cat > "$STAGE/READ ME — first launch.txt" <<TXT
ASCII Arcade is not notarized (no paid Apple Developer account), so Gatekeeper
blocks the very first launch. To open it:

  1. Drag "ASCII Arcade.app" onto the Applications folder.
  2. In Applications, right-click (Control-click) "ASCII Arcade" -> Open.
  3. Click "Open" in the dialog. macOS remembers it; later launches are normal.

Or, in Terminal:
  xattr -dr com.apple.quarantine "/Applications/ASCII Arcade.app"

Then look for the round (circle) icon in your menu bar to pick a scene.
TXT

rm -f "$DMG"
hdiutil create -volname "$VOL" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"
echo "Built $DMG"

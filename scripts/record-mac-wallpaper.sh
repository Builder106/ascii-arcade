#!/usr/bin/env bash
# Records the macOS app's wallpaper window, and nothing else on screen, while
# it steps through scenes and themes. This is the take the Tier 2 UI demo in
# ui-demo/ is cut from.
#
#   bash scripts/record-mac-wallpaper.sh [out.mov]
#
# Needs ASCII Arcade running, plus Accessibility and Screen Recording
# permission for whatever runs this script. It presses the ◎ menu's items
# through Accessibility, so the menu never opens and no keys are sent, and
# `screencapture -l` grabs only the wallpaper window, so other windows never
# show up. The scene, theme and auto-cycle you had are put back at the end.
set -euo pipefail

out="${1:-e2e/recordings/footage/mac-wallpaper.mov}"
mkdir -p "$(dirname "$out")"

ui='tell application "System Events" to tell process "AsciiArcade" to'
item='of menu 1 of menu bar item 1 of menu bar 1'
press() { osascript -e "$ui perform action \"AXPress\" of menu item \"$1\" $item" >/dev/null; }
# An unchecked item reads back as "missing value", so compare against the mark.
checked() { [ "$(osascript -e "$ui value of attribute \"AXMenuItemMarkChar\" of menu item \"$1\" $item" 2>/dev/null)" = "✓" ]; }
current() {
  local name
  for name in "$@"; do checked "$name" && { echo "$name"; return; }; done
}

# The wallpaper is the app's only window, at desktop level.
window=$(osascript -l JavaScript -e '
  ObjC.import("CoreGraphics");
  var list = $.CGWindowListCopyWindowInfo($.kCGWindowListOptionAll, 0);
  var id = "";
  for (var i = 0; i < $.CFArrayGetCount(list); i++) {
    var w = ObjC.deepUnwrap(ObjC.castRefToObject($.CFArrayGetValueAtIndex(list, i)));
    if (w.kCGWindowOwnerName == "ASCII Arcade") id = w.kCGWindowNumber;
  }
  id')
[ -n "$window" ] || { echo "ASCII Arcade isn't running" >&2; exit 1; }

scene=$(current Donut Helix Matrix Life Pipes)
theme=$(current Hacker Amber Ice Ghost)
cycling=no
checked "Auto-cycle when idle" && cycling=yes
doom=no
checked "Enable DOOM Scene" && doom=yes

restore() {
  checked "Enable DOOM Scene" && [ "$doom" = no ] && press "Enable DOOM Scene"
  [ -n "$scene" ] && press "$scene"
  [ -n "$theme" ] && press "$theme"
  if checked "Auto-cycle when idle"; then [ "$cycling" = no ] && press "Auto-cycle when idle"
  else [ "$cycling" = yes ] && press "Auto-cycle when idle"; fi
  echo "restored ${scene:-?} / ${theme:-?}, auto-cycle $cycling"
}
trap restore EXIT

# Auto-cycle would change scenes on its own mid-take.
checked "Auto-cycle when idle" && press "Auto-cycle when idle"
press Hacker
press Donut
sleep 1.5

screencapture -x -v -V82 -l"$window" "$out" &
recording=$!
start=$(date +%s)
at() { while [ $(( $(date +%s) - start )) -lt "$1" ]; do sleep 0.1; done; }

at 7;  press Helix
at 13; press Matrix
at 19; press Life
at 25; press Pipes
at 31; press Matrix; press Amber
at 37; press Donut;  press Ice
at 43; press Helix;  press Ghost
at 49; press Life;   press Hacker
# DOOM's own demo plays with no input, so key capture stays off.
at 55; checked "Enable DOOM Scene" || press "Enable DOOM Scene"
sleep 0.5
press DOOM
at 72; press Matrix
wait "$recording"
echo "wrote $out"

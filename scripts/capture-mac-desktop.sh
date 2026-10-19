#!/usr/bin/env bash
# Captures the rest of the Mac around the wallpaper as stills for the Tier 2
# UI demo: the menu bar, the Dock, and the ◎ menu open in each state the demo
# switches through, with the item about to be picked highlighted.
#
#   bash scripts/capture-mac-desktop.sh [out-dir]
#
# Run it with ASCII Arcade running and Accessibility and Screen Recording
# allowed. It brings Finder forward so the menu bar reads "Finder", slides
# your windows aside with Show Desktop, and opens the ◎ menu eight times, so
# leave the Mac alone for the 20 seconds or so it takes. Windows, scene and
# theme go back afterwards. scripts/cut-video-clips.sh crops the stills.
set -uo pipefail

out="${1:-e2e/recordings/footage/mac-desktop}"
mkdir -p "$out"

ui='tell application "System Events" to tell process "AsciiArcade" to'
item='of menu 1 of menu bar item 1 of menu bar 1'
press() { osascript -e "$ui perform action \"AXPress\" of menu item \"$1\" $item" >/dev/null 2>&1; }
checked() { [ "$(osascript -e "$ui value of attribute \"AXMenuItemMarkChar\" of menu item \"$1\" $item" 2>/dev/null)" = "✓" ]; }
escape() { osascript -e 'tell application "System Events" to key code 53' >/dev/null 2>&1; }
show_desktop() { "/System/Applications/Mission Control.app/Contents/MacOS/Mission Control" 1 2>/dev/null; }

scene=""; for s in Donut Helix Matrix Life Pipes; do checked "$s" && scene=$s; done
theme=""; for t in Hacker Amber Ice Ghost; do checked "$t" && theme=$t; done
doom=no; checked "Enable DOOM Scene" && doom=yes

restore() {
  escape
  if checked "Enable DOOM Scene"; then [ "$doom" = no ] && press "Enable DOOM Scene"; fi
  [ -n "$scene" ] && press "$scene"
  [ -n "$theme" ] && press "$theme"
  show_desktop
  echo "restored ${scene:-?} / ${theme:-?}"
}
trap restore EXIT

osascript -e 'tell application "Finder" to activate'
sleep 0.8
show_desktop
sleep 1.5

# Screen points: the menu bar is the top 24, the Dock sits in the bottom 140,
# and the ◎ menu hangs below its icon at x 961.
screencapture -x -R0,0,1440,24 "$out/menubar.png"
screencapture -x -R0,760,1440,140 "$out/dock-strip.png"

# Opens the menu, highlights the item the demo picks next the way a hover
# would, and grabs the icon and menu together.
menu() {
  (osascript -e "$ui perform action \"AXPress\" of menu bar item 1 of menu bar 1" >/dev/null 2>&1 &)
  sleep 0.9
  osascript -e "$ui set value of attribute \"AXSelected\" of menu item \"$2\" $item to true" >/dev/null 2>&1
  sleep 0.3
  screencapture -x -R930,0,300,660 "$out/menu-$1.png"
  escape
  sleep 0.5
}

press Hacker
press Donut;  sleep 0.4; menu helix  Helix
press Helix;  sleep 0.4; menu matrix Matrix
press Matrix; sleep 0.4; menu life   Life
press Life;   sleep 0.4; menu pipes  Pipes
press Pipes;  sleep 0.4; menu amber  Amber
press Matrix; press Amber;  sleep 0.4; menu ice   Ice
press Donut;  press Ice;    sleep 0.4; menu ghost Ghost
press Life;   press Hacker; checked "Enable DOOM Scene" || press "Enable DOOM Scene"
sleep 0.6; menu doom DOOM
echo "wrote stills to $out"

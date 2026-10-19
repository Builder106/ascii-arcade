#!/usr/bin/env bash
# Cuts the shots the HyperFrames videos use out of the footage takes, into
# trailer/public/clips/ and ui-demo/public/clips/. The trailer's takes come
# from e2e/capture/footage.mjs; the UI demo's Mac take comes from
# scripts/record-mac-wallpaper.sh. Each clip runs a little past the length its
# composition shows, so the renderer never reaches the end of a source.
# An optional filter crops a shot to the part its composition shows.
#
#   bash scripts/cut-video-clips.sh [footage-dir]
#
# The footage directory defaults to e2e/recordings/footage.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
footage="${1:-$root/e2e/recordings/footage}"
tail_pad=0.3

# project clip      take           start  length  [filter]
shots="
trailer  power     attract        0.00   7.01
trailer  desk      wall           8.90   6.78
trailer  m1        wall           27.20  1.05
trailer  m2        wall           31.20  1.04
trailer  m3        wall           35.20  1.05
trailer  m4        wall           39.20  1.02
trailer  m5        wall           44.80  1.05
trailer  m6        wall           47.20  1.04
trailer  doom-in   doom           2.62   1.58
trailer  doom-play doom           7.00   5.74
trailer  snap      player         31.83  6.26
trailer  coin      attract        55.00  5.88
ui-demo  donut     mac-wallpaper  2.00   7.50   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  matrix    mac-wallpaper  11.10  3.74   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  life      mac-wallpaper  17.10  3.67   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  pipes     mac-wallpaper  23.10  4.41   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  amber     mac-wallpaper  29.10  3.30   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  ice       mac-wallpaper  35.10  3.32   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  ghost     mac-wallpaper  41.10  3.29   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  doom-in   mac-wallpaper  54.10  2.49   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
ui-demo  doom-play mac-wallpaper  60.30  5.36   crop=2880:1620:0:90,scale=1920:1080:flags=lanczos
"

while read -r project clip take start length filter; do
  [ -z "$project" ] && continue
  out="$root/$project/public/clips/$clip.mp4"
  src="$footage/$take.mp4"
  [ -f "$src" ] || src="$footage/$take.mov"
  mkdir -p "$(dirname "$out")"
  ffmpeg -nostdin -y -loglevel error -ss "$start" -i "$src" \
    -t "$(echo "$length + $tail_pad" | bc)" -an -r 30 ${filter:+-vf "$filter"} \
    -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -movflags +faststart "$out"
  echo "$project/public/clips/$clip.mp4"
done <<<"$shots"

# The UI demo's desktop stills from scripts/capture-mac-desktop.sh, cropped
# to the menu bar, the Dock, the open ◎ icon and each menu (Retina pixels).
stills="$footage/mac-desktop"
if [ -d "$stills" ]; then
  desk="$root/ui-demo/public/desktop"
  mkdir -p "$desk"
  crop() { ffmpeg -nostdin -y -loglevel error -i "$stills/$1" -vf "crop=$2" "$desk/$3"; echo "ui-demo/public/desktop/$3"; }
  cp "$stills/menubar.png" "$desk/menubar.png"
  crop dock-strip.png 1656:173:612:97 dock.png
  crop menu-helix.png 110:48:30:0 status-open.png
  for m in helix matrix life pipes amber ice ghost; do crop "menu-$m.png" 506:1116:38:62 "menu-$m.png"; done
  crop menu-doom.png 506:1212:38:62 menu-doom.png
fi

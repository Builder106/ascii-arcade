#!/usr/bin/env bash
# Cuts the shots the HyperFrames videos use out of the takes that
# e2e/capture/footage.mjs records, into trailer/public/clips/ and
# ui-demo/public/clips/. Each clip runs a little past the length its
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
"

while read -r project clip take start length filter; do
  [ -z "$project" ] && continue
  out="$root/$project/public/clips/$clip.mp4"
  mkdir -p "$(dirname "$out")"
  ffmpeg -nostdin -y -loglevel error -ss "$start" -i "$footage/$take.mp4" \
    -t "$(echo "$length + $tail_pad" | bc)" -an -r 30 ${filter:+-vf "$filter"} \
    -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -movflags +faststart "$out"
  echo "$project/public/clips/$clip.mp4"
done <<<"$shots"

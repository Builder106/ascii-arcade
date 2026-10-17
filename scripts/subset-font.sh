#!/usr/bin/env bash
# Subset the site's three typefaces to the glyphs they actually draw: IBM
# Plex Mono for the grid, Handjet for the body copy, Pixelify Sans for the
# display type.
#
# Plex Mono is one cut, the 400, which carries the live grid and stands in
# for the symbols Handjet lacks: printable ASCII covers the luminance ramp,
# the Matrix alphabet and the boot screen, and the extras are Life's block,
# Pipes' box drawing, arrows, the multiplication sign and typographic quotes. Without the block and box glyphs
# those two scenes render as gaps. Everything bold is Pixelify Sans, so there
# is no bold Plex cut. Plex Mono has no ⌘ ⌥ ⇧ or ◎; those fall back to the system
# monospace, which on a Mac is the font the real menu bar uses anyway. See
# docs/landing-page-design.md.
#
# Source is the npm package on jsDelivr, pinned. IBM publishes only woff2 there
# (no ttf or otf), so pyftsubset needs the [woff] extra to read the input.
set -euo pipefail

PLEX_VERSION="2.5.0"
SRC_BASE="https://cdn.jsdelivr.net/npm/@ibm/plex-mono@${PLEX_VERSION}/fonts/complete/woff2"
OFL_URL="https://raw.githubusercontent.com/IBM/plex/master/LICENSE.txt"

# Pixelify Sans sets the headline, station headings, buttons and the header
# brand, and the cabinet's marquee and labels. It is a variable font (weight
# 400 to 700); the subset keeps the weight axis. Source is the Fontsource
# package on jsDelivr, pinned, which ships the Latin range as woff2.
PIXELIFY_VERSION="5.3.0"
PIXELIFY_BASE="https://cdn.jsdelivr.net/npm/@fontsource-variable/pixelify-sans@${PIXELIFY_VERSION}"

# Handjet, the LED dot-matrix face, sets the body copy, notes and the status
# line. Its glyphs are built from one reused element, so the subset is tiny.
# It ships as a single static instance at weight 500: at 400 the elements run
# together at body size. Source is the Fontsource package on jsDelivr, pinned.
HANDJET_VERSION="5.3.0"
HANDJET_BASE="https://cdn.jsdelivr.net/npm/@fontsource-variable/handjet@${HANDJET_VERSION}"
HANDJET_WEIGHT=500

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/site/fonts"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# U+0020-007E  printable ASCII
# U+00D7       multiplication sign, terminal sizes like 72×18
# U+2190-2193  arrows, DOOM controls
# U+2197       north-east arrow, outbound links
# U+2713       check mark, menu state
# U+2019 201C 201D 2026  apostrophe, quotes, ellipsis
# U+2588       full block, Life
# U+2500 2502 250C 2510 2514 2518  box drawing, Pipes
GRID='U+0020-007E,U+00D7,U+2190-2193,U+2197,U+2713,U+2019,U+201C,U+201D,U+2026,U+2588,U+2500,U+2502,U+250C,U+2510,U+2514,U+2518'
DISPLAY='U+0020-007E,U+2019,U+201C,U+201D,U+2026'

mkdir -p "$OUT"

subset() {
  local cut="$1" weight="$2" unicodes="$3"
  curl -fsSL -o "$WORK/$cut.woff2" "$SRC_BASE/IBMPlexMono-$cut.woff2"
  uv tool run --from "fonttools[woff]" pyftsubset "$WORK/$cut.woff2" \
    --output-file="$OUT/IBMPlexMono-$weight-subset.woff2" \
    --flavor=woff2 \
    --layout-features='kern' \
    --no-hinting \
    --desubroutinize \
    --unicodes="$unicodes"
}

subset Regular 400 "$GRID"

curl -fsSL -o "$WORK/pixelify.woff2" "$PIXELIFY_BASE/files/pixelify-sans-latin-wght-normal.woff2"
uv tool run --from "fonttools[woff]" pyftsubset "$WORK/pixelify.woff2" \
  --output-file="$OUT/PixelifySans-subset.woff2" \
  --flavor=woff2 \
  --layout-features='kern' \
  --no-hinting \
  --desubroutinize \
  --unicodes="$DISPLAY"

curl -fsSL -o "$WORK/handjet.woff2" "$HANDJET_BASE/files/handjet-latin-wght-normal.woff2"
uv tool run --from "fonttools[woff]" fonttools varLib.instancer "$WORK/handjet.woff2" \
  "wght=$HANDJET_WEIGHT" -o "$WORK/handjet-static.woff2"
uv tool run --from "fonttools[woff]" pyftsubset "$WORK/handjet-static.woff2" \
  --output-file="$OUT/Handjet-subset.woff2" \
  --flavor=woff2 \
  --layout-features='kern' \
  --no-hinting \
  --desubroutinize \
  --unicodes="$DISPLAY"

curl -fsSL -o "$OUT/OFL.txt" "$OFL_URL"
curl -fsSL -o "$OUT/OFL-PixelifySans.txt" "$PIXELIFY_BASE/LICENSE"
curl -fsSL -o "$OUT/OFL-Handjet.txt" "$HANDJET_BASE/LICENSE"

ls -lh "$OUT"

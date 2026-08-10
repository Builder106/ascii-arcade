#!/usr/bin/env bash
# Subset IBM Plex Mono to the glyphs the site actually draws.
#
# Printable ASCII covers the luminance ramp, the Matrix alphabet and all page
# copy. The seven extras are Life's block and Pipes' box drawing; without them
# those two scenes render as gaps. See docs/landing-page-design.md.
#
# Source is the npm package on jsDelivr, pinned. IBM publishes only woff2 there
# (no ttf or otf), so pyftsubset needs the [woff] extra to read the input.
set -euo pipefail

PLEX_VERSION="2.5.0"
SRC_URL="https://cdn.jsdelivr.net/npm/@ibm/plex-mono@${PLEX_VERSION}/fonts/complete/woff2/IBMPlexMono-Regular.woff2"
OFL_URL="https://raw.githubusercontent.com/IBM/plex/master/LICENSE.txt"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/site/fonts"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$OUT"

curl -fsSL -o "$WORK/IBMPlexMono-Regular.woff2" "$SRC_URL"

# U+0020-007E  printable ASCII
# U+2588       full block, Life
# U+2500 2502  horizontal and vertical, Pipes
# U+250C 2510  top corners, Pipes
# U+2514 2518  bottom corners, Pipes
uv tool run --from "fonttools[woff]" pyftsubset "$WORK/IBMPlexMono-Regular.woff2" \
  --output-file="$OUT/IBMPlexMono-subset.woff2" \
  --flavor=woff2 \
  --layout-features='' \
  --unicodes='U+0020-007E,U+2588,U+2500,U+2502,U+250C,U+2510,U+2514,U+2518'

curl -fsSL -o "$OUT/OFL.txt" "$OFL_URL"

echo "source:  $(du -h "$WORK/IBMPlexMono-Regular.woff2" | cut -f1)"
echo "subset:  $(du -h "$OUT/IBMPlexMono-subset.woff2" | cut -f1)"
ls -lh "$OUT"

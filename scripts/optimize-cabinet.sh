#!/usr/bin/env bash
# Compress the cabinet model exported from assets/cabinet/arcade-cabinet.blend
# into site/models/cabinet.glb.
#
# Export the GLB from Blender first (the COL_Cabinet collection, glTF Binary,
# +Y up, modifiers applied), then run this on the build host, never on the
# Mac. Meshopt rather than Draco: its decoder ships with three.js, so there
# are no decoder files to host.
#
# Usage: scripts/optimize-cabinet.sh path/to/cabinet-raw.glb
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_VERSION="4.5.1"
IN="${1:?usage: optimize-cabinet.sh path/to/cabinet-raw.glb}"
OUT="$ROOT/site/models/cabinet.glb"

mkdir -p "$(dirname "$OUT")"
# The page finds nodes by name (SCREEN, STICK, BUTTON_0, the PART_* groups,
# the SCREEN_CENTER/UP/OUT empties) and paints its own textures onto UVs that
# no material in the file references, so nothing is joined or pruned.
# De-duplication still merges materials with identical settings, so every
# placeholder material the page repaints (MAT_Price, MAT_CoinLabel,
# MAT_Marquee and the rest) must differ in the .blend, even slightly.
npx --yes "@gltf-transform/cli@$CLI_VERSION" optimize "$IN" "$OUT" \
  --compress meshopt --join false --flatten false --instance false --palette false \
  --simplify false --texture-compress false --prune false
npx --yes "@gltf-transform/cli@$CLI_VERSION" inspect "$OUT" --format md | head -40
node "$ROOT/scripts/check-cabinet-model.mjs" "$OUT"
printf 'built: %s (%s bytes)\n' "$OUT" "$(wc -c < "$OUT")"

#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 NEW_OUTPUT_DIRECTORY" >&2
  exit 2
fi
OUT="$1"
if [ -e "$OUT" ] || [ -L "$OUT" ]; then
  echo "Output already exists: $OUT" >&2
  exit 1
fi

bash "$ROOT/scripts/build-wasm.sh"
mkdir -p "$OUT/pkg"
cp "$ROOT/shells/wallpaper-engine/index.html" "$ROOT/shells/wallpaper-engine/wallpaper.js" "$OUT/"
sed 's/^export class Grid/class Grid/' "$ROOT/site/renderer.js" > "$OUT/renderer.js"
wasm-bindgen --target no-modules --no-typescript --out-dir "$OUT/pkg" \
  "${CARGO_TARGET_DIR:-$ROOT/target}/wasm32-unknown-unknown/release/aa_wasm.wasm"
python3 - "$OUT/pkg/aa_wasm_bg.wasm" "$OUT/pkg/wasm-bytes.js" <<'PY'
import base64
import pathlib
import sys

encoded = base64.b64encode(pathlib.Path(sys.argv[1]).read_bytes()).decode("ascii")
pathlib.Path(sys.argv[2]).write_text(f'const wallpaperWasmBase64 = "{encoded}";\n')
PY
echo "Wallpaper files created in $OUT"

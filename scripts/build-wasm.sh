#!/usr/bin/env bash
# Build aa-wasm for the browser into site/pkg/.
#
# Runs on ampere-dev, never on the Mac: it produces target/ and needs the
# wasm32 toolchain. See docs/landing-page-design.md.
#
# The CLI version is read from crates/aa-wasm/Cargo.toml rather than hardcoded.
# wasm-bindgen aborts if the CLI and the crate disagree, and having the two
# pinned in separate places is how they end up disagreeing.
#
# wasm-opt then runs at -O3, which optimizes for speed and still cuts the
# module by about a quarter: rustc's output leaves a lot for binaryen to fold.
# That headroom is what keeps the first load under its 150 kB budget.
set -euo pipefail

BINARYEN_VERSION="132.0.0"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/site/pkg"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

WB_VERSION="$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' "$ROOT/crates/aa-wasm/Cargo.toml")"
if [ -z "$WB_VERSION" ]; then
  echo "could not read the wasm-bindgen pin from crates/aa-wasm/Cargo.toml" >&2
  exit 1
fi

rustup target add wasm32-unknown-unknown

if ! wasm-bindgen --version 2>/dev/null | grep -q "$WB_VERSION"; then
  cargo install wasm-bindgen-cli --version "$WB_VERSION" --locked
fi

cargo build -p aa-wasm --release --target wasm32-unknown-unknown

mkdir -p "$OUT"
wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir "$OUT" \
  "$TARGET_DIR/wasm32-unknown-unknown/release/aa_wasm.wasm"

npx -y -p "binaryen@$BINARYEN_VERSION" wasm-opt -O3 \
  --enable-bulk-memory \
  --enable-nontrapping-float-to-int \
  --enable-sign-ext \
  --enable-mutable-globals \
  "$OUT/aa_wasm_bg.wasm" -o "$OUT/aa_wasm_bg.wasm"

echo "built: $OUT"
ls -lh "$OUT"

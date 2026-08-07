#!/usr/bin/env bash
# Build aa-wasm for the browser into site/pkg/.
#
# Runs on ampere-dev, never on the Mac: it produces target/ and needs the
# wasm32 toolchain. See docs/landing-page-design.md.
#
# The CLI version is read from crates/aa-wasm/Cargo.toml rather than hardcoded.
# wasm-bindgen aborts if the CLI and the crate disagree, and having the two
# pinned in separate places is how they end up disagreeing.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/site/pkg"

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
  "$ROOT/target/wasm32-unknown-unknown/release/aa_wasm.wasm"

echo "built: $OUT"
ls -lh "$OUT"

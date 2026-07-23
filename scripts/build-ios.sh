#!/usr/bin/env bash
# Build aa-ffi as an XCFramework for the native iOS shell.
#
# Outputs:
#   shells/ios/Frameworks/AaEngine.xcframework
#
# Prerequisites:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#   Xcode command-line tools (xcodebuild)
#   cbindgen: cargo install cbindgen
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="aa-ffi"
LIB_NAME="libaa_ffi.a"
HEADERS_DIR="$REPO_ROOT/crates/aa-ffi"
OUT_DIR="$REPO_ROOT/shells/ios/Frameworks"
XCFRAMEWORK="$OUT_DIR/AaEngine.xcframework"

cd "$REPO_ROOT"

if ! command -v cbindgen &>/dev/null; then
  echo "cbindgen not found — installing..."
  cargo install cbindgen
  export PATH="$HOME/.cargo/bin:$PATH"
fi

echo "── Regenerating aa_engine.h from aa-ffi/src/lib.rs ──────────────"
cbindgen --config "$HEADERS_DIR/cbindgen.toml" --crate "$CRATE" --output "$HEADERS_DIR/aa_engine.h"

echo "── Installing Rust iOS targets ──────────────────────────────────"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

# aa-ffi's Cargo.toml declares crate-type = ["cdylib"] (what Android's
# cargo-ndk build needs); `cargo rustc -- --crate-type staticlib` overrides
# that for this invocation only, producing the .a iOS links against without
# also building a cdylib nobody on this platform uses.
echo "── Building for aarch64-apple-ios (device) ──────────────────────"
cargo rustc -p "$CRATE" --target aarch64-apple-ios --release -- --crate-type staticlib

echo "── Building for aarch64-apple-ios-sim (Apple Silicon sim) ──────"
cargo rustc -p "$CRATE" --target aarch64-apple-ios-sim --release -- --crate-type staticlib

echo "── Building for x86_64-apple-ios (Intel sim) ───────────────────"
cargo rustc -p "$CRATE" --target x86_64-apple-ios --release -- --crate-type staticlib

echo "── Lipo-ing simulator slices ────────────────────────────────────"
LIPO_SIM_DIR="$REPO_ROOT/target/lipo-ios-sim/release"
mkdir -p "$LIPO_SIM_DIR"
lipo -create \
  "$REPO_ROOT/target/aarch64-apple-ios-sim/release/$LIB_NAME" \
  "$REPO_ROOT/target/x86_64-apple-ios/release/$LIB_NAME" \
  -output "$LIPO_SIM_DIR/$LIB_NAME"

echo "── Assembling XCFramework ───────────────────────────────────────"
mkdir -p "$OUT_DIR"
rm -rf "$XCFRAMEWORK"
xcodebuild -create-xcframework \
  -library "$REPO_ROOT/target/aarch64-apple-ios/release/$LIB_NAME" \
  -headers "$HEADERS_DIR" \
  -library "$LIPO_SIM_DIR/$LIB_NAME" \
  -headers "$HEADERS_DIR" \
  -output "$XCFRAMEWORK"

echo "── Done ─────────────────────────────────────────────────────────"
echo "XCFramework: $XCFRAMEWORK"

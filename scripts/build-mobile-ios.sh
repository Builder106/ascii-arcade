#!/usr/bin/env bash
# Build aa-ffi as an XCFramework for the Expo iOS native module.
#
# Outputs:
#   shells/mobile/modules/aa-engine/ios/AaEngine.xcframework
#
# Prerequisites:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#   Xcode command-line tools (xcodebuild)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="aa-ffi"
LIB_NAME="libaa_ffi.a"
OUT_DIR="$REPO_ROOT/shells/mobile/modules/aa-engine/ios"
XCFRAMEWORK="$OUT_DIR/AaEngine.xcframework"

cd "$REPO_ROOT"

echo "── Installing Rust iOS targets ──────────────────────────────────"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

echo "── Building for aarch64-apple-ios (device) ──────────────────────"
cargo build -p "$CRATE" --target aarch64-apple-ios --release

echo "── Building for aarch64-apple-ios-sim (Apple Silicon sim) ──────"
cargo build -p "$CRATE" --target aarch64-apple-ios-sim --release

echo "── Building for x86_64-apple-ios (Intel sim) ───────────────────"
cargo build -p "$CRATE" --target x86_64-apple-ios --release

echo "── Lipo-ing simulator slices into a fat lib ─────────────────────"
LIPO_SIM_DIR="$REPO_ROOT/target/lipo-ios-sim/release"
mkdir -p "$LIPO_SIM_DIR"
lipo -create \
  "$REPO_ROOT/target/aarch64-apple-ios-sim/release/$LIB_NAME" \
  "$REPO_ROOT/target/x86_64-apple-ios/release/$LIB_NAME" \
  -output "$LIPO_SIM_DIR/$LIB_NAME"

echo "── Assembling XCFramework ───────────────────────────────────────"
rm -rf "$XCFRAMEWORK"
xcodebuild -create-xcframework \
  -library "$REPO_ROOT/target/aarch64-apple-ios/release/$LIB_NAME" \
  -headers "$REPO_ROOT/crates/aa-ffi" \
  -library "$LIPO_SIM_DIR/$LIB_NAME" \
  -headers "$REPO_ROOT/crates/aa-ffi" \
  -output "$XCFRAMEWORK"

echo "── Done ─────────────────────────────────────────────────────────"
echo "XCFramework: $XCFRAMEWORK"

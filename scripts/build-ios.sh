#!/usr/bin/env bash
# Build aa-ffi as an XCFramework for the native iOS shell.
#
# Outputs:
#   shells/ios/Frameworks/AaEngine.xcframework
#
# Set AA_IOS_BUILD_ROOT to keep intermediate files outside the checkout.
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
  cargo install cbindgen --locked
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
fi

echo "── Regenerating aa_engine.h from aa-ffi/src/lib.rs ──────────────"
cbindgen --config "$HEADERS_DIR/cbindgen.toml" --crate "$CRATE" --output "$HEADERS_DIR/aa_engine.h"

BUILD_ROOT="${AA_IOS_BUILD_ROOT:-${CARGO_TARGET_DIR:-$REPO_ROOT/target/ios}}"
HEADER_STAGE="$BUILD_ROOT/headers"
rm -rf "$HEADER_STAGE"
mkdir -p "$HEADER_STAGE"
cp "$HEADERS_DIR/aa_engine.h" "$HEADER_STAGE/aa_engine.h"
cp "$HEADERS_DIR/module.modulemap" "$HEADER_STAGE/module.modulemap"

if [ "$(uname -s)" = "Linux" ]; then
  DARWIN_SDK_ROOT="${AA_DARWIN_SDK_ROOT:-$HOME/platform/sdk/xtool/darwin.xtoolsdk}"
  TOOLSET_BIN="$DARWIN_SDK_ROOT/toolset/bin"
  DARWIN_CLANG="${AA_DARWIN_CLANG:-$(command -v clang || true)}"
  if [ -z "$DARWIN_CLANG" ] && [ -x "$HOME/platform/tools/swiftly/bin/clang" ]; then
    DARWIN_CLANG="$HOME/platform/tools/swiftly/bin/clang"
  fi
  IOS_SDK_PATH="$DARWIN_SDK_ROOT/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk"
  IOS_SDK_VERSION="$(sed -n 's/.*\"Version\":\"\([^\"]*\)\".*/\1/p' "$IOS_SDK_PATH/SDKSettings.json")"
  DARWIN_LD="$TOOLSET_BIN/ld64.lld"
  if [ ! -d "$IOS_SDK_PATH" ] || [ -z "$IOS_SDK_VERSION" ] || [ -z "$DARWIN_CLANG" ] || [ ! -x "$DARWIN_LD" ]; then
    echo "Darwin SDK compiler and linker not found at $DARWIN_SDK_ROOT — install it with scripts/build-xtool-ios.sh." >&2
    exit 1
  fi
  export SDKROOT="$IOS_SDK_PATH"
  export PATH="$TOOLSET_BIN:$PATH"
  export CC_aarch64_apple_ios="$DARWIN_CLANG"
  export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$DARWIN_CLANG"
  export CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS="-C link-arg=-fuse-ld=$DARWIN_LD -C link-arg=-isysroot -C link-arg=$IOS_SDK_PATH -C link-arg=-Wl,-syslibroot,$IOS_SDK_PATH -C link-arg=-arch -C link-arg=arm64 -C link-arg=-Wl,-platform_version,ios,12.0,$IOS_SDK_VERSION"
fi

echo "── Installing Rust iOS targets ──────────────────────────────────"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

# aa-ffi's Cargo.toml declares crate-type = ["cdylib"] for Android. The
# `cargo rustc` invocation also requests a staticlib for iOS. Cargo may emit
# both formats, so the Linux path configures the Darwin linker for the extra
# output and copies only the static archive into the XCFramework.
#
# Each target gets its own --target-dir. On an Apple Silicon runner,
# aarch64-apple-ios-sim shares a CPU arch with the host, and Cargo's unit
# fingerprint doesn't account for the --crate-type override — reusing one
# target/ directory across these invocations can silently skip emitting the
# .a lipo expects. Isolated target dirs sidestep the collision.
#
# CI's rust-cache@v2 caches the entire target/ directory. Since Cargo's
# fingerprint doesn't track --crate-type overrides, a cached build can
# silently provide a cdylib when staticlib is needed. Clean the isolated
# target dirs first to force a fresh build with the correct crate-type.
DEVICE_TARGET_DIR="$BUILD_ROOT/device"
SIM_ARM_TARGET_DIR="$BUILD_ROOT/sim-arm64"
SIM_X86_TARGET_DIR="$BUILD_ROOT/sim-x86_64"

rm -rf "$DEVICE_TARGET_DIR" "$SIM_ARM_TARGET_DIR" "$SIM_X86_TARGET_DIR"

echo "── Building for aarch64-apple-ios (device) ──────────────────────"
cargo rustc -p "$CRATE" --target aarch64-apple-ios --target-dir "$DEVICE_TARGET_DIR" --release -- --crate-type staticlib

if [ "$(uname -s)" = "Linux" ]; then
  echo "── Assembling Linux-built device XCFramework ───────────────────"
  rm -rf "$XCFRAMEWORK"
  mkdir -p "$XCFRAMEWORK/ios-arm64/Headers"
  cp "$DEVICE_TARGET_DIR/aarch64-apple-ios/release/deps/$LIB_NAME" \
    "$XCFRAMEWORK/ios-arm64/$LIB_NAME"
  cp "$HEADER_STAGE/aa_engine.h" "$XCFRAMEWORK/ios-arm64/Headers/aa_engine.h"
  cp "$HEADER_STAGE/module.modulemap" "$XCFRAMEWORK/ios-arm64/Headers/module.modulemap"
  cp "$REPO_ROOT/scripts/ios-xcframework-info.plist" "$XCFRAMEWORK/Info.plist"
  echo "── Done ─────────────────────────────────────────────────────────"
  echo "XCFramework: $XCFRAMEWORK"
  exit 0
fi

echo "── Building for aarch64-apple-ios-sim (Apple Silicon sim) ──────"
cargo rustc -p "$CRATE" --target aarch64-apple-ios-sim --target-dir "$SIM_ARM_TARGET_DIR" --release -- --crate-type staticlib

echo "── Building for x86_64-apple-ios (Intel sim) ───────────────────"
cargo rustc -p "$CRATE" --target x86_64-apple-ios --target-dir "$SIM_X86_TARGET_DIR" --release -- --crate-type staticlib

echo "── Lipo-ing simulator slices ────────────────────────────────────"
LIPO_SIM_DIR="$BUILD_ROOT/lipo-ios-sim/release"
mkdir -p "$LIPO_SIM_DIR"
lipo -create \
  "$SIM_ARM_TARGET_DIR/aarch64-apple-ios-sim/release/deps/$LIB_NAME" \
  "$SIM_X86_TARGET_DIR/x86_64-apple-ios/release/deps/$LIB_NAME" \
  -output "$LIPO_SIM_DIR/$LIB_NAME"

echo "── Assembling XCFramework ───────────────────────────────────────"
mkdir -p "$OUT_DIR"
rm -rf "$XCFRAMEWORK"
xcodebuild -create-xcframework \
  -library "$DEVICE_TARGET_DIR/aarch64-apple-ios/release/deps/$LIB_NAME" \
  -headers "$HEADER_STAGE" \
  -library "$LIPO_SIM_DIR/$LIB_NAME" \
  -headers "$HEADER_STAGE" \
  -output "$XCFRAMEWORK"

echo "── Done ─────────────────────────────────────────────────────────"
echo "XCFramework: $XCFRAMEWORK"

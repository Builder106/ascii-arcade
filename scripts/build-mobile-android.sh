#!/usr/bin/env bash
# Build aa-ffi as native shared libraries for the Expo Android native module.
#
# Outputs (placed where Gradle picks them up automatically via jniLibs.srcDirs):
#   shells/mobile/modules/aa-engine/android/src/main/jniLibs/arm64-v8a/libaa_ffi.so
#   shells/mobile/modules/aa-engine/android/src/main/jniLibs/armeabi-v7a/libaa_ffi.so
#   shells/mobile/modules/aa-engine/android/src/main/jniLibs/x86_64/libaa_ffi.so
#
# Prerequisites:
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   Android NDK installed; ANDROID_NDK_HOME set (or NDK_HOME).
#   cargo-ndk: cargo install cargo-ndk
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="aa-ffi"
JNILIBS="$REPO_ROOT/shells/mobile/modules/aa-engine/android/src/main/jniLibs"

cd "$REPO_ROOT"

if ! command -v cargo-ndk &>/dev/null; then
  echo "cargo-ndk not found — installing..."
  cargo install cargo-ndk
fi

echo "── Installing Android Rust targets ──────────────────────────────"
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

echo "── Building for arm64-v8a ───────────────────────────────────────"
cargo ndk -t arm64-v8a   -o "$JNILIBS" build -p "$CRATE" --release

echo "── Building for armeabi-v7a ─────────────────────────────────────"
cargo ndk -t armeabi-v7a -o "$JNILIBS" build -p "$CRATE" --release

echo "── Building for x86_64 (emulator) ──────────────────────────────"
cargo ndk -t x86_64      -o "$JNILIBS" build -p "$CRATE" --release

echo "── Done ─────────────────────────────────────────────────────────"
echo "jniLibs written to: $JNILIBS"
ls -lh "$JNILIBS"/*/libaa_ffi.so

#!/usr/bin/env bash
# Build aa-ffi as a native shared library for the Android live-wallpaper shell.
#
# Output: shells/android/app/src/main/jniLibs/arm64-v8a/libaa_ffi.so
# (picked up by Gradle automatically — app/build.gradle.kts's
# sourceSets.main.jniLibs.srcDirs already points here; gitignored, rebuilt on
# the next Gradle build if missing.)
#
# Only arm64-v8a for now: the locally-installed emulator system image
# (android-35 google_apis_playstore) is arm64-v8a-only, and arm64 covers
# real devices too. Add armeabi-v7a/x86_64 `cargo ndk -t` lines back before a
# wider device matrix or Play Store submission needs them.
#
# Prerequisites:
#   rustup target add aarch64-linux-android
#   Android NDK installed; ANDROID_NDK_HOME set (or NDK_HOME).
#   cargo-ndk: cargo install cargo-ndk
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="aa-ffi"
JNILIBS="$REPO_ROOT/shells/android/app/src/main/jniLibs"
MIN_SDK=26 # matches app/build.gradle.kts's minSdk

cd "$REPO_ROOT"

if ! command -v cargo-ndk &>/dev/null; then
  echo "cargo-ndk not found — installing..."
  cargo install cargo-ndk
fi

echo "── Installing Android Rust target ───────────────────────────────"
rustup target add aarch64-linux-android

echo "── Building for arm64-v8a (platform $MIN_SDK) ────────────────────"
cargo ndk -t arm64-v8a --platform "$MIN_SDK" -o "$JNILIBS" build -p "$CRATE" --release

echo "── Done ─────────────────────────────────────────────────────────"
echo "jniLibs written to: $JNILIBS"
ls -lh "$JNILIBS"/*/libaa_ffi.so

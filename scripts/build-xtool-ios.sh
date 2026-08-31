#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_ROOT="${AA_XTOOL_SDK_ROOT:-$HOME/platform/sdk/xtool}"
XDG_ROOT="${AA_XTOOL_XDG_CONFIG_HOME:-$SDK_ROOT/config}"
BUILD_ROOT="${AA_XTOOL_BUILD_ROOT:-$HOME/platform/build/ascii-arcade/xtool}"
SDK_BUNDLE="$SDK_ROOT/darwin.artifactbundle"
XCODE_INPUT="${1:-}"

if [ -z "$XCODE_INPUT" ]; then
  echo "Usage: $0 /path/to/Xcode.xip [xtool dev build options]" >&2
  exit 2
fi

if [ ! -e "$XCODE_INPUT" ]; then
  echo "Xcode input not found: $XCODE_INPUT" >&2
  exit 1
fi

export XDG_CONFIG_HOME="$XDG_ROOT"
mkdir -p "$SDK_ROOT" "$XDG_ROOT" "$BUILD_ROOT"

if [ ! -d "$SDK_BUNDLE" ]; then
  xtool sdk build "$XCODE_INPUT" "$SDK_ROOT" --arch arm64
fi

if [ ! -d "$SDK_BUNDLE" ]; then
  echo "xtool did not produce the expected SDK bundle: $SDK_BUNDLE" >&2
  exit 1
fi

if ! xtool sdk status 2>/dev/null | rg -q '^Installed at '; then
  swift sdk install "$SDK_BUNDLE"
fi

AA_DARWIN_SDK_ROOT="$SDK_BUNDLE" \
  AA_IOS_BUILD_ROOT="$BUILD_ROOT/cargo" "$REPO_ROOT/scripts/build-ios.sh"

cd "$REPO_ROOT/shells/ios"
xtool dev build --configuration release "${@:2}"

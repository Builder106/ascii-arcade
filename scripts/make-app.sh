#!/bin/zsh
set -euo pipefail

# Assemble a double-clickable AsciiArcade.app from the SwiftPM build.
# SwiftPM can't emit a .app, so we build the release binary and wrap it.
#
# Env overrides:
#   OUT_DIR       where the .app lands           (default: ./dist)
#   VERSION       CFBundleShortVersionString      (default: 1.0.0)
#   SCRATCH_PATH  relocate .build off a synced volume (SwiftPM build.db can
#                 throw disk-I/O errors on Google Drive / network shares)
#   INCLUDE_DOOM  1 to bundle the GPL-2.0 doom_ascii binary (off by default;
#                 if you set it you must also redistribute doom_ascii's source)
#   UNIVERSAL     1 to build a universal (arm64 + x86_64) binary instead of
#                 native-arch-only. Needed for release builds: GitHub's
#                 macos-15 runners are Apple Silicon, so a plain build there
#                 would silently ship arm64-only and not run on Intel Macs.

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
cd "$ROOT"

APP_NAME="ASCII Arcade"
EXEC_NAME="AsciiArcade"
BUNDLE_ID="com.builder106.ascii-arcade"
VERSION="${VERSION:-1.0.0}"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"
APP="$OUT_DIR/$APP_NAME.app"

BUILD_ARGS=(-c release)
if [ -n "${SCRATCH_PATH:-}" ]; then
	BUILD_ARGS+=(--scratch-path "$SCRATCH_PATH")
fi
if [ "${UNIVERSAL:-0}" = "1" ]; then
	BUILD_ARGS+=(--arch arm64 --arch x86_64)
fi

echo "Building $EXEC_NAME (release)…"
swift build "${BUILD_ARGS[@]}" --product "$EXEC_NAME"
BIN_DIR="$(swift build "${BUILD_ARGS[@]}" --show-bin-path)"
BIN="$BIN_DIR/$EXEC_NAME"
[ -x "$BIN" ] || { echo "build did not produce $BIN" >&2; exit 1; }

echo "Assembling $APP…"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$EXEC_NAME"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>$APP_NAME</string>
	<key>CFBundleDisplayName</key><string>$APP_NAME</string>
	<key>CFBundleExecutable</key><string>$EXEC_NAME</string>
	<key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>$VERSION</string>
	<key>CFBundleVersion</key><string>$VERSION</string>
	<key>CFBundleIconFile</key><string>AppIcon</string>
	<key>LSMinimumSystemVersion</key><string>13.0</string>
	<key>LSUIElement</key><true/>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSHumanReadableCopyright</key><string>GPL-2.0. doom_ascii (GPL-2.0) is fetched separately.</string>
</dict>
</plist>
PLIST

if [ -f "$ROOT/assets/icon-512.png" ]; then
	ICONSET="$(mktemp -d)/AppIcon.iconset"
	mkdir -p "$ICONSET"
	for size in 16 32 128 256 512; do
		dbl=$((size * 2))
		sips -z $size $size "$ROOT/assets/icon-512.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
		sips -z $dbl $dbl "$ROOT/assets/icon-512.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
	done
	iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
	rm -rf "$(dirname "$ICONSET")"
else
	echo "warning: assets/icon-512.png missing — app will have no icon" >&2
fi

# Bundle the BSD-licensed Freedoom IWADs so DOOM works without a download.
if [ -d "$ROOT/wad" ]; then
	mkdir -p "$APP/Contents/Resources/wad"
	cp "$ROOT"/wad/*.wad "$APP/Contents/Resources/wad/" 2>/dev/null || true
fi

if [ "${INCLUDE_DOOM:-0}" = "1" ] && [ -x "$ROOT/bin/doom_ascii" ]; then
	cp "$ROOT/bin/doom_ascii" "$APP/Contents/Resources/doom_ascii"
	chmod +x "$APP/Contents/Resources/doom_ascii"
	echo "NOTE: bundled GPL-2.0 doom_ascii — you must also ship its source."
fi

# Sign with a stable self-signed identity if one exists, so rebuilds keep their
# TCC grants (Accessibility, Screen Recording). Ad-hoc signing pins the
# designated requirement to the cdhash, which changes every build and makes
# macOS re-prompt for permissions. Run scripts/setup-signing.sh once to create
# the identity. This is NOT notarization — recipients on other Macs still
# right-click → Open (or clear the quarantine xattr) once.
# Note: list without -v. A self-signed cert is usable for signing but reports
# as "not trusted", so -v (valid-only) would hide it and force the ad-hoc path.
SIGN_IDENTITY="${SIGN_IDENTITY:-ASCII Arcade Local}"
if security find-identity -p codesigning 2>/dev/null | grep -qF "$SIGN_IDENTITY"; then
	codesign --force --deep --sign "$SIGN_IDENTITY" "$APP" >/dev/null 2>&1 \
		|| { echo "error: signing with '$SIGN_IDENTITY' failed" >&2; exit 1; }
else
	echo "warning: no '$SIGN_IDENTITY' identity — ad-hoc signing; macOS will re-prompt" >&2
	echo "         for Accessibility on every rebuild. Run scripts/setup-signing.sh once." >&2
	codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
fi

echo "Built $APP"

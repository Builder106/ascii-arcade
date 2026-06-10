#!/bin/zsh
set -euo pipefail

# Installs the (optional) hotword watcher as a LaunchAgent. Type the hotword
# anywhere and DOOM pops up in a browser tab. The wallpaper app does NOT need
# this — it's only for the browser bonus.

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
cd "$ROOT"

LABEL="net.local.ascii-arcade-watcher"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"

swift build -c release
mkdir -p "$HOME/Library/LaunchAgents" logs

# Generate the plist with this checkout's absolute paths (avoids stale hardcoded paths).
cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>$LABEL</string>
	<key>ProgramArguments</key>
	<array>
		<string>$ROOT/.build/release/WatcherCLI</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<true/>
	<key>EnvironmentVariables</key>
	<dict>
		<key>DOOM_WAD_DIR</key>
		<string>$ROOT/wad</string>
	</dict>
	<key>StandardOutPath</key>
	<string>$ROOT/logs/ascii-arcade-watcher.out</string>
	<key>StandardErrorPath</key>
	<string>$ROOT/logs/ascii-arcade-watcher.err</string>
</dict>
</plist>
EOF

launchctl unload "$PLIST" 2>/dev/null || true
launchctl load -w "$PLIST"
echo "Loaded LaunchAgent: $LABEL"

#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
cd "$ROOT"

swift build -c release
mkdir -p "$HOME/Library/LaunchAgents" logs
cp LaunchAgents/net.local.doom-watcher.plist "$HOME/Library/LaunchAgents/"
launchctl unload "$HOME/Library/LaunchAgents/net.local.doom-watcher.plist" 2>/dev/null || true
launchctl load -w "$HOME/Library/LaunchAgents/net.local.doom-watcher.plist"

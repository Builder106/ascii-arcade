# DOOM

A macOS background daemon that listens for a hotword typed anywhere on the system and, on match, launches text-mode DOOM in a browser window. Built on top of [doom-ascii](https://github.com/wojciech-graj/doom-ascii) (vendored in `doom-ascii/`).

## Components

- **`Sources/Hotword`** — KMP-style hotword detector. Streams characters from the system event tap and fires when the configured pattern matches inside a sliding time window.
- **`Sources/PTYBridge`** — wraps the `doom_ascii` binary in a pseudo-terminal so its frame output can be piped to clients and keyboard input forwarded back.
- **`Sources/Server`** — Vapor HTTP/WebSocket server. Streams DOOM frames to a browser front-end (`Sources/Server/Public/`) and forwards key events to the PTY.
- **`Sources/WatcherCLI`** — long-running daemon. Watches keystrokes, ensures the server is up on hotword detection, and opens the browser to it. Installed as a `LaunchAgent`.
- **`wad/`** — bundled Freedoom WADs (`freedoom1.wad`, `freedoom2.wad`) used as game data.

## Build & install

```bash
# 1. Build the vendored doom-ascii binary
./scripts/setup.sh                  # produces ./bin/doom_ascii

# 2. Build the Swift targets
swift build -c release              # produces .build/release/{Server,WatcherCLI}

# 3. Install the watcher as a LaunchAgent
./scripts/install_agent.sh          # loads LaunchAgents/net.local.doom-watcher.plist
```

The watcher needs Accessibility permission (System Settings → Privacy & Security → Accessibility) to read keystrokes globally.

## Run manually (without the agent)

```bash
DOOM_PORT=8787 .build/release/Server      # http://127.0.0.1:8787
.build/release/WatcherCLI                 # listens for the hotword
```

## Status

Working in pieces — the server, PTY bridge, and hotword detector each have tests (`Tests/HotwordTests`, `Tests/ServerTests`). End-to-end integration is the rough edge.

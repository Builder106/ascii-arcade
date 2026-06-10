# ASCII Arcade

A macOS live-wallpaper customizer that renders ASCII scenes as your desktop
background. Pick a spinning [Andy Sloane donut](https://www.a1k0n.net/2011/07/20/donut-math.html),
a precessing helix, or **playable text-mode DOOM** — all drawn straight onto the
desktop, behind your windows.

It's the merge of two earlier projects: `donut` (the ASCII wallpaper host) and
`DOOM` (text-mode DOOM over a PTY). DOOM is now just another *scene*: its frames
are reconstructed from the `doom_ascii` terminal stream and rendered with the same
CRT-styled text drawing as the donut, with keystrokes forwarded so you can play
it as your wallpaper.

## Scenes

| Scene | What it is | Interactive |
|-------|------------|-------------|
| **Donut** | The classic rotating ASCII torus | — |
| **Helix** | A precessing double-helix variant | — |
| **DOOM** | `doom_ascii` rendered to the desktop | ✅ keyboard |

Switch scenes and themes from the menu-bar `◎` item, or cycle scenes with **⌘⌥C**.
Themes: Hacker (green), Amber, Ice, Ghost.

## DOOM controls

Forwarded to `doom_ascii` while DOOM is the active wallpaper (toggle with
*"Capture keys for DOOM"* in the menu):

| Action | Key |
|--------|-----|
| Move / turn | Arrow keys |
| Strafe | `,` `.` |
| Fire | Space |
| Use / open | `E` |
| Run | `]` |
| Weapons | `1`–`7` |
| Confirm / menu | Return / Esc |

> Playing DOOM as a wallpaper needs **Accessibility** permission (System Settings →
> Privacy & Security → Accessibility) so the app can read keystrokes globally. While
> capture is on, keystrokes drive DOOM regardless of which app is focused.

## Build & run

Requires macOS 13+ and a Swift toolchain.

```bash
# 1. Build the vendored doom_ascii binary into ./bin (doom-ascii is GPL; fetched, not vendored)
./scripts/setup.sh

# 2. Build everything
swift build

# 3. Run the wallpaper app
swift run AsciiArcade
```

The Freedoom IWADs ship in `wad/`, so DOOM works out of the box. Quitting the app
restores your original wallpaper.

## Browser bonus

DOOM can also be played in a browser tab via a Vapor WebSocket server (xterm.js
frontend) — handy for sharing or for machines without Accessibility access:

```bash
DOOM_PORT=8787 swift run Server   # http://127.0.0.1:8787
```

Optionally, `scripts/install_agent.sh` installs a LaunchAgent that watches for the
hotword `doom` typed anywhere and pops the browser DOOM up automatically.

## How it works

```mermaid
flowchart LR
    Menu["Menu bar ◎\n(scene + theme)"] --> View
    Keys["Global key monitor"] -->|DOOM active| View
    subgraph App["AsciiArcade (desktop-level window)"]
        View["SceneView\nCRT text drawing"]
    end
    View -->|frame(atTime:)| Scene{AsciiScene}
    Scene -->|donut / helix| Gen["ShapeFrameGenerator\n(pure math)"]
    Scene -->|DOOM| Doom["DoomScene"]
    Doom -->|spawn + keys| PTY["PTYProcess\n→ doom_ascii"]
    PTY -->|ANSI stream| Buf["DoomScreenBuffer\n(parse → char grid)"]
    Buf -->|snapshot| Doom
```

- **`AsciiArcadeCore`** — the frame generators, the `AsciiScene` protocol, and the
  DOOM glue (`DoomScreenBuffer` parses the ANSI stream into a char grid;
  `DoomScene` owns the PTY; `DoomLauncher` resolves the binary + IWAD).
- **`PTYBridge`** — spawns `doom_ascii` in a pseudo-terminal and pipes its output.
- **`AsciiArcade`** — the AppKit wallpaper host (scene picker, themes, key forwarding).
- **`Server`** / **`Hotword`** / **`WatcherCLI`** — the optional browser path.

## Layout

```
Sources/AsciiArcadeCore   frame generators + scene/DOOM glue
Sources/PTYBridge         pseudo-terminal wrapper
Sources/AsciiArcade       wallpaper app (executable)
Sources/Server            Vapor browser server (executable, bonus)
Sources/Hotword           hotword detector
Sources/WatcherCLI        hotword → browser daemon (executable, bonus)
wad/                      committed Freedoom IWADs
bin/                      doom_ascii binary (built by setup.sh)
```

## License

This project's code is MIT — see [LICENSE](LICENSE). Note that `doom_ascii`
([wojciech-graj/doom-ascii](https://github.com/wojciech-graj/doom-ascii)) is
GPL-2.0 and is *fetched and built* by `setup.sh` rather than redistributed here;
the bundled [Freedoom](https://freedoom.github.io/) IWADs are BSD-licensed.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)"  srcset="assets/banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.svg">
    <img alt="ASCII Arcade — live ASCII wallpapers for macOS" src="assets/banner-dark.svg" width="100%">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/Builder106/ascii-arcade/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Builder106/ascii-arcade/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://swift.org"><img alt="Swift" src="https://img.shields.io/badge/Swift-5.10%2B-orange.svg"></a>
  <img alt="Platform" src="https://img.shields.io/badge/macOS-13%2B-black.svg">
  <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-GPL--2.0-blue.svg"></a>
</p>

A macOS live-wallpaper customizer that renders ASCII scenes as your desktop
background. Pick a spinning [Andy Sloane donut](https://www.a1k0n.net/2011/07/20/donut-math.html),
Matrix rain, Conway's Game of Life, or the old pipes screensaver — all drawn
straight onto the desktop, behind your windows. **Playable text-mode DOOM** is
also in there as an opt-in scene (see below).

It started as the merge of two earlier projects: `donut` (the ASCII wallpaper
host) and `DOOM` (text-mode DOOM over a PTY). DOOM is just another *scene*: its
frames are reconstructed from the `doom_ascii` terminal stream and rendered with
the same CRT-styled text drawing as the donut, with keystrokes forwarded so you
can play it as your wallpaper.

## Scenes

| Scene | What it is | Colour | Interactive |
| ------- | ------------ | -------- | ------------- |
| **Donut** | The classic rotating ASCII torus | theme | — |
| **Helix** | A precessing double-helix variant | theme | — |
| **Matrix** | Falling digital rain with bright heads + fading trails | theme-tinted | — |
| **Life** | Conway's Game of Life, seeded with classic patterns; auto-reseeds when it stalls | theme-tinted | — |
| **Pipes** | The old pipes screensaver in box-drawing glyphs | per-pipe hue | — |
| **DOOM** *(opt-in)* | `doom_ascii` rendered to the desktop, in its native colours | full colour | ✅ keyboard |

Switch scenes and themes from the menu-bar `◎` item, or cycle scenes with **⌘⌥C**.

**DOOM is opt-in.** A playable shooter isn't what everyone wants greeting them
on a shared or school machine, so DOOM stays out of the Scene list, ⌘⌥C
cycling, and idle auto-cycle until you turn on *"Enable DOOM Scene"* in the
menu. Once enabled, it behaves like any other scene.

**Capture.** macOS's native ⌘⇧3/⌘⇧4 skips the wallpaper layer — use the in-app shortcuts instead: **⌘⌥S** saves a PNG to ~/Desktop and copies it to the clipboard; **⌘⌥R** records a 3-second MP4 clip and opens it in Finder when done. Both commands are also in the `◎` menu under *Capture*.

**Colour.** Scenes can paint each glyph individually: DOOM uses its own native
palette, while Matrix, Life, and the math scenes key off the theme's text colour
(green rain under Hacker, amber rain under Amber, …). Themes: Hacker (green),
Amber, Ice, Ghost.

**Scene Settings.** Each scene exposes a few knobs under *Scene Settings* in the
menu — Matrix speed/density, Life speed/cell-size, Pipes speed/count.

**Auto-cycle when idle.** Toggle *Auto-cycle when idle* and ASCII Arcade rotates
through the scenes like a slideshow after ~90 s with no input, advancing to the
next scene every 20 s, then snaps back to your chosen scene the moment you touch
the keyboard or mouse. DOOM is skipped by the rotation unless you've opted into
it. Rendering also pauses while the displays sleep.

## DOOM controls

Needs *"Enable DOOM Scene"* turned on first (see [Scenes](#scenes) above).
Forwarded to `doom_ascii` while DOOM is the active wallpaper (toggle with
*"Capture keys for DOOM"* in the menu):

| Action | Key |
| -------- | ----- |
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

## Install

ASCII Arcade ships as a normal macOS app — it lives in your menu bar (the `◎`
icon), remembers your scene/theme/settings between launches, and can start at
login. Grab `ASCII-Arcade.dmg` from the [releases page](https://github.com/Builder106/ascii-arcade/releases),
then:

1. Open the DMG and drag **ASCII Arcade** onto **Applications**.
2. Because it isn't notarized (no paid Apple Developer account), Gatekeeper
   blocks the first launch — **right-click (Control-click) the app → Open**,
   then click **Open** in the dialog. macOS remembers it from then on.
   - Terminal equivalent: `xattr -dr com.apple.quarantine "/Applications/ASCII Arcade.app"`
3. Look for the `◎` icon in your menu bar to pick a scene/theme.

The donut and the other math/colour scenes work immediately. **DOOM** is the one
piece that isn't bundled — `doom_ascii` is GPL-2.0, so it's fetched and built
separately (see below); until then the DOOM scene shows a hint.

## Build & run from source

Requires macOS 13+ and a Swift 6 toolchain (Xcode 16+) — the pinned Vapor
release declares Swift tools 6.0.

```bash
./scripts/setup.sh        # fetch + build the GPL doom_ascii binary into ./bin
swift build               # build everything
swift run AsciiArcade     # run the wallpaper app
```bash

The Freedoom IWADs ship in `wad/`, so DOOM works out of the box. Quitting the app
restores your original wallpaper.

### Package the app yourself

```bash
./scripts/make-app.sh     # assemble dist/ASCII Arcade.app (release build + icon + WADs)
./scripts/make-dmg.sh     # wrap it in dist/ASCII-Arcade.dmg
```bash

`make-app.sh` ad-hoc signs the bundle and bundles the BSD Freedoom WADs. It does
**not** bundle the GPL `doom_ascii` by default; set `INCLUDE_DOOM=1` to include
it (you then must also redistribute its source). On a cloud-synced checkout, set
`SCRATCH_PATH=/tmp/aa-build` so SwiftPM's `build.db` doesn't choke.

## Browser

Any built-in scene streams to a browser tab via the Rust `aa-web` shell —
handy for sharing or for machines without a wallpaper target:

```bash
cargo run -p aa-web   # http://127.0.0.1:8788
```bash

Pick a scene from the dropdown; the server streams ANSI truecolor frames at
30 fps over a WebSocket to an xterm.js terminal.

Optionally, `scripts/install_agent.sh` installs a LaunchAgent that watches for the
hotword `doom` typed anywhere and pops the browser up automatically.

## Terminal (Rust CLI)

The unified `aa` binary (`shells/cli`) plays scenes directly in a terminal —
no wallpaper, no browser — and doubles as the browser/autostart entry point on
Linux and Windows:

```bash
cargo run -p aa -- play donut     # render live in this terminal (q to quit)
cargo run -p aa -- web            # same server as aa-web, via a subcommand
cargo run -p aa -- scenes         # list built-in scene ids
```bash

DOOM is opt-in here too, and more strictly than on macOS: it isn't even
compiled in unless you build with `--features doom` (an optional dependency,
so a normal build doesn't pull in the PTY-spawning GPL code at all), and even
then `aa scenes`/`aa play`/`aa web` won't reveal or accept it without
`--enable-doom` (or `AA_WEB_ENABLE_DOOM=1` for the standalone `aa-web`
binary):

```bash
cargo run -p aa --features doom -- play doom --enable-doom
```bash

`aa run` (the actual desktop-wallpaper mode on Linux/Windows) doesn't offer
DOOM at all — playing DOOM as your literal wallpaper needs fixed-grid bitmap
compositing and global keyboard capture that only the macOS shell implements
today.

## Demo

<details>
<summary>Donut — rotating ASCII torus (browser)</summary>

![Donut rotating in the browser terminal](assets/demo/donut.gif)

</details>

<details>
<summary>Matrix rain — digital rain + scene switcher (browser)</summary>

![Matrix rain with scene picker in the browser terminal](assets/demo/matrix.gif)

</details>

Recorded by the [playwright-bdd demo suite](e2e/) against the `aa-web` server:

```bash
cd e2e && npm install && npm run demo   # writes e2e/recordings/*.mp4
```bash

## How it works

```mermaid
flowchart LR
    Menu["Menu bar ◎<br/>scene + theme"] --> View
    Keys["Global key monitor"] -->|DOOM active| View
    subgraph App["AsciiArcade desktop-level window"]
        View["SceneView<br/>CRT text drawing"]
    end
    View -->|each frame| Scene{AsciiScene}
    Scene -->|donut or helix| Gen["ShapeFrameGenerator<br/>pure math"]
    Scene -->|DOOM| Doom["DoomScene"]
    Doom -->|spawn + keys| PTY["PTYProcess<br/>doom_ascii"]
    PTY -->|ANSI stream| Buf["DoomScreenBuffer<br/>parse to char grid"]
    Buf -->|snapshot| Doom
```bash

- **`AsciiArcadeCore`** — the frame generators, the `AsciiScene` protocol, and the
  DOOM glue (`DoomScreenBuffer` parses the ANSI stream into a char grid;
  `DoomScene` owns the PTY; `DoomLauncher` resolves the binary + IWAD). Colour
  scenes return a `ColoredFrame` (a glyph grid plus a parallel grid of optional
  per-cell `RGBColor`); the stateful ones (Matrix, Life, Pipes) share a
  `SteppedScene` base that runs a fixed-timestep simulation off the render clock.
- **`PTYBridge`** — spawns `doom_ascii` in a pseudo-terminal and pipes its output.
- **`AsciiArcade`** — the AppKit wallpaper host (scene picker, themes, key forwarding).
- **`Server`** / **`Hotword`** / **`WatcherCLI`** — the optional browser path.

## Layout

```bash
Sources/AsciiArcadeCore   frame generators + scene/DOOM glue
Sources/PTYBridge         pseudo-terminal wrapper
Sources/AsciiArcade       wallpaper app (executable)
Sources/Hotword           hotword detector
Sources/WatcherCLI        hotword → browser daemon (executable, bonus)
Server/                   Vapor browser server, its own SwiftPM package
                          (keeps Vapor out of the wallpaper app's build)
wad/                      committed Freedoom IWADs
bin/                      doom_ascii binary (built by setup.sh)
```bash

## License

This project is GPL-2.0 — see [LICENSE](LICENSE). `doom_ascii`
([wojciech-graj/doom-ascii](https://github.com/wojciech-graj/doom-ascii)) is
also GPL-2.0 and is fetched and built by `setup.sh`; the bundled
[Freedoom](https://freedoom.github.io/) IWADs are BSD-licensed.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)"  srcset="assets/banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.svg">
    <img alt="ASCII Arcade: live ASCII wallpapers for macOS" src="assets/banner-dark.svg" width="100%">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/Builder106/ascii-arcade/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Builder106/ascii-arcade/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://swift.org"><img alt="Swift 6" src="https://img.shields.io/badge/Swift-6-orange.svg"></a>
  <a href="https://www.rust-lang.org"><img alt="Rust" src="https://img.shields.io/badge/Rust-stable-b7410e.svg"></a>
  <img alt="macOS 13+" src="https://img.shields.io/badge/macOS-13%2B-black.svg">
  <a href="#license"><img alt="License: GPL-2.0" src="https://img.shields.io/badge/license-GPL--2.0-blue.svg"></a>
</p>

ASCII Arcade puts live ASCII animation on your Mac desktop, under every window.
Pick Matrix rain, a spinning donut, Conway's Life or a pipes screensaver from
the menu bar, or switch on DOOM and play it as your wallpaper. Clicks go
straight through to whatever's underneath, and quitting brings your old
wallpaper back.

**Try it in the browser:** [ascii-arcade-doom.vercel.app](https://ascii-arcade-doom.vercel.app/)
runs the same engine in WebAssembly, on a 3D arcade cabinet you can drive with
the joystick.

## Demo

<details open>
<summary>The site: power-on and attract mode</summary>

![The landing page powering on, then the cabinet pulling out of the glass](assets/demo/attract.gif)

</details>

<details>
<summary>The site: scenes and themes on the cabinet</summary>

![Scene and theme buttons repainting the cabinet's screen and trim](assets/demo/cabinet.gif)

</details>

<details>
<summary>Donut in the browser terminal</summary>

![Donut rotating in the browser terminal](assets/demo/donut.gif)

</details>

<details>
<summary>Matrix rain and the scene picker in the browser terminal</summary>

![Matrix rain with scene picker in the browser terminal](assets/demo/matrix.gif)

</details>

There's also a [trailer](trailer/) and a vertical [social cut](ui-demo/), both
built in HyperFrames from the same footage as the site GIFs.

## Scenes

| Scene | What it is | Colour | Interactive |
| --- | --- | --- | --- |
| **Donut** | The classic rotating 3D ASCII donut | Theme | No |
| **Helix** | A rotating DNA double helix | Theme | No |
| **Matrix** | Falling code rain with glowing trails | Theme | No |
| **Life** | Conway's Game of Life | Theme | No |
| **Pipes** | The 3D pipes screensaver | Multi-colour | No |
| **DOOM** *(off by default)* | Playable DOOM, drawn in ASCII | Full colour | Yes, keyboard |

Switch scenes and themes from the `◎` menu, or cycle scenes with **⌘⌥C**.

**Themes.** Hacker (green), Amber, Ice and Ghost. Scenes paint each glyph
separately: DOOM keeps its own palette, and the rest take their colour from the
theme, so Matrix rain turns amber under Amber.

**Scene settings.** Each scene has a few knobs under *Scene Settings*: Matrix
speed and density, Life speed and cell size, Pipes speed and count.

**Auto-cycle when idle.** Turn it on and the wallpaper works like a slideshow.
After about 90 s with no input it moves to the next scene every 20 s, then
snaps back to the scene you picked as soon as you touch the keyboard or mouse.
DOOM stays out of the rotation unless you've enabled it. Rendering pauses while
the displays sleep.

**Capture.** macOS's ⌘⇧3 and ⌘⇧4 skip the wallpaper layer, so the app has its
own shortcuts: **⌘⌥S** saves a PNG to `~/Desktop` and copies it to the
clipboard, and **⌘⌥E** records a 3-second MP4 and shows it in Finder. Both are
also in the menu under *Capture*.

## DOOM

DOOM is off by default so the wallpaper stays safe for any screen. Turn on
*Enable DOOM Scene* in the menu to unlock it. While it's the wallpaper and
*Capture keys for DOOM* is on, these keys go to the game:

| Action | Key |
| --- | --- |
| Move and turn | Arrow keys |
| Strafe | `,` `.` |
| Fire | Space |
| Use / open | `E` |
| Run | `]` |
| Weapons | `1` to `7` |
| Confirm / menu | Return / Esc |

> Capturing keys needs **Accessibility** permission (System Settings → Privacy &
> Security → Accessibility). While capture is on, keystrokes drive DOOM no matter
> which app has focus.

## Install

Download `ASCII-Arcade.dmg` from the
[latest release](https://github.com/Builder106/ascii-arcade/releases/latest).
The app lives in your menu bar, remembers your scene, theme and settings, and
can start at login.

1. Open the DMG and drag **ASCII Arcade** onto **Applications**.
2. The app isn't notarized, so Gatekeeper blocks the first launch. Control-click
   the app, choose **Open**, then click **Open** again. macOS remembers after
   that. Or run this in Terminal:

   ```bash
   xattr -dr com.apple.quarantine "/Applications/ASCII Arcade.app"
   ```

3. Find `◎` in your menu bar and pick a scene.

Every scene except DOOM works straight away. `doom_ascii` is GPL-2.0, so it
isn't bundled; build it with `./scripts/setup.sh` (below). Until then the DOOM
scene shows a hint.

## Build from source (macOS)

You need macOS 13 or later and a Swift 6 toolchain (Xcode 16 or later).

```bash
./scripts/setup.sh        # fetch and build the GPL doom_ascii binary into ./bin
swift build               # build everything
swift run AsciiArcade     # run the wallpaper app
```

The Freedoom IWADs are committed in `wad/`, so DOOM works once `doom_ascii` is
built.

To package the app yourself:

```bash
./scripts/make-app.sh     # dist/ASCII Arcade.app (release build, icon, WADs)
./scripts/make-dmg.sh     # dist/ASCII-Arcade.dmg
```

`make-app.sh` ad-hoc signs the bundle and includes the BSD-licensed Freedoom
WADs. It leaves out the GPL `doom_ascii` unless you set `INCLUDE_DOOM=1`, and if
you do, you have to distribute its source too. If your checkout is on a synced
drive (iCloud, Google Drive), set `SCRATCH_PATH=/tmp/aa-build` so SwiftPM's
`build.db` doesn't fail.

## Linux, Windows and the terminal

The Rust port shares one engine, `aa-core`, across every other platform. The
`aa` binary plays scenes in a terminal and runs the wallpaper on Linux (X11 or
wlr-layer-shell) and Windows:

```bash
cargo run -p aa -- play donut                 # in this terminal (q to quit)
cargo run -p aa -- run matrix --theme amber   # as the desktop wallpaper
cargo run -p aa -- scenes                     # list scene ids
cargo run -p aa -- autostart enable           # start at login
```

Any scene can also stream to a browser tab, which is handy for sharing:

```bash
cargo run -p aa-web       # http://127.0.0.1:8788
```

The server sends ANSI truecolour frames at 30 fps over a WebSocket to an
xterm.js terminal; pick the scene from the dropdown.

DOOM is stricter here than on macOS. It isn't compiled in unless you build with
`--features doom`, and even then `aa scenes`, `aa play` and `aa web` hide it
without `--enable-doom` (or `AA_WEB_ENABLE_DOOM=1` for the standalone `aa-web`):

```bash
cargo run -p aa --features doom -- play doom --enable-doom
```

`aa run` doesn't offer DOOM at all. Playing it as your wallpaper needs bitmap
compositing and global key capture, and only the macOS app has those so far.

## How it works

![ASCII Arcade scene flow](assets/architecture.png)

The menu bar picks a scene and theme, and SceneView renders each frame. Donut
and Helix are pure math. DOOM runs `doom_ascii` in a pseudo-terminal, parses its
ANSI output into a character grid, and hands back a snapshot. Global key capture
feeds DOOM while it's the active scene.

On macOS:

- **`AsciiArcadeCore`** has the frame generators, the `AsciiScene` protocol and
  the DOOM glue. `DoomScreenBuffer` parses the ANSI stream, `DoomScene` owns the
  PTY, and `DoomLauncher` finds the binary and IWAD. Colour scenes return a
  `ColoredFrame`, a glyph grid with a matching grid of optional per-cell
  `RGBColor`. Matrix, Life and Pipes share `SteppedScene`, which runs a
  fixed-timestep simulation off the render clock.
- **`PTYBridge`** spawns `doom_ascii` in a pseudo-terminal and pipes its output.
- **`AsciiArcade`** is the AppKit wallpaper host: scene picker, themes, key
  forwarding.
- **`Hotword`** and **`WatcherCLI`**, with `scripts/install_agent.sh`, install a
  LaunchAgent that opens the browser when you type `doom` anywhere.

## Layout

```text
Sources/            macOS app (Swift): AsciiArcadeCore, PTYBridge, AsciiArcade,
                    Hotword, WatcherCLI
Server/             Vapor browser server, its own SwiftPM package
crates/aa-core      Rust engine: scenes, frames, themes, DOOM ANSI buffer
crates/aa-render    software rasteriser from frames to RGBA pixels
crates/aa-doom      DOOM over a portable PTY
crates/aa-ffi       C ABI for the iOS and Android shells
crates/aa-wasm      WebAssembly wrapper for the site
shells/             cli, linux, windows, web, ios, android
site/               the landing page (no build step besides the WASM)
e2e/                Playwright: site specs, demo recordings, footage capture
trailer/, ui-demo/  HyperFrames videos
wad/                Freedoom IWADs
bin/                doom_ascii, built by scripts/setup.sh
```

To record the README GIFs again, use `cd e2e && npm run demo` for the terminal
ones and `npm run footage` for the site ones.

## License

GPL-2.0, see [LICENSE](LICENSE). `doom_ascii`
([wojciech-graj/doom-ascii](https://github.com/wojciech-graj/doom-ascii)) is
also GPL-2.0 and is fetched and built by `setup.sh`. The bundled
[Freedoom](https://freedoom.github.io/) IWADs are BSD-licensed.

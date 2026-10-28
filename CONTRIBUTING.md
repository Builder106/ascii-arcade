# Contributing to ASCII Arcade

ASCII Arcade has a Swift app for macOS and Rust wallpaper hosts for Windows and
Linux. The Rust scene engine, renderer, DOOM launcher, and controllers should
keep the same scenes and settings available on each desktop.

## Dev setup

Requires macOS 13+ and a Swift toolchain (Xcode or the open-source toolchain).

```bash
./scripts/setup.sh      # clone + build the doom_ascii binary into ./bin
swift build             # build all targets
swift test              # run the unit tests
swift run AsciiArcade   # run the wallpaper app
```

`doom_ascii`is GPL-2.0 and is fetched/compiled by`setup.sh` — it is never
committed. The Freedoom IWADs in `wad/` are committed (BSD-licensed) so DOOM
works without a download.

## Packaging the app

`scripts/make-app.sh`assembles`dist/ASCII Arcade.app` (release build + Info.plist

+ `.icns`from`assets/icon-512.png` + bundled Freedoom WADs, ad-hoc signed), and

`scripts/make-dmg.sh` wraps it in a DMG with first-launch instructions. The build
is intentionally **unsigned/un-notarized** (no Apple Developer account); users
right-click and choose Open once. Local `make-app.sh` builds leave `doom_ascii`
out unless `INCLUDE_DOOM=1` is set. Official releases include the engine,
Freedoom data, the matching GPL-2.0 source archive, and its license. App settings
persist via `UserDefaults` under the bundle id
`com.builder106.ascii-arcade`; resource lookup (`DoomLauncher`) checks
`Bundle.main`first so it works both from the`.app`and from`swift run`.

## Releasing

Releases are cut manually, roughly monthly — there's no enforced schedule,
just a target rhythm. When `main` is in good shape:

```bash
./scripts/cut-release.sh 0.2.0
```

This bumps `Cargo.toml`'s workspace version, commits, tags, and pushes, which
triggers `.github/workflows/release.yml`. That workflow re-verifies the
tagged commit (`swift test`+`cargo test`, matrixed across macOS/Linux/
Windows) before building anything, then publishes a GitHub Release with the
macOS DMG, Windows and Linux desktop-controller archives, native-shell binaries,
and the cross-platform `aa` CLI for all three OSes.

The macOS release build sets `INCLUDE_DOOM=1`. Windows and Linux packages also
include the engine and its matching source archive. Rust release binaries build
with `--features doom`, and the scene remains hidden until the user enables it
in the desktop controls or passes `--enable-doom`.

## Project layout

+ `Sources/AsciiArcadeCore`— frame generators, the`AsciiScene` protocol, and

  the DOOM glue (`DoomScreenBuffer`, `DoomScene`, `DoomLauncher`).

+ `Sources/PTYBridge` — pseudo-terminal process wrapper.
+ `Sources/AsciiArcade` — the AppKit wallpaper host.
+ `Sources/Hotword`, `Sources/WatcherCLI` — the optional browser path's local

  half (hotword watcher).

+ `Server/` — the Vapor-dependent browser server, split into its own SwiftPM

  package (`Server/Package.swift`) so building the wallpaper app itself never
  resolves Vapor. Build/run it with `swift run --package-path Server Server`.

## Adding a scene

Pick the lightest base that fits:

+ **Pure, stateless math** (donut, helix): implement `ShapeFrameGenerator`

  (`frame(atTime:) -> String`returning`height`rows of`width` columns) and add
  a `GeneratorScene`entry to`makeScenes()`in`Sources/AsciiArcade/main.swift`.

+ **Stateful simulation** (Matrix, Life, Pipes): subclass `SteppedScene` and

  override `reset()`, `step()`, `render()`, and `stepInterval`. The base turns the
  host's "frame at time `t`" pull into a fixed-timestep loop, and gives you grid
  sizing, base-colour, and settings plumbing for free.

+ **Externally driven / interactive** (DOOM): conform to `AsciiScene` directly,

  override `isInteractive`to return`true`, and manage your own backing work in
  `start()`/`stop()`(see`DoomScene`). `isInteractive` is also the opt-in gate:
  `AppDelegate` (`Sources/AsciiArcade/main.swift`) hides any scene with
  `isInteractive == true` from the Scene menu and from cycling until the user
  turns on *"Enable DOOM Scene"* — so a new interactive scene inherits the same
  opt-in behaviour for free, no extra plumbing needed.

To draw in colour, return a `ColoredFrame`from`coloredFrame(atTime:)` — a glyph
grid plus a parallel grid of optional `RGBColor`(a`nil` cell is painted in the
theme colour). Leave `coloredFrame`returning`nil` for a monochrome scene. Key
your palette off `applyBaseColor(_:)` if you want the scene to follow the theme.

Expose tunables by overriding `settings`(a list of`SceneSetting`, each a few
discrete `SceneOption`s) and reading them back with `settingValue(_:default:)`;
the host renders them under *Scene Settings* automatically.

## Guardrails

+ **Don't block the main thread in `frame(atTime:)`.** It's called from the

  display link ~60fps. Heavy/async work belongs off-main (see `DoomScene` feeding
  `DoomScreenBuffer` from the PTY read queue).

+ **Keep generator math in `AsciiArcadeCore` and free of AppKit** so it stays

  unit-testable.

+ **Every frame must be exactly `height`rows ×`width` columns.** Tests assert

  this; ragged frames break the centered text layout. A `ColoredFrame`'s `chars`
  and `colors`arrays must both be exactly`width * height` (the initializer
  precondition-checks this).

+ **Keep glyphs single-cell.** The colour renderer assumes one monospaced,

  single-UTF-16 glyph per cell when it maps colour runs onto the string; ASCII and
  box-drawing/block glyphs are safe, full-width CJK and emoji are not.

## GNOME integration checks

The GNOME Shell 46 extension lives in `shells/linux/gnome`. It owns the renderer
process, reads RGBA frames asynchronously, and displays them in the desktop
background group. Scene, theme, settings, idle rotation, DOOM opt-in, key
capture, opacity, and launch-at-login use the bundled GSettings schema. The
Linux installer installs that schema and extension per user.

On Linux with GNOME Shell 46, GJS, Xvfb, ImageMagick, and D-Bus installed:

```bash
cargo build --locked -p aa-linux
cargo test --locked -p aa-linux
python3 scripts/test-desktop-launcher.py
python3 scripts/test-gnome-desktop.py --renderer target/debug/aa-linux --artifacts gnome-test-artifacts
```

The GNOME check uses a private session bus and temporary settings and extension
directories. Its observer extension is installed only in that temporary test
session. It checks frame upload, background placement, scene switching, process
cleanup, and disable/re-enable behavior. It saves a screenshot and shell log.
It does not verify physical monitor hotplug or lock/unlock through GDM. Key
capture and idle rotation use Mutter's session input APIs and still need a
runtime check in the nested session.

GNOME 46/GJS 1.80 can log `Gio.UnixInputStream has been moved to a separate
platform-specific library` when `Gio.Subprocess.get_stdout_pipe()` wraps stdout.
That platform binding warning is allowed and remains visible; the extension
does not call the deprecated `Gio.UnixInputStream` API. A nested session also
logs missing optional desktop services, including GDM and the calendar server.
The missing-renderer check intentionally raises an ASCII Arcade notification.

## Commit / PR

+ Use Conventional Commit subjects such as `feat:`, `fix:`, `chore:`, and `docs:`.
+ Run the relevant Rust or Swift checks before opening a PR.
+ Add a `JOURNAL.md` entry for any non-obvious decision or pivot.

## Out of scope

+ Sound (the ASCII renderer is video-only).
+ Distributing `doom_ascii` without its matching source archive and license.

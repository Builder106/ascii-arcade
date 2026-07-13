# Contributing to ASCII Arcade

Thanks for your interest! This is a small macOS project; the bar is "keeps the
wallpaper smooth and the scenes faithful."

## Dev setup

Requires macOS 13+ and a Swift toolchain (Xcode or the open-source toolchain).

```bash
./scripts/setup.sh      # clone + build the doom_ascii binary into ./bin
swift build             # build all targets
swift test              # run the unit tests
swift run AsciiArcade   # run the wallpaper app
```

`doom_ascii` is GPL-2.0 and is fetched/compiled by `setup.sh` — it is never
committed. The Freedoom IWADs in `wad/` are committed (BSD-licensed) so DOOM
works without a download.

## Packaging the app

`scripts/make-app.sh` assembles `dist/ASCII Arcade.app` (release build + Info.plist
+ `.icns` from `assets/icon-512.png` + bundled Freedoom WADs, ad-hoc signed), and
`scripts/make-dmg.sh` wraps it in a DMG with first-launch instructions. The build
is intentionally **unsigned/un-notarized** (no Apple Developer account) — users
right-click → Open once. Don't bundle `doom_ascii` by default; it's behind the
opt-in `INCLUDE_DOOM=1` flag because shipping a GPL binary carries source-offer
obligations. App settings persist via `UserDefaults` under the bundle id
`com.builder106.ascii-arcade`; resource lookup (`DoomLauncher`) checks
`Bundle.main` first so it works both from the `.app` and from `swift run`.

## Project layout

- `Sources/AsciiArcadeCore` — frame generators, the `AsciiScene` protocol, and
  the DOOM glue (`DoomScreenBuffer`, `DoomScene`, `DoomLauncher`).
- `Sources/PTYBridge` — pseudo-terminal process wrapper.
- `Sources/AsciiArcade` — the AppKit wallpaper host.
- `Sources/Server`, `Sources/Hotword`, `Sources/WatcherCLI` — the optional
  browser path.

## Adding a scene

Pick the lightest base that fits:

- **Pure, stateless math** (donut, helix): implement `ShapeFrameGenerator`
  (`frame(atTime:) -> String` returning `height` rows of `width` columns) and add
  a `GeneratorScene` entry to `makeScenes()` in `Sources/AsciiArcade/main.swift`.
- **Stateful simulation** (Matrix, Life, Pipes): subclass `SteppedScene` and
  override `reset()`, `step()`, `render()`, and `stepInterval`. The base turns the
  host's "frame at time `t`" pull into a fixed-timestep loop, and gives you grid
  sizing, base-colour, and settings plumbing for free.
- **Externally driven / interactive** (DOOM): conform to `AsciiScene` directly,
  override `isInteractive` to return `true`, and manage your own backing work in
  `start()`/`stop()` (see `DoomScene`). `isInteractive` is also the opt-in gate:
  `AppDelegate` (`Sources/AsciiArcade/main.swift`) hides any scene with
  `isInteractive == true` from the Scene menu and from cycling until the user
  turns on *"Enable DOOM Scene"* — so a new interactive scene inherits the same
  opt-in behaviour for free, no extra plumbing needed.

To draw in colour, return a `ColoredFrame` from `coloredFrame(atTime:)` — a glyph
grid plus a parallel grid of optional `RGBColor` (a `nil` cell is painted in the
theme colour). Leave `coloredFrame` returning `nil` for a monochrome scene. Key
your palette off `applyBaseColor(_:)` if you want the scene to follow the theme.

Expose tunables by overriding `settings` (a list of `SceneSetting`, each a few
discrete `SceneOption`s) and reading them back with `settingValue(_:default:)`;
the host renders them under *Scene Settings* automatically.

## Guardrails

- **Don't block the main thread in `frame(atTime:)`.** It's called from the
  display link ~60fps. Heavy/async work belongs off-main (see `DoomScene` feeding
  `DoomScreenBuffer` from the PTY read queue).
- **Keep generator math in `AsciiArcadeCore` and free of AppKit** so it stays
  unit-testable.
- **Every frame must be exactly `height` rows × `width` columns.** Tests assert
  this; ragged frames break the centered text layout. A `ColoredFrame`'s `chars`
  and `colors` arrays must both be exactly `width * height` (the initializer
  precondition-checks this).
- **Keep glyphs single-cell.** The colour renderer assumes one monospaced,
  single-UTF-16 glyph per cell when it maps colour runs onto the string; ASCII and
  box-drawing/block glyphs are safe, full-width CJK and emoji are not.

## Commit / PR

- Conventional-ish commit subjects (`feat:`, `fix:`, `chore:`, `docs:`).
- Run `swift build` and `swift test` before opening a PR.
- Add a `JOURNAL.md` entry for any non-obvious decision or pivot.

## Out of scope

- Sound (the ASCII renderer is video-only).
- Bundling `doom_ascii` binaries (license + portability); keep it fetched.
- Non-macOS platforms — the host is AppKit + CoreVideo.

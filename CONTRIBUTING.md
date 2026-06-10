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

## Project layout

- `Sources/AsciiArcadeCore` — frame generators, the `AsciiScene` protocol, and
  the DOOM glue (`DoomScreenBuffer`, `DoomScene`, `DoomLauncher`).
- `Sources/PTYBridge` — pseudo-terminal process wrapper.
- `Sources/AsciiArcade` — the AppKit wallpaper host.
- `Sources/Server`, `Sources/Hotword`, `Sources/WatcherCLI` — the optional
  browser path.

## Adding a scene

Most new scenes are pure frame math: implement `ShapeFrameGenerator`
(`frame(atTime:) -> String` returning `height` rows of `width` columns) and add a
`GeneratorScene` entry to `makeScenes()` in `Sources/AsciiArcade/main.swift`. For
an interactive or externally-driven scene, conform to `AsciiScene` directly (see
`DoomScene` for the pattern).

## Guardrails

- **Don't block the main thread in `frame(atTime:)`.** It's called from the
  display link ~60fps. Heavy/async work belongs off-main (see `DoomScene` feeding
  `DoomScreenBuffer` from the PTY read queue).
- **Keep generator math in `AsciiArcadeCore` and free of AppKit** so it stays
  unit-testable.
- **Every frame must be exactly `height` rows × `width` columns.** Tests assert
  this; ragged frames break the centered text layout.

## Commit / PR

- Conventional-ish commit subjects (`feat:`, `fix:`, `chore:`, `docs:`).
- Run `swift build` and `swift test` before opening a PR.
- Add a `JOURNAL.md` entry for any non-obvious decision or pivot.

## Out of scope

- Sound (the ASCII renderer is video-only).
- Bundling `doom_ascii` binaries (license + portability); keep it fetched.
- Non-macOS platforms — the host is AppKit + CoreVideo.

# DonutWallpaper

A macOS app that renders Andy Sloane's classic spinning ASCII donut as a live desktop wallpaper, with theme presets (Hacker green, Amber, Ice, Ghost).

## Layout

Swift package, two products:
- **`DonutCore`** (library) — pure frame generators. `DonutFrameGenerator` does the torus rotation/projection math; `HelixFrameGenerator` and `ShapeFrameGenerator` are alternate shapes.
- **`DonutWallpaperApp`** (executable) — AppKit host. Renders the frames, picks a theme, and pushes the result to the desktop wallpaper slot.

Tests in `Tests/DonutCoreTests` cover the frame generators.

## Build & run

```bash
swift build -c release
swift run DonutWallpaperApp
```

Requires macOS 13+.

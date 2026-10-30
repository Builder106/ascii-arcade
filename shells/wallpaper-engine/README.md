# Wallpaper Engine prototype

This package renders the built-in donut scene using the same WebAssembly engine
as the website. Build it on a machine with the project's WASM toolchain:

```sh
bash scripts/package-wallpaper-engine.sh /path/to/new/wallpaper-folder
```

Copy the resulting folder to Windows and drag its `index.html` into Wallpaper
Engine's Create Wallpaper button. The editor copies the folder into its own
project directory. The package uses local files and respects Wallpaper Engine's
frame-rate and pause settings. Its WASM bytes are embedded in a local script so
the browser does not need to fetch them from a `file://` URL.

This is a prototype. Test the local WASM loader in Wallpaper Engine on Windows
before relying on it. Resizing, frame rate, pause and resume, and multi-monitor
behavior still need validation there. Scene and theme selection are not yet
implemented. DOOM is not included.

# JOURNAL — ASCII Arcade

> Dated log of decisions, pivots, incidents, and quotes. Add entries as
> things happen — retrospectives need this raw material to land.
> Reverse-chronological; one paragraph max per entry.

## 2026-06-10 — Merged donut + DOOM into ascii-arcade #milestone #decision

Combined the two sibling projects into one repo with `git subtree` so both
commit histories are preserved. The unifying idea (per the owner): not two
separate things, but one live-wallpaper customizer where the spinning donut and
playable text-mode DOOM are both selectable desktop backgrounds. DOOM became
just another `AsciiScene` rendered with the same CRT text drawing as the donut.

## 2026-06-10 — DOOM-as-wallpaper needs a screen buffer, not a terminal #decision

`doom_ascii` emits each frame as a full ANSI redraw — cursor-home (`ESC[;H`),
optional clear, then per-pixel truecolor SGR codes followed by a block glyph.
Rather than embed a full terminal emulator, wrote a minimal `DoomScreenBuffer`
that honors home/clear/erase and strips the SGR colour codes — just enough to
reconstruct the glyph grid for a monochrome themed wallpaper. The block glyphs
happen to suit the donut aesthetic.

## 2026-06-10 — Kept the Vapor browser path as a bonus #decision

The product is now desktop-first, but chose to keep `Server` / `Hotword` /
`WatcherCLI` so DOOM stays playable in a browser tab too (useful where global
keystroke capture isn't available). Refactored the server's binary/IWAD lookup
into a shared `DoomLauncher` so the app and the server resolve DOOM identically.

## 2026-04-29 — Donut wallpaper host landed #milestone

The `donut` project's initial commit: `DonutCore` (the torus and helix frame
generators) plus an AppKit host that paints ASCII into a desktop-level window
with CRT scanlines, a soft glow, and theme presets (Hacker / Amber / Ice /
Ghost). This host is the foundation ASCII Arcade is built on.

## 2026-02-19 — Ghost Protocol audit stub #incident

A `GOALS.md` was auto-generated for the DOOM project by a "Ghost Protocol audit"
showing 0/0 goals complete — a sign the project had drifted with no tracked
objectives. Dropped during the merge in favor of this journal.

## 2025-09-24 — DOOM-over-PTY prototype #milestone

DOOM's initial SwiftPM workspace: a PTY bridge wrapping `doom_ascii`, a Vapor
WebSocket server streaming frames to an xterm.js frontend, a KMP-style hotword
detector, and a LaunchAgent watcher that opened the browser on the hotword. Its
README summed up the state: "Working in pieces — end-to-end integration is the
rough edge." The merge turned that PTY bridge into the heart of the DOOM scene.

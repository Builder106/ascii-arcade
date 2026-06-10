# JOURNAL — ASCII Arcade

> Dated log of decisions, pivots, incidents, and quotes. Add entries as
> things happen — retrospectives need this raw material to land.
> Reverse-chronological; one paragraph max per entry.

## 2026-06-10 — Pushed public + CI toolchain mismatch #milestone #incident #decision

Pushed to https://github.com/Builder106/ascii-arcade (public; description + 11
topics) and added a 1200×630 social-preview card. The first CI run failed on a
real toolchain mismatch: the macos-14 runner ships Swift 5.10, which can't read
the `Package.resolved` (format v3) my local Swift 6.3 wrote — so it discarded the
pin and re-resolved to the latest Vapor (4.121.4), which itself requires Swift
tools 6.0 → `error: using Swift tools version 6.0.0 but the installed version is
5.10.0`. Fixed by moving CI to `macos-15` (Xcode 16 / Swift 6) to match the
committed pin, and bumped `actions/checkout` v4→v5 to clear the Node 20
deprecation. Takeaway: **this project now requires a Swift 6 toolchain** because
the committed Vapor pin (4.121.4) declares tools 6.0 — worth stating in the
README's build requirements if older toolchains need support.

## 2026-06-10 — Scaffolded the repo baseline #milestone #incident

Added the storefront baseline: hand-authored SVG banner (light/dark, 1200×420)
with PNG fallbacks, a phosphor-donut logo + apple-touch-icon, shields.io badges,
a macOS CI workflow (build+test, plus a job proving setup.sh still builds
doom_ascii), and a playwright-bdd demo suite for the browser-DOOM surface. Two
environment gotchas surfaced while validating the demo: SwiftPM's package cache
breaks under a global git `safe.bareRepository=explicit` (worked around with a
`GIT_CONFIG_*` env override), and SwiftPM's `build.db` throws `disk I/O error`
on this Google-Drive-synced checkout — recording the live demo needs a
local-disk clone. The scaffold is validated (`bddgen` generates the specs); the
live capture is left to the user on local disk.

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

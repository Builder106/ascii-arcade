# JOURNAL — ASCII Arcade

> Dated log of decisions, pivots, incidents, and quotes. Add entries as
> things happen — retrospectives need this raw material to land.
> Reverse-chronological; one paragraph max per entry.

## 2026-06-23 — Made it a real desktop app (unsigned, self-distributed) #decision #milestone

Promoted ascii-arcade from a `swift run` tool to an installable `.app` + DMG.
Owner's call on scope: do Tier 1 (proper local app) *and* Tier 2 (distribution)
but **skip the Apple Developer account** — recipients bypass Gatekeeper manually
(right-click → Open / `xattr -dr com.apple.quarantine`). Added: `UserDefaults`
persistence of scene/theme/capture/idle/per-scene settings (restored on launch;
returning users also get their theme wallpaper back, first-run leaves the desktop
alone); launch-at-login via `SMAppService.mainApp`; bundle-aware resource lookup
in `DoomLauncher` (checks `Bundle.main` so WADs/doom resolve from the .app, not
just cwd); and `scripts/make-app.sh` + `make-dmg.sh` (release build → Info.plist
with `LSUIElement` → `.icns` from the 512 PNG → bundled Freedoom WADs → ad-hoc
sign → DMG with an Applications drop + first-launch note). Verified the full
save/quit/relaunch/restore cycle against the bundled app. Honoured the existing
"don't redistribute GPL doom_ascii" policy: WADs (BSD) are bundled, doom_ascii is
behind an opt-in `INCLUDE_DOOM=1` flag. Mac App Store stays off the table —
global key capture + arbitrary wallpaper-setting don't survive sandboxing.

## 2026-06-23 — Life read as noise; reseeded it with classic patterns #feedback #decision

Owner watched the Life scene and said the designs weren't clear — it looked like
a sparse scatter rather than anything recognisable. Root cause: I'd seeded it
with uniform random soup, which Conway's rules famously decay into "ash" (a
field of tiny 1–2 cell still-lifes and blinkers). Three changes: (1) seed with
*curated* patterns — gliders, lightweight spaceships, pulsars, Gosper glider
guns, acorns/R-pentominoes — stamped at random positions/orientations, so it
grows recognisable shapes and sustained motion; (2) run the sim on a coarser
*logical* grid scaled up into solid `█` blocks (new Cell-size setting, default
3×3 px/cell) so structures are big enough to read; (3) drop the now-irrelevant
random-density setting. The `#` glyph at 1-px cells was a big part of the "looks
like noise" problem — solid blocks read far better. Also a reminder logged: the
desktop window is transparent, so `screencapture` composites it over a white
matte instead of the real black desktop — the grey background in shared
screenshots is a capture artifact, not the actual wallpaper.

## 2026-06-23 — Fixed the colour-scene lag with a batched Core Text renderer #incident #decision

The per-cell colour path I'd just added lagged badly on the dense scenes (Fire,
Matrix). Root cause: `draw(_:)` rebuilt an `NSMutableAttributedString` every
frame and called `addAttribute(.foregroundColor, range:)` per colour run — and
on a smooth gradient like Fire almost every one of the ~10k cells is its own
run, so that's thousands of attribute mutations plus a full Cocoa text layout
every frame, at the display's full refresh rate, re-measuring the font each
frame too. Replaced the whole text path with a **batched Core Text renderer**:
bucket every non-blank cell's glyph by colour, then one `setFillColor` +
`CTFontDrawGlyphs` per bucket. Fire's ~10k cells collapse to ~35 buckets (its
37-entry palette), Matrix to ~230, Donut to 1. Also cached font metrics +
glyph + CGColor lookups (no per-frame `("@").size(...)`), and capped redraws to
~30fps via the `CVDisplayLink` callback (ASCII doesn't need 120Hz; the text
fill is the hot path). Measured after: avgDraw **~8ms/frame** across Donut/
Matrix/Fire at a 176×57 grid — comfortably inside the 33ms budget, steady
23–25fps with no stutter (was visibly lagging before). Gotcha: drawing Core
Text glyphs upright in an `isFlipped` `NSView` needs an explicit
`translateBy(y: height) + scaleBy(y: -1)` on the context with positions
converted to that y-up space. Instrumentation left behind, env-gated: run with
`ASCII_FPS=1` to log scene/grid/fps/avgDraw/batches once a second.

## 2026-06-23 — Expanded the cabinet: five scenes + per-cell colour #milestone #decision

Added five new scenes (Matrix rain, Doom-fire, Conway's Game of Life, a pipes
screensaver, and a big block-digit clock) and a colour pipeline so they're not
all stuck in one theme tint. The design call that made this clean: a
platform-neutral `RGBColor`/`ColoredFrame` in `AsciiArcadeCore` (no AppKit) plus
an optional `coloredFrame(atTime:)` on `AsciiScene` that defaults to `nil` — so
the donut/helix monochrome path is untouched and the host only takes the
per-cell-colour branch when a scene opts in. The four stateful scenes share a new
`SteppedScene` base that converts the host's "frame at time `t`" pull into a
fixed-timestep simulation (accumulate `dt`, clamp after stalls, cap catch-up
steps) — everything runs on the main thread, so unlike `DoomScene` it needs no
locking. DOOM now keeps the SGR truecolor it used to discard (`DoomScreenBuffer`
tracks a `currentColor` and a parallel colour grid), so it renders in its native
palette on the desktop too. Gotcha worth remembering: in the AppKit host
`RGBColor` is ambiguous because AppKit transitively imports the legacy Quickdraw
`RGBColor` from ApplicationServices — had to qualify it as
`AsciiArcadeCore.RGBColor`. Also added *Scene Settings* (per-scene discrete knobs
surfaced as menu submenus) and *Auto-cycle when idle* (poll `CGEventSource`
idle seconds; slideshow the scenes after 90 s, snap back on input; pause
rendering on display sleep via `NSWorkspace` notifications).

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

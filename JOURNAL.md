# JOURNAL — ASCII Arcade

> Dated log of decisions, pivots, incidents, and quotes. Add entries as
> things happen — retrospectives need this raw material to land.
> Reverse-chronological; one paragraph max per entry.

## 2026-06-26 — E2e demo suite live on Rust web shell; two MP4 recordings produced #milestone

Replaced the Swift Vapor / DOOM webServer in the playwright demo config with the new `aa-web` axum shell. Added `aa_core::ansi::frame_to_ansi` (ANSI truecolor run-length encoder) so any built-in scene can be streamed to an xterm.js terminal over WebSocket. New `shells/web` crate serves a scene-picker page + WebSocket endpoint at `/ws/{scene}` at 30 fps. Demo suite: warmup + DOOM steps generalized to scene-agnostic; `01-doom.feature` repurposed as the donut demo; `02-matrix.feature` exercises the runtime scene switcher. All four tests passed on first run (`npm run demo`) and the reporter wrote `donut-in-the-browser-*.mp4` and `matrix-rain-in-the-browser-*.mp4` to `e2e/recordings/`.

## 2026-06-25 — Windows shell done; icon layer is a Windows Server/RDP limitation #decision

After confirming the wallpaper renders and application windows appear correctly above it, we hit one remaining issue: desktop icons disappear when aa-windows runs on Windows Server 2019 via RDP. Tried five approaches (WS_CHILD + HWND_BOTTOM, WS_POPUP + SetWindowPos, dropping WM_SPAWN_WORKERW) — none restored the icon layer on Server. Root cause: Windows Server's desktop shell in an RDP session uses a different DWM composition path from Windows 10/11 consumer editions; the icon layer (SHELLDLL_DefView) doesn't coexist with a custom GDI wallpaper surface the same way. Confirmed NOT a code bug: normal application windows (Server Manager, CMD) correctly appear above the wallpaper in all tests. Declared the Windows shell done pending one native Windows 10/11 desktop test for icon behavior. GCP VM stopped; branch merged to main.

## 2026-06-25 — WorkerW confirmed working on real Windows (GCP VM via FreeRDP) #milestone

Ran `aa-windows.exe donut` on a GCP Windows Server 2019 spot VM (e2-medium, us-central1-a) connected via FreeRDP. The WorkerW technique works: the ASCII donut rendered behind the CMD window and other normal windows, confirming the wallpaper layer is correctly below the interactive window stack. Three bugs hit and fixed before the render succeeded: (1) missing VCRUNTIME140.dll — fixed permanently with `target-feature=+crt-static` in `.cargo/config.toml`; (2) no app icon — fixed by generating `icon.ico` from `assets/logo.svg` via rsvg-convert + ImageMagick and embedding via `winres` in `build.rs`; (3) `WS_CHILD` + null parent crash — `CreateWindowExW` with `WS_CHILD` style requires a valid parent HWND; passing `HWND::default()` crashed silently; fixed by passing the WorkerW host HWND directly. One known caveat specific to Windows Server / RDP sessions: the desktop icons disappear after `WM_SPAWN_WORKERW` — on real Windows 10/11 consumer desktops the icon layer survives. Also patched: `#![windows_subsystem = "windows"]` added so the binary runs without a console window and can't be killed by closing one.

## 2026-06-24 — Saw the actual pixels: headless sway + grim, caught a theme bug #milestone #incident

Closed the "never eyeballed the render" gap on the Wayland side without a physical display: ran **headless sway** on `ampere-dev` (`WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman`), had its config `exec` launch `aa-linux` + `grim` to screenshot the output, then `scp`'d the PNG back to look at it. First capture (`aa-linux matrix amber`) showed the Matrix rain rendering correctly as a real layer-shell background surface — but **green, not amber**. The bug: the Windows/Linux shells never called `Scene::apply_base_color(theme.text)`, so colour scenes ignored the theme (the macOS host does call it — that's why Matrix "turns amber under Amber" there). Every compile/clippy/test gate was green; only looking at the pixels caught it. One-line fix in each shell; re-captured and confirmed amber. Lesson reinforced (cf. the DOOM-upside-down entry): pick a concrete visual expectation and verify against it — "it compiles and renders something" isn't "it renders the right thing."

## 2026-06-24 — Wayland backend written + whole stack verified on real Linux #milestone

Owner handed over an Ubuntu 24.04 / aarch64 VM (`ampere-dev`, Oracle Cloud) — the Linux build host the Wayland backend needed. Workflow: author the `wlr-layer-shell` backend locally with the normal edit tools, `rsync` the workspace to the VM, and `cargo check`/`clippy`/`test` there natively (the VM is headless — no compositor — so this verifies compilation and the full test suite, not on-screen rendering). The Wayland module is a real smithay-client-toolkit `background`-layer surface (handlers + delegate macros + `SlotPool` shm + `wl_surface.frame`-driven animation); first compile surfaced only 8 `u32`/`usize` mix-ups in the blit loop — the SCTK trait/delegate wiring was right. After that: `aa-linux` (X11 **and** Wayland) compiles + `clippy -D warnings` clean, and the **entire workspace's tests pass natively on aarch64** (39+28+9). Bonus wins from having a real Linux box: caught that `scripts/setup.sh` was zsh-only (`CLEANUP() { … }` missing its `;`, plus a `#!/bin/zsh` shebang) — fixed to `#!/usr/bin/env bash`, then it built `doom_ascii` on Linux and the **DOOM e2e spawn test passed over forkpty**, so DOOM-on-Rust is now proven on macOS *and* Linux. Residual gap is now narrow and honest: the shells compile + run (graceful headless failure confirmed) but on-screen wallpaper rendering — WorkerW on real Windows, layer-shell on a real wlroots/KDE display — still hasn't been eyeballed.

## 2026-06-24 — Both native shells landed (WorkerW + X11) #milestone

After the agent assigned the shells got only as far as the Linux Cargo features before the session cutoff, finished both by hand. `aa-windows`: the WorkerW dance (spawn via Progman `0x052C`, `EnumWindows` for the `SHELLDLL_DefView` sibling, `SetParent` a render window in) + a per-frame GDI `StretchDIBits` blit of the `aa_render` buffer (BGRA, top-down DIB). `aa-linux`: an X11 root-pixmap backend on pure-Rust `x11rb` — paint a pixmap, publish `_XROOTPMAP_ID`/`ESETROOT_PMAP_ID`, re-blit + `clear_area` each frame, `put_image` banded under the server's max request size; also covers XWayland. Both verified by cross-`cargo check` + `clippy -D warnings` (`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`) — they can't link on the macOS dev box, so CI's native runners are the runtime gate. The honest gap: the **Wayland `wlr-layer-shell` backend is a documented stub** — its crates (`smithay-client-toolkit`/`wayland-client`) link `libwayland` and won't even compile-check from macOS, so writing it blind would ship unverifiable code; it needs a Linux build host or a CI iteration loop. X11/XWayland covers the verifiable path for now.

## 2026-06-24 — Rust engine ported + DOOM proven over portable-pty #milestone

Fanned the Rust port out across four parallel background agents (worktree-isolated, one crate each): scenes, rasteriser, DOOM driver, shells. Three landed cleanly; all four hit a session limit mid-task and none committed, so the orchestrator captured their uncommitted worktrees, merged, and finished the wiring. Result on `rust-crossplatform`: `aa-core` has all seven scenes (donut/helix/matrix/fire/pipes/life/clock) + the `Stepper` fixed-timestep helper; `aa-render` rasterises frames through an embedded 8×16 font with glow + scanline FX; `aa-doom` drives real `doom_ascii` over `portable-pty` — the **end-to-end spawn is verified rendering on macOS**, which is the proof the cross-platform DOOM bet holds (same API = ConPTY on Windows, forkpty on Linux). Workspace is green: tests + `clippy -D warnings` + `fmt`. Two real bugs caught integrating agent output: a float-drift accumulator in `Stepper` that silently swallowed a due step (fixed with an epsilon), and a screen-buffer test that wrongly assumed terminal auto-wrap (doom emits an explicit newline per scanline, so no-wrap is correct). **Still open:** the two native shells (`aa-windows` WorkerW, `aa-linux` X11/layer-shell) are still skeletons — the agent assigned them only got as far as the Linux Cargo features before the cutoff. That's the actual product layer and the next real build.

## 2026-06-24 — Going cross-platform native: Rust engine + per-OS wallpaper shells #decision #pivot

Owner wants the *native* wallpaper experience (a window behind the desktop icons) on Windows and Linux, not just the existing browser path. Accepted the irreducible cost: a native wallpaper is a different OS mechanism per platform (macOS desktop-level `NSWindow`; Windows WorkerW reparenting à la Lively; Linux X11 root-pixmap + `wlr-layer-shell`), so the shells can't share code — only the engine can. Decision: **rewrite the ~1800-line `AsciiArcadeCore` engine in Rust** rather than ship the Swift core as a C-ABI lib or go Swift-everywhere. Rationale — Rust has the best-supported shell tooling on every target (`windows` crate for WorkerW, `smithay`/layer-shell + `x11rb` for Linux), and `portable-pty` covers ConPTY *and* forkpty uniformly, which solves DOOM-on-Windows for free (current `PTYBridge` is `forkpty`-only, doesn't exist on Windows). Linux scope deliberately bounded to X11 + wlroots/KDE-Wayland; **GNOME-Wayland punted** (needs a Shell extension — biggest pain, separate distribution burden). Open question deferred: whether the macOS shell stays Swift/AppKit (keep working code) or also moves to Rust for a single codebase.

## 2026-06-24 — Stable signing identity to stop repeated Accessibility prompts #incident #decision

`make-app.sh` ad-hoc signed (`codesign --sign -`), which pins the app's designated requirement to the binary's `cdhash`. Every rebuild changed the hash, so macOS TCC treated each reinstall as a brand-new app and re-prompted for Accessibility, leaving a graveyard of dead grants. Fix: a one-time self-signed code-signing identity (`scripts/setup-signing.sh`) makes the requirement identity-based (`identifier … and certificate leaf = H"…"`) and constant across builds. Gotchas hit along the way: OpenSSL 3 writes a PKCS#12 MAC that macOS `security import` rejects ("MAC verification failed") — needs `-legacy -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1` plus a non-empty password; and the self-signed cert reports `NOT_TRUSTED` so the `make-app.sh` guard must list identities without `-v` (valid-only hides it). codesign signs with the untrusted cert fine, and TCC pins the leaf hash literally so trust is irrelevant. `reinstall.sh` now self-heals by recreating the identity if it's missing.

## 2026-06-24 — DOOM gamma defaulted to level 2 (was OFF) #decision

Owner asked "is there supposed to be this much shadow?" looking at a near-black DOOM frame. It was faithful — DOOM ships with `usegamma` OFF (its darkest), so unlit/distant sectors fade to pure black, and we paint that honestly (confirmed it's opaque black colour data, not the transparent→white-matte capture artifact, so not a missing-pixel bug). But "authentic" isn't the goal for a wallpaper. The bundled `doom_ascii` accepts `-fixgamma N` (0=off … 4=brightest; same as the in-game F11 / `key_menu_gamma`). Added a `DOOM_GAMMA` env knob to `DoomLauncher`, default 2 — dev capture went from a black void with two lit pillars to a fully legible room (walls, fences, enemies, stairs). Fits the owner's standing preference for legible-bold visuals over noise.

## 2026-06-24 — DOOM rendered upside down since the bitmap path landed #incident

`drawBitmap` computed each cell's y as `rect.maxY - (row+1)*cellH`, but `SceneView.isFlipped` is `true` (y grows downward, origin top-left) — so row 0 landed at the bottom and the whole framebuffer rendered vertically flipped. The glyph path dodges this with its `translateBy(y:viewH)/scaleBy(1,-1)` transform; the bitmap path drew directly and never compensated. Fix: map row 0 to `rect.minY` (`yTop = rect.minY + row*cellH`). Lesson worth keeping: this shipped flipped through *four* of my own verification screenshots and I read them all as fine — an HUD-at-top DOOM frame looks plausible enough to fool a glance, and I anchored on "is it colourful and detailed" instead of "is the status bar where it belongs." The owner caught it in one look. Pick a known landmark (DOOM status bar = bottom; messages = top) before calling a frame correct.

## 2026-06-24 — DOOM default resolution bumped to native 320×200 #decision

After the black-screen fix, owner asked "is this the clearest DOOM can be in ASCII?" — it wasn't; the default was `scaling=2` (160×100). Demoed `scaling=1` side by side: at native 320×200 the HUD digits, the red menu text, the blue "NEW GAME", and the FREEDOOM ∞ wordmark all become legible where they were mush before. doom_ascii can't exceed this — it's the engine's internal framebuffer. Owner chose native-as-default over a menu toggle or keeping the lighter mode, accepting the cost (~24-28fps + ~40ms/frame vs ~30fps + ~16ms). Changed `DoomScene`'s default `scaling` 2→1; `DOOM_SCALING` env still overrides for a lighter frame. Worth remembering: for DOOM the app doesn't render glyphs at all — `drawBitmap` paints each cell as a solid colour rect, so "clarity" is purely pixel resolution; true ASCII-ramp glyphs would read more retro but less clear.

## 2026-06-24 — DOOM black-screen regression: stripped binary + invisible message #incident

Reported "DOOM now just shows a black screen." Two compounding causes: (1) the recent reinstalls defaulted to `INCLUDE_DOOM=0`, so the installed `.app` no longer bundled `doom_ascii` and DOOM hit the "not found" message path; (2) the new fixed-resolution `drawBitmap` host path (added for the pixelation fix) skipped every cell with a `nil` colour — and `showMessage` writes uncoloured text — so the message painted nothing and read as pure black. Fixed `drawBitmap` to fall back to `themeTextColor` for uncoloured non-blank cells (matching the glyph path), and reinstalled with `INCLUDE_DOOM=1`. Verified the render path itself was never broken: a dev build run from the repo root (where `bin/doom_ascii` resolves) shows DOOM crisp at grid 320×100, ~30fps — HUD, face, and wall textures all sharp, exactly the resolution win the pixelation rework was after.

## 2026-06-23 — Added in-app screenshot and 3-second clip recorder #milestone #decision

Made capturing the wallpaper a first-class feature after observing that macOS's native ⌘⇧3/4 skips the desktop-level window entirely (it samples the wallpaper compositor, not the live window backing store). The fix lives in the app: "Save Screenshot (⌘⌥S)" and "Record 3-Sec Clip (⌘⌥R)" under a new Capture section in the menu bar. Both use `CGWindowListCreateImage(.null, .optionIncludingWindow, windowID, .bestResolution)` to pull directly from the window's backing store — this bypasses the compositor and works regardless of window level. Screenshot saves a PNG to ~/Desktop and also copies it to the clipboard (so ⌘V works immediately). The clip recorder fires a `DispatchSourceTimer` at 15 fps on a background serial queue, converts each `CGImage` to a `CVPixelBuffer`, feeds it to an `AVAssetWriterInputPixelBufferAdaptor`, then finalises a .mp4 via `AVAssetWriter` at the 3-second mark (or earlier on manual stop). The status-bar `◎` button blinks `◉` during recording and flashes `✓` / `✗` on outcome — the user never sees "window level", "compositor", or "backing store". Design call: both shortcuts (⌘⌥S, ⌘⌥R) are handled in the global NSEvent monitor alongside the existing ⌘⌥C scene-cycle shortcut, so they work even when no app is frontmost.

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

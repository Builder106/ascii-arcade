# Interactive DOOM (WASM)

`docs/landing-page-design.md` deliberately left this out: shipping a WASM
build of `doom_ascii` makes the site a GPL-2.0 distributor, with the
source-offer obligations that brings, and that reversal deserved its own
decision rather than falling out of a landing page as a side effect. This
document is that decision. It covers making the hero's "Play it" button
start a genuine, interactive game, in place of the message it shows today.

Implemented as two plans, not one. `doom-ascii` is a real, 72-file
Chocolate-Doom-derived C codebase with no prior Emscripten port to copy —
getting the toolchain, the patched entry point, and the WASM-to-canvas
pixel pipeline working at all is genuinely exploratory in a way that
picking a keyboard-mapping table or writing a licence footer is not.
**Plan A** covers the toolchain, the patches, and a first successful
compile that renders DOOM's boot screen through the canvas — no input yet.
**Plan B** — input, touch controls, session lifecycle, licensing artifacts,
tests — gets written once Plan A is verified working, grounded in what
actually happened rather than more upfront guessing about a codebase this
document's author had not yet compiled.

## Why the platform-backend approach

`doom-ascii` — like every `doomgeneric`-based source port — isolates all
platform I/O behind a small callback interface declared in
`src/doomgeneric.h`: `DG_Init`, `DG_DrawFrame`, `DG_SleepMs`,
`DG_GetTicksMs`, `DG_ReadInput`/`DG_GetKey` (this fork splits input into a
queue-refill call and a one-event-at-a-time drain, not a single poll), and
`DG_SetWindowTitle`. `src/doomgeneric_ascii.c` — the file that makes the
existing native build print ANSI to a terminal — is one implementation of
that interface, plus the ASCII/braille glyph-encoding logic specific to a
terminal. Nothing about DOOM's game logic, menu, or 3D renderer is
terminal-specific; only that file is, and even then only its `DG_*`
implementations, not the engine it sits in front of.

The `DG_*` callbacks are invoked from a small number of fixed call sites
deep in Chocolate-Doom-derived engine code (`I_FinishUpdate` in
`src/i_video.c` calls `DG_DrawFrame`; `I_GetEvent` in `src/i_input.c` calls
`DG_ReadInput`/`DG_GetKey`) — none of that call graph changes for a new
backend. What's genuinely new: `src/i_main.c`'s `main()` calls
`dg_Create()` (allocates the pixel buffer, calls `DG_Init` once) then
`D_DoomMain()`, which never returns — it ends inside `D_DoomLoop()`
(`src/d_main.c`), whose body is a literal `while (1)`. See "The blocking
main loop" for what changes there.

```text
doom-ascii source (pinned upstream commit)
  +- existing game/menu/renderer logic (untouched)
  +- src/doomgeneric_wasm.c (new platform backend: DG_Init, DG_SleepMs,
  |    DG_GetTicksMs, DG_SetWindowTitle, DG_ReadInput/DG_GetKey — the
  |    input side is a small ring buffer JS feeds via an exported function)
  \- patched src/d_main.c's D_DoomLoop — emscripten_set_main_loop()
       replaces the while(1); everything before it (one-time engine init)
       runs first as it already does. src/i_main.c is untouched:
       emscripten_set_main_loop(fn, 0, 1)'s simulate_infinite_loop mode
       throws a JS exception to unwind the C stack rather than returning,
       so D_DoomMain()/main() keep behaving as if D_DoomLoop never
       returns, exactly like the native build
            \- DG_ScreenBuffer (uint32_t*, one pixel per DOOMGENERIC_RESX x
               RESY cell — DOOM's own render resolution, not a character
               grid) exported via EMSCRIPTEN_KEEPALIVE
                 \- site/doom-play.js (new) feeds keys in, reads the pixel
                    buffer out, converts pixel -> glyph (see "Rendering")
                      \- its own <canvas>, painted via the existing
                         site/renderer.js atlas Renderer (a second
                         instance — see "Rendering", below)
```

## The blocking main loop

`D_DoomLoop()` (`src/d_main.c`) does one-time setup (`I_InitGraphics()` and
friends), then enters:

```c
while (1)
{
    I_StartFrame ();
    TryRunTics ();
    S_UpdateSounds (players[consoleplayer].mo);
    if (screenvisible)
        D_Display ();
}
```

which cannot run as-is under Emscripten without blocking the tab's main
thread. Two ways to reconcile that exist: compile with Emscripten's
`ASYNCIFY`, which transforms a blocking loop automatically at the cost of a
larger binary and roughly half the execution speed, or restructure by hand
— the four lines above become the callback body passed to
`emscripten_set_main_loop()`, dropping the `while (1)` itself since
Emscripten's own `requestAnimationFrame`-driven loop replaces it.

This design takes the manual route: no runtime penalty, which matters for
something meant to feel responsive under player input. It's also less
invasive than it first looks — the loop body doesn't change, only what
drives it — but it does touch `d_main.c` and `i_main.c`, both shared engine
files, not just the new platform file. One more consequence: `TryRunTics`
can reach `I_Sleep`/`DG_SleepMs` on certain code paths (network-sync
fallback), and an actual sleep would freeze the browser tab under a
`requestAnimationFrame`-driven loop — `DG_SleepMs` becomes a no-op in the
WASM backend, not a real sleep.

The restructuring and the new platform file are both captured as patch
files (see "Build pipeline"), so the exact, reviewable delta from upstream
is never a mystery diff buried in a build script.

## Rendering

`DG_ScreenBuffer` is a pixel buffer — `DOOMGENERIC_RESX * DOOMGENERIC_RESY`
`uint32_t`s, DOOM's own internal render resolution (320x200 divided by a
`-scaling` factor), each pixel already resolved to a colour via the
engine's palette. It is not a character grid; the ASCII-art conversion in
the native build is `doomgeneric_ascii.c`'s job, done in C, and none of
that glyph-encoding logic is reused here.

The conversion happens in JS instead, and it's simple: this mirrors the
same block-mode look the recorded attract loop already uses (`-chars
block`, captured via `scripts/record-doom.py`) — a dark pixel becomes a
bare space (matching `Renderer.paint()`'s existing skip-on-space
behaviour), everything else becomes a solid block glyph (`█`) in that
pixel's own colour. No luminance ramp, no ANSI quantization to port; the
buffer already carries full 24-bit colour per cell, which is strictly more
than `Renderer.paint()` needs. This keeps the new C surface to exactly the
`DG_*` callbacks — no glyph logic on the C side at all.

Live gameplay does **not** share `#grid`, the canvas the page's ambient
background scenes (Donut, Matrix, and so on) already paint to. `#grid` is
driven by `SceneDriver`'s own `requestAnimationFrame` loop, independent of
and indifferent to whether a DOOM session is active — painting a second,
unrelated animation into the same canvas would mean two independent loops
racing to draw the same pixels.

Instead, `doom-play.js` owns a second `<canvas>`, positioned over the hero
box, with its own `Renderer` instance from `site/renderer.js` — the same
glyph-atlas code, a different element and a different backing buffer. The
ambient background `SceneDriver` loop is paused for the duration of a
session (resumed on exit): with the visitor's attention and the page's
frame budget both committed to the game, continuing to animate an
off-focus background scene is wasted GPU/main-thread work, not a feature.

## Input

`doom-play.js` listens for keyboard and touch simultaneously — never
mutually exclusive, so a touchscreen laptop with a keyboard gets both:

- **Keyboard.** A translation table from `KeyboardEvent.code` to
  doomgeneric's key constants, covering menu navigation (arrows, Enter,
  Escape) and in-game controls (arrows or WASD to move, a fire key).
  Escape doubles as the exit path: it opens doomgeneric's own in-game menu,
  whose "quit" already terminates the session. Every mapped key gets
  `preventDefault()` while a session is active, so arrows and space stop
  double-acting as page scroll.
- **Touch.** An on-screen overlay — d-pad, fire/use, enter/escape — shown
  only during an active session, feature-detected via
  `matchMedia("(pointer: coarse)")` or `"ontouchstart" in window`. Touch
  buttons feed the same `DG_GetKey` queue as keyboard input; from the C
  side there is only one input source.
- **Focus and scroll.** On start, focus moves into the game canvas and the
  document's scroll is locked for the duration — arrow keys and Space are
  already claimed as game input, so leaving scroll live would mean the
  page silently moving under an active game. Stop (button, always visible)
  or Escape-to-quit-via-menu both tear the session down, release the
  scroll lock, and return focus to the "Play it" button.

## Build pipeline

A new `scripts/build-doom-wasm.sh`, parallel to the existing
`scripts/build-wasm.sh` for `aa-wasm`, run on `ampere-dev` like every other
build in this repo:

1. Clone `doom-ascii` at a **pinned commit SHA** — not `--depth 1` off the
   default branch the way today's `scripts/setup.sh` does. Pinned to
   `b5188d7c9c4da6c81264a7803e8725ac3df2cfea` (`Release 0.3.1`, verified
   current `HEAD` at the time of writing this document). The pin is what
   the GPL source-offer link (see "Licensing") points at; it has to stay
   exact.
2. Add `src/doomgeneric_wasm.c` (new file, not a patch — nothing upstream
   to diff against) and apply `patches/doom-wasm/main-loop.patch`, the
   `emscripten_set_main_loop` restructuring of `src/d_main.c`'s
   `D_DoomLoop` (guarded behind `#ifdef __EMSCRIPTEN__`, so the native
   build this repo already ships is byte-for-byte unaffected).
3. Compile with `emcc`, compiling the same 72-file `SRC` list the native
   Makefile already builds (`Makefile`'s `SRC` variable) plus the new file,
   swapped for `doomgeneric_ascii.c`. No SDL dependency exists anywhere in
   this codebase to route through Emscripten's SDL port — the only link
   flag the native build uses is `-lm`. Emscripten (`emsdk`) is a new
   toolchain dependency on `ampere-dev`, installed alongside the existing
   Rust/`wasm-bindgen` toolchain — the two don't interact.
4. Package `freedoom1.wad` into Emscripten's virtual filesystem so the
   engine's ordinary `fopen`-based WAD loading (`D_FindIWAD` →
   `D_FindWADByName` → `W_AddFile`, all generic Chocolate-Doom code with no
   backend-specific branching) resolves it unmodified. **This boundary is
   unverified** — confirming `-iwad`/`fopen` actually resolves through
   Emscripten's `MEMFS` the way it does through a real filesystem is part
   of Plan A's implementation work, not assumed here.
5. Output lands in `site/doom-wasm/`, gitignored like `site/pkg/` — the
   same "commit only on release" strategy already planned for `aa-wasm`.

## WAD hosting and preload

`freedoom1.wad` (27 MB) is not committed to git. It's hosted as a GitHub
Release asset — the same pattern as the `site/pkg` release strategy — and
fetched at runtime, not build time. Freedoom's own licence already permits
this; it's the same WAD `scripts/setup.sh` and the attract-mode recording
already use, so this opens no new licensing question, just a new place it's
served from.

After `boot()` settles, a `requestIdleCallback`-scheduled fetch pulls the
WAD and the `doom-wasm` runtime together, in the background, so a click on
"Play it" is a near-instant start rather than a wait. That preload is
skipped when `navigator.connection?.saveData` or `prefers-reduced-data` is
set — on a metered connection, "Play it" instead fetches on click, with a
visible loading state, rather than costing every visitor ~27 MB regardless
of whether they ever press play.

## Licensing and GPL compliance

Three concrete artifacts, not aspirational language:

- `patches/doom-wasm/*.patch` — the exact, reviewable diff from the pinned
  upstream commit; this plus the pin *is* the corresponding source, in the
  sense that anyone can reproduce the exact binary from them.
- A visible line near the feature (dock or footer, not buried) linking the
  pinned commit on `wojciech-graj/doom-ascii` and naming the GPL-2.0
  licence — a durable, specific reference rather than "see upstream."
- `LICENSES/doom-ascii.GPL-2.0` — the licence text itself, vendored, so the
  offer doesn't depend on any external site staying up.

Keeping the new platform backend in its own file, communicating with the
rest of the site only through the narrow buffer/key-queue interface, is
also what keeps this a "mere aggregation" of independent programs rather
than a combined work — the standard, lower-risk reading, not a guarantee,
and not legal advice.

## Failure behavior

Matches the rest of the site: every enhancement here degrades, nothing
hard-fails silently.

- WAD or WASM fetch fails → "Play it" shows a visible error state, not a
  silent no-op.
- WASM instantiation or `emscripten_set_main_loop` throws during startup →
  caught, and the button falls back to today's static message ("not
  playable here — clone the repo and run it locally") rather than leaving
  a half-started session on screen.
- Emscripten/WebAssembly unsupported at all → same static fallback message
  the button already shows, unchanged.

## Accessibility

The attract-mode `<pre id="doomFrame">` stays `aria-hidden` — it's still
decorative, unchanged by this feature. The new game canvas is a real
interactive feature and gets a `role`/`aria-label` identifying it as a game
with keyboard instructions, plus the focus management described under
"Input." This is a deliberately minimal bar, not a full non-visual way to
play — consistent with how most canvas-rendered browser games ship — and
is called out here explicitly rather than left as a silent gap.

## Testing and CI tier

This adds a genuine integration-level surface — WASM instantiation, a
network-dependent preload path, simulated input — beyond the unit and e2e
tiers the rest of the site relies on. New: `e2e/tests/site/doom-play.spec.ts`,
covering session start/stop (both Stop and Escape-via-menu tear down and
restore attract mode cleanly), scroll lock engaging and releasing,
keyboard and simulated-touch input both registering, and the `saveData`
fallback path under a mocked network condition.

`budget.spec.ts`'s 150 kB budget explicitly does **not** grow to include
`doom-wasm/` or the WAD — worth a comment in that test explaining why, so
it doesn't read as an oversight later; the whole point of the preload
strategy is that this weight never blocks or counts against the page's own
load.

`a11y.spec.ts` currently excludes `#doomFrame` from axe entirely
(`aria-hidden`, decorative). That blanket exclusion needs to be narrowed:
the attract-mode `<pre>` keeps its exclusion, but the new game canvas gets
its own scoped check for the minimal `role`/`aria-label` requirement above,
rather than inheriting a skip that no longer describes it.

## Out of scope

- **Audio.** Confirmed, not assumed: `doom-ascii`'s sound backend is
  guarded behind an `ORIGCODE` build flag that its Makefile never defines,
  so `sound_module` stays `NULL` and every audio call is already a no-op in
  the existing native build. There is nothing to port.
- **Save games, multiplayer, IWAD selection.** Boots straight into
  Freedoom via doomgeneric's own menu; no save/load UI, no network play, no
  choice of WAD.
- **Older-browser fallback beyond the existing message.** No WebAssembly,
  no Emscripten support, no `emscripten_set_main_loop` — same static
  fallback the button shows today, not a degraded-but-playable mode.
- **Offline play / caching the WAD beyond normal HTTP cache behaviour.** No
  service worker, no explicit cache management.

# Interactive DOOM — Plan A: Toolchain, Build, Walking Skeleton

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Get `doom-ascii` compiling to WebAssembly via Emscripten and rendering DOOM's boot/menu screen — real pixels, not a placeholder — through the site's existing glyph-atlas canvas renderer. No keyboard/touch input, no session lifecycle, no licensing artifacts, no polish. Proof that the toolchain and the pixel-to-canvas pipeline actually work end to end.

**Architecture:** A new platform backend (`doomgeneric_wasm.c`) implements the six `DG_*`callbacks`doomgeneric.h`requires; a small,`__EMSCRIPTEN__`-guarded patch to `d_main.c`'s `D_DoomLoop`swaps its native`while(1)`for`emscripten_set_main_loop()`. Neither change touches DOOM's engine, menu, or renderer — see `docs/doom-wasm-design.md`for the full reasoning.`site/doom-play.js`loads the compiled module, reads the exported pixel buffer every frame, converts it to glyphs (dark → space, lit →`█`in that pixel's colour), and paints via a dedicated`Renderer`instance from`site/renderer.js`.

**Tech Stack:** C99 (`doom-ascii`, pinned commit `b5188d7c9c4da6c81264a7803e8725ac3df2cfea`), Emscripten (`emsdk`), the existing vanilla-ES-module site stack.

## Global Constraints

- All builds run on `ampere-dev`, never the Mac (`verify-on-vm`/`dev-on-vm`, per this environment's standing rule).
- The native macOS build (`scripts/setup.sh`, `doomgeneric_ascii.c`) must remain byte-for-byte unaffected — the `D_DoomLoop`patch is guarded behind`#ifdef __EMSCRIPTEN__`, never a runtime flag.
- `doom-ascii`source is pinned to commit`b5188d7c9c4da6c81264a7803e8725ac3df2cfea` for this entire plan — every clone in every task uses this exact SHA, not a branch.
- `site/doom-wasm/`(build output) is gitignored, matching`site/pkg/`'s existing pattern — nothing here is committed except the source files this plan creates under `patches/doom-wasm/`and`site/`.
- No new JS dependencies. `site/doom-play.js`is a vanilla ES module, same as every other file in`site/`.

---

## Task 1: Pin the source, verify the native build still works at that commit

**Files:**

- Create: `patches/doom-wasm/README.md`

**Interfaces:**

- Produces: confirmation that `b5188d7c9c4da6c81264a7803e8725ac3df2cfea` is a valid, buildable commit — every later task clones this exact SHA.

- [ ] **Step 1: Clone the pinned commit on ampere-dev and confirm the SHA**

Run:

```bash
ssh ampere-dev "rm -rf /tmp/doom-wasm-verify && git clone https://github.com/wojciech-graj/doom-ascii.git /tmp/doom-wasm-verify && cd /tmp/doom-wasm-verify && git checkout b5188d7c9c4da6c81264a7803e8725ac3df2cfea && git rev-parse HEAD"
```

Expected: last line prints exactly `b5188d7c9c4da6c81264a7803e8725ac3df2cfea`.

- [ ] **Step 2: Confirm the native build still succeeds at this pinned commit**

Run:

```bash
ssh ampere-dev "cd /tmp/doom-wasm-verify && make CFLAGS='-O2 -w' && find . -name 'doom-ascii' -type f"
```

Expected: build succeeds, a `doom-ascii`binary is found. This is the same Makefile`scripts/setup.sh` already drives against a floating branch — this step just proves the pin didn't land on a broken commit.

- [ ] **Step 3: Clean up and write the pin record**

```bash
ssh ampere-dev "rm -rf /tmp/doom-wasm-verify"
```

Create `patches/doom-wasm/README.md`:

```markdown

# doom-wasm patches

Everything in this directory targets `wojciech-graj/doom-ascii` pinned to
commit `b5188d7c9c4da6c81264a7803e8725ac3df2cfea` (`Release 0.3.1`,
2025-07-21). This pin is what `scripts/build-doom-wasm.sh` checks out and
what the GPL-2.0 source-offer link in the site's footer/dock points at —
it must never silently float to a newer commit. Bumping it is a deliberate,
reviewed action: re-verify every file this directory touches still applies
cleanly and every line number `doomgeneric_wasm.c`'s comments reference is
still accurate.

- `doomgeneric_wasm.c` — new platform backend implementing doomgeneric's

  `DG_*` interface for a browser instead of a terminal. Not a patch; a
  whole new file, copied into the pinned clone's `src/` at build time.

- `main-loop.patch`— the`emscripten_set_main_loop` restructuring of

  `src/d_main.c`'s `D_DoomLoop`, guarded behind `#ifdef __EMSCRIPTEN__` so
  the native build (`scripts/setup.sh`) is unaffected.

See `docs/doom-wasm-design.md` for the full design and
`docs/doom-wasm-plan-a.md` for how these get built and verified.
```

- [ ] **Step 4: Commit**

```bash
git add patches/doom-wasm/README.md
git commit -m "docs: pin doom-ascii commit for the WASM port"
```

---

## Task 2: Install Emscripten on ampere-dev

**Files:** none (environment setup only)

**Interfaces:**

- Produces: a working `emcc`on`ampere-dev`'s `PATH`, callable from every later task's build commands.

- [ ] **Step 1: Install emsdk**

Run:

```bash
ssh ampere-dev "cd ~ && [ -d emsdk ] || git clone https://github.com/emscripten-core/emsdk.git"
ssh ampere-dev "cd ~/emsdk && ./emsdk install latest && ./emsdk activate latest"
```

- [ ] **Step 2: Confirm `emcc` works from a fresh, non-interactive shell**

This matters because build scripts invoked over SSH don't source interactive shell rc files by default — `emsdk_env.sh`has to be sourced explicitly in every script that calls`emcc`, not just once by hand.

Run:

```bash
ssh ampere-dev "bash -lc 'source ~/emsdk/emsdk_env.sh && emcc --version'"
```

Expected: prints an `emcc (Emscripten gcc/clang-like replacement)` version line, no error.

- [ ] **Step 3: Confirm it doesn't conflict with the existing Rust/wasm-bindgen toolchain**

Run:

```bash
ssh ampere-dev "bash -lc 'source ~/emsdk/emsdk_env.sh && which emcc && which cargo && which wasm-bindgen'"
```

Expected: all three resolve to paths, no `PATH`collision (emsdk's shims live under`~/emsdk`, cargo's under `~/.cargo/bin`— different directories, both fine on`PATH` simultaneously).

No commit for this task — environment-only, nothing in the repo changes.

---

## Task 3: The WASM platform backend (`doomgeneric_wasm.c`)

**Files:**

- Create: `patches/doom-wasm/doomgeneric_wasm.c`

**Interfaces:**

- Consumes: `doomgeneric.h`'s `DG_*`declarations and`DG_ScreenBuffer`/`DOOMGENERIC_RESX`/`DOOMGENERIC_RESY`(all declared there, defined in`doomgeneric.c`, verified via direct source read — see `docs/doom-wasm-design.md`).
- Produces: three `EMSCRIPTEN_KEEPALIVE`C functions later tasks (and Plan B) call from JS:`wasm_get_screen_buffer() -> uint32_t*`(as a numeric pointer, per Emscripten's calling convention),`wasm_get_screen_width() -> unsigned`, `wasm_get_screen_height() -> unsigned`. Plan B additionally calls `wasm_push_key(int pressed, unsigned char key)`, implemented here now since it's part of the same file, even though nothing calls it until Plan B wires up input — `DG_ReadInput`/`DG_GetKey` need the queue to exist to compile against, regardless of whether anything feeds it yet.

- [ ] **Step 1: Write the file**

Create `patches/doom-wasm/doomgeneric_wasm.c`:

```c
/*

* WASM platform backend for doom-ascii, implementing the DG_* interface
* doomgeneric.h declares. Nothing about the engine, menu, or renderer
* changes — this is the same seam doomgeneric_ascii.c implements for a
* terminal, implemented here for a browser instead.

 *

* DG_ScreenBuffer is a pixel buffer, not a character grid: DOOMGENERIC_RESX
* * DOOMGENERIC_RESY uint32_t values, one per pixel, already resolved
* through DOOM's palette. Converting that to glyphs happens in JS
* (site/doom-play.js), not here — this file's only job is to make the
* buffer and its dimensions reachable from JS.

 */

# include <emscripten.h>

# include "doomgeneric.h"

# define KEY_QUEUE_LEN 64

typedef struct
{
    unsigned char key;
    int pressed;
} wasm_key_event_t;

static wasm_key_event_t key_queue[KEY_QUEUE_LEN];
static int key_queue_head = 0;
static int key_queue_tail = 0;

/* Called from JS (Plan B) on every keydown/keyup and touch-control press.

* Silently drops the event if the queue is full rather than blocking —
* DOOM polls this once per tic (roughly 35Hz), so a full 64-slot queue
* means input is arriving faster than the game can consume it, and

 *dropping the newest event is the right failure mode for a game loop.*/
EMSCRIPTEN_KEEPALIVE
void wasm_push_key(int pressed, unsigned char key)
{
    int next = (key_queue_tail + 1) % KEY_QUEUE_LEN;
    if (next == key_queue_head)
    {
        return;
    }
    key_queue[key_queue_tail].pressed = pressed;
    key_queue[key_queue_tail].key = key;
    key_queue_tail = next;
}

void DG_Init(void)
{
    /* DG_ScreenBuffer is already allocated by dg_Create() (doomgeneric.c)
     *before DG_Init runs. No terminal, no termios, nothing to set up.*/
}

void DG_DrawFrame(void)
{
    /* JS reads DG_ScreenBuffer directly via wasm_get_screen_buffer(); there

    * is nothing to push from the C side. This function exists only
    * because the DG_*interface requires it to be defined.*/

}

void DG_SleepMs(uint32_t ms)
{
    /* A real sleep would block the browser's main thread under

    * emscripten_set_main_loop, freezing the tab. TryRunTics's
    * network-sync fallback path can reach this; as a no-op it just

     *spins faster, which is harmless for a local single-player game.*/
    (void)ms;
}

uint32_t DG_GetTicksMs(void)
{
    return (uint32_t)emscripten_get_now();
}

int DG_GetKey(int *pressed, unsigned char *key)
{
    if (key_queue_head == key_queue_tail)
    {
        return 0;
    }
    *pressed = key_queue[key_queue_head].pressed;
    *key = key_queue[key_queue_head].key;
    key_queue_head = (key_queue_head + 1) % KEY_QUEUE_LEN;
    return 1;
}

void DG_ReadInput(void)
{
    /* The terminal backend polls raw stdin bytes here. There is nothing to

    * poll: wasm_push_key() already fills key_queue directly from JS event

     *listeners as they fire.*/
}

void DG_SetWindowTitle(const char *title)
{
    EM_ASM({ document.title = UTF8ToString($0); }, title);
}

EMSCRIPTEN_KEEPALIVE
uint32_t *wasm_get_screen_buffer(void)
{
    return DG_ScreenBuffer;
}

EMSCRIPTEN_KEEPALIVE
unsigned wasm_get_screen_width(void)
{
    return DOOMGENERIC_RESX;
}

EMSCRIPTEN_KEEPALIVE
unsigned wasm_get_screen_height(void)
{
    return DOOMGENERIC_RESY;
}
```

- [ ] **Step 2: Commit**

```bash
git add patches/doom-wasm/doomgeneric_wasm.c
git commit -m "feat: add the WASM platform backend for doom-ascii"
```

(Nothing to compile yet — `emcc`isn't wired up until Task 5. This file is verified by successfully compiling as part of that task, not standalone; a lone`DG_*` implementation has no meaningful test in isolation from the engine code that calls it.)

---

## Task 4: The main-loop patch (`d_main.c`)

**Files:**

- Create: `patches/doom-wasm/main-loop.patch`

**Interfaces:**

- Produces: a patch file later tasks' build script applies with `patch -p1`(or`git apply`) against the pinned clone's `src/d_main.c`.

- [ ] **Step 1: Reproduce the exact original `D_DoomLoop` and write the patched version**

The verified original, `src/d_main.c` (function starts around line 408):

```c
void D_DoomLoop (void)
{
    if (bfgedition &&
        (demorecording || (gameaction == ga_playdemo) || netgame))
    {
        printf(" WARNING: You are playing using one of the Doom Classic\n"
               " IWAD files shipped with the Doom 3: BFG Edition. These are\n"
               " known to be incompatible with the regular IWAD files and\n"
               " may cause demos and network games to get out of sync.\n");
    }

    if (demorecording)
        G_BeginRecording ();

    main_loop_started = true;

    TryRunTics();

    I_SetWindowTitle(gamedescription);
    I_GraphicsCheckCommandLine();
    I_SetGrabMouseCallback(D_GrabMouseCallback);
    I_InitGraphics();
    I_EnableLoadingDisk();

    V_RestoreBuffer();
    R_ExecuteSetViewSize();

    D_StartGameLoop();

    if (testcontrols)
    {
        wipegamestate = gamestate;
    }

    while (1)
    {
        // frame syncronous IO operations
        I_StartFrame ();

        TryRunTics (); // will run at least one tic

        S_UpdateSounds (players[consoleplayer].mo);// move positional sounds

        // Update display, next frame, with current state.
        if (screenvisible)
        {
            D_Display ();
        }
    }
}
```

The patched version — the `while (1)` body becomes a step function; everything before it is untouched; guarded so the native build compiles identically to before:

```c

# ifdef __EMSCRIPTEN__

# include <emscripten.h>

static void D_DoomLoopStep(void)
{
    // frame syncronous IO operations
    I_StartFrame ();

    TryRunTics (); // will run at least one tic

    S_UpdateSounds (players[consoleplayer].mo);// move positional sounds

    // Update display, next frame, with current state.
    if (screenvisible)
    {
        D_Display ();
    }
}

# endif

void D_DoomLoop (void)
{
    if (bfgedition &&
        (demorecording || (gameaction == ga_playdemo) || netgame))
    {
        printf(" WARNING: You are playing using one of the Doom Classic\n"
               " IWAD files shipped with the Doom 3: BFG Edition. These are\n"
               " known to be incompatible with the regular IWAD files and\n"
               " may cause demos and network games to get out of sync.\n");
    }

    if (demorecording)
        G_BeginRecording ();

    main_loop_started = true;

    TryRunTics();

    I_SetWindowTitle(gamedescription);
    I_GraphicsCheckCommandLine();
    I_SetGrabMouseCallback(D_GrabMouseCallback);
    I_InitGraphics();
    I_EnableLoadingDisk();

    V_RestoreBuffer();
    R_ExecuteSetViewSize();

    D_StartGameLoop();

    if (testcontrols)
    {
        wipegamestate = gamestate;
    }

# ifdef __EMSCRIPTEN__

    emscripten_set_main_loop(D_DoomLoopStep, 0, 1);

# else

    while (1)
    {
        // frame syncronous IO operations
        I_StartFrame ();

        TryRunTics (); // will run at least one tic

        S_UpdateSounds (players[consoleplayer].mo);// move positional sounds

        // Update display, next frame, with current state.
        if (screenvisible)
        {
            D_Display ();
        }
    }

# endif

}
```

`emscripten_set_main_loop(D_DoomLoopStep, 0, 1)`: `0` fps means sync to the browser's display refresh rate (`requestAnimationFrame`) rather than a fixed timer; `1` (`simulate_infinite_loop`) makes the call throw a JS exception to unwind the C stack instead of returning — confirmed against Emscripten's own docs — so `D_DoomMain()`and`main()`keep behaving exactly as if`D_DoomLoop`never returns, identical to the native build.`i_main.c` needs no change at all.

- [ ] **Step 2: Generate the actual patch file**

On `ampere-dev`, against a fresh pinned checkout:

```bash
ssh ampere-dev "rm -rf /tmp/doom-wasm-patch && git clone https://github.com/wojciech-graj/doom-ascii.git /tmp/doom-wasm-patch && cd /tmp/doom-wasm-patch && git checkout b5188d7c9c4da6c81264a7803e8725ac3df2cfea"
```

Edit `/tmp/doom-wasm-patch/src/d_main.c`on the VM to match the patched version above (replace the`D_DoomLoop` function body exactly as shown), then:

```bash
ssh ampere-dev "cd /tmp/doom-wasm-patch && git diff src/d_main.c" > "/tmp/main-loop.patch"
```

Copy that output into this repo:

```bash
cp /tmp/main-loop.patch "patches/doom-wasm/main-loop.patch"
ssh ampere-dev "rm -rf /tmp/doom-wasm-patch"
rm /tmp/main-loop.patch
```

- [ ] **Step 3: Verify the patch applies cleanly to a fresh pinned clone**

```bash
ssh ampere-dev "rm -rf /tmp/doom-wasm-apply-check && git clone https://github.com/wojciech-graj/doom-ascii.git /tmp/doom-wasm-apply-check && cd /tmp/doom-wasm-apply-check && git checkout b5188d7c9c4da6c81264a7803e8725ac3df2cfea"
scp "patches/doom-wasm/main-loop.patch" "ampere-dev:/tmp/doom-wasm-apply-check/"
ssh ampere-dev "cd /tmp/doom-wasm-apply-check && git apply main-loop.patch && grep -c '__EMSCRIPTEN__' src/d_main.c"
```

Expected: `git apply`exits 0 with no output (silent success), and the`grep -c`prints`2`(the`#ifdef`and`#endif` around the new step function, at minimum — could be more depending on exact patch shape, but must be nonzero).

```bash
ssh ampere-dev "rm -rf /tmp/doom-wasm-apply-check"
```

- [ ] **Step 4: Commit**

```bash
git add patches/doom-wasm/main-loop.patch
git commit -m "feat: add the emscripten_set_main_loop patch for D_DoomLoop"
```

---

## Task 5: The build script and first compile

**Files:**

- Create: `scripts/build-doom-wasm.sh`
- Create: `wad/freedoom1.wad`reference (already exists per`scripts/record-doom.py`'s use of it — confirm, don't recreate)

**Interfaces:**

- Consumes: `patches/doom-wasm/doomgeneric_wasm.c`, `patches/doom-wasm/main-loop.patch`(Tasks 3-4),`emcc`on`PATH`after sourcing`emsdk_env.sh` (Task 2).
- Produces: `site/doom-wasm/doom.js`and`site/doom-wasm/doom.wasm`(plus a`.data`file from`--preload-file`) — an ES module default-exporting an async factory function (Emscripten `MODULARIZE`+`EXPORT_ES6`output), matching the shape`site/engine.js`already expects from`site/pkg/aa_wasm.js`.

- [ ] **Step 1: Confirm the WAD this repo already uses**

```bash
ls -la "wad/freedoom1.wad"
```

Expected: the file exists (it's what `scripts/record-doom.py`already builds the attract-mode capture against). If missing,`scripts/setup.sh` or a sibling script already documents how to fetch it — don't invent a new source for it here.

- [ ] **Step 2: Write the build script**

Create `scripts/build-doom-wasm.sh`:

```bash

# !/usr/bin/env bash

# Builds doom-ascii to WebAssembly: pinned upstream clone, this repo's

# patches applied, compiled with Emscripten. Run on ampere-dev — never

# locally, per this repo's standing VM-only build rule

set -euo pipefail

ROOT="$(cd "$(dirname "$0")"/..; pwd)"
PATCH_DIR="$ROOT/patches/doom-wasm"
WAD="$ROOT/wad/freedoom1.wad"
OUT_DIR="$ROOT/site/doom-wasm"

PINNED_COMMIT="b5188d7c9c4da6c81264a7803e8725ac3df2cfea"

if [ ! -f "$WAD" ]; then
    echo "missing $WAD" >&2
    exit 1
fi

if ! command -v emcc >/dev/null 2>&1; then
    # emsdk_env.sh isn't sourced by non-interactive shells by default.
    if [ -f "$HOME/emsdk/emsdk_env.sh" ]; then
        # shellcheck source=/dev/null
        source "$HOME/emsdk/emsdk_env.sh"
    fi
fi
if ! command -v emcc >/dev/null 2>&1; then
    echo "emcc not found; run: cd ~/emsdk && ./emsdk install latest && ./emsdk activate latest" >&2
    exit 1
fi

BUILD_DIR="$(mktemp -d /tmp/doom-wasm-build-XXXXXX)"
CLEANUP() { rm -rf "$BUILD_DIR" || true; }
trap CLEANUP EXIT

cd "$BUILD_DIR"
git clone https://github.com/wojciech-graj/doom-ascii.git
cd doom-ascii
git checkout "$PINNED_COMMIT"

cp "$PATCH_DIR/doomgeneric_wasm.c" src/doomgeneric_wasm.c
git apply "$PATCH_DIR/main-loop.patch"

# Same 72-file list the native Makefile's SRC variable builds, with

# doomgeneric_ascii.c swapped for the new WASM backend

SRC_FILES="i_main.c dummy.c am_map.c doomdef.c doomstat.c dstrings.c d_event.c d_items.c d_iwad.c \
    d_loop.c d_main.c d_mode.c d_net.c f_finale.c f_wipe.c g_game.c hu_lib.c hu_stuff.c info.c \
    i_cdmus.c i_endoom.c i_joystick.c i_scale.c i_sound.c i_system.c i_timer.c memio.c m_argv.c \
    m_bbox.c m_cheat.c m_config.c m_controls.c m_fixed.c m_menu.c m_misc.c m_random.c \
    p_ceilng.c p_doors.c p_enemy.c p_floor.c p_inter.c p_lights.c p_map.c p_maputl.c p_mobj.c \
    p_plats.c p_pspr.c p_saveg.c p_setup.c p_sight.c p_spec.c p_switch.c p_telept.c p_tick.c \
    p_user.c r_bsp.c r_data.c r_draw.c r_main.c r_plane.c r_segs.c r_sky.c r_things.c sha1.c \
    sounds.c statdump.c st_lib.c st_stuff.c s_sound.c tables.c v_video.c wi_stuff.c \
    w_checksum.c w_file.c w_main.c w_wad.c z_zone.c w_file_stdc.c i_input.c i_video.c \
    doomgeneric.c doomgeneric_wasm.c"

mkdir -p "$OUT_DIR"

cd src

# shellcheck disable=SC2086

emcc $SRC_FILES \
    -O2 -DNORMALUNIX -DLINUX -std=c99 \
    -s WASM=1 \
    -s MODULARIZE=1 \
    -s EXPORT_ES6=1 \
    -s EXPORT_NAME=DoomModule \
    -s 'EXPORTED_FUNCTIONS=["_main","_wasm_push_key","_wasm_get_screen_buffer","_wasm_get_screen_width","_wasm_get_screen_height"]' \
    -s 'EXPORTED_RUNTIME_METHODS=["cwrap"]' \
    -s ALLOW_MEMORY_GROWTH=1 \
    --preload-file "$WAD@/freedoom1.wad" \
    -o "$OUT_DIR/doom.js"

echo "built $OUT_DIR/doom.js"
ls -la "$OUT_DIR"
```

- [ ] **Step 3: Make it executable and add the output directory to `.gitignore`**

```bash
chmod +x scripts/build-doom-wasm.sh
```

Add to `.gitignore`(same pattern as the existing`site/pkg/` entry):

```text
site/doom-wasm/
```

- [ ] **Step 4: Run it on ampere-dev and fix whatever the compiler says**

This is the first real compile of a 72-file codebase against a target it has never been built for. Expect this not to succeed on the first attempt — that's normal for a first-time port, not a sign the plan is wrong. Common, well-documented classes of failure for this kind of port and their fixes:

- **Missing/incompatible POSIX headers** (`termios.h`, direct `ioctl`calls, etc., likely in`i_system.c`or leftover terminal-detection code Emscripten's libc doesn't provide a full implementation of): guard the offending include/call with`#ifndef __EMSCRIPTEN__`, matching the same pattern `main-loop.patch` already established, rather than trying to make Emscripten's libc satisfy a call it was never going to need for a canvas-driven build.
- **Duplicate symbol / multiple definition errors mentioning `doomgeneric_ascii`**: means the file list above wasn't fully substituted somewhere, or a stray build cache from a previous attempt persisted — the script's `mktemp -d`build directory should prevent the latter, so check the`SRC_FILES` substitution first.
- **Undefined reference to a `DG_*`or`I_*`function at link time**: means`doomgeneric_wasm.c`is missing an implementation the engine actually calls that wasn't in the six functions confirmed by research — check the exact undefined symbol name against`doomgeneric.h`'s full declaration list first, not just the six covered here.

Run:

```bash
scp scripts/build-doom-wasm.sh ampere-dev:~/ascii-arcade-build-doom-wasm.sh
scp -r patches ampere-dev:~/ascii-arcade-patches
scp wad/freedoom1.wad ampere-dev:~/ascii-arcade-freedoom1.wad
```

Adjust paths and run interactively on the VM until it succeeds — this step is iterative, not scripted end-to-end, since the exact fix depends on whatever the compiler actually says:

```bash
ssh ampere-dev

# on the VM

source ~/emsdk/emsdk_env.sh
mkdir -p ~/ascii-arcade-wasm-test
cp ~/ascii-arcade-build-doom-wasm.sh ~/ascii-arcade-wasm-test/build.sh

# edit build.sh's PATCH_DIR/WAD/OUT_DIR to point at ~/ascii-arcade-patches

# ~/ascii-arcade-freedoom1.wad, ~/ascii-arcade-wasm-test/out respectively

# then

bash ~/ascii-arcade-wasm-test/build.sh
```

Once it succeeds standalone on the VM, back-port whatever source fix was needed into `patches/doom-wasm/doomgeneric_wasm.c`and/or a new small addition to`patches/doom-wasm/main-loop.patch`(or a second patch file if the fix is unrelated to the main-loop change — name it for what it fixes, e.g.`patches/doom-wasm/posix-guards.patch`), and re-verify `scripts/build-doom-wasm.sh`itself (the real one in this repo, via`verify-on-vm`, not the manually-copied VM scratch copy) produces the same successful result end to end:

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "bash scripts/build-doom-wasm.sh"
```

Expected: `built site/doom-wasm/doom.js`printed, and`site/doom-wasm/`contains`doom.js`, `doom.wasm`, and `doom.data`.

- [ ] **Step 5: Clean up VM scratch files**

```bash
ssh ampere-dev "rm -rf ~/ascii-arcade-wasm-test ~/ascii-arcade-build-doom-wasm.sh ~/ascii-arcade-patches ~/ascii-arcade-freedoom1.wad"
```

- [ ] **Step 6: Commit**

```bash
git add scripts/build-doom-wasm.sh .gitignore

# If Step 4 required source fixes

git add patches/doom-wasm/
git commit -m "feat: add the doom-wasm build script"
```

(If Step 4's compiler errors required changes to `doomgeneric_wasm.c` or a new patch file, note in the commit body what class of fix was needed and why — future maintainers bumping the pinned commit will hit the same class of issue and need to know it was expected, not a regression.)

---

## Task 6: Prove it actually runs — a Node smoke test

**Files:**

- Create: `scripts/smoke-test-doom-wasm.mjs`

**Interfaces:**

- Consumes: `site/doom-wasm/doom.js`'s default export (an async factory function, per `MODULARIZE`+`EXPORT_ES6`), `wasm_get_screen_buffer`/`wasm_get_screen_width`/`wasm_get_screen_height` (Task 3).
- Produces: confirmation, independent of any browser/canvas rendering concerns, that the module loads, the WAD resolves through Emscripten's virtual filesystem, and `DG_ScreenBuffer`contains real, non-uniform pixel data after the engine has ticked a few frames — the single most important unverified claim from`docs/doom-wasm-design.md`.

This step matters on its own, separate from Task 7's canvas work, because it isolates two very different failure classes: if this fails, the problem is in the WASM module itself (WAD loading, the engine, the patch) and has nothing to do with rendering; if this passes but Task 7 fails, the problem is JS-side pixel-to-canvas conversion.

- [ ] **Step 1: Write the smoke test**

Create `scripts/smoke-test-doom-wasm.mjs`:

```javascript
// Loads the compiled doom-wasm module outside a browser (Node) and checks
// that DG_ScreenBuffer contains real pixel data after a few ticks — proof
// the engine is actually running, not just linking. Run after
// scripts/build-doom-wasm.sh, before wiring anything up to the page.
import DoomModule from "../site/doom-wasm/doom.js";

const mod = await DoomModule({
  arguments: ["-iwad", "/freedoom1.wad"],
  print: (text) => console.log("[doom stdout]", text),
  printErr: (text) => console.error("[doom stderr]", text),
});

const getBuffer = mod.cwrap("wasm_get_screen_buffer", "number", []);
const getWidth = mod.cwrap("wasm_get_screen_width", "number", []);
const getHeight = mod.cwrap("wasm_get_screen_height", "number", []);

// D_DoomLoop's emscripten_set_main_loop runs on rAF in a browser; Node has
// no rAF, so Emscripten falls back to its own timer-driven equivalent —
// give it real wall-clock time to tick a few frames before reading.
await new Promise((resolve) => setTimeout(resolve, 2000));

const width = getWidth();
const height = getHeight();
const ptr = getBuffer();

if (width <= 0 || height <= 0) {
  console.error(`FAIL: invalid dimensions ${width}x${height}`);
  process.exit(1);
}
if (ptr === 0) {
  console.error("FAIL: wasm_get_screen_buffer returned a null pointer");
  process.exit(1);
}

const pixels = new Uint32Array(mod.HEAPU8.buffer, ptr, width * height);
const distinctValues = new Set(pixels).size;

console.log(`buffer: ${width}x${height}, ${pixels.length} pixels, ${distinctValues} distinct colour values`);

if (distinctValues <= 1) {
  console.error("FAIL: buffer is uniform — engine likely isn't rendering (still on a blank/black screen, or not ticking at all)");
  process.exit(1);
}

console.log("PASS: doom-wasm module loads, ticks, and produces varied pixel data");
process.exit(0);
```

- [ ] **Step 2: Run it on ampere-dev**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "node scripts/smoke-test-doom-wasm.mjs"
```

Expected: `PASS: doom-wasm module loads, ticks, and produces varied pixel data`. If it fails with a WAD/file-not-found-style error from inside the WASM module's own stderr output, that's the `--preload-file`/`fopen`-against-`MEMFS`boundary flagged as unverified in the design doc — the fix is almost certainly in how`-iwad`is being resolved relative to`--preload-file`'s mount path (`/freedoom1.wad`here, matching the`$WAD@/freedoom1.wad`mapping in`build-doom-wasm.sh`); confirm the mount path and the `-iwad` argument agree exactly, including the leading slash.

- [ ] **Step 3: Commit**

```bash
git add scripts/smoke-test-doom-wasm.mjs
git commit -m "test: add a Node smoke test proving doom-wasm actually ticks"
```

---

## Task 7: Paint it — `site/doom-play.js` skeleton

**Files:**

- Create: `site/doom-play.js`
- Modify: `site/renderer.js`— no code change, just confirming its exported`Renderer` class is reused as-is (it already accepts any canvas in its constructor; nothing about it assumes there's only one instance).

**Interfaces:**

- Consumes: `Renderer`from`site/renderer.js` (`new Renderer(canvas)`, `.resize(cssW, cssH, fontPx)`, `.paint(glyphs, colors, themeColor)`— all as it exists today, unmodified),`DoomModule`default export from`site/doom-wasm/doom.js`(Task 5),`wasm_get_screen_buffer`/`wasm_get_screen_width`/`wasm_get_screen_height` (Task 3).
- Produces: `loadDoomSkeleton(canvas)` — an exported async function this task's own test drives directly. Plan B replaces/wraps this with the full session-lifecycle API (`startDoomSession`/`stopDoomSession` or similar); this task's job is only proving pixels reach the canvas, not designing that final API.

- [ ] **Step 1: Write the skeleton**

Create `site/doom-play.js`:

```javascript
/*

* Loads the compiled doom-wasm module and paints its pixel buffer through
* the site's existing glyph-atlas Renderer. This is Plan A's walking
* skeleton: it proves the pixel-to-canvas pipeline works. Plan B adds
* input, session start/stop, touch controls, and scroll-lock on top of
* this same file — nothing here is throwaway, but nothing here is the
* final "Play it" integration either.

 */
import { Renderer } from "./renderer.js";

// DG_ScreenBuffer's pixel format, confirmed by direct source read: each
// uint32_t is BGRA byte order (byte0=B, byte1=G, byte2=R, byte3=A) on a
// little-endian target, with alpha always written as 0 (unused). Reading
// that same uint32_t as a little-endian int gives 0x00RRGGBB — R at bits
// 16-23, G at bits 8-15, B at bits 0-7 — which is not what it looks like
// from the byte order alone; verified against i_video.c's I_FinishUpdate,
// which writes R at bit-offset 16, G at 8, B at 0 into this same buffer.
function unpackPixel(word) {
  return {
    r: (word >>> 16) & 0xff,
    g: (word >>> 8) & 0xff,
    b: word & 0xff,
  };
}

// Same block-mode look the recorded attract loop already uses: a dark
// pixel becomes a bare space (Renderer.paint() already skips spaces), a
// lit pixel becomes a solid block glyph in that pixel's own colour. No
// luminance ramp — DOOM's buffer already carries full colour per cell.
// Not a value pulled from doomgeneric_ascii.c — its own glyph-encoding
// logic was deliberately not read during research (it's terminal-specific
// and wasn't going to be reused either way). This is a starting point to
// tune by eye once Task 7's test is passing and the output is visible.
const DARK_THRESHOLD = 24;

function pixelsToGlyphs(pixels, count) {
  const glyphs = new Array(count);
  const colors = new Uint32Array(count);
  for (let i = 0; i < count; i++) {
    const { r, g, b } = unpackPixel(pixels[i]);
    if (r < DARK_THRESHOLD && g < DARK_THRESHOLD && b < DARK_THRESHOLD) {
      glyphs[i] = " ";
      colors[i] = 0;
    } else {
      glyphs[i] = "█";
      colors[i] = 0xff000000 | (r << 16) | (g << 8) | b;
    }
  }
  return { glyphs, colors };
}

/**

* Loads doom-wasm, paints its output to `canvas` on every animation frame,
* and returns a handle with a `stop()` method to end the paint loop.
* Plan A's own proof-of-life — not yet wired to any button.

 */
export async function loadDoomSkeleton(canvas) {
  const mod = await (await import("./doom-wasm/doom.js")).default({
    arguments: ["-iwad", "/freedoom1.wad"],
  });

  const getBuffer = mod.cwrap("wasm_get_screen_buffer", "number", []);
  const getWidth = mod.cwrap("wasm_get_screen_width", "number", []);
  const getHeight = mod.cwrap("wasm_get_screen_height", "number", []);

  const width = getWidth();
  const height = getHeight();
  const renderer = new Renderer(canvas);
  const rect = canvas.getBoundingClientRect();
  renderer.resize(rect.width, rect.height, Math.max(1, rect.height / height));
  // Force the grid to exactly DOOM's own resolution rather than whatever
  // fell out of the font-size measurement above — this is DOOM's pixel
  // buffer, not prose text, and every pixel needs its own cell.
  renderer.cols = width;
  renderer.rows = height;

  let running = true;
  const themeColor = { r: 200, g: 200, b: 200 };

  const draw = () => {
    if (!running) return;
    const ptr = getBuffer();
    if (ptr !== 0) {
      const pixels = new Uint32Array(mod.HEAPU8.buffer, ptr, width * height);
      const { glyphs, colors } = pixelsToGlyphs(pixels, width * height);
      renderer.paint(glyphs, colors, themeColor);
    }
    requestAnimationFrame(draw);
  };
  requestAnimationFrame(draw);

  return {
    stop() {
      running = false;
    },
  };
}
```

- [ ] **Step 2: Write a Playwright test proving pixels actually land on the canvas**

Create `e2e/tests/site/doom-play-skeleton.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";

test("doom-wasm skeleton paints non-empty pixels to its own canvas", async ({ page }) => {
  await page.goto("/site/");

  const painted = await page.evaluate(async () => {
    const { loadDoomSkeleton } = await import("/site/doom-play.js");
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 400;
    canvas.style.width = "640px";
    canvas.style.height = "400px";
    document.body.appendChild(canvas);

    const handle = await loadDoomSkeleton(canvas);
    // D_DoomLoop syncs to requestAnimationFrame; give the engine real
    // frames to tick and reach a non-blank screen (boot logo / menu).
    await new Promise((resolve) => setTimeout(resolve, 3000));

    const ctx = canvas.getContext("2d")!;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    let lit = 0;
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] > 20 || data[i + 1] > 20 || data[i + 2] > 20) lit++;
    }

    handle.stop();
    canvas.remove();
    return lit;
  });

  expect(painted).toBeGreaterThan(0);
});
```

- [ ] **Step 3: Run it**

```bash
/Users/yinkavaughan/bin/verify-on-vm "/Users/yinkavaughan/My Drive (yvaughan@wesleyan.edu)/CS/projects/personal/ascii-arcade" "cd e2e && npx playwright test tests/site/doom-play-skeleton.spec.ts"
```

Expected: PASS. This test intentionally does not check axe/a11y or the `budget.spec.ts` weight limit — this canvas isn't wired to the real page yet, it's a standalone proof this task creates and destroys within the test itself.

If it fails: check first whether Task 6's Node smoke test still passes (isolates whether the regression is in the WASM module or in this task's JS). If Task 6 still passes but this fails, the bug is almost certainly in `unpackPixel`'s byte-order math or in `renderer.cols`/`renderer.rows`being set after`Renderer.resize()`already computed cell dimensions from a different, wrong grid size — verify by logging`getWidth()`/`getHeight()`against what`renderer.resize()`'s own return value reports.

- [ ] **Step 4: Commit**

```bash
git add site/doom-play.js e2e/tests/site/doom-play-skeleton.spec.ts
git commit -m "feat: render doom-wasm's pixel buffer through the canvas renderer"
```

---

## Self-Review Notes

**Spec coverage against `docs/doom-wasm-design.md`:** Plan A covers "Why the platform-backend approach," "The blocking main loop," and the pixel-buffer half of "Rendering" (the JS conversion function and dedicated-canvas parts of "Rendering," all of "Input," "Licensing and GPL compliance," "Failure behavior," "Accessibility," and the WAD-hosting/preload *strategy*— as opposed to the WAD*loading mechanism*, which this plan does verify via `--preload-file` — are explicitly Plan B's, matching the design doc's own two-plan split.

**Known gap carried into Plan B on purpose:** this plan's `pixelsToGlyphs`runs on every frame in JS as a plain loop over`width * height`values (typically small — DOOM's native internal resolution divided by a scaling factor, on the order of thousands of cells, not the site's full-viewport character grid). If profiling during Plan B's session-lifecycle work shows this loop competing for frame budget once the ambient`SceneDriver` and this loop coexist briefly during session start/stop, that's a Plan B concern, not this one — Plan A's own test (Task 7) only asserts pixels land on screen, not any performance bound.

**Type/interface consistency check:** `Renderer.paint(glyphs, colors, themeColor)`'s existing signature (verified against the current `site/renderer.js`) is used unchanged in Task 7. `wasm_get_screen_buffer`/`_width`/`_height`names match exactly between Task 3's C definitions, Task 5's`EXPORTED_FUNCTIONS`list, Task 6's`cwrap`calls, and Task 7's`cwrap` calls — no drift.

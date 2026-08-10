/*
 * Loads the compiled doom-wasm module and paints its pixel buffer through
 * the site's existing glyph-atlas Renderer. This is Plan A's walking
 * skeleton: it proves the pixel-to-canvas pipeline works. Plan B adds
 * input, session start/stop, touch controls, and scroll-lock on top of
 * this same file — nothing here is throwaway, but nothing here is the
 * final "Play it" integration either.
 */
import { Renderer, measureCell } from "./renderer.js";

// doomgeneric's own key constants, verified against the pinned commit's
// src/doomkeys.h directly (not recalled from memory or another source
// port's conventions — vanilla Doom's byte values are not ASCII for the
// non-printable ones).
const KEY_UPARROW = 0xad;
const KEY_DOWNARROW = 0xaf;
const KEY_LEFTARROW = 0xac;
const KEY_RIGHTARROW = 0xae;
const KEY_USE = 0xa2;
const KEY_FIRE = 0xa3;
const KEY_ESCAPE = 27;
const KEY_ENTER = 13;

// Multiple JS codes map to the same doomgeneric constant (WASD alongside
// arrows): doomgeneric only understands its own fixed key space, not which
// physical key produced an event.
const KEY_MAP = {
  ArrowUp: KEY_UPARROW,
  ArrowDown: KEY_DOWNARROW,
  ArrowLeft: KEY_LEFTARROW,
  ArrowRight: KEY_RIGHTARROW,
  KeyW: KEY_UPARROW,
  KeyS: KEY_DOWNARROW,
  KeyA: KEY_LEFTARROW,
  KeyD: KEY_RIGHTARROW,
  ControlLeft: KEY_FIRE,
  ControlRight: KEY_FIRE,
  Space: KEY_USE,
  Enter: KEY_ENTER,
  Escape: KEY_ESCAPE,
};

/**
 * Largest font size that fits `cols` columns by `rows` rows into `box`
 * without overflowing either axis, using Renderer's own cell-sizing
 * formula (measureCell) so the number this returns and the cell size
 * Renderer.resize() computes from it actually agree.
 *
 * Sizing by height alone (what this used to do) badly under-fills width:
 * monospace cells are much taller than wide, so a grid sized only to fit
 * the box's height ends up far narrower than the box — exactly the "why
 * is the playable box so small" symptom this replaced.
 */
function fitDoomFont(box, cols, rows) {
  const cellAt100 = cellAspectAt100();
  const byWidth = box.width / cols / (cellAt100.w / 100);
  const byHeight = box.height / rows / (cellAt100.h / 100);
  return Math.max(1, Math.min(byWidth, byHeight));
}

// Cached: same probe context, same font, every call — no reason to
// recreate the canvas each time fitDoomFont or gridColsForBox runs.
let cellAt100Cache = null;
function cellAspectAt100() {
  if (!cellAt100Cache) {
    const probe = document.createElement("canvas").getContext("2d");
    probe.font = `100px "IBM Plex Mono", monospace`;
    cellAt100Cache = measureCell(probe, 100);
  }
  return cellAt100Cache;
}

/**
 * How many glyph columns the source buffer's `srcHeight` rows need to make
 * the rendered grid's aspect ratio match `box`'s aspect ratio exactly, so
 * fitDoomFont's width and height constraints bind at the same time instead
 * of one of them leaving the box letterboxed.
 *
 * A fixed "double the source width" rule (matching doom_ascii's own
 * "-chars block" convention) only cancels the glyph cell's own aspect
 * ratio — it reproduces the source buffer's pixel aspect ratio, not
 * whatever aspect ratio `.open__doom` happens to be. Those matched by
 * coincidence for the recorded attract-mode capture (record-doom.py's
 * cols/rows were hand-tuned against this box already); they don't for a
 * canvas grid sized independently at runtime. Solving for cols directly
 * from the box's own measured aspect ratio removes the coincidence.
 *
 * `cellAspect` (height:width of one glyph cell) is a parameter rather than
 * measured internally: canvas text metrics don't scale linearly all the
 * way down to the small font sizes DOOM's box ends up needing (hinting
 * rounds glyph advances to whole device pixels, more aggressively at small
 * sizes), so a ratio measured at a large reference size is only a first
 * estimate. loadDoomSkeleton calls this twice — once with that estimate,
 * once with the real ratio Renderer measures at the actual font size.
 */
function gridColsForBox(box, srcHeight, cellAspect) {
  const boxAspect = box.width / box.height;
  return Math.max(1, Math.round(boxAspect * cellAspect * srcHeight));
}

// DG_ScreenBuffer's pixel format, confirmed by direct source read and by
// the module's own startup log ("red_off: 16, green_off: 8, blue_off: 0"):
// each uint32_t read as a little-endian int is 0x00RRGGBB — R at bits
// 16-23, G at bits 8-15, B at bits 0-7. Alpha is always 0 (unused).
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
//
// Not a value pulled from doomgeneric_ascii.c — its own glyph-encoding
// logic was deliberately not read during research (it's terminal-specific
// and wasn't going to be reused either way). This is a starting point to
// tune by eye once Task 7's test is passing and the output is visible.
const DARK_THRESHOLD = 24;

// Nearest-neighbour horizontal resample from srcWidth source pixels to
// outWidth glyph columns (rows stay one-to-one with source pixels).
// outWidth is gridColsForBox's answer, not a fixed multiple of srcWidth —
// this fills whatever box the canvas actually has (see gridColsForBox)
// rather than reproducing the source buffer's own pixel aspect ratio.
function pixelsToGlyphs(pixels, srcWidth, srcHeight, outWidth) {
  const glyphs = new Array(outWidth * srcHeight);
  const colors = new Uint32Array(outWidth * srcHeight);
  for (let y = 0; y < srcHeight; y++) {
    const rowBase = y * srcWidth;
    const outBase = y * outWidth;
    for (let x = 0; x < outWidth; x++) {
      const srcX = Math.min(srcWidth - 1, Math.floor((x * srcWidth) / outWidth));
      const { r, g, b } = unpackPixel(pixels[rowBase + srcX]);
      const dark = r < DARK_THRESHOLD && g < DARK_THRESHOLD && b < DARK_THRESHOLD;
      glyphs[outBase + x] = dark ? " " : "█";
      colors[outBase + x] = dark ? 0 : 0xff000000 | (r << 16) | (g << 8) | b;
    }
  }
  return { glyphs, colors };
}

// A faithful port of vanilla DOOM's own screen-melt (src/f_wipe.c's
// wipe_initMelt/wipe_doMelt, read directly from the pinned doom-ascii
// commit). That C code is compiled into this build
// (f_wipe.c is in build-doom-wasm.sh's SRC_FILES) and does run for real
// in-game transitions via D_Display(), but it operates on i_video.c's
// 8-bit paletted framebuffer — a completely different representation
// from the RGB pixel buffer DG_ScreenBuffer exposes to JS, so reusing it
// directly here isn't practical. This reimplements the same column-fall
// algorithm at glyph-cell granularity instead of raw pixels, which suits
// this site's glyph-based rendering better than the pixel version would
// anyway. Column seeding, the dy acceleration formula (including its
// well-known cap-at-8 quirk), and the reveal/clear order are copied
// as-is from the original for authenticity, not reinvented.
const MELT_TIC_MS = 1000 / 35; // vanilla DOOM's own tic rate

function seedMeltColumns(cols) {
  const y = new Int32Array(cols);
  y[0] = -Math.floor(Math.random() * 16);
  for (let i = 1; i < cols; i++) {
    const r = Math.floor(Math.random() * 3) - 1;
    y[i] = y[i - 1] + r;
    if (y[i] > 0) y[i] = 0;
    else if (y[i] === -16) y[i] = -15;
  }
  return y;
}

// Mutates `scratch` and `y` in place. Each falling column overwrites only
// the rows it reveals this tick with the *current* live frame — once
// written, a row is never touched again, so a column freezes whatever
// gameplay was live at the instant its falling edge passed that row. That
// asymmetry (revealed rows are frozen, the "start" screen redraws fresh
// every tick) is the original algorithm, not a shortcut: wipe_doMelt does
// the same thing, since the whole point is a live game rendering under a
// wipe of a screen that's actually static.
function meltStep(scratch, endFrame, y, cols, rows) {
  let done = true;
  for (let i = 0; i < cols; i++) {
    if (y[i] < 0) {
      y[i]++;
      done = false;
      continue;
    }
    if (y[i] >= rows) continue;
    done = false;

    let dy = y[i] < 16 ? y[i] + 1 : 8;
    if (y[i] + dy >= rows) dy = rows - y[i];
    for (let j = 0; j < dy; j++) {
      const idx = (y[i] + j) * cols + i;
      scratch.glyphs[idx] = endFrame.glyphs[idx];
      scratch.colors[idx] = endFrame.colors[idx];
    }
    y[i] += dy;

    // Below the falling edge is still the ("start") screen — blank here,
    // since there's no captured frame to melt from (see loadDoomSkeleton).
    for (let row = y[i]; row < rows; row++) {
      const idx = row * cols + i;
      scratch.glyphs[idx] = " ";
      scratch.colors[idx] = 0;
    }
  }
  return done;
}

// readFrame reads doom's *live* buffer, not a fixed "after" image — same
// as real DOOM, where the game keeps simulating and rendering underneath
// the wipe the whole time it plays.
function meltReveal(renderer, cols, rows, readFrame, themeColor) {
  return new Promise((resolve) => {
    const y = seedMeltColumns(cols);
    const scratch = {
      glyphs: new Array(cols * rows).fill(" "),
      colors: new Uint32Array(cols * rows),
    };
    let acc = 0;
    let last = null;

    const step = (now) => {
      // The gap between meltReveal() being called and this callback's
      // first real firing spans the tail of WASM instantiation — anchor
      // the clock here instead of at call time, or that whole gap reads
      // as elapsed animation time and the catch-up loop below burns
      // through most columns in one frame instead of animating them.
      if (last === null) {
        last = now;
        requestAnimationFrame(step);
        return;
      }

      // Caps how many tics one frame can catch up on a stall (a slow
      // frame, a backgrounded tab) — without this, a long gap plays the
      // same way the bug above did: a burst of ticks collapsed into a
      // single paint instead of a visible animation.
      acc = Math.min(acc + (now - last), MELT_TIC_MS * 8);
      last = now;

      let done = false;
      let ticked = false;
      while (acc >= MELT_TIC_MS) {
        const frame = readFrame();
        if (!frame) break; // engine hasn't rendered its first frame yet
        acc -= MELT_TIC_MS;
        ticked = true;
        done = meltStep(scratch, frame, y, cols, rows);
        if (done) break;
      }
      if (ticked) renderer.paint(scratch.glyphs, scratch.colors, themeColor);

      if (done) resolve();
      else requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  });
}

// Shown only during an active session, and only on a touch-capable device
// — a mouse/keyboard visitor never sees this. Feature-detected rather than
// UA-sniffed: matchMedia("(pointer: coarse)") covers touchscreen laptops
// too, not just phones, matching the design's "never mutually exclusive
// with keyboard" intent (a touchscreen laptop with a keyboard gets both).
function touchCapable() {
  return matchMedia("(pointer: coarse)").matches || "ontouchstart" in window;
}

const TOUCH_BUTTONS = [
  { label: "↑", key: KEY_UPARROW, className: "doom-controls__up" },
  { label: "↓", key: KEY_DOWNARROW, className: "doom-controls__down" },
  { label: "←", key: KEY_LEFTARROW, className: "doom-controls__left" },
  { label: "→", key: KEY_RIGHTARROW, className: "doom-controls__right" },
  { label: "Fire", key: KEY_FIRE, className: "doom-controls__fire" },
  { label: "Use", key: KEY_USE, className: "doom-controls__use" },
  { label: "Enter", key: KEY_ENTER, className: "doom-controls__enter" },
  { label: "Esc", key: KEY_ESCAPE, className: "doom-controls__esc" },
];

function buildTouchControls(mount, push) {
  const el = document.createElement("div");
  el.className = "doom-controls";
  el.setAttribute("role", "group");
  el.setAttribute("aria-label", "Touch controls");

  const cleanups = [];
  for (const spec of TOUCH_BUTTONS) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `doom-controls__btn ${spec.className}`;
    btn.textContent = spec.label;

    // touchstart/touchend, not click: click fires after a delay on touch
    // devices and doesn't give a press/release pair at all, which is what
    // a game button needs (holding "fire" must keep firing, not act like
    // a single click). preventDefault on touchstart stops the synthetic
    // mouse events + scroll gesture Safari/Chrome would otherwise generate
    // from the touch.
    const onStart = (e) => {
      e.preventDefault();
      push(1, spec.key);
    };
    const onEnd = (e) => {
      e.preventDefault();
      push(0, spec.key);
    };
    btn.addEventListener("touchstart", onStart, { passive: false });
    btn.addEventListener("touchend", onEnd, { passive: false });
    btn.addEventListener("touchcancel", onEnd, { passive: false });
    cleanups.push(() => {
      btn.removeEventListener("touchstart", onStart);
      btn.removeEventListener("touchend", onEnd);
      btn.removeEventListener("touchcancel", onEnd);
    });

    el.append(btn);
  }

  mount.append(el);

  return {
    el,
    destroy() {
      for (const fn of cleanups) fn();
      el.remove();
    },
  };
}

/**
 * Loads doom-wasm, paints its output to `canvas` on every animation frame,
 * and returns a handle with a `stop()` method to end the paint loop.
 * Plan A's own proof-of-life — not yet wired to any button.
 */
export async function loadDoomSkeleton(canvas, { onSessionEnd } = {}) {
  const mod = await (await import("./doom-wasm/doom.js")).default({
    // doom-ascii's dg_Create() sets DOOMGENERIC_RESX/RESY to 320/scaling
    // (see scripts/record-doom.py's own comment on this same flag). With
    // no -scaling argument that defaults to 80x50, and pixelsToGlyphs
    // paints one glyph per source pixel — at 80x50 stretched to fill the
    // play box, each glyph is a huge solid-color square with no
    // recognizable detail. -scaling 1 is the fork's native resolution
    // (320x200, vanilla DOOM's own internal resolution) — the most detail
    // this pipeline can show; menu text in particular needs every pixel it
    // can get since it's rendered as blocky glyphs, not anti-aliased.
    arguments: ["-iwad", "/freedoom1.wad", "-scaling", "1"],
    // doom.wasm/doom.data sit next to doom.js. Without this, the loader
    // resolves them relative to the page URL, which is normally correct
    // in a browser — set explicitly anyway so this matches exactly what
    // scripts/smoke-test-doom-wasm.mjs verified working in Node.
    locateFile: (path) => new URL(`./doom-wasm/${path}`, import.meta.url).pathname,
  });

  const push = mod.cwrap("wasm_push_key", null, ["number", "number"]);

  const getBuffer = mod.cwrap("wasm_get_screen_buffer", "number", []);
  const getWidth = mod.cwrap("wasm_get_screen_width", "number", []);
  const getHeight = mod.cwrap("wasm_get_screen_height", "number", []);

  const width = getWidth();
  const height = getHeight();
  const renderer = new Renderer(canvas);
  const rect = canvas.getBoundingClientRect();

  // Solved from the box's own aspect ratio (see gridColsForBox), not a
  // fixed multiple of `width` — that's what makes this fill the box on
  // both axes instead of letterboxing on whichever one doesn't happen to
  // match DOOM's native pixel aspect ratio. Two passes: the first uses a
  // cell-aspect estimate measured at a large reference font size, then
  // resize() gives a real measurement at the actual (much smaller) font
  // size actually used — hinting means those don't quite agree, so the
  // second pass corrects gridCols/fontPx against the real one.
  const estimatedCell = cellAspectAt100();
  let gridCols = gridColsForBox(rect, height, estimatedCell.h / estimatedCell.w);
  let fontPx = fitDoomFont(rect, gridCols, height);
  renderer.resize(rect.width, rect.height, fontPx);

  gridCols = gridColsForBox(rect, height, renderer.cell.h / renderer.cell.w);
  fontPx = fitDoomFont(rect, gridCols, height);
  renderer.resize(rect.width, rect.height, fontPx);

  // Force the grid to exactly the resolution gridColsForBox solved for,
  // rather than whatever fell out of gridSize() inside resize() (which
  // measures against the box's raw pixel dimensions, not the cols/rows
  // this buffer actually has) — this is DOOM's pixel buffer, not prose
  // text, and every pixel needs its own cell.
  renderer.cols = gridCols;
  renderer.rows = height;

  const previousOverflow = document.documentElement.style.overflow;
  document.documentElement.style.overflow = "hidden";
  canvas.tabIndex = 0;
  canvas.focus();

  const onKeyDown = (e) => {
    const key = KEY_MAP[e.code];
    if (key === undefined) return;
    e.preventDefault();
    push(1, key);
  };
  const onKeyUp = (e) => {
    const key = KEY_MAP[e.code];
    if (key === undefined) return;
    e.preventDefault();
    push(0, key);
  };
  addEventListener("keydown", onKeyDown);
  addEventListener("keyup", onKeyUp);

  const touchControls = touchCapable() ? buildTouchControls(canvas.parentElement, push) : null;

  let running = true;
  const themeColor = { r: 200, g: 200, b: 200 };

  // Shared by the melt (as its live "end" frame, read once per melt tic)
  // and the ongoing draw loop below — same extraction either way, just
  // called on a different clock.
  const readFrame = () => {
    const ptr = getBuffer();
    if (ptr === 0) return null;
    const pixels = new Uint32Array(mod.HEAPU8.buffer, ptr, width * height);
    return pixelsToGlyphs(pixels, width, height, gridCols);
  };

  const draw = () => {
    if (!running) return;
    const frame = readFrame();
    if (frame) renderer.paint(frame.glyphs, frame.colors, themeColor);
    requestAnimationFrame(draw);
  };

  // The screen-melt reveal (see meltReveal above) plays once, right as
  // the engine's first real frames become available, then the normal
  // continuous draw loop takes over. `running` can go false during the
  // melt itself only via a bug — stop() isn't reachable until this
  // promise resolves and the caller gets its handle back — but the guard
  // costs nothing and keeps that invariant from being silently assumed.
  await meltReveal(renderer, gridCols, height, readFrame, themeColor);
  if (running) requestAnimationFrame(draw);

  return {
    push,
    stop() {
      running = false;
      removeEventListener("keydown", onKeyDown);
      removeEventListener("keyup", onKeyUp);
      touchControls?.destroy();
      document.documentElement.style.overflow = previousOverflow;
    },
  };
}

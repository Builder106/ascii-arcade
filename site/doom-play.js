/*
 * Loads the compiled doom-wasm module and paints its pixel buffer through
 * the site's existing glyph-atlas Renderer. This is Plan A's walking
 * skeleton: it proves the pixel-to-canvas pipeline works. Plan B adds
 * input, session start/stop, touch controls, and scroll-lock on top of
 * this same file — nothing here is throwaway, but nothing here is the
 * final "Play it" integration either.
 */
import { Renderer, measureCell } from "./renderer.js";

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

/**
 * Loads doom-wasm, paints its output to `canvas` on every animation frame,
 * and returns a handle with a `stop()` method to end the paint loop.
 * Plan A's own proof-of-life — not yet wired to any button.
 */
export async function loadDoomSkeleton(canvas) {
  const mod = await (await import("./doom-wasm/doom.js")).default({
    arguments: ["-iwad", "/freedoom1.wad"],
    // doom.wasm/doom.data sit next to doom.js. Without this, the loader
    // resolves them relative to the page URL, which is normally correct
    // in a browser — set explicitly anyway so this matches exactly what
    // scripts/smoke-test-doom-wasm.mjs verified working in Node.
    locateFile: (path) => new URL(`./doom-wasm/${path}`, import.meta.url).pathname,
  });

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

  let running = true;
  const themeColor = { r: 200, g: 200, b: 200 };

  const draw = () => {
    if (!running) return;
    const ptr = getBuffer();
    if (ptr !== 0) {
      const pixels = new Uint32Array(mod.HEAPU8.buffer, ptr, width * height);
      const { glyphs, colors } = pixelsToGlyphs(pixels, width, height, gridCols);
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

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
  const probe = document.createElement("canvas").getContext("2d");
  probe.font = `100px "IBM Plex Mono", monospace`;
  const cellAt100 = measureCell(probe, 100);
  const byWidth = box.width / cols / (cellAt100.w / 100);
  const byHeight = box.height / rows / (cellAt100.h / 100);
  return Math.max(1, Math.min(byWidth, byHeight));
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

// Two glyph cells per source pixel, horizontally — matching doom_ascii's
// own "-chars block" convention (what the recorded attract loop already
// captures). A monospace character cell is roughly twice as tall as it is
// wide, so mapping one square-ish pixel to one cell squashes the image
// horizontally; two cells per pixel is what makes the result look
// correctly proportioned rather than compressed.
function pixelsToGlyphs(pixels, srcWidth, srcHeight) {
  const outWidth = srcWidth * 2;
  const glyphs = new Array(outWidth * srcHeight);
  const colors = new Uint32Array(outWidth * srcHeight);
  for (let y = 0; y < srcHeight; y++) {
    for (let x = 0; x < srcWidth; x++) {
      const { r, g, b } = unpackPixel(pixels[y * srcWidth + x]);
      const dark = r < DARK_THRESHOLD && g < DARK_THRESHOLD && b < DARK_THRESHOLD;
      const glyph = dark ? " " : "█";
      const color = dark ? 0 : 0xff000000 | (r << 16) | (g << 8) | b;
      const out = y * outWidth + x * 2;
      glyphs[out] = glyph;
      colors[out] = color;
      glyphs[out + 1] = glyph;
      colors[out + 1] = color;
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
  const gridCols = width * 2; // two glyph cells per source pixel — see pixelsToGlyphs
  const renderer = new Renderer(canvas);
  const rect = canvas.getBoundingClientRect();
  const fontPx = fitDoomFont(rect, gridCols, height);
  renderer.resize(rect.width, rect.height, fontPx);
  // Force the grid to exactly DOOM's own resolution (doubled) rather than
  // whatever fell out of gridSize() inside resize() (which measures
  // against the box's raw pixel dimensions, not the cols/rows this buffer
  // actually has) — this is DOOM's pixel buffer, not prose text, and every
  // pixel needs its own cell.
  renderer.cols = gridCols;
  renderer.rows = height;

  let running = true;
  const themeColor = { r: 200, g: 200, b: 200 };

  const draw = () => {
    if (!running) return;
    const ptr = getBuffer();
    if (ptr !== 0) {
      const pixels = new Uint32Array(mod.HEAPU8.buffer, ptr, width * height);
      const { glyphs, colors } = pixelsToGlyphs(pixels, width, height);
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

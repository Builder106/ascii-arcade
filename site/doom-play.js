/*
 * Loads the compiled doom-wasm module and paints its pixel buffer through
 * the site's existing glyph-atlas Renderer. This is Plan A's walking
 * skeleton: it proves the pixel-to-canvas pipeline works. Plan B adds
 * input, session start/stop, touch controls, and scroll-lock on top of
 * this same file — nothing here is throwaway, but nothing here is the
 * final "Play it" integration either.
 */
import { Renderer } from "./renderer.js";

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

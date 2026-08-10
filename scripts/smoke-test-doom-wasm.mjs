// Loads the compiled doom-wasm module outside a browser (Node) and checks
// that DG_ScreenBuffer contains real pixel data after a few ticks — proof
// the engine is actually running, not just linking. Run after
// scripts/build-doom-wasm.sh, before wiring anything up to the page.
import DoomModule from "../site/doom-wasm/doom.js";

const mod = await DoomModule({
  arguments: ["-iwad", "/freedoom1.wad"],
  print: (text) => console.log("[doom stdout]", text),
  printErr: (text) => console.error("[doom stderr]", text),
  // doom.wasm/doom.data sit next to doom.js. Without this, Emscripten's
  // generated loader resolves them relative to the current working
  // directory instead — fine in a browser (relative to the page URL), but
  // wrong here since this script isn't run from site/doom-wasm/.
  locateFile: (path) => new URL(`../site/doom-wasm/${path}`, import.meta.url).pathname,
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

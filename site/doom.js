/*
 * DOOM as a frame source. Like the Mac app's fixed-grid path, DOOM is a
 * scaled colour bitmap, not glyphs on the text grid: doom-ascii's block mode
 * draws one solid block per pixel. Two bitmaps feed it. The attract loop plays a recording of
 * DOOM's own demo (scripts/record-doom.py), and "Play DOOM here" loads the
 * real doom-ascii WebAssembly build (scripts/build-doom-wasm.sh) on request.
 */

const KEY_UPARROW = 0xad;
const KEY_DOWNARROW = 0xaf;
const KEY_LEFTARROW = 0xac;
const KEY_RIGHTARROW = 0xae;
const KEY_USE = 0xa2;
const KEY_FIRE = 0xa3;
const KEY_ENTER = 13;

export const KEY_MAP = {
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
};

class BitmapSource {
  constructor(width, height, aspect) {
    this.aspect = aspect;
    this.canvas = document.createElement("canvas");
    this.canvas.width = width;
    this.canvas.height = height;
    this.ctx = this.canvas.getContext("2d");
    this.image = this.ctx.createImageData(width, height);
    this.words = new Uint32Array(this.image.data.buffer);
    this.rect = { x: 0, y: 0, w: width, h: height };
    this.shown = null;
  }

  // Cover on landscape screens. On portrait ones the copy fills the lower
  // half, so DOOM covers the upper part instead, cropped at the sides.
  size(cols, rows, cell) {
    const W = cols * cell.w;
    const H = rows * cell.h;
    if (W >= H) {
      const h = Math.max(W / this.aspect, H);
      const w = h * this.aspect;
      this.rect = { x: (W - w) / 2, y: (H - h) / 2, w, h };
    } else {
      const h = Math.max(W / this.aspect, H * 0.45);
      const w = h * this.aspect;
      this.rect = { x: (W - w) / 2, y: 0, w, h };
    }
  }

  tint() {}

  // Pixels arrive as 0x00RRGGBB; ImageData wants RGBA bytes, which read as
  // 0xAABBGGRR on a little-endian machine.
  frame(t) {
    const px = this.pixelsAt(t);
    if (px !== this.shown) {
      const out = this.words;
      for (let i = 0; i < out.length; i++) {
        const c = px[i];
        out[i] = 0xff000000 | ((c & 0xff) << 16) | (c & 0xff00) | ((c >>> 16) & 0xff);
      }
      this.ctx.putImageData(this.image, 0, 0);
      this.shown = px;
    }
    return { bitmap: this.canvas, rect: this.rect };
  }
}

/** The recorded demo: palette-indexed runs, -1 ends a row, -2 is a dark run. */
export async function loadRecording(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${url}: ${res.status}`);
  const { fps, cols, rows, palette, frames } = await res.json();
  const colors = palette.map((hex) => parseInt(hex.slice(1), 16));
  const decoded = frames.map((runs) => {
    const px = new Uint32Array(cols * rows);
    let row = 0;
    let i = 0;
    for (const [c, n] of runs) {
      if (c === -1) {
        i = ++row * cols;
        continue;
      }
      px.fill(c >= 0 ? colors[c] : 0, i, i + n);
      i += n;
    }
    return px;
  });
  // Rows are text rows, each standing for two of DOOM's 200 pixel lines.
  const source = new BitmapSource(cols, rows, cols / (rows * 2));
  source.pixelsAt = (t) => decoded[Math.floor(t * fps) % decoded.length];
  return source;
}

/** The playable build. Resolves to a frame source with a `press(code, down)` method. */
export async function loadDoom() {
  const { default: create } = await import("./doom-wasm/doom.js");
  const mod = await create({
    arguments: ["-iwad", "/freedoom1.wad", "-scaling", "1"],
    locateFile: (path) => new URL(`./doom-wasm/${path}`, import.meta.url).pathname,
    print: () => {},
    printErr: () => {},
  });
  const push = mod.cwrap("wasm_push_key", null, ["number", "number"]);
  const buffer = mod.cwrap("wasm_get_screen_buffer", "number", []);
  const width = mod.cwrap("wasm_get_screen_width", "number", [])();
  const height = mod.cwrap("wasm_get_screen_height", "number", [])();

  const source = new BitmapSource(width, height, width / height);
  source.pixelsAt = () => new Uint32Array(mod.HEAPU8.buffer, buffer(), width * height);
  source.press = (code, down) => {
    const key = KEY_MAP[code];
    if (key === undefined) return false;
    push(down ? 1 : 0, key);
    return true;
  };
  return source;
}

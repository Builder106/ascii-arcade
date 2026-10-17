/*
 * The power-on test: what an arcade board shows when it's switched on,
 * drawn in the character grid. An alignment grid, colour bars, a memory test
 * sweeping down the screen, then the logo and a status box that types the
 * page's real loading steps and fills a progress bar as each one finishes.
 * The box is an honest loader: a step's result appears only when that step
 * is done.
 */

const GRID_END = 0.3; // seconds from the start of the test
const BARS_END = 0.55;
const RAM_END = 0.95;
const WIPE = 0.3; // the logo reveals top to bottom over this long
const TYPE = 90; // characters per second in the status box
const BLOCK = "█";

// Colour bars, left to right, as a test card has them.
const BARS = [0xffc0c0c0, 0xffc0c000, 0xff00c0c0, 0xff00c000, 0xffc000c0, 0xffc00000, 0xff0000c0];

// A 5 by 5 pixel font, just the letters the logo and the cabinet's signs
// need. Its C stays open three rows tall, where a display face's closes up
// into an O once the sign is small and glowing.
export const FONT = {
  A: ["01110", "10001", "11111", "10001", "10001"],
  S: ["01111", "10000", "01110", "00001", "11110"],
  C: ["01111", "10000", "10000", "10000", "01111"],
  I: ["11111", "00100", "00100", "00100", "11111"],
  R: ["11110", "10001", "11110", "10010", "10001"],
  D: ["11110", "10001", "10001", "10001", "11110"],
  E: ["11111", "10000", "11110", "10000", "11111"],
  N: ["10001", "11001", "10101", "10011", "10001"],
  O: ["01110", "10001", "10001", "10001", "01110"],
  T: ["11111", "00100", "00100", "00100", "00100"],
  " ": ["00000", "00000", "00000", "00000", "00000"],
};

const HEX = "0123456789ABCDEF";
const hash = (n) => {
  n = Math.imul(n ^ (n >>> 15), 0x2c1b3c6d);
  n = Math.imul(n ^ (n >>> 12), 0x297a2d39);
  return (n ^ (n >>> 15)) >>> 0;
};

export class BootLog {
  constructor(t0) {
    this.t0 = t0;
    this.steps = [];
    this.cols = 1;
    this.rows = 1;
  }

  size(cols, rows) {
    this.cols = cols;
    this.rows = rows;
    this.colors = new Uint32Array(cols * rows);
  }

  tint() {}

  /** Adds a step; call the returned function with its result when it finishes. */
  step(label) {
    const s = { label, result: null };
    this.steps.push(s);
    return (result) => {
      s.result = result;
    };
  }

  lines() {
    return this.steps.map((s) => `${s.label} ${".".repeat(Math.max(2, 14 - s.label.length))} `);
  }

  /** Whether every step has a result and the box has finished typing. */
  finished(t) {
    if (this.steps.some((s) => s.result === null)) return false;
    const chars = this.lines().reduce((n, l) => n + l.length, 0);
    return t - this.t0 >= RAM_END + chars / TYPE;
  }

  frame(t) {
    const { cols, rows } = this;
    const cells = new Array(cols * rows).fill(" ");
    const colors = this.colors.fill(0);
    const put = (r, c, ch, color = 0) => {
      if (r < 0 || c < 0 || r >= rows || c >= cols) return;
      cells[r * cols + c] = ch;
      colors[r * cols + c] = color;
    };
    const text = (r, c, s) => {
      for (let i = 0; i < s.length; i++) put(r, c + i, s[i]);
    };
    const k = t - this.t0;

    if (k < GRID_END) {
      // Alignment grid.
      for (let r = 0; r < rows; r++) {
        for (let c = 0; c < cols; c++) {
          const v = c % 10 === 0;
          const h = r % 5 === 0;
          if (v || h) put(r, c, v && h ? "+" : v ? "|" : "-");
        }
      }
    } else if (k < BARS_END) {
      // Colour bars, with a short strip of their mirror along the bottom.
      for (let c = 0; c < cols; c++) {
        const i = Math.floor((c / cols) * BARS.length);
        for (let r = 0; r < rows; r++) {
          const bottom = r > rows * 0.78;
          put(r, c, BLOCK, BARS[bottom ? BARS.length - 1 - i : i]);
        }
      }
    } else if (k < RAM_END) {
      // Memory test: rows of hex fill down the screen behind a sweep line.
      const sweep = Math.floor(((k - BARS_END) / (RAM_END - BARS_END)) * rows);
      for (let r = 0; r < Math.min(sweep, rows); r++) {
        for (let c = 0; c + 2 < cols; c += 3) {
          const n = hash(r * 977 + c * 131 + 7);
          put(r, c, HEX[n & 15]);
          put(r, c + 1, HEX[(n >>> 4) & 15]);
        }
      }
      for (let c = 0; c < cols; c++) put(sweep, c, BLOCK);
    } else {
      this.drawLogo(put, k - RAM_END);
      this.drawBox(put, text, t, k - RAM_END);
    }
    return { glyphs: cells.join(""), colors };
  }

  // The logo in 5 by 5 block letters, two columns per pixel so the pixels
  // come out square, stacked on two lines when the screen is narrow. A
  // dotted shadow sits one cell down and right. It wipes in top to bottom.
  drawLogo(put, k) {
    const { cols, rows } = this;
    const wide = 12 * 6 * 2 <= cols - 4;
    const words = wide ? ["ASCII ARCADE"] : ["ASCII", "ARCADE"];
    const sx = wide || 6 * 6 * 2 <= cols - 4 ? 2 : 1;
    const top = Math.max(2, Math.floor(rows * 0.12));
    const height = words.length * 6;
    const reveal = Math.floor(Math.min(1, k / WIPE) * (height + 1));
    const pixels = [];
    words.forEach((word, w) => {
      const width = word.length * 6 * sx - sx;
      const left = Math.floor((cols - width) / 2);
      [...word].forEach((ch, i) => {
        const glyph = FONT[ch];
        for (let y = 0; y < 5; y++) {
          const row = w * 6 + y;
          if (row >= reveal) continue;
          for (let x = 0; x < 5; x++) {
            if (glyph[y][x] !== "1") continue;
            for (let s = 0; s < sx; s++) pixels.push([top + row, left + (i * 6 + x) * sx + s]);
          }
        }
      });
    });
    // Shadows first, so no pixel's shadow lands on another pixel.
    for (const [r, c] of pixels) put(r + 1, c + 1, ".");
    for (const [r, c] of pixels) put(r, c, BLOCK);
    this.boxTop = top + height + 1;
  }

  // The status box: real loading steps, a progress bar, then PRESS START.
  drawBox(put, text, t, k) {
    const { cols } = this;
    const out = this.lines();
    const inner = 24;
    const left = Math.max(1, Math.floor((cols - inner - 4) / 2));
    const top = this.boxTop ?? 10;
    const height = out.length + 3;
    put(top, left, "┌");
    put(top, left + inner + 3, "┐");
    put(top + height, left, "└");
    put(top + height, left + inner + 3, "┘");
    for (let c = 1; c < inner + 3; c++) {
      put(top, left + c, "─");
      put(top + height, left + c, "─");
    }
    for (let r = 1; r < height; r++) {
      put(top + r, left, "│");
      put(top + r, left + inner + 3, "│");
    }
    let budget = Math.floor(k * TYPE);
    let cursor = null;
    let done = 0; // steps whose line is typed and whose result is in
    out.forEach((line, i) => {
      if (budget < 0) return;
      const typed = line.slice(0, budget);
      budget -= line.length;
      const result = this.steps[i].result;
      const complete = budget >= 0 && result !== null;
      if (complete) done++;
      const shown = complete ? typed + result : typed;
      text(top + 1 + i, left + 2, shown);
      if (!cursor && !complete) cursor = [top + 1 + i, left + 2 + shown.length];
    });
    const bar = 16;
    const fill = Math.round((done / out.length) * bar);
    text(top + out.length + 2, left + 2, `[${BLOCK.repeat(fill)}${"-".repeat(bar - fill)}] ${done}/${out.length}`);
    if (cursor && Math.floor(t * 3) % 2 === 0) put(cursor[0], cursor[1], BLOCK);
    if (!cursor && Math.floor(t * 2.5) % 2 === 0) {
      const label = "PRESS START";
      text(top + height + 2, Math.floor((cols - label.length) / 2), label);
    }
  }
}

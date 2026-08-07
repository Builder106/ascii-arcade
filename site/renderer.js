/*
 * Canvas character-grid painter.
 *
 * Colour is why this is canvas and not a <pre>: Matrix has bright heads with
 * fading trails and Pipes carries per-pipe hue, so per-cell colour in the DOM
 * would mean thousands of spans a frame. Runs of one colour are batched into a
 * single fillText call to keep that affordable.
 */

/** Cell metrics for the current font, measured rather than assumed. */
export function measureCell(ctx, fontPx) {
  const m = ctx.measureText("M");
  return { w: m.width, h: fontPx * 1.2 };
}

/** How many whole cells fit, never zero. */
export function gridSize(pxW, pxH, cell) {
  return {
    cols: Math.max(1, Math.floor(pxW / cell.w)),
    rows: Math.max(1, Math.floor(pxH / cell.h)),
  };
}

export class Renderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d", { alpha: false });
    this.cols = 0;
    this.rows = 0;
    this.cell = { w: 8, h: 16 };
    this.background = "#000";
  }

  /** Device pixel ratio is capped at 2; beyond that costs fill rate for nothing. */
  resize(cssW, cssH, fontPx) {
    const dpr = Math.min(2, globalThis.devicePixelRatio || 1);
    this.canvas.width = Math.floor(cssW * dpr);
    this.canvas.height = Math.floor(cssH * dpr);
    this.canvas.style.width = `${cssW}px`;
    this.canvas.style.height = `${cssH}px`;

    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.ctx.font = `${fontPx}px "IBM Plex Mono", monospace`;
    this.ctx.textBaseline = "top";

    this.cell = measureCell(this.ctx, fontPx);
    const g = gridSize(cssW, cssH, this.cell);
    this.cols = g.cols;
    this.rows = g.rows;
    return g;
  }

  /**
   * `glyphs` is row-major and `cols * rows` long, indexable as a string or an
   * array. A `colors` entry of 0 means paint in `themeColor`, matching the
   * sentinel aa-wasm packs.
   */
  paint(glyphs, colors, themeColor) {
    const { ctx, cols, rows, cell } = this;

    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = this.background;
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.restore();

    const fallback = `rgb(${themeColor.r},${themeColor.g},${themeColor.b})`;

    for (let y = 0; y < rows; y++) {
      let run = "";
      let runStart = 0;
      let runColor = null;

      // One past the end so the final run of a row gets flushed by the same
      // branch that flushes a colour change.
      for (let x = 0; x <= cols; x++) {
        const i = y * cols + x;
        const ch = x < cols ? glyphs[i] : null;
        const color =
          x < cols ? (colors[i] === 0 ? fallback : cssFromPacked(colors[i])) : null;

        if (ch === null || color !== runColor) {
          if (run.trim().length > 0) {
            ctx.fillStyle = runColor;
            ctx.fillText(run, runStart * cell.w, y * cell.h);
          }
          if (ch === null) break;
          run = ch;
          runStart = x;
          runColor = color;
        } else {
          run += ch;
        }
      }
    }
  }
}

function cssFromPacked(packed) {
  const r = (packed >>> 16) & 0xff;
  const g = (packed >>> 8) & 0xff;
  const b = packed & 0xff;
  return `rgb(${r},${g},${b})`;
}

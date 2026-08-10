/*
 * Canvas character-grid painter.
 *
 * Each frame paints up to ~20k cells. The previous implementation batched
 * same-colour runs into `fillText` calls, which works for scenes whose
 * colour is shared across large regions (Donut, Helix) but not for ones
 * whose colour varies almost every cell — Matrix's per-column fade trail
 * degenerates to nearly one `fillText` call per character. Measured at
 * ~1,500 calls a frame, enough to drop frames during scroll, since fillText
 * re-shapes and re-rasterizes glyphs from the vector font on every call.
 *
 * Glyphs are rasterized once into a monochrome atlas instead. Painting a
 * frame blits tinted copies of that atlas into one shared pixel buffer in
 * plain typed-array writes, then hands the canvas a single putImageData —
 * one canvas call a frame regardless of how many distinct colours are on
 * screen.
 */

// The exact glyph set every scene emits (grepped from aa-core's scene
// sources) and the exact set scripts/subset-font.sh bakes into the shipped
// font — the two have to agree, since a glyph outside the font's coverage
// would already have been falling back to a system font under the old
// fillText path. Fixed and closed, so the atlas can be built once.
const GLYPHSET = (() => {
  const chars = [];
  for (let c = 0x20; c <= 0x7e; c++) chars.push(String.fromCharCode(c));
  chars.push("█", "─", "│", "┌", "┐", "└", "┘");
  return chars;
})();

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

    // Off-canvas: measuring text, reading back the background colour, and
    // building the glyph atlas. Never drawn to the page.
    this.probe = document
      .createElement("canvas")
      .getContext("2d", { willReadFrequently: true });

    this.cellPx = { w: 8, h: 16 };
    this.atlas = null;
    this.glyphIndex = new Map();
    this.frame = null;
    this.frame32 = null;
  }

  /** Device pixel ratio is capped at 2; beyond that costs fill rate for nothing. */
  resize(cssW, cssH, fontPx) {
    const dpr = Math.min(2, globalThis.devicePixelRatio || 1);
    this.canvas.width = Math.floor(cssW * dpr);
    this.canvas.height = Math.floor(cssH * dpr);
    this.canvas.style.width = `${cssW}px`;
    this.canvas.style.height = `${cssH}px`;

    // The buffer this paints into is raw device pixels (ImageData ignores
    // any transform), so the canvas's own CTM stays identity from here on.
    this.ctx.setTransform(1, 0, 0, 1, 0, 0);

    this.probe.font = `${fontPx}px "IBM Plex Mono", monospace`;
    this.cell = measureCell(this.probe, fontPx);
    const g = gridSize(cssW, cssH, this.cell);
    this.cols = g.cols;
    this.rows = g.rows;

    this.cellPx = {
      w: Math.max(1, Math.round(this.cell.w * dpr)),
      h: Math.max(1, Math.round(this.cell.h * dpr)),
    };
    this.buildAtlas(fontPx * dpr);

    this.frame = this.ctx.createImageData(this.canvas.width, this.canvas.height);
    this.frame32 = new Uint32Array(this.frame.data.buffer);
    return g;
  }

  /** Rasterizes every glyph the app can emit once, in white, at device-pixel size. */
  buildAtlas(devicePx) {
    const { w: cw, h: ch } = this.cellPx;
    const atlasCanvas = document.createElement("canvas");
    atlasCanvas.width = cw * GLYPHSET.length;
    atlasCanvas.height = ch;
    const actx = atlasCanvas.getContext("2d", { willReadFrequently: true });
    actx.font = `${devicePx}px "IBM Plex Mono", monospace`;
    actx.textBaseline = "top";
    actx.fillStyle = "#fff";

    this.glyphIndex.clear();
    GLYPHSET.forEach((glyph, i) => {
      this.glyphIndex.set(glyph, i);
      if (glyph !== " ") actx.fillText(glyph, i * cw, 0);
    });

    this.atlas = actx.getImageData(0, 0, atlasCanvas.width, atlasCanvas.height);
  }

  /** The current `background` CSS colour string, resolved to RGB by the browser's own parser. */
  backgroundRGB() {
    const p = this.probe;
    p.fillStyle = this.background;
    p.fillRect(0, 0, 1, 1);
    const d = p.getImageData(0, 0, 1, 1).data;
    return [d[0], d[1], d[2]];
  }

  /**
   * `glyphs` is row-major and `cols * rows` long, indexable as a string or an
   * array. A `colors` entry of 0 means paint in `themeColor`, matching the
   * sentinel aa-wasm packs.
   */
  paint(glyphs, colors, themeColor) {
    const { cols, rows, cellPx, atlas, glyphIndex, frame, frame32 } = this;
    if (!frame) return;

    const out = frame.data;
    const canvasW = this.canvas.width;
    const canvasH = this.canvas.height;
    const [bgR, bgG, bgB] = this.backgroundRGB();
    // Fits one native, optimized fill instead of cols*rows*cellPx.w*cellPx.h
    // individual writes. Little-endian byte order (R at the lowest address)
    // is universal on browser-capable hardware, so this is not a portability
    // gap in practice.
    frame32.fill((255 << 24) | (bgB << 16) | (bgG << 8) | bgR);

    const atlasW = atlas.width;
    const atlasData = atlas.data;

    for (let y = 0; y < rows; y++) {
      const destY0 = y * cellPx.h;
      const maxDy = Math.min(cellPx.h, canvasH - destY0);
      if (maxDy <= 0) continue;

      for (let x = 0; x < cols; x++) {
        const i = y * cols + x;
        const ch = glyphs[i];
        if (ch === " " || ch === undefined) continue;
        const slot = glyphIndex.get(ch);
        if (slot === undefined) continue;

        const destX0 = x * cellPx.w;
        const maxDx = Math.min(cellPx.w, canvasW - destX0);
        if (maxDx <= 0) continue;

        const packed = colors[i];
        const r = packed === 0 ? themeColor.r : (packed >>> 16) & 0xff;
        const g = packed === 0 ? themeColor.g : (packed >>> 8) & 0xff;
        const b = packed === 0 ? themeColor.b : packed & 0xff;

        const atlasX0 = slot * cellPx.w;

        for (let dy = 0; dy < maxDy; dy++) {
          const atlasRow = (dy * atlasW + atlasX0) * 4;
          const destRow = ((destY0 + dy) * canvasW + destX0) * 4;
          for (let dx = 0; dx < maxDx; dx++) {
            const a = atlasData[atlasRow + dx * 4 + 3];
            if (a === 0) continue;
            const o = destRow + dx * 4;
            if (a === 255) {
              out[o] = r;
              out[o + 1] = g;
              out[o + 2] = b;
            } else {
              const t = a / 255;
              out[o] = bgR + (r - bgR) * t;
              out[o + 1] = bgG + (g - bgG) * t;
              out[o + 2] = bgB + (b - bgB) * t;
            }
            out[o + 3] = 255;
          }
        }
      }
    }

    this.ctx.putImageData(frame, 0, 0);
  }
}

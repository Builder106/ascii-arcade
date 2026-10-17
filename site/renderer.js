/*
 * One canvas of character cells behind the whole page. Frames come from
 * sources (aa-core scenes, or DOOM's bitmap), and switching source dissolves
 * cell by cell over a stable random threshold per cell, the way the Mac app's
 * SceneView hands one scene to the next.
 */

const FONT = '"IBM Plex Mono", ui-monospace, Menlo, monospace';
const BLOCK = "█";

export class Grid {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d", { alpha: false });
    this.cols = 0;
    this.rows = 0;
    this.cell = { w: 8, h: 16, ascent: 12 };
    this.thresholds = new Float32Array(0);
    this.mask = new Uint8Array(0);
    this.hexes = new Map();
  }

  resize(width, height, fontPx, dpr = Math.min(window.devicePixelRatio || 1, 2)) {
    const { ctx, canvas } = this;
    ctx.font = `400 ${fontPx}px ${FONT}`;
    const m = ctx.measureText("M");
    const ascent = m.fontBoundingBoxAscent || fontPx * 0.8;
    const descent = m.fontBoundingBoxDescent || fontPx * 0.25;
    this.cell = { w: m.width, h: Math.ceil(ascent + descent), ascent };
    this.cols = Math.ceil(width / this.cell.w);
    this.rows = Math.ceil(height / this.cell.h);
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.font = `400 ${fontPx}px ${FONT}`;

    const n = this.cols * this.rows;
    this.thresholds = new Float32Array(n);
    let s = 0x9e3779b9;
    for (let i = 0; i < n; i++) {
      s ^= s << 13;
      s ^= s >>> 17;
      s ^= s << 5;
      this.thresholds[i] = (s >>> 0) / 4294967296;
    }
    this.mask = new Uint8Array(n);
  }

  /** Keeps the cells under these rects (CSS px) empty, so copy sits in a clean hole. */
  setMask(rects) {
    const { w, h } = this.cell;
    this.mask.fill(0);
    for (const r of rects) {
      if (!r.width || !r.height) continue;
      const c0 = Math.max(0, Math.floor(r.left / w));
      const c1 = Math.min(this.cols, Math.ceil(r.right / w));
      const r0 = Math.max(0, Math.floor(r.top / h));
      const r1 = Math.min(this.rows, Math.ceil(r.bottom / h));
      for (let y = r0; y < r1; y++) this.mask.fill(1, y * this.cols + c0, y * this.cols + c1);
    }
  }

  /**
   * Paints frame `a`, with frame `b` holding every cell whose threshold is
   * below `p`. A bitmap frame (DOOM) is drawn whole first, and every cell that
   * belongs to a text frame, or to the mask, is cleared back to ground on top.
   */
  paint(a, b, p, ground, ink) {
    const { ctx, cols, rows, mask, thresholds } = this;
    const { w, h, ascent } = this.cell;
    ctx.fillStyle = ground;
    ctx.fillRect(0, 0, cols * w, rows * h);

    const bm = a?.bitmap ? a : b?.bitmap ? b : null;
    if (bm) {
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(bm.bitmap, bm.rect.x, bm.rect.y, bm.rect.w, bm.rect.h);
    }

    const cleared = [];
    const buckets = new Map();
    for (let i = 0; i < cols * rows; i++) {
      const f = b && thresholds[i] < p ? b : a;
      if (mask[i] || (bm && f && !f.bitmap)) {
        if (bm) cleared.push(i);
        if (mask[i]) continue;
      }
      if (!f || f.bitmap) continue;
      const color = f.colors[i] || 0;
      const ch = f.glyphs ? f.glyphs[i] : color ? BLOCK : " ";
      if (!ch || ch === " ") continue;
      let list = buckets.get(color);
      if (!list) buckets.set(color, (list = []));
      list.push(i, ch);
    }

    ctx.fillStyle = ground;
    for (const i of cleared) ctx.fillRect((i % cols) * w, Math.floor(i / cols) * h, w + 0.5, h + 0.5);

    for (const [color, list] of buckets) {
      ctx.fillStyle = color ? this.hex(color) : ink;
      for (let k = 0; k < list.length; k += 2) {
        const i = list[k];
        const x = (i % cols) * w;
        const y = Math.floor(i / cols) * h;
        if (list[k + 1] === BLOCK) ctx.fillRect(x, y, w + 0.5, h + 0.5);
        else ctx.fillText(list[k + 1], x, y + ascent);
      }
    }
  }

  hex(color) {
    let s = this.hexes.get(color);
    if (!s) {
      s = `#${(color & 0xffffff).toString(16).padStart(6, "0")}`;
      this.hexes.set(color, s);
    }
    return s;
  }
}

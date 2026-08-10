/*
 * Scene transitions dissolve character by character instead of crossfading
 * opacity. Each cell gets a fixed threshold; as progress sweeps 0 to 1 a cell
 * flips the moment progress passes its threshold. One array and one comparison
 * per cell, and it is something images cannot do.
 */

/**
 * Deterministic thresholds. Seeded so a resize or a scrub backwards reproduces
 * the same dissolve rather than reshuffling under the reader.
 */
export function makeThresholds(count, seed) {
  const out = new Float32Array(count);
  // xorshift32 needs a non-zero state, and seed 0 is a plausible caller input.
  let s = (seed >>> 0) || 0x9e3779b9;
  for (let i = 0; i < count; i++) {
    s ^= s << 13;
    s >>>= 0;
    s ^= s >>> 17;
    s ^= s << 5;
    s >>>= 0;
    out[i] = s / 0xffffffff;
  }
  return out;
}

/**
 * Mix two frames at `progress`. Glyph and colour move together, so a cell
 * never shows one scene's character in the other's colour.
 */
export function blend(fromGlyphs, toGlyphs, fromColors, toColors, thresholds, progress) {
  const n = thresholds.length;
  const glyphs = new Array(n);
  const colors = new Uint32Array(n);

  for (let i = 0; i < n; i++) {
    const flipped = progress >= 1 || (progress > 0 && progress > thresholds[i]);
    glyphs[i] = flipped ? toGlyphs[i] : fromGlyphs[i];
    colors[i] = flipped ? toColors[i] : fromColors[i];
  }
  return { glyphs, colors };
}

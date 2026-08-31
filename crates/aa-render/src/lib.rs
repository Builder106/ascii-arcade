//! # aa-render
//!
//! Shared software rasteriser. Both the Windows and Linux shells ultimately blit
//! an RGBA pixel buffer to their wallpaper surface, so the work of turning an
//! [`aa_core::Frame`] into pixels — glyph atlas lookup, per-cell colour, the
//! theme's background, and the CRT scanline/glow effects ported from the macOS
//! `SceneView` — lives here once.
//!
//! ## Pipeline
//!
//! 1. Fill the whole buffer with [`Theme::background`].
//! 2. For each non-blank cell, blit its glyph from the embedded 8×16 bitmap
//!    [`font`] using the cell's colour (or [`Theme::text`] when `None`),
//!    scaled to fit `cell_w × cell_h`.
//! 3. If `glow` is set, add a cheap separable-box-blur bloom of the lit pixels
//!    back over the image — the software analogue of the macOS layer shadow
//!    (`shadowColor = text`, `shadowRadius = 10`, `shadowOpacity ≈ 0.45`).
//! 4. If `scanlines` is set, darken every other row — the macOS
//!    `CAReplicatorLayer` stripe (1px black at ~18% alpha every 2px).
//!
//! The output is tightly-packed RGBA8888, row-major, fully opaque.

pub mod font;

use aa_core::{Frame, RgbColor, Theme};

/// A tightly-packed RGBA8888 pixel buffer, row-major, `width * height * 4` bytes.
#[derive(Clone, Debug)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl PixelBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        PixelBuffer {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Fill every pixel with an opaque `color`.
    fn fill(&mut self, color: RgbColor) {
        for px in self.pixels.as_chunks_mut::<4>().0 {
            px[0] = color.r;
            px[1] = color.g;
            px[2] = color.b;
            px[3] = 255;
        }
    }
}

/// How to rasterise a [`Frame`]: cell metrics, theme, and the CRT effects.
#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub cell_w: u32,
    pub cell_h: u32,
    pub theme: Theme,
    pub scanlines: bool,
    pub glow: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            cell_w: 8,
            cell_h: 16,
            theme: Theme::HACKER,
            scanlines: true,
            glow: true,
        }
    }
}

/// Rasterise `frame` into a fresh [`PixelBuffer`].
///
/// Background comes from `opts.theme.background`; each non-blank cell's glyph is
/// blitted in `cell.color` (falling back to `opts.theme.text`). CRT scanlines
/// and the glow/bloom are applied when their flags are set. The buffer is always
/// at least 1×1 so callers never get a zero-area surface.
pub fn render(frame: &Frame, opts: &RenderOptions) -> PixelBuffer {
    let cell_w = opts.cell_w.max(1);
    let cell_h = opts.cell_h.max(1);
    let w = (frame.width as u32 * cell_w).max(1);
    let h = (frame.height as u32 * cell_h).max(1);

    let mut buf = PixelBuffer::new(w, h);
    buf.fill(opts.theme.background);

    // Draw every non-blank glyph. The background is captured first so the bloom
    // pass can tell "lit" pixels (anything brighter than the background) apart.
    for row in 0..frame.height {
        for col in 0..frame.width {
            let cell = frame.cells[frame.idx(col, row)];
            if cell.ch == ' ' {
                continue;
            }
            let color = cell.color.unwrap_or(opts.theme.text);
            blit_glyph(
                &mut buf,
                cell.ch,
                col as u32 * cell_w,
                row as u32 * cell_h,
                cell_w,
                cell_h,
                color,
            );
        }
    }

    if opts.glow {
        apply_glow(&mut buf, opts.theme.background);
    }
    if opts.scanlines {
        apply_scanlines(&mut buf);
    }

    buf
}

/// Blit one glyph into `buf` at top-left pixel `(x0, y0)`, scaled to fit a
/// `cell_w × cell_h` cell. Lit font bits are painted in `color`; blank bits are
/// left untouched so the background (and any neighbour's overlap) shows through.
/// Unknown / non-printable characters map to a blank glyph (nothing drawn).
fn blit_glyph(
    buf: &mut PixelBuffer,
    ch: char,
    x0: u32,
    y0: u32,
    cell_w: u32,
    cell_h: u32,
    color: RgbColor,
) {
    let bitmap = match glyph_bitmap(ch) {
        Some(b) => b,
        None => return,
    };
    let bw = buf.width;
    let bh = buf.height;
    // Nearest-neighbour scale from the GLYPH_W × GLYPH_H source into the cell.
    for dy in 0..cell_h {
        let py = y0 + dy;
        if py >= bh {
            break;
        }
        let src_row = (dy * font::GLYPH_H as u32 / cell_h) as usize;
        let bits = bitmap[src_row];
        if bits == 0 {
            continue;
        }
        let row_off = (py * bw) as usize * 4;
        for dx in 0..cell_w {
            let px = x0 + dx;
            if px >= bw {
                break;
            }
            let src_col = (dx * font::GLYPH_W as u32 / cell_w) as u8;
            // Source columns run left (MSB, bit 7) to right (LSB, bit 0).
            if bits & (0x80 >> src_col) != 0 {
                let off = row_off + px as usize * 4;
                buf.pixels[off] = color.r;
                buf.pixels[off + 1] = color.g;
                buf.pixels[off + 2] = color.b;
                buf.pixels[off + 3] = 255;
            }
        }
    }
}

/// The bitmap for a printable ASCII glyph, or `None` for anything outside the
/// embedded range (the caller skips it, matching the macOS "missing glyph" path).
fn glyph_bitmap(ch: char) -> Option<&'static [u8; font::GLYPH_H]> {
    let code = u32::from(ch);
    if code < u32::from(font::FIRST) || code > u32::from(font::LAST) {
        return None;
    }
    Some(&font::GLYPHS[(code as u8 - font::FIRST) as usize])
}

/// Additive bloom: blur the lit pixels' colour (their excess over the
/// background) and add that blurred halo back. This is the software stand-in for
/// the macOS layer shadow that haloed the glowing text — a green glyph bleeds a
/// green fringe onto its dark neighbours, an amber one an amber fringe. Cheap:
/// three separable box-blur passes (horizontal then vertical) per channel,
/// approximating a Gaussian of the macOS `shadowRadius` feel.
fn apply_glow(buf: &mut PixelBuffer, bg: RgbColor) {
    let w = buf.width as usize;
    let h = buf.height as usize;
    if w == 0 || h == 0 {
        return;
    }

    // Per-channel "excess over background": how much brighter than the backdrop
    // each pixel is. Glyph strokes read high; background reads ~0. We blur this
    // signed-up-from-zero excess so the halo carries the glyph's own colour.
    let mut ch_r = vec![0u16; w * h];
    let mut ch_g = vec![0u16; w * h];
    let mut ch_b = vec![0u16; w * h];
    for (i, px) in buf.pixels.as_chunks::<4>().0.iter().enumerate() {
        ch_r[i] = px[0].saturating_sub(bg.r) as u16;
        ch_g[i] = px[1].saturating_sub(bg.g) as u16;
        ch_b[i] = px[2].saturating_sub(bg.b) as u16;
    }

    const RADIUS: usize = 3;
    const PASSES: usize = 3;
    let mut tmp = vec![0u16; w * h];
    for chan in [&mut ch_r, &mut ch_g, &mut ch_b] {
        for _ in 0..PASSES {
            box_blur_h(chan, &mut tmp, w, h, RADIUS);
            box_blur_v(&tmp, chan, w, h, RADIUS);
        }
    }

    // Add the blurred halo back. STRENGTH ≈ the macOS shadowOpacity (0.45).
    const STRENGTH_NUM: u32 = 45;
    const STRENGTH_DEN: u32 = 100;
    for (i, px) in buf.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let hr = ch_r[i] as u32 * STRENGTH_NUM / STRENGTH_DEN;
        let hg = ch_g[i] as u32 * STRENGTH_NUM / STRENGTH_DEN;
        let hb = ch_b[i] as u32 * STRENGTH_NUM / STRENGTH_DEN;
        px[0] = (px[0] as u32 + hr).min(255) as u8;
        px[1] = (px[1] as u32 + hg).min(255) as u8;
        px[2] = (px[2] as u32 + hb).min(255) as u8;
    }
}

/// One horizontal box-blur pass (running-sum, O(n) per row).
fn box_blur_h(src: &[u16], dst: &mut [u16], w: usize, h: usize, radius: usize) {
    let window = (radius * 2 + 1) as u32;
    for y in 0..h {
        let row = y * w;
        let mut sum: u32 = 0;
        // Prime the window over [0, radius], clamping at the left edge.
        for x in 0..=radius.min(w - 1) {
            sum += src[row + x] as u32;
        }
        for x in 0..w {
            dst[row + x] = (sum / window) as u16;
            let add = x + radius + 1;
            if add < w {
                sum += src[row + add] as u32;
            }
            if x >= radius {
                sum -= src[row + x - radius] as u32;
            }
        }
    }
}

/// One vertical box-blur pass (running-sum, O(n) per column).
fn box_blur_v(src: &[u16], dst: &mut [u16], w: usize, h: usize, radius: usize) {
    let window = (radius * 2 + 1) as u32;
    for x in 0..w {
        let mut sum: u32 = 0;
        for y in 0..=radius.min(h - 1) {
            sum += src[y * w + x] as u32;
        }
        for y in 0..h {
            dst[y * w + x] = (sum / window) as u16;
            let add = y + radius + 1;
            if add < h {
                sum += src[add * w + x] as u32;
            }
            if y >= radius {
                sum -= src[(y - radius) * w + x] as u32;
            }
        }
    }
}

/// Darken every other pixel row to suggest CRT scanlines — the software port of
/// the macOS `CAReplicatorLayer` (a 1px black stripe at ~18% alpha every 2px).
fn apply_scanlines(buf: &mut PixelBuffer) {
    const DARKEN_NUM: u32 = 82; // (1.0 - 0.18) * 100, rounded
    const DARKEN_DEN: u32 = 100;
    let w = buf.width as usize;
    let h = buf.height as usize;
    for y in (1..h).step_by(2) {
        let row = y * w * 4;
        for px in buf.pixels[row..row + w * 4].as_chunks_mut::<4>().0 {
            px[0] = (px[0] as u32 * DARKEN_NUM / DARKEN_DEN) as u8;
            px[1] = (px[1] as u32 * DARKEN_NUM / DARKEN_DEN) as u8;
            px[2] = (px[2] as u32 * DARKEN_NUM / DARKEN_DEN) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_core::Cell;

    /// Build a frame with `ch` (optional colour) at `(x, y)`, rest blank.
    fn one_cell_frame(w: usize, h: usize, x: usize, y: usize, cell: Cell) -> Frame {
        let mut f = Frame::blank(w, h);
        f.set(x, y, cell);
        f
    }

    fn pixel(buf: &PixelBuffer, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let off = ((y * buf.width + x) * 4) as usize;
        (
            buf.pixels[off],
            buf.pixels[off + 1],
            buf.pixels[off + 2],
            buf.pixels[off + 3],
        )
    }

    #[test]
    fn output_dimensions_match_grid_times_cell() {
        let f = Frame::blank(10, 4);
        let opts = RenderOptions {
            cell_w: 8,
            cell_h: 16,
            ..Default::default()
        };
        let buf = render(&f, &opts);
        assert_eq!(buf.width, 80);
        assert_eq!(buf.height, 64);
        assert_eq!(buf.pixels.len(), 80 * 64 * 4);
    }

    #[test]
    fn empty_grid_still_yields_nonzero_buffer() {
        let f = Frame::blank(0, 0);
        let buf = render(&f, &RenderOptions::default());
        assert!(buf.width >= 1 && buf.height >= 1);
        assert_eq!(buf.pixels.len() as u32, buf.width * buf.height * 4);
    }

    #[test]
    fn all_blank_frame_is_pure_background() {
        // Disable FX so the assertion is exactly "every pixel == background".
        let opts = RenderOptions {
            theme: Theme::HACKER,
            scanlines: false,
            glow: false,
            ..Default::default()
        };
        let f = Frame::blank(6, 3);
        let buf = render(&f, &opts);
        let bg = opts.theme.background;
        for px in buf.pixels.as_chunks::<4>().0 {
            assert_eq!((px[0], px[1], px[2], px[3]), (bg.r, bg.g, bg.b, 255));
        }
    }

    #[test]
    fn blank_frame_with_scanlines_only_touches_background_rows() {
        // Scanlines on a blank Hacker frame: background is pure black, so
        // darkening it leaves black. Output must still be entirely background.
        let opts = RenderOptions {
            theme: Theme::HACKER,
            scanlines: true,
            glow: true,
            ..Default::default()
        };
        let buf = render(&Frame::blank(4, 4), &opts);
        let bg = opts.theme.background;
        for px in buf.pixels.as_chunks::<4>().0 {
            assert_eq!((px[0], px[1], px[2]), (bg.r, bg.g, bg.b));
        }
    }

    #[test]
    fn lit_cell_differs_from_background() {
        let opts = RenderOptions {
            cell_w: 8,
            cell_h: 16,
            theme: Theme::HACKER,
            scanlines: false,
            glow: false,
        };
        let red = RgbColor::new(255, 0, 0);
        let f = one_cell_frame(3, 3, 1, 1, Cell::new('A', Some(red)));
        let buf = render(&f, &opts);
        let bg = opts.theme.background;

        // Somewhere inside the lit cell there must be a pixel that isn't the
        // background colour (the glyph's strokes).
        let mut found_lit = false;
        for cy in 0..opts.cell_h {
            for cx in 0..opts.cell_w {
                let px = pixel(&buf, opts.cell_w + cx, opts.cell_h + cy);
                if (px.0, px.1, px.2) != (bg.r, bg.g, bg.b) {
                    found_lit = true;
                    // Lit pixels of an 'A' drawn red must be exactly red.
                    assert_eq!((px.0, px.1, px.2), (255, 0, 0));
                }
            }
        }
        assert!(found_lit, "lit cell produced no non-background pixels");
    }

    #[test]
    fn cell_without_color_uses_theme_text() {
        let opts = RenderOptions {
            cell_w: 8,
            cell_h: 16,
            theme: Theme::HACKER,
            scanlines: false,
            glow: false,
        };
        let f = one_cell_frame(1, 1, 0, 0, Cell::new('#', None));
        let buf = render(&f, &opts);
        let text = opts.theme.text;
        let lit = buf
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|px| (px[0], px[1], px[2]) == (text.r, text.g, text.b));
        assert!(
            lit,
            "an uncoloured cell should be painted in the theme text colour"
        );
    }

    #[test]
    fn unknown_glyph_draws_nothing() {
        let opts = RenderOptions {
            scanlines: false,
            glow: false,
            ..Default::default()
        };
        // A codepoint outside printable ASCII -> blank glyph, pure background.
        let f = one_cell_frame(2, 2, 0, 0, Cell::new('\u{2603}', Some(RgbColor::WHITE)));
        let buf = render(&f, &opts);
        let bg = opts.theme.background;
        for px in buf.pixels.as_chunks::<4>().0 {
            assert_eq!((px[0], px[1], px[2]), (bg.r, bg.g, bg.b));
        }
    }

    #[test]
    fn glow_spreads_brightness_beyond_the_glyph() {
        // With glow on, a single lit cell on a dark theme should brighten some
        // background pixels around it that the bare glyph never touched.
        let no_fx = RenderOptions {
            theme: Theme::HACKER,
            scanlines: false,
            glow: false,
            ..Default::default()
        };
        let with_glow = RenderOptions {
            glow: true,
            ..no_fx
        };
        let f = one_cell_frame(5, 5, 2, 2, Cell::new('@', Some(RgbColor::new(0, 255, 0))));
        let plain = render(&f, &no_fx);
        let bloomed = render(&f, &with_glow);

        let mut spread = false;
        for (a, b) in plain
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(bloomed.pixels.as_chunks::<4>().0.iter())
        {
            // A pixel that was background in `plain` but brighter in `bloomed`.
            if a[1] == Theme::HACKER.background.g && b[1] > a[1] {
                spread = true;
                break;
            }
        }
        assert!(
            spread,
            "glow did not spread any brightness onto background pixels"
        );
    }

    #[test]
    fn font_covers_all_printable_ascii() {
        for code in font::FIRST..=font::LAST {
            assert!(glyph_bitmap(code as char).is_some());
        }
        // Space exists but is entirely blank.
        assert!(glyph_bitmap(' ').unwrap().iter().all(|&b| b == 0));
        // A glyph with strokes is non-empty.
        assert!(glyph_bitmap('A').unwrap().iter().any(|&b| b != 0));
    }

    #[test]
    fn test_render_edge_cases() {
        // Zero cell width/height should clamp to 1
        let f = one_cell_frame(1, 1, 0, 0, Cell::new('A', None));
        let opts = RenderOptions {
            cell_w: 0,
            cell_h: 0,
            theme: Theme::HACKER,
            scanlines: true,
            glow: true,
        };
        let buf = render(&f, &opts);
        assert_eq!(buf.width, 1);
        assert_eq!(buf.height, 1);

        // Empty PixelBuffer glow early return
        let mut empty_buf = PixelBuffer::new(0, 0);
        apply_glow(&mut empty_buf, Theme::HACKER.background);
        assert_eq!(empty_buf.pixels.len(), 0);

        // Character outside printable ASCII in blit_glyph
        let mut buf_glyph = PixelBuffer::new(8, 16);
        blit_glyph(
            &mut buf_glyph,
            '\0',
            0,
            0,
            8,
            16,
            RgbColor::new(255, 255, 255),
        );
    }
}

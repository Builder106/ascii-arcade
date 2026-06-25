//! # aa-render
//!
//! Shared software rasteriser. Both the Windows and Linux shells ultimately blit
//! an RGBA pixel buffer to their wallpaper surface, so the work of turning an
//! [`aa_core::Frame`] into pixels — glyph atlas lookup, per-cell colour, the
//! theme's background, and the CRT scanline/glow effects ported from the macOS
//! `SceneView` — lives here once.
//!
//! STATUS: skeleton. The real rasteriser is implemented by the aa-render
//! work-stream. The signature below is the contract the shells code against.

use aa_core::{Frame, Theme};

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
}

/// How to rasterise a [`Frame`]: target pixel size, cell metrics, theme, FX.
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
/// TODO(aa-render): real glyph atlas + colour + FX. The stub paints the theme
/// background so a shell wired to this shows a solid themed wallpaper, proving
/// the surface plumbing before the rasteriser lands.
pub fn render(frame: &Frame, opts: &RenderOptions) -> PixelBuffer {
    let w = frame.width as u32 * opts.cell_w;
    let h = frame.height as u32 * opts.cell_h;
    let mut buf = PixelBuffer::new(w.max(1), h.max(1));
    let bg = opts.theme.background;
    for px in buf.pixels.chunks_exact_mut(4) {
        px[0] = bg.r;
        px[1] = bg.g;
        px[2] = bg.b;
        px[3] = 255;
    }
    buf
}

//! Render one donut frame and dump it as a binary PPM (P6) so the rasteriser's
//! look — glyphs, scanlines, glow — can be eyeballed without a windowing shell.
//!
//! Run:
//!   cargo run -p aa-render --example donut_ppm -- /tmp/donut.ppm
//!
//! Defaults to `donut.ppm` in the current directory. Convert/view with e.g.
//! `magick donut.ppm donut.png` or any PPM-aware viewer. Not committed.

use std::io::Write;

use aa_core::scene::Scene;
use aa_core::scenes::donut::DonutScene;
use aa_render::{render, RenderOptions};

fn main() -> std::io::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "donut.ppm".to_string());

    let mut donut = DonutScene::new();
    donut.set_grid(80, 30);
    // A non-zero time so the torus is rotated into a recognisable pose.
    let frame = donut.frame(1.2);

    let opts = RenderOptions {
        cell_w: 8,
        cell_h: 16,
        theme: aa_core::Theme::HACKER,
        scanlines: true,
        glow: true,
    };
    let buf = render(&frame, &opts);

    let mut out = std::fs::File::create(&path)?;
    write!(out, "P6\n{} {}\n255\n", buf.width, buf.height)?;
    // PPM is RGB; drop the alpha channel from each RGBA pixel.
    let mut rgb = Vec::with_capacity((buf.width * buf.height * 3) as usize);
    for px in buf.pixels.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    out.write_all(&rgb)?;

    eprintln!(
        "wrote {} ({}x{}) — {} non-blank cells",
        path,
        buf.width,
        buf.height,
        frame.cells.iter().filter(|c| c.ch != ' ').count()
    );
    Ok(())
}

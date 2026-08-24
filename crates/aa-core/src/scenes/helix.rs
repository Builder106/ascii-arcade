//! An infinite helix, lit and z-buffered, flown down from an aerial view.
//!
//! Port of `HelixFrameGenerator.swift`. A math scene in the same mould as
//! [`crate::scenes::donut`]: hold the grid size, compute a fresh [`Frame`] from
//! `t`. Monochrome, so every cell's colour is left `None`.
//!
//! The coil itself never moves and has no fixed length — the helix is
//! conceptually infinite, so each frame only generates the window of turns
//! currently near the camera (`VISIBLE_TURNS` worth of `u`, centered on
//! wherever the camera currently is) rather than a fixed-size coil. The
//! shape is translation-invariant along its own axis, so a freshly
//! generated window looks identical to the last one — nothing to stitch or
//! blend, just keep sliding the window as the camera moves.
//!
//! The only motion is the camera: it rides the coil's own central axis,
//! looking straight down it (aerial view), and descends at a constant rate
//! forever — one direction, no turning around, no bouncing between ends,
//! because an infinite helix has no ends to bounce between. (Earlier
//! versions rotated or tilted the object itself, and later oscillated the
//! camera back and forth over a finite coil; all three read as wrong for
//! this scene — only the camera should move, continuously, in one
//! direction.)
//!
//! `depth` is just how far below the camera a point sits (`cam_z - pz`),
//! since the camera only ever looks down. Points at or above the camera
//! (`depth <= NEAR_CLIP`) are skipped the way a near clip plane would skip
//! them, rather than letting `1/depth` blow up as a turn's height passes
//! the camera's current position. The camera rides the coil's exact
//! central axis (radius zero), while the tube's surface never reaches that
//! axis (`R - R_MINOR` is well clear of zero) — so depth can approach
//! `NEAR_CLIP` as the camera passes a turn's height, but the point is never
//! actually *at* the camera in 3D, only close in the one axial coordinate.

use crate::frame::Frame;
use crate::scene::Scene;

const LUMINANCE: &[u8] = b".,-~:;=!*#$@ ";

pub struct HelixScene {
    width: usize,
    height: usize,
}

impl HelixScene {
    // Tube major/minor radius and vertical pitch — the coil's cross-section
    // and rise-per-turn. R_MINOR is kept small relative to PITCH (vertical
    // turn spacing is ~6x the tube diameter) so consecutive turns show real
    // background gaps between them and read as a coil rather than a solid
    // filled blob — a thicker tube visually merges turns together well
    // before they touch in 3D.
    const R: f64 = 1.5;
    const R_MINOR: f64 = 0.2;
    const PITCH: f64 = 0.4;

    // How many turns' worth of u-range to generate around the camera each
    // frame — enough to see turns both ahead of and behind it, like a
    // tunnel, without generating the (literally unbounded) rest of the coil.
    const VISIBLE_TURNS: f64 = 3.0;

    // Units/sec the camera descends. Constant and one-directional.
    const CAM_SPEED: f64 = 0.5;

    // Reference axial distance used only to calibrate the projection scale
    // (`K1`) — roughly one turn's vertical spacing, so a turn at a "normal"
    // viewing distance fills a sensible fraction of the screen.
    const CAM_REF_DEPTH: f64 = 2.5;
    // Depths shallower than this are treated as behind/at the camera and
    // skipped, rather than letting `1/depth` blow up as a turn's height
    // passes the camera's current position.
    const NEAR_CLIP: f64 = 0.3;

    pub fn new() -> Self {
        HelixScene {
            width: 10,
            height: 10,
        }
    }
}

impl Default for HelixScene {
    fn default() -> Self {
        HelixScene::new()
    }
}

impl Scene for HelixScene {
    fn display_name(&self) -> &str {
        "Helix"
    }

    fn set_grid(&mut self, width: usize, height: usize) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
        }
    }

    fn frame(&mut self, t: f64) -> Frame {
        use std::f64::consts::PI;
        let (w, h) = (self.width, self.height);
        let screen = w * h;
        let mut zbuf = vec![0.0f64; screen];
        let mut frame = Frame::blank(w, h);

        let cam_z = -Self::CAM_SPEED * t;

        let projection = Self::CAM_REF_DEPTH * 2.0 / (8.0 * (Self::R + Self::R_MINOR));
        let k1 = w.min(h) as f64 * projection;

        // The window of turns to generate this frame, centered on wherever
        // the camera currently is (pz ≈ PITCH * u, so u_center is just the
        // camera's position rescaled into u-space).
        let u_center = cam_z / Self::PITCH;
        let u_half_range = Self::VISIBLE_TURNS * PI;
        let u_end = u_center + u_half_range;

        let mut u = u_center - u_half_range;
        while u < u_end {
            let (cos_u, sin_u) = (u.cos(), u.sin());
            let mut v = 0.0;
            while v < 2.0 * PI {
                let (cos_v, sin_v) = (v.cos(), v.sin());

                let px = cos_u * (Self::R + Self::R_MINOR * cos_v);
                let py = sin_u * (Self::R + Self::R_MINOR * cos_v);
                let pz = Self::PITCH * u + Self::R_MINOR * sin_v;

                let ny = cos_v * sin_u;
                let nz = sin_v;

                // The camera looks straight down the coil's axis from its
                // current position, so depth is just the axial gap between
                // camera and point — no rotation needed at all, since
                // neither the coil nor the camera's orientation ever
                // changes, only the camera's position along the axis.
                let depth = cam_z - pz;
                if depth > Self::NEAR_CLIP {
                    let ooz = 1.0 / depth;

                    // Light from (0, 1, −1)/√2, fixed in world space (the
                    // coil never rotates, so the normal needs no rotating).
                    let l = ny - nz;

                    if l > 0.0 {
                        let xp = (w as f64 / 2.0 + k1 * ooz * px) as isize;
                        let yp = (h as f64 / 2.0 - k1 * ooz * py) as isize;
                        if xp >= 0 && yp >= 0 {
                            let (xp, yp) = (xp as usize, yp as usize);
                            if xp < w && yp < h {
                                let i = yp * w + xp;
                                if ooz > zbuf[i] {
                                    zbuf[i] = ooz;
                                    let li = (l * 5.66) as isize;
                                    let li = li.clamp(0, LUMINANCE.len() as isize - 1) as usize;
                                    frame.cells[i].ch = LUMINANCE[li] as char;
                                }
                            }
                        }
                    }
                }

                v += 0.07;
            }
            u += 0.04;
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_correct_dimensions() {
        let mut s = HelixScene::new();
        s.set_grid(40, 20);
        let f = s.frame(1.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn renders_some_glyphs() {
        let mut s = HelixScene::new();
        s.set_grid(80, 40);
        let f = s.frame(1.0);
        let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
        assert!(non_blank > 50, "helix should fill a chunk of the grid");
    }

    #[test]
    fn deterministic_for_same_time() {
        let mut s = HelixScene::new();
        s.set_grid(60, 30);
        assert_eq!(s.frame(2.5).text(), s.frame(2.5).text());
    }

    #[test]
    fn stays_visible_across_an_arbitrarily_long_descent() {
        // Unlike a bounded coil, an infinite one has no legitimate "camera
        // has passed the whole thing" moment — the window always follows
        // the camera, so this expects the coil visible at every sampled
        // instant, including very large t. Large t also exercises u_center
        // (and therefore u itself) at large magnitudes, which is the one
        // place long uptimes could plausibly degrade — cos/sin range
        // reduction losing precision far from zero — so this doubles as a
        // long-uptime regression check.
        let mut s = HelixScene::new();
        s.set_grid(80, 40);
        for t in [0.0, 5.0, 13.0, 47.0, 500.0, 10_000.0, 1_000_000.0] {
            let f = s.frame(t);
            let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
            assert!(
                non_blank > 50,
                "coil should stay visible at every point along an infinite descent (t={t}, non_blank={non_blank})"
            );
        }
    }

    #[test]
    fn display_name_and_default() {
        let mut s = HelixScene::default();
        assert_eq!(s.display_name(), "Helix");

        // Zero dimensions ignored
        s.set_grid(0, 0);
        s.set_grid(50, 0);
        s.set_grid(0, 30);
        let f = s.frame(0.0);
        assert_eq!(f.width, 10);
        assert_eq!(f.height, 10);
    }
}

//! Rotating, precessing helix — a tube wound into a coil, lit and z-buffered.
//!
//! Port of `HelixFrameGenerator.swift`. A math scene in the same mould as
//! [`crate::scenes::donut`]: hold the grid size, compute a fresh [`Frame`] from
//! `t`. Monochrome, so every cell's colour is left `None`.

use crate::frame::Frame;
use crate::scene::Scene;

const LUMINANCE: &[u8] = b".,-~:;=!*#$@ ";

pub struct HelixScene {
    width: usize,
    height: usize,
}

impl HelixScene {
    // Tube major/minor radius, turn count and vertical pitch — the coil shape.
    const R: f64 = 1.5;
    const R_MINOR: f64 = 0.4;
    const NUM_TURNS: f64 = 2.5;
    const PITCH: f64 = 0.4;

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

        let a = t * 1.0;
        let b = t * 0.5;
        let c = (t * 0.4).sin() * 0.6; // precessing wobble around z-axis
        let (cos_a, sin_a) = (a.cos(), a.sin());
        let (cos_b, sin_b) = (b.cos(), b.sin());
        let (cos_c, sin_c) = (c.cos(), c.sin());

        let k2 = 5.0;
        // Reduced 3.0 → 2.0 to shrink the helix to roughly donut scale.
        let projection = k2 * 2.0 / (8.0 * (Self::R + Self::R_MINOR));
        let k1 = w.min(h) as f64 * projection;

        let half_height = Self::PITCH * Self::NUM_TURNS * PI;

        let mut u = 0.0;
        while u < Self::NUM_TURNS * 2.0 * PI {
            let (cos_u, sin_u) = (u.cos(), u.sin());
            let mut v = 0.0;
            while v < 2.0 * PI {
                let (cos_v, sin_v) = (v.cos(), v.sin());

                let px = cos_u * (Self::R + Self::R_MINOR * cos_v);
                let py = sin_u * (Self::R + Self::R_MINOR * cos_v);
                let pz = Self::PITCH * u - half_height + Self::R_MINOR * sin_v;

                let nx = cos_v * cos_u;
                let ny = cos_v * sin_u;
                let nz = sin_v;

                // Rz(C) — precession
                let px_c = px * cos_c - py * sin_c;
                let py_c = px * sin_c + py * cos_c;
                let nx_c = nx * cos_c - ny * sin_c;
                let ny_c = nx * sin_c + ny * cos_c;

                // Rx(A)
                let py1 = py_c * cos_a - pz * sin_a;
                let pz1 = py_c * sin_a + pz * cos_a;
                let ny1 = ny_c * cos_a - nz * sin_a;
                let nz1 = ny_c * sin_a + nz * cos_a;

                // Ry(B)
                let x = px_c * cos_b + pz1 * sin_b;
                let y = py1;
                let z = k2 - px_c * sin_b + pz1 * cos_b;
                let ooz = 1.0 / z;

                let ny_rot = ny1;
                let nz_rot = -nx_c * sin_b + nz1 * cos_b;

                // Light from (0, 1, −1)/√2
                let l = ny_rot - nz_rot;

                if l > 0.0 {
                    let xp = (w as f64 / 2.0 + k1 * ooz * x) as isize;
                    let yp = (h as f64 / 2.0 - k1 * ooz * y) as isize;
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
}

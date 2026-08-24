//! Spinning torus — the canonical Andy-Sloane donut.
//!
//! Reference port of `DonutFrameGenerator.swift`. This is the pattern every
//! other math scene follows: hold the grid size, implement `Scene::frame` to
//! compute a fresh [`Frame`] from `t`. Monochrome, so every cell's colour is
//! left `None` (the shell paints them in the theme colour).

use crate::frame::Frame;
use crate::scene::Scene;

const LUMINANCE: &[u8] = b".,-~:;=!*#$@ ";

pub struct DonutScene {
    width: usize,
    height: usize,
}

impl DonutScene {
    pub fn new() -> Self {
        DonutScene {
            width: 10,
            height: 10,
        }
    }
}

impl Default for DonutScene {
    fn default() -> Self {
        DonutScene::new()
    }
}

impl Scene for DonutScene {
    fn display_name(&self) -> &str {
        "Donut"
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

        let (a, b) = (t * 1.0, t * 0.5);
        let (cos_a, sin_a) = (a.cos(), a.sin());
        let (cos_b, sin_b) = (b.cos(), b.sin());

        let r1 = 1.0;
        let r2 = 2.0;
        let k2 = 5.0;
        let projection = k2 * 3.0 / (8.0 * (r1 + r2));
        let k1 = w.min(h) as f64 * projection;

        let mut theta = 0.0;
        while theta < 2.0 * PI {
            let (cos_th, sin_th) = (theta.cos(), theta.sin());
            let mut phi = 0.0;
            while phi < 2.0 * PI {
                let (cos_ph, sin_ph) = (phi.cos(), phi.sin());
                let circle_x = r2 + r1 * cos_th;
                let circle_y = r1 * sin_th;

                let x =
                    circle_x * (cos_b * cos_ph + sin_a * sin_b * sin_ph) - circle_y * cos_a * sin_b;
                let y =
                    circle_x * (sin_b * cos_ph - sin_a * cos_b * sin_ph) + circle_y * cos_a * cos_b;
                let z = k2 + cos_a * circle_x * sin_ph + circle_y * sin_a;
                let ooz = 1.0 / z;

                let xp = (w as f64 / 2.0 + k1 * ooz * x) as isize;
                let yp = (h as f64 / 2.0 - k1 * ooz * y) as isize;

                let lum = cos_ph * cos_th * sin_b - cos_a * cos_th * sin_ph - sin_a * sin_th
                    + cos_b * (cos_a * sin_th - cos_th * sin_a * sin_ph);

                if lum > 0.0 && xp >= 0 && yp >= 0 {
                    let (xp, yp) = (xp as usize, yp as usize);
                    if xp < w && yp < h {
                        let i = yp * w + xp;
                        if ooz > zbuf[i] {
                            zbuf[i] = ooz;
                            let li = (lum * 8.0) as isize;
                            let li = li.clamp(0, LUMINANCE.len() as isize - 1) as usize;
                            frame.cells[i].ch = LUMINANCE[li] as char;
                        }
                    }
                }
                phi += 0.02;
            }
            theta += 0.07;
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_correct_dimensions() {
        let mut d = DonutScene::new();
        d.set_grid(40, 20);
        let f = d.frame(1.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        // The text view has 20 rows joined by 19 newlines.
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn renders_some_glyphs() {
        let mut d = DonutScene::new();
        d.set_grid(80, 40);
        let f = d.frame(1.0);
        let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
        assert!(non_blank > 50, "donut should fill a chunk of the grid");
    }

    #[test]
    fn deterministic_for_same_time() {
        let mut d = DonutScene::new();
        d.set_grid(60, 30);
        assert_eq!(d.frame(2.5).text(), d.frame(2.5).text());
    }

    #[test]
    fn display_name_and_default() {
        let mut d = DonutScene::default();
        assert_eq!(d.display_name(), "Donut");

        // Zero grid dimensions should be ignored (keep previous/default dimensions)
        d.set_grid(0, 0);
        d.set_grid(20, 0);
        d.set_grid(0, 10);
        let f = d.frame(0.0);
        assert_eq!(f.width, 10);
        assert_eq!(f.height, 10);
    }

    #[test]
    fn various_angles_and_sizes_exercise_zbuf_and_luminance() {
        let mut d = DonutScene::new();
        for &(w, h) in &[(5, 5), (15, 10), (40, 20), (80, 40), (120, 60)] {
            d.set_grid(w, h);
            for i in 0..50 {
                let t = i as f64 * 0.15;
                let f = d.frame(t);
                assert_eq!(f.width, w);
                assert_eq!(f.height, h);
            }
        }
    }
}

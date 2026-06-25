//! Falling-glyph "digital rain".
//!
//! Port of `MatrixRainScene.swift`. Each column is an independent stream with a
//! bright head and a trail that fades from the theme colour to black. Keying the
//! palette off `base_color` (set via [`Scene::apply_base_color`]) means the rain
//! is green under the Hacker theme, amber under Amber, and so on.
//!
//! Stateful: seeds from [`SeededRng`] and drives a fixed-timestep simulation via
//! [`Stepper`].

use crate::color::RgbColor;
use crate::frame::{Cell, Frame};
use crate::rng::SeededRng;
use crate::scene::{Scene, SceneOption, SceneSetting};
use crate::scenes::stepped::Stepper;

/// ASCII-only so every glyph is exactly one monospaced cell wide.
const GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@#$%&*+=-<>?/\\|{}[]()";

#[derive(Clone)]
struct Column {
    head: f64,  // row index of the leading glyph (may be < 0)
    speed: f64, // rows per second
    trail: i64, // trail length in rows
    active: bool,
    glyphs: Vec<u8>,
}

pub struct MatrixScene {
    width: usize,
    height: usize,
    base_color: RgbColor,
    columns: Vec<Column>,
    rng: SeededRng,
    speed: f64,
    density: f64,
    stepper: Stepper,
}

impl MatrixScene {
    pub fn new() -> Self {
        MatrixScene {
            width: 10,
            height: 10,
            // Default mirrors SteppedScene's initial baseColor (bright green).
            base_color: RgbColor::new(0, 255, 65),
            columns: Vec::new(),
            rng: SeededRng::new(0x5EED_1234),
            speed: 16.0,
            density: 0.72,
            stepper: Stepper::new(),
        }
    }

    fn step_interval(&self) -> f64 {
        1.0 / 60.0
    }

    fn make_column(&mut self, spawn_above: bool) -> Column {
        let h = self.height;
        let lo = (h / 6).max(4) as i64;
        let hi = ((h * 2) / 3).max(6) as i64;
        let trail = self.rng.next_range_inclusive(lo, hi);
        let speed = self.speed * self.rng.next_range_f64(0.6, 1.3);
        let start = if spawn_above {
            -self.rng.next_range_f64(0.0, h.max(1) as f64)
        } else {
            0.0
        };
        let glyphs = (0..h.max(1)).map(|_| *self.rng.choose(GLYPHS)).collect();
        Column { head: start, speed, trail, active: false, glyphs }
    }

    fn reset(&mut self) {
        let (w, h) = (self.width, self.height);
        self.rng = SeededRng::new(
            0x5EED_1234 ^ (w as u64).wrapping_mul(2654435761).wrapping_add(h as u64),
        );
        self.columns = (0..w).map(|_| self.make_column(true)).collect();
        // Stagger initial activation so the screen fills in rather than all at once.
        let density = self.density;
        for i in 0..self.columns.len() {
            let active = self.rng.next_f64() < density;
            self.columns[i].active = active;
            if active {
                self.columns[i].head = self.rng.next_range_f64(0.0, h.max(1) as f64);
            }
        }
    }

    fn step(&mut self) {
        let dt = self.step_interval();
        let h = self.height;
        let density = self.density;
        for i in 0..self.columns.len() {
            if !self.columns[i].active {
                // Re-activate idle columns to drift toward the density target.
                if self.rng.next_f64() < density * 0.02 {
                    let mut col = self.make_column(true);
                    col.active = true;
                    self.columns[i] = col;
                }
                continue;
            }
            self.columns[i].head += self.columns[i].speed * dt;
            // Shimmer: occasionally mutate a glyph in the visible trail.
            if self.rng.next_f64() < 0.10 && h > 0 {
                let r = self.rng.next_below(h);
                let g = *self.rng.choose(GLYPHS);
                self.columns[i].glyphs[r] = g;
            }
            if self.columns[i].head - self.columns[i].trail as f64 > h as f64 {
                let mut col = self.make_column(true);
                col.active = self.rng.next_f64() < density;
                self.columns[i] = col;
            }
        }
    }

    fn render(&self) -> Frame {
        let (w, h) = (self.width, self.height);
        let mut frame = Frame::blank(w, h);
        let head_color = RgbColor::WHITE.mixed(self.base_color, 0.30);

        for (x, col) in self.columns.iter().enumerate() {
            if !col.active {
                continue;
            }
            let head_row = col.head.floor() as i64;
            for d in 0..col.trail {
                let row = head_row - d;
                if row < 0 || row >= h as i64 {
                    continue;
                }
                let idx = row as usize * w + x;
                let glyph = col.glyphs[row as usize % col.glyphs.len()];
                let color = if d == 0 {
                    head_color
                } else {
                    let brightness = (1.0 - d as f64 / col.trail as f64).max(0.06);
                    self.base_color.scaled(brightness)
                };
                frame.cells[idx] = Cell::new(glyph as char, Some(color));
            }
        }
        frame
    }
}

impl Default for MatrixScene {
    fn default() -> Self {
        MatrixScene::new()
    }
}

impl Scene for MatrixScene {
    fn display_name(&self) -> &str {
        "Matrix"
    }

    fn set_grid(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        if width == self.width && height == self.height && !self.columns.is_empty() {
            return;
        }
        self.width = width;
        self.height = height;
        self.reset();
    }

    fn frame(&mut self, t: f64) -> Frame {
        if self.columns.len() != self.width {
            self.reset();
        }
        let steps = self.stepper.advance(t, self.step_interval());
        for _ in 0..steps {
            self.step();
        }
        self.render()
    }

    fn apply_base_color(&mut self, color: RgbColor) {
        self.base_color = color;
    }

    fn settings(&self) -> Vec<SceneSetting> {
        vec![
            SceneSetting::new(
                "speed",
                "Speed",
                vec![
                    SceneOption::new("Slow", 9.0),
                    SceneOption::new("Normal", 16.0),
                    SceneOption::new("Fast", 26.0),
                ],
                1,
            ),
            SceneSetting::new(
                "density",
                "Density",
                vec![
                    SceneOption::new("Sparse", 0.45),
                    SceneOption::new("Normal", 0.72),
                    SceneOption::new("Dense", 0.95),
                ],
                1,
            ),
        ]
    }

    fn apply_setting(&mut self, id: &str, value: f64) {
        match id {
            "speed" => self.speed = value,
            "density" => self.density = value,
            _ => {}
        }
    }

    fn start(&mut self) {
        self.stepper.reset();
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_correct_dimensions() {
        let mut s = MatrixScene::new();
        s.set_grid(40, 20);
        let f = s.frame(1.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn renders_some_glyphs() {
        let mut s = MatrixScene::new();
        s.set_grid(60, 30);
        s.start();
        let mut f = s.frame(0.0);
        for i in 1..=60 {
            f = s.frame(i as f64 / 30.0);
        }
        let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
        assert!(non_blank > 30, "rain should fill a chunk of the grid");
    }

    #[test]
    fn keys_colour_off_base() {
        let mut s = MatrixScene::new();
        s.set_grid(40, 20);
        s.apply_base_color(RgbColor::new(255, 166, 0)); // amber
        s.start();
        let mut f = s.frame(0.0);
        for i in 1..=60 {
            f = s.frame(i as f64 / 30.0);
        }
        // Some trail cell should be a scaled amber (more red+green than blue).
        let tinted = f
            .cells
            .iter()
            .filter_map(|c| c.color)
            .any(|c| c.r > c.b && c.g > c.b);
        assert!(tinted, "matrix should tint trails with the base colour");
    }

    #[test]
    fn deterministic_for_fixed_seed_and_times() {
        let times: Vec<f64> = (0..=90).map(|i| i as f64 / 30.0).collect();
        let run = || {
            let mut s = MatrixScene::new();
            s.set_grid(50, 25);
            s.start();
            let mut last = String::new();
            for &t in &times {
                last = s.frame(t).text();
            }
            last
        };
        assert_eq!(run(), run());
    }
}

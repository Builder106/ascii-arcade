//! The classic "Doom fire" cellular effect (after Fabien Sanglard's writeup).
//!
//! Port of `FireScene.swift`. The bottom row is held at maximum heat and each
//! cell above cools from the cell below it with a little random lateral drift.
//! Heat maps to both a glyph (density ramp) and a black→red→orange→yellow→white
//! palette. A colour scene: every lit cell carries an explicit [`RgbColor`].
//!
//! Stateful, so it seeds from [`SeededRng`] and drives a fixed-timestep
//! simulation via [`Stepper`]; the palette is fixed (independent of the theme).

use crate::color::RgbColor;
use crate::frame::{Cell, Frame};
use crate::rng::SeededRng;
use crate::scene::{Scene, SceneOption, SceneSetting};
use crate::scenes::stepped::Stepper;

const MAX_HEAT: i32 = 36;
const RAMP: &[u8] = b" ..::--==+++***###%%@@";

pub struct FireScene {
    width: usize,
    height: usize,
    heat: Vec<i32>, // row-major, 0..=MAX_HEAT
    palette: Vec<RgbColor>,
    rng: SeededRng,
    intensity: f64,
    wind: f64,
    stepper: Stepper,
}

impl FireScene {
    pub fn new() -> Self {
        FireScene {
            width: 10,
            height: 10,
            heat: Vec::new(),
            palette: build_palette(),
            rng: SeededRng::new(0xF15E_0FEE),
            intensity: 1.0,
            wind: 0.0,
            stepper: Stepper::new(),
        }
    }

    fn step_interval(&self) -> f64 {
        1.0 / 30.0
    }

    fn intensity(&self) -> i32 {
        self.intensity as i32
    }

    fn wind(&self) -> i32 {
        self.wind as i32
    }

    /// Seed value held on the bottom row, per intensity.
    fn bottom_seed(&self) -> i32 {
        if self.intensity() == 0 {
            28
        } else {
            MAX_HEAT
        }
    }

    /// Cooling constant, per intensity.
    fn cooling(&self) -> i32 {
        match self.intensity() {
            0 => 2,
            1 => 1,
            _ => 0,
        }
    }

    fn reset(&mut self) {
        self.heat = vec![0; self.width * self.height];
        self.ignite_bottom_row();
    }

    fn ignite_bottom_row(&mut self) {
        if self.height == 0 {
            return;
        }
        let base = (self.height - 1) * self.width;
        let seed = self.bottom_seed();
        for x in 0..self.width {
            self.heat[base + x] = seed;
        }
    }

    fn step(&mut self) {
        self.ignite_bottom_row();
        let (w, h) = (self.width, self.height);
        let size = (w * h) as i32;
        let wind = self.wind();
        let cooling = self.cooling();
        for x in 0..w {
            for y in 1..h {
                let src = y * w + x;
                let pixel = self.heat[src];
                if pixel <= 0 {
                    self.heat[src - w] = 0;
                    continue;
                }
                let rand = self.rng.next_range_inclusive(0, 3) as i32;
                let mut dst = src as i32 - rand + 1 + wind - w as i32;
                if dst < 0 {
                    dst = src as i32 - w as i32;
                }
                if dst >= size {
                    dst = size - 1;
                }
                let decay = (rand & 1) + cooling;
                self.heat[dst as usize] = (pixel - decay).max(0);
            }
        }
    }

    fn render(&self) -> Frame {
        let (w, h) = (self.width, self.height);
        let mut frame = Frame::blank(w, h);
        for i in 0..(w * h) {
            let heat = self.heat[i];
            if heat <= 0 {
                continue;
            }
            let ramp_idx =
                ((heat as usize) * (RAMP.len() - 1) / MAX_HEAT as usize).min(RAMP.len() - 1);
            let color = self.palette[(heat as usize).min(self.palette.len() - 1)];
            frame.cells[i] = Cell::new(RAMP[ramp_idx] as char, Some(color));
        }
        frame
    }
}

impl Default for FireScene {
    fn default() -> Self {
        FireScene::new()
    }
}

impl Scene for FireScene {
    fn display_name(&self) -> &str {
        "Fire"
    }

    fn set_grid(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        if width == self.width && height == self.height && !self.heat.is_empty() {
            return;
        }
        self.width = width;
        self.height = height;
        self.reset();
    }

    fn frame(&mut self, t: f64) -> Frame {
        if self.heat.len() != self.width * self.height {
            self.reset();
        }
        let steps = self.stepper.advance(t, self.step_interval());
        for _ in 0..steps {
            self.step();
        }
        self.render()
    }

    fn settings(&self) -> Vec<SceneSetting> {
        vec![
            SceneSetting::new(
                "intensity",
                "Intensity",
                vec![
                    SceneOption::new("Calm", 0.0),
                    SceneOption::new("Normal", 1.0),
                    SceneOption::new("Inferno", 2.0),
                ],
                1,
            ),
            SceneSetting::new(
                "wind",
                "Wind",
                vec![
                    SceneOption::new("Left", -1.0),
                    SceneOption::new("None", 0.0),
                    SceneOption::new("Right", 1.0),
                ],
                1,
            ),
        ]
    }

    fn apply_setting(&mut self, id: &str, value: f64) {
        match id {
            "intensity" => self.intensity = value,
            "wind" => self.wind = value,
            _ => {}
        }
    }

    fn start(&mut self) {
        self.stepper.reset();
        self.reset();
    }
}

/// 37-entry black→red→orange→yellow→white gradient indexed by heat.
fn build_palette() -> Vec<RgbColor> {
    let stops: [(f64, RgbColor); 7] = [
        (0.00, RgbColor::new(0, 0, 0)),
        (0.15, RgbColor::new(70, 0, 0)),
        (0.35, RgbColor::new(180, 30, 0)),
        (0.55, RgbColor::new(240, 100, 0)),
        (0.75, RgbColor::new(255, 180, 40)),
        (0.90, RgbColor::new(255, 230, 120)),
        (1.00, RgbColor::new(255, 255, 255)),
    ];
    (0..=MAX_HEAT)
        .map(|h| {
            let t = h as f64 / MAX_HEAT as f64;
            let mut lo = stops[0];
            let mut hi = stops[stops.len() - 1];
            for i in 0..(stops.len() - 1) {
                if t >= stops[i].0 && t <= stops[i + 1].0 {
                    lo = stops[i];
                    hi = stops[i + 1];
                    break;
                }
            }
            let span = hi.0 - lo.0;
            let local = if span > 0.0 { (t - lo.0) / span } else { 0.0 };
            lo.1.mixed(hi.1, local)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_correct_dimensions() {
        let mut s = FireScene::new();
        s.set_grid(40, 20);
        let f = s.frame(1.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn renders_some_glyphs_after_settling() {
        let mut s = FireScene::new();
        s.set_grid(60, 30);
        s.start();
        // Drive a couple of seconds so heat propagates up from the bottom row.
        let mut f = s.frame(0.0);
        for i in 1..=60 {
            f = s.frame(i as f64 / 30.0);
        }
        let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
        assert!(non_blank > 50, "fire should fill a chunk of the grid");
    }

    #[test]
    fn lit_cells_carry_colour() {
        let mut s = FireScene::new();
        s.set_grid(40, 20);
        s.start();
        let mut f = s.frame(0.0);
        for i in 1..=40 {
            f = s.frame(i as f64 / 30.0);
        }
        let coloured = f.cells.iter().filter(|c| c.color.is_some()).count();
        assert!(coloured > 0, "fire is a colour scene");
    }

    #[test]
    fn deterministic_for_fixed_seed_and_times() {
        let times: Vec<f64> = (0..=90).map(|i| i as f64 / 30.0).collect();
        let run = |times: &[f64]| {
            let mut s = FireScene::new();
            s.set_grid(50, 25);
            s.start();
            let mut last = String::new();
            for &t in times {
                last = s.frame(t).text();
            }
            last
        };
        assert_eq!(run(&times), run(&times));
    }
}

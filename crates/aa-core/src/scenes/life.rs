//! Conway's Game of Life on a toroidal (wrap-around) grid.
//!
//! Port of `GameOfLifeScene.swift`. Seeded with classic patterns — glider guns,
//! spaceships, pulsars, methuselahs — rather than random soup (which just decays
//! into a scatter of tiny still-lifes). Cells are drawn as solid blocks on a
//! coarser logical grid scaled up, so the shapes are big enough to read. Live
//! cells are tinted by age (newborns flash bright, then settle to the theme
//! colour, keyed off `base_color`) and the board auto-reseeds when it stalls.
//!
//! Stateful: seeds from [`SeededRng`] and drives a fixed-timestep simulation via
//! [`Stepper`].

use crate::color::RgbColor;
use crate::frame::{Cell, Frame};
use crate::rng::SeededRng;
use crate::scene::{Scene, SceneOption, SceneSetting};
use crate::scenes::stepped::Stepper;

pub struct LifeScene {
    width: usize,
    height: usize,
    base_color: RgbColor,
    // Logical grid (coarser than the pixel grid, scaled up by `cell_size`).
    cols: usize,
    rows: usize,
    alive: Vec<bool>,
    age: Vec<i32>,
    prev1: Vec<bool>, // 1 generation ago
    prev2: Vec<bool>, // 2 generations ago (catches period-2 oscillators)
    stable_steps: i32,
    rng: SeededRng,
    speed: f64,
    size: f64,
    stepper: Stepper,
}

impl LifeScene {
    pub fn new() -> Self {
        LifeScene {
            width: 10,
            height: 10,
            base_color: RgbColor::new(0, 255, 65),
            cols: 0,
            rows: 0,
            alive: Vec::new(),
            age: Vec::new(),
            prev1: Vec::new(),
            prev2: Vec::new(),
            stable_steps: 0,
            rng: SeededRng::new(0x11FE_C0DE),
            speed: 9.0,
            size: 3.0,
            stepper: Stepper::new(),
        }
    }

    fn step_interval(&self) -> f64 {
        1.0 / self.speed.max(1.0)
    }

    fn cell_size(&self) -> usize {
        (self.size as usize).max(1)
    }

    fn recompute_logical(&mut self) {
        let cs = self.cell_size();
        self.cols = (self.width / cs).max(1);
        self.rows = (self.height / cs).max(1);
    }

    fn reset(&mut self) {
        self.recompute_logical();
        self.seed();
    }

    fn seed(&mut self) {
        let size = self.cols * self.rows;
        self.rng = SeededRng::new(
            self.rng.next_u64()
                ^ ((self.cols as u64).wrapping_mul(73856093)
                    ^ (self.rows as u64).wrapping_mul(19349663)),
        );
        self.alive = vec![false; size];
        self.age = vec![0; size];
        self.prev1 = self.alive.clone();
        self.prev2 = self.alive.clone();
        self.stable_steps = 0;

        let area = size;
        self.stamp((area / 180).max(6), &[&GLIDER]);
        self.stamp((area / 450).max(3), &[&LWSS]);
        let mut oscillators: Vec<&[(i64, i64)]> = vec![&BLINKER, &TOAD, &BEACON];
        if self.cols >= 16 && self.rows >= 16 {
            oscillators.push(&PULSAR);
        }
        self.stamp((area / 700).max(2), &oscillators);
        self.stamp((area / 1400).max(1), &[&ACORN, &R_PENTOMINO]);
        if self.cols >= 40 && self.rows >= 12 {
            self.stamp((1 + area / 2500).min(2), &[&GOSPER_GUN]);
        }
    }

    fn stamp(&mut self, count: usize, pool: &[&[(i64, i64)]]) {
        for _ in 0..count {
            let pick = self.rng.next_below(pool.len());
            let pattern = self.oriented(pool[pick]);
            let ox = self.rng.next_below(self.cols) as i64;
            let oy = self.rng.next_below(self.rows) as i64;
            for (x, y) in pattern {
                let gx = (((ox + x) % self.cols as i64) + self.cols as i64) % self.cols as i64;
                let gy = (((oy + y) % self.rows as i64) + self.rows as i64) % self.rows as i64;
                self.alive[gy as usize * self.cols + gx as usize] = true;
            }
        }
    }

    /// Randomly rotate (0/90/180/270) and optionally mirror a pattern, then
    /// normalise it to the origin, so the same template appears in many guises.
    fn oriented(&mut self, cells: &[(i64, i64)]) -> Vec<(i64, i64)> {
        let mut out: Vec<(i64, i64)> = cells.to_vec();
        if self.rng.next_bool() {
            out = out.iter().map(|&(x, y)| (-x, y)).collect();
        }
        let rotations = self.rng.next_below(4);
        for _ in 0..rotations {
            out = out.iter().map(|&(x, y)| (-y, x)).collect();
        }
        let min_x = out.iter().map(|&(x, _)| x).min().unwrap_or(0);
        let min_y = out.iter().map(|&(_, y)| y).min().unwrap_or(0);
        out.iter().map(|&(x, y)| (x - min_x, y - min_y)).collect()
    }

    fn step(&mut self) {
        let size = self.cols * self.rows;
        if size == 0 || self.alive.len() != size {
            return;
        }
        let (cols, rows) = (self.cols, self.rows);
        let mut next = vec![false; size];
        let mut next_age = vec![0i32; size];
        for y in 0..rows {
            let y_up = (y + rows - 1) % rows;
            let y_dn = (y + 1) % rows;
            for x in 0..cols {
                let x_l = (x + cols - 1) % cols;
                let x_r = (x + 1) % cols;
                let mut n = 0;
                if self.alive[y_up * cols + x_l] {
                    n += 1;
                }
                if self.alive[y_up * cols + x] {
                    n += 1;
                }
                if self.alive[y_up * cols + x_r] {
                    n += 1;
                }
                if self.alive[y * cols + x_l] {
                    n += 1;
                }
                if self.alive[y * cols + x_r] {
                    n += 1;
                }
                if self.alive[y_dn * cols + x_l] {
                    n += 1;
                }
                if self.alive[y_dn * cols + x] {
                    n += 1;
                }
                if self.alive[y_dn * cols + x_r] {
                    n += 1;
                }
                let i = y * cols + x;
                let live = if self.alive[i] {
                    n == 2 || n == 3
                } else {
                    n == 3
                };
                next[i] = live;
                if live {
                    next_age[i] = if self.alive[i] {
                        (self.age[i] + 1).min(999)
                    } else {
                        0
                    };
                }
            }
        }

        // Reseed if the board emptied or settled into a fixed/period-2 pattern.
        let population = next.iter().filter(|&&b| b).count();
        if population == 0 {
            self.seed();
            return;
        }
        if next == self.prev1 || next == self.prev2 {
            self.stable_steps += 1;
        } else {
            self.stable_steps = 0;
        }
        self.prev2 = std::mem::take(&mut self.prev1);
        self.prev1 = std::mem::replace(&mut self.alive, next);
        self.age = next_age;
        if self.stable_steps > 8 {
            self.seed();
        }
    }

    fn render(&self) -> Frame {
        let (w, h) = (self.width, self.height);
        let mut frame = Frame::blank(w, h);
        if self.cols == 0 || self.rows == 0 || self.alive.len() != self.cols * self.rows {
            return frame;
        }
        let young = RgbColor::WHITE.mixed(self.base_color, 0.5);
        let s = self.cell_size();
        for ly in 0..self.rows {
            for lx in 0..self.cols {
                if !self.alive[ly * self.cols + lx] {
                    continue;
                }
                let age = self.age[ly * self.cols + lx];
                let color = if age == 0 {
                    young
                } else {
                    self.base_color.scaled((1.0 - age as f64 * 0.05).max(0.45))
                };
                // Paint the cell_size × cell_size block this logical cell covers.
                for by in 0..s {
                    let py = ly * s + by;
                    if py >= h {
                        break;
                    }
                    let row_base = py * w;
                    for bx in 0..s {
                        let px = lx * s + bx;
                        if px >= w {
                            break;
                        }
                        frame.cells[row_base + px] = Cell::new('█', Some(color));
                    }
                }
            }
        }
        frame
    }
}

impl Default for LifeScene {
    fn default() -> Self {
        LifeScene::new()
    }
}

impl Scene for LifeScene {
    fn display_name(&self) -> &str {
        "Life"
    }

    fn set_grid(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        if width == self.width && height == self.height && !self.alive.is_empty() {
            return;
        }
        self.width = width;
        self.height = height;
        self.reset();
    }

    fn frame(&mut self, t: f64) -> Frame {
        if self.alive.is_empty() {
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
                    SceneOption::new("Slow", 4.0),
                    SceneOption::new("Normal", 9.0),
                    SceneOption::new("Fast", 16.0),
                ],
                1,
            ),
            SceneSetting::new(
                "size",
                "Cell size",
                vec![
                    SceneOption::new("Small", 2.0),
                    SceneOption::new("Medium", 3.0),
                    SceneOption::new("Large", 4.0),
                ],
                1,
            ),
        ]
    }

    fn apply_setting(&mut self, id: &str, value: f64) {
        match id {
            "speed" => self.speed = value,
            "size" => {
                // settingsChanged(): a cell-size change resizes the logical grid
                // → reseed; otherwise leave the running board alone.
                self.size = value;
                let (oc, or) = (self.cols, self.rows);
                self.recompute_logical();
                if self.cols != oc || self.rows != or {
                    self.seed();
                }
            }
            _ => {}
        }
    }

    fn start(&mut self) {
        self.stepper.reset();
        self.reset();
    }
}

// Classic Life patterns as `(x, y)` cell offsets.
const GLIDER: [(i64, i64); 5] = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
const BLINKER: [(i64, i64); 3] = [(0, 0), (1, 0), (2, 0)];
const TOAD: [(i64, i64); 6] = [(1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)];
const BEACON: [(i64, i64); 8] =
    [(0, 0), (1, 0), (0, 1), (1, 1), (2, 2), (3, 2), (2, 3), (3, 3)];
const R_PENTOMINO: [(i64, i64); 5] = [(1, 0), (2, 0), (0, 1), (1, 1), (1, 2)];
const ACORN: [(i64, i64); 7] = [(1, 0), (3, 1), (0, 2), (1, 2), (4, 2), (5, 2), (6, 2)];

/// Lightweight spaceship (travels across the torus).
const LWSS: [(i64, i64); 9] = [
    (0, 0),
    (3, 0),
    (4, 1),
    (0, 2),
    (4, 2),
    (1, 3),
    (2, 3),
    (3, 3),
    (4, 3),
];

/// Period-3 pulsar — a big, instantly recognisable flashing shape.
const PULSAR: [(i64, i64); 48] = [
    // bars at y = 0, 5, 7, 12 for x ∈ {2,3,4,8,9,10}
    (2, 0),
    (3, 0),
    (4, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (2, 5),
    (3, 5),
    (4, 5),
    (8, 5),
    (9, 5),
    (10, 5),
    (2, 7),
    (3, 7),
    (4, 7),
    (8, 7),
    (9, 7),
    (10, 7),
    (2, 12),
    (3, 12),
    (4, 12),
    (8, 12),
    (9, 12),
    (10, 12),
    // posts at x = 0, 5, 7, 12 for y ∈ {2,3,4,8,9,10}
    (0, 2),
    (5, 2),
    (7, 2),
    (12, 2),
    (0, 3),
    (5, 3),
    (7, 3),
    (12, 3),
    (0, 4),
    (5, 4),
    (7, 4),
    (12, 4),
    (0, 8),
    (5, 8),
    (7, 8),
    (12, 8),
    (0, 9),
    (5, 9),
    (7, 9),
    (12, 9),
    (0, 10),
    (5, 10),
    (7, 10),
    (12, 10),
];

/// Gosper glider gun — continuously emits gliders.
const GOSPER_GUN: [(i64, i64); 36] = [
    (0, 4),
    (0, 5),
    (1, 4),
    (1, 5),
    (10, 4),
    (10, 5),
    (10, 6),
    (11, 3),
    (11, 7),
    (12, 2),
    (12, 8),
    (13, 2),
    (13, 8),
    (14, 5),
    (15, 3),
    (15, 7),
    (16, 4),
    (16, 5),
    (16, 6),
    (17, 5),
    (20, 2),
    (20, 3),
    (20, 4),
    (21, 2),
    (21, 3),
    (21, 4),
    (22, 1),
    (22, 5),
    (24, 0),
    (24, 1),
    (24, 5),
    (24, 6),
    (34, 2),
    (34, 3),
    (35, 2),
    (35, 3),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_correct_dimensions() {
        let mut s = LifeScene::new();
        s.set_grid(40, 20);
        let f = s.frame(1.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn seeds_some_live_cells() {
        let mut s = LifeScene::new();
        s.set_grid(80, 40);
        s.start();
        let f = s.frame(0.0);
        let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
        assert!(non_blank > 0, "life should seed classic patterns");
    }

    #[test]
    fn keys_colour_off_base() {
        let mut s = LifeScene::new();
        s.set_grid(60, 30);
        s.apply_base_color(RgbColor::new(255, 166, 0)); // amber
        s.start();
        let f = s.frame(0.0);
        let tinted = f
            .cells
            .iter()
            .filter_map(|c| c.color)
            .any(|c| c.r > 0 && c.g > 0);
        assert!(tinted, "life cells carry the base colour");
    }

    #[test]
    fn deterministic_for_fixed_seed_and_times() {
        let times: Vec<f64> = (0..=60).map(|i| i as f64 / 9.0).collect();
        let run = || {
            let mut s = LifeScene::new();
            s.set_grid(60, 30);
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

//! The old "pipes" screensaver in ASCII.
//!
//! Port of `PipesScene.swift`. Several pipes wander the grid drawing box-drawing
//! segments and corners in their own colour, turning at random and at the edges.
//! When the board fills past a threshold it clears and respawns — an endless,
//! ever-changing weave. A colour scene: each pipe carries a distinct HSV hue, so
//! it does *not* key off the theme.
//!
//! Stateful: seeds from [`SeededRng`] and drives a fixed-timestep simulation via
//! [`Stepper`].

use crate::color::RgbColor;
use crate::frame::{Cell, Frame};
use crate::rng::SeededRng;
use crate::scene::{Scene, SceneOption, SceneSetting};
use crate::scenes::stepped::Stepper;

// Directions: 0 = up, 1 = right, 2 = down, 3 = left.
const DX: [i64; 4] = [0, 1, 0, -1];
const DY: [i64; 4] = [-1, 0, 1, 0];

#[derive(Clone, Copy)]
struct Pipe {
    x: i64,
    y: i64,
    dir: usize,
    color: RgbColor,
}

pub struct PipesScene {
    width: usize,
    height: usize,
    grid: Vec<char>,
    colors: Vec<Option<RgbColor>>,
    pipes: Vec<Pipe>,
    filled: usize,
    hue_cursor: u32,
    rng: SeededRng,
    speed: f64,
    pipe_count: f64,
    stepper: Stepper,
}

impl PipesScene {
    pub fn new() -> Self {
        PipesScene {
            width: 10,
            height: 10,
            grid: Vec::new(),
            colors: Vec::new(),
            pipes: Vec::new(),
            filled: 0,
            hue_cursor: 0,
            rng: SeededRng::new(0xC0FFEE),
            speed: 22.0,
            pipe_count: 4.0,
            stepper: Stepper::new(),
        }
    }

    fn step_interval(&self) -> f64 {
        1.0 / self.speed.max(1.0)
    }

    fn pipe_count(&self) -> usize {
        self.pipe_count as usize
    }

    fn reset(&mut self) {
        let size = self.width * self.height;
        self.rng = SeededRng::new(0xC0FFEE ^ (size as u64).wrapping_mul(2654435761));
        self.grid = vec![' '; size];
        self.colors = vec![None; size];
        self.filled = 0;
        let n = self.pipe_count();
        self.pipes = (0..n).map(|_| self.spawn_pipe(None)).collect();
    }

    fn in_bounds(&self, x: i64, y: i64) -> bool {
        x >= 0 && x < self.width as i64 && y >= 0 && y < self.height as i64
    }

    fn spawn_pipe(&mut self, color: Option<RgbColor>) -> Pipe {
        let c = color.unwrap_or_else(|| self.next_hue());
        let x = self.rng.next_below(self.width.max(1)) as i64;
        let y = self.rng.next_below(self.height.max(1)) as i64;
        let dir = self.rng.next_below(4);
        Pipe { x, y, dir, color: c }
    }

    fn next_hue(&mut self) -> RgbColor {
        let hue = self.hue_cursor as f64 * 47.0;
        self.hue_cursor += 1;
        hsv(hue.rem_euclid(360.0), 0.65, 1.0)
    }

    fn step(&mut self) {
        let size = self.width * self.height;
        if size == 0 || self.grid.len() != size {
            return;
        }
        for i in 0..self.pipes.len() {
            let mut pipe = self.pipes[i];
            self.advance(&mut pipe);
            self.pipes[i] = pipe;
        }
        if self.filled > (size * 55) / 100 {
            self.grid = vec![' '; size];
            self.colors = vec![None; size];
            self.filled = 0;
            let n = self.pipe_count();
            self.pipes = (0..n).map(|_| self.spawn_pipe(None)).collect();
        }
    }

    fn advance(&mut self, pipe: &mut Pipe) {
        // Pick the next heading: mostly continue, sometimes turn; always choose a
        // direction that keeps the pipe on the grid.
        let turning = self.rng.next_f64() < 0.18;
        let preferred = if turning {
            if self.rng.next_bool() {
                (pipe.dir + 1) % 4
            } else {
                (pipe.dir + 3) % 4
            }
        } else {
            pipe.dir
        };
        let candidates = [
            preferred,
            (pipe.dir + 1) % 4,
            (pipe.dir + 3) % 4,
            pipe.dir,
            (pipe.dir + 2) % 4,
        ];
        let mut new_dir = pipe.dir;
        for &c in &candidates {
            if self.in_bounds(pipe.x + DX[c], pipe.y + DY[c]) {
                new_dir = c;
                break;
            }
        }

        // Draw the connector at the current cell joining where we came from
        // (opposite of current heading) to where we're going (new_dir).
        let incoming = (pipe.dir + 2) % 4;
        let glyph = connector(incoming, new_dir);
        let idx = pipe.y as usize * self.width + pipe.x as usize;
        if self.grid[idx] == ' ' {
            self.filled += 1;
        }
        self.grid[idx] = glyph;
        self.colors[idx] = Some(pipe.color);

        let nx = pipe.x + DX[new_dir];
        let ny = pipe.y + DY[new_dir];
        if self.in_bounds(nx, ny) {
            pipe.x = nx;
            pipe.y = ny;
        } else {
            // Boxed in — wrap to a fresh spot rather than stall.
            *pipe = self.spawn_pipe(Some(pipe.color));
        }
        pipe.dir = new_dir;
    }

    fn render(&self) -> Frame {
        let (w, h) = (self.width, self.height);
        let mut frame = Frame::blank(w, h);
        if self.grid.len() == w * h {
            for i in 0..(w * h) {
                frame.cells[i] = Cell::new(self.grid[i], self.colors[i]);
            }
        }
        frame
    }
}

impl Default for PipesScene {
    fn default() -> Self {
        PipesScene::new()
    }
}

impl Scene for PipesScene {
    fn display_name(&self) -> &str {
        "Pipes"
    }

    fn set_grid(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        if width == self.width && height == self.height && !self.grid.is_empty() {
            return;
        }
        self.width = width;
        self.height = height;
        self.reset();
    }

    fn frame(&mut self, t: f64) -> Frame {
        if self.grid.len() != self.width * self.height {
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
                "speed",
                "Speed",
                vec![
                    SceneOption::new("Slow", 12.0),
                    SceneOption::new("Normal", 22.0),
                    SceneOption::new("Fast", 40.0),
                ],
                1,
            ),
            SceneSetting::new(
                "pipes",
                "Pipes",
                vec![
                    SceneOption::new("Few", 2.0),
                    SceneOption::new("Some", 4.0),
                    SceneOption::new("Many", 8.0),
                ],
                1,
            ),
        ]
    }

    fn apply_setting(&mut self, id: &str, value: f64) {
        match id {
            "speed" => self.speed = value,
            "pipes" => {
                self.pipe_count = value;
                // settingsChanged() reseeds in Swift.
                self.reset();
            }
            _ => {}
        }
    }

    fn start(&mut self) {
        self.stepper.reset();
        self.reset();
    }
}

/// Box-drawing glyph joining two cell edges, given as direction indices
/// (0 = up, 1 = right, 2 = down, 3 = left).
fn connector(a: usize, b: usize) -> char {
    if a == b {
        return if a % 2 == 0 { '│' } else { '─' };
    }
    let (lo, hi) = (a.min(b), a.max(b));
    match (lo, hi) {
        (0, 2) => '│', // up + down
        (1, 3) => '─', // right + left
        (1, 2) => '┌', // right + down
        (2, 3) => '┐', // down + left
        (0, 1) => '└', // up + right
        (0, 3) => '┘', // up + left
        _ => '+',
    }
}

/// HSV → RGB so each pipe gets a distinct, saturated hue.
fn hsv(h: f64, s: f64, v: f64) -> RgbColor {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = if (0.0..1.0).contains(&hp) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&hp) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&hp) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&hp) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&hp) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    RgbColor::new(
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_has_correct_dimensions() {
        let mut s = PipesScene::new();
        s.set_grid(40, 20);
        let f = s.frame(1.0);
        assert_eq!(f.width, 40);
        assert_eq!(f.height, 20);
        assert_eq!(f.cells.len(), 800);
        assert_eq!(f.text().lines().count(), 20);
    }

    #[test]
    fn draws_coloured_segments() {
        let mut s = PipesScene::new();
        s.set_grid(60, 30);
        s.start();
        let mut f = s.frame(0.0);
        for i in 1..=60 {
            f = s.frame(i as f64 / 30.0);
        }
        let non_blank = f.cells.iter().filter(|c| c.ch != ' ').count();
        assert!(non_blank > 10, "pipes should draw a weave of segments");
        let coloured = f.cells.iter().filter(|c| c.color.is_some()).count();
        assert!(coloured > 0, "pipes is a colour scene");
    }

    #[test]
    fn deterministic_for_fixed_seed_and_times() {
        let times: Vec<f64> = (0..=120).map(|i| i as f64 / 30.0).collect();
        let run = || {
            let mut s = PipesScene::new();
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

    #[test]
    fn hsv_primary_red() {
        assert_eq!(hsv(0.0, 1.0, 1.0), RgbColor::new(255, 0, 0));
    }
}
